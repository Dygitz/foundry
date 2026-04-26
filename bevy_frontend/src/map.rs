#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, player::*, rendering::*, resources::*,
    storage::*, ui::*, world::*,
};

pub(crate) fn map_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<WorldRenderConfig>,
    mut ui_state: ResMut<UiState>,
    mut map: ResMut<MapState>,
    player_query: Query<&Transform, With<Player>>,
) {
    if !keys.just_pressed(KeyCode::KeyM) {
        return;
    }
    match ui_state.mode {
        UiMode::Map => {
            ui_state.mode = UiMode::None;
            map.drag_last_cursor = None;
        }
        UiMode::None => {
            if let Ok(transform) = player_query.single() {
                map.full_view.center_tile =
                    world_pos_to_tile_pos(transform.translation.truncate(), &config);
            }
            map.full_view.px_per_tile = FULL_MAP_DEFAULT_PX_PER_TILE;
            map.drag_last_cursor = None;
            ui_state.mode = UiMode::Map;
        }
        _ => {}
    }
}

pub(crate) fn full_map_input_system(
    buttons: Res<ButtonInput<MouseButton>>,
    mut scroll: EventReader<MouseWheel>,
    windows: Query<&Window>,
    ui_state: Res<UiState>,
    mut map: ResMut<MapState>,
) {
    if ui_state.mode != UiMode::Map {
        map.drag_last_cursor = None;
        scroll.clear();
        return;
    }

    for ev in scroll.read() {
        let factor = (1.0 + ev.y * 0.12).clamp(0.7, 1.3);
        map.full_view.px_per_tile = (map.full_view.px_per_tile * factor)
            .clamp(FULL_MAP_MIN_PX_PER_TILE, FULL_MAP_MAX_PX_PER_TILE);
    }

    let Ok(window) = windows.single() else {
        map.drag_last_cursor = None;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        map.drag_last_cursor = None;
        return;
    };

    if buttons.just_pressed(MouseButton::Left) {
        map.drag_last_cursor = Some(cursor);
        return;
    }
    if buttons.pressed(MouseButton::Left) {
        if let Some(previous) = map.drag_last_cursor {
            let delta = cursor - previous;
            map.full_view.center_tile.x -= delta.x / map.full_view.px_per_tile;
            map.full_view.center_tile.y += delta.y / map.full_view.px_per_tile;
        }
        map.drag_last_cursor = Some(cursor);
    } else {
        map.drag_last_cursor = None;
    }
}

pub(crate) fn minimap_visibility_system(
    ui_state: Res<UiState>,
    mut query: Query<&mut Visibility, With<MinimapRoot>>,
) {
    if !ui_state.is_changed() {
        return;
    }
    let visible = if ui_state.mode == UiMode::Map {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut visibility in &mut query {
        *visibility = visible;
    }
}

pub(crate) fn map_load_pump_system(
    init_task: Res<StorageInitTask>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    config: Res<WorldRenderConfig>,
    mut load: ResMut<MapLoadState>,
    mut map: ResMut<MapState>,
    mut images: ResMut<Assets<Image>>,
    mut status: ResMut<StorageStatus>,
) {
    if load.loaded || !init_task.ready {
        return;
    }

    if load.task.is_none() {
        let storage = services.storage.clone();
        let world_id = session.world_id.clone();
        let layer = config.layer;
        load.task = Some(
            AsyncComputeTaskPool::get()
                .spawn_local(async move { storage.load_map_chunks(&world_id, layer).await }),
        );
    }

    let Some(task) = load.task.as_mut() else {
        return;
    };
    let Some(result) = future::block_on(future::poll_once(task)) else {
        return;
    };

    match result {
        Ok(records) => {
            for record in records {
                if record.rgba.len() != MAP_CHUNK_BYTES {
                    status.record_error(&StorageError::DecodeFailed(format!(
                        "map chunk {} has {} bytes, expected {MAP_CHUNK_BYTES}",
                        record.key.to_key_string(),
                        record.rgba.len()
                    )));
                    continue;
                }
                let (resource_kinds, resource_amounts) =
                    normalize_map_resource_metadata(record.resource_kinds, record.resource_amounts);
                let image = images.add(build_map_chunk_image(&record.rgba));
                map.explored.insert(
                    record.key,
                    MapChunk {
                        rgba: record.rgba,
                        resource_kinds,
                        resource_amounts,
                        image,
                        updated_at_ms: record.updated_at_ms,
                    },
                );
            }
            status.mark_ok();
        }
        Err(error) => status.record_error(&error),
    }
    load.task = None;
    load.loaded = true;
}

pub(crate) fn map_save_system(
    time: Res<Time>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut map: ResMut<MapState>,
    mut save: ResMut<MapSaveState>,
    mut status: ResMut<StorageStatus>,
) {
    save.timer.tick(time.delta());

    if let Some(mut save_task) = save.in_flight.take() {
        if let Some(result) = future::block_on(future::poll_once(&mut save_task.task)) {
            match result {
                Ok(()) => {
                    for _ in 0..save_task.pending_count {
                        if let Some(record) = map.pending_saves.pop_front() {
                            map.queued_for_save.remove(&record.key);
                        }
                    }
                    for key in save_task.keys {
                        if map.pending_saves.iter().any(|record| record.key == key) {
                            map.queued_for_save.insert(key);
                        }
                    }
                    status.mark_ok();
                }
                Err(error) => status.record_error(&error),
            }
        } else {
            save.in_flight = Some(save_task);
            return;
        }
    }

    if save.in_flight.is_some()
        || status.state == StorageState::Paused
        || !save.timer.just_finished()
        || map.pending_saves.is_empty()
    {
        return;
    }

    let batch: Vec<MapChunkRecordWrite> = map
        .pending_saves
        .iter()
        .take(save.max_per_flush)
        .cloned()
        .collect();
    if batch.is_empty() {
        return;
    }
    let pending_count = batch.len();
    let keys = batch.iter().map(|record| record.key.clone()).collect();
    let storage = services.storage.clone();
    let world_id = session.world_id.clone();
    save.in_flight = Some(MapSaveTask {
        task: AsyncComputeTaskPool::get()
            .spawn_local(async move { storage.put_map_chunks(&world_id, batch).await }),
        pending_count,
        keys,
    });
}

pub(crate) fn map_ui_render_system(
    mut commands: Commands,
    map: Res<MapState>,
    ui_state: Res<UiState>,
    windows: Query<&Window>,
    config: Res<WorldRenderConfig>,
    session: Res<WorldSession>,
    player_query: Query<&Transform, With<Player>>,
    content_query: Query<(Entity, &MapContent)>,
    children_query: Query<&Children, With<MapContent>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_tile = world_pos_to_tile_pos(player_transform.translation.truncate(), &config);
    let window = windows.single().ok();
    let window_size = window
        .map(|window| Vec2::new(window.width(), window.height()))
        .unwrap_or(Vec2::new(MINIMAP_SIZE, MINIMAP_SIZE));
    let cursor_pos = window.and_then(Window::cursor_position);

    for (entity, content) in &content_query {
        if content.kind == MapSurfaceKind::Minimap && ui_state.mode == UiMode::Map {
            clear_map_content(&mut commands, entity, &children_query);
            continue;
        }
        if content.kind == MapSurfaceKind::Full && ui_state.mode != UiMode::Map {
            clear_map_content(&mut commands, entity, &children_query);
            continue;
        }

        let (viewport, center_tile, px_per_tile, marker_size, surface_origin) = match content.kind {
            MapSurfaceKind::Minimap => (
                Vec2::splat(MINIMAP_SIZE),
                player_tile,
                MINIMAP_PX_PER_TILE,
                5.0,
                Vec2::new(
                    window_size.x - MINIMAP_MARGIN - MINIMAP_OUTER_SIZE + MINIMAP_FRAME,
                    window_size.y - MINIMAP_MARGIN - MINIMAP_OUTER_SIZE + MINIMAP_FRAME,
                ),
            ),
            MapSurfaceKind::Full => (
                window_size,
                map.full_view.center_tile,
                map.full_view.px_per_tile,
                8.0,
                Vec2::ZERO,
            ),
        };
        let hovered_resource = cursor_pos.and_then(|cursor| {
            let local_cursor = cursor - surface_origin;
            if local_cursor.x < 0.0
                || local_cursor.y < 0.0
                || local_cursor.x >= viewport.x
                || local_cursor.y >= viewport.y
            {
                return None;
            }
            let (tile_x, tile_y) =
                map_local_cursor_to_tile(local_cursor, center_tile, px_per_tile, viewport);
            map_resource_node_summary(&map, &session, config.layer, tile_x, tile_y)
                .map(|summary| (local_cursor, summary))
        });

        clear_map_content(&mut commands, entity, &children_query);
        commands.entity(entity).with_children(|parent| {
            spawn_map_chunk_nodes(
                parent,
                &map,
                center_tile,
                px_per_tile,
                viewport,
                content.kind == MapSurfaceKind::Minimap,
            );
            spawn_map_player_marker(
                parent,
                player_tile,
                center_tile,
                px_per_tile,
                viewport,
                marker_size,
            );
            if let Some((local_cursor, summary)) = hovered_resource {
                spawn_map_resource_tooltip(parent, local_cursor, viewport, summary);
            }
        });
    }
}

pub(crate) fn chunk_world_size(config: &WorldRenderConfig) -> f32 {
    config.tile_size * CHUNK_EDGE as f32
}

pub(crate) fn required_radius_chunks(
    viewport: Vec2,
    scale: f32,
    chunk_size: f32,
    margin: i32,
) -> (i32, i32) {
    let half_w = (viewport.x * 0.5) * scale;
    let half_h = (viewport.y * 0.5) * scale;
    let rx = (half_w / chunk_size).ceil() as i32 + margin;
    let ry = (half_h / chunk_size).ceil() as i32 + margin;
    (rx.max(0), ry.max(0))
}

pub(crate) fn world_pos_to_tile_pos(world_pos: Vec2, config: &WorldRenderConfig) -> Vec2 {
    world_pos / config.tile_size
}

pub(crate) fn upsert_map_snapshot(
    map: &mut MapState,
    images: &mut Assets<Image>,
    key: &ChunkKey,
    data: &SimChunkData,
    world_seed: u64,
    updated_at_ms: u64,
) {
    let rgba = map_snapshot_pixels(data, world_seed);
    let (resource_kinds, resource_amounts) = map_resource_metadata(data);
    let changed = match map.explored.get_mut(key) {
        Some(chunk)
            if chunk.rgba == rgba
                && chunk.resource_kinds == resource_kinds
                && chunk.resource_amounts == resource_amounts =>
        {
            false
        }
        Some(chunk) => {
            let rgba_changed = chunk.rgba != rgba;
            chunk.rgba = rgba.clone();
            chunk.resource_kinds = resource_kinds.clone();
            chunk.resource_amounts = resource_amounts.clone();
            chunk.updated_at_ms = updated_at_ms;
            if rgba_changed {
                if let Some(image) = images.get_mut(&chunk.image) {
                    *image = build_map_chunk_image(&rgba);
                } else {
                    chunk.image = images.add(build_map_chunk_image(&rgba));
                }
            }
            true
        }
        None => {
            let image = images.add(build_map_chunk_image(&rgba));
            map.explored.insert(
                key.clone(),
                MapChunk {
                    rgba: rgba.clone(),
                    resource_kinds: resource_kinds.clone(),
                    resource_amounts: resource_amounts.clone(),
                    image,
                    updated_at_ms,
                },
            );
            true
        }
    };

    if changed {
        queue_map_snapshot_save(
            map,
            key.clone(),
            rgba,
            resource_kinds,
            resource_amounts,
            updated_at_ms,
        );
    }
}

pub(crate) fn queue_map_snapshot_save(
    map: &mut MapState,
    key: ChunkKey,
    rgba: Vec<u8>,
    resource_kinds: Vec<ResourceId>,
    resource_amounts: Vec<u16>,
    updated_at_ms: u64,
) {
    if let Some(existing) = map
        .pending_saves
        .iter_mut()
        .find(|record| record.key == key)
    {
        existing.rgba = rgba;
        existing.resource_kinds = resource_kinds;
        existing.resource_amounts = resource_amounts;
        existing.updated_at_ms = updated_at_ms;
    } else {
        map.pending_saves.push_back(MapChunkRecordWrite {
            key: key.clone(),
            rgba,
            resource_kinds,
            resource_amounts,
            updated_at_ms,
        });
    }
    map.queued_for_save.insert(key);
}

pub(crate) fn clear_map_content(
    commands: &mut Commands,
    entity: Entity,
    children_query: &Query<&Children, With<MapContent>>,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
}

pub(crate) fn spawn_map_chunk_nodes(
    parent: &mut ChildSpawnerCommands,
    map: &MapState,
    center_tile: Vec2,
    px_per_tile: f32,
    viewport: Vec2,
    snap_to_pixels: bool,
) {
    let chunk_tiles = CHUNK_EDGE as f32;
    let chunk_px = chunk_tiles * px_per_tile;
    for (key, chunk) in &map.explored {
        let min_x = key.coord.cx as f32 * chunk_tiles;
        let max_y = (key.coord.cy as f32 + 1.0) * chunk_tiles;
        let mut left = viewport.x * 0.5 + (min_x - center_tile.x) * px_per_tile;
        let mut top = viewport.y * 0.5 - (max_y - center_tile.y) * px_per_tile;
        let mut size = chunk_px;
        if snap_to_pixels {
            left = left.round();
            top = top.round();
            size = chunk_px.round();
        }

        if left > viewport.x || top > viewport.y || left + size < 0.0 || top + size < 0.0 {
            continue;
        }

        parent.spawn((
            ImageNode::new(chunk.image.clone()),
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                ..default()
            },
        ));
    }
}

pub(crate) fn spawn_map_player_marker(
    parent: &mut ChildSpawnerCommands,
    player_tile: Vec2,
    center_tile: Vec2,
    px_per_tile: f32,
    viewport: Vec2,
    size: f32,
) {
    let left = viewport.x * 0.5 + (player_tile.x - center_tile.x) * px_per_tile - size * 0.5;
    let top = viewport.y * 0.5 - (player_tile.y - center_tile.y) * px_per_tile - size * 0.5;
    if left > viewport.x || top > viewport.y || left + size < 0.0 || top + size < 0.0 {
        return;
    }
    parent.spawn((
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(1.0, 0.9, 0.25)),
        BorderColor(Color::srgb(0.05, 0.05, 0.05)),
    ));
}

pub(crate) fn spawn_map_resource_tooltip(
    parent: &mut ChildSpawnerCommands,
    cursor: Vec2,
    viewport: Vec2,
    summary: ResourceNodeSummary,
) {
    let max_left = (viewport.x - MAP_TOOLTIP_WIDTH).max(0.0);
    let max_top = (viewport.y - MAP_TOOLTIP_HEIGHT).max(0.0);
    let mut left = cursor.x + MAP_TOOLTIP_OFFSET;
    if left + MAP_TOOLTIP_WIDTH > viewport.x {
        left = cursor.x - MAP_TOOLTIP_WIDTH - MAP_TOOLTIP_OFFSET;
    }
    let mut top = cursor.y + MAP_TOOLTIP_OFFSET;
    if top + MAP_TOOLTIP_HEIGHT > viewport.y {
        top = cursor.y - MAP_TOOLTIP_HEIGHT - MAP_TOOLTIP_OFFSET;
    }
    let label = format!(
        "{}: {} left",
        resource_display_name(summary.kind),
        summary.total
    );

    parent
        .spawn((
            Node {
                width: Val::Px(MAP_TOOLTIP_WIDTH),
                height: Val::Px(MAP_TOOLTIP_HEIGHT),
                position_type: PositionType::Absolute,
                left: Val::Px(left.clamp(0.0, max_left)),
                top: Val::Px(top.clamp(0.0, max_top)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.05, 0.88)),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.35)),
        ))
        .with_children(|tooltip| {
            tooltip.spawn((
                Text::new(label),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.94, 0.84)),
            ));
        });
}

pub(crate) fn map_local_cursor_to_tile(
    cursor: Vec2,
    center_tile: Vec2,
    px_per_tile: f32,
    viewport: Vec2,
) -> (i32, i32) {
    let tile_x = center_tile.x + (cursor.x - viewport.x * 0.5) / px_per_tile;
    let tile_y = center_tile.y + (viewport.y * 0.5 - cursor.y) / px_per_tile;
    (tile_x.floor() as i32, tile_y.floor() as i32)
}

pub(crate) fn map_resource_node_summary(
    map: &MapState,
    session: &WorldSession,
    layer: ChunkLayer,
    tile_x: i32,
    tile_y: i32,
) -> Option<ResourceNodeSummary> {
    let origin = map_resource_at(map, session, layer, tile_x, tile_y)?;
    let mut total = 0u32;
    let mut visited = HashSet::new();
    let mut pending = VecDeque::from([(tile_x, tile_y)]);

    while let Some((x, y)) = pending.pop_front() {
        if !visited.insert((x, y)) {
            continue;
        }
        let Some(cell) = map_resource_at(map, session, layer, x, y) else {
            continue;
        };
        if cell.kind != origin.kind {
            continue;
        }

        total += cell.amount as u32;
        pending.push_back((x + 1, y));
        pending.push_back((x - 1, y));
        pending.push_back((x, y + 1));
        pending.push_back((x, y - 1));
    }

    (total > 0).then_some(ResourceNodeSummary {
        kind: origin.kind,
        total,
    })
}

pub(crate) fn map_resource_at(
    map: &MapState,
    session: &WorldSession,
    layer: ChunkLayer,
    tile_x: i32,
    tile_y: i32,
) -> Option<MapResourceCell> {
    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, layer);
    let chunk = map.explored.get(&key)?;
    let idx = local_y as usize * CHUNK_EDGE as usize + local_x as usize;
    let kind = chunk.resource_kinds.get(idx).copied().unwrap_or(RES_NONE);
    let amount = chunk.resource_amounts.get(idx).copied().unwrap_or(0);

    if kind == RES_NONE || amount == 0 {
        return None;
    }
    Some(MapResourceCell { kind, amount })
}

pub(crate) fn map_resource_metadata(data: &SimChunkData) -> (Vec<ResourceId>, Vec<u16>) {
    let mut resource_kinds = vec![RES_NONE; CHUNK_TILE_COUNT];
    let mut resource_amounts = vec![0; CHUNK_TILE_COUNT];
    for (idx, resource) in data
        .resources
        .iter()
        .copied()
        .take(CHUNK_TILE_COUNT)
        .enumerate()
    {
        if resource.kind != RES_NONE && resource.amount > 0 {
            resource_kinds[idx] = resource.kind;
            resource_amounts[idx] = resource.amount;
        }
    }
    (resource_kinds, resource_amounts)
}

pub(crate) fn normalize_map_resource_metadata(
    resource_kinds: Vec<ResourceId>,
    resource_amounts: Vec<u16>,
) -> (Vec<ResourceId>, Vec<u16>) {
    if resource_kinds.len() == CHUNK_TILE_COUNT && resource_amounts.len() == CHUNK_TILE_COUNT {
        return (resource_kinds, resource_amounts);
    }
    (vec![RES_NONE; CHUNK_TILE_COUNT], vec![0; CHUNK_TILE_COUNT])
}

pub(crate) fn resource_display_name(kind: ResourceId) -> &'static str {
    match kind {
        RES_IRON => "Iron Ore",
        RES_COPPER => "Copper Ore",
        RES_COAL => "Coal",
        RES_STONE => "Stone",
        _ => "Resource",
    }
}

pub(crate) fn build_map_chunk_image(rgba: &[u8]) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: CHUNK_EDGE as u32,
            height: CHUNK_EDGE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub(crate) fn map_snapshot_pixels(data: &SimChunkData, world_seed: u64) -> Vec<u8> {
    let edge = CHUNK_EDGE as usize;
    let mut pixels = Vec::with_capacity(MAP_CHUNK_BYTES);

    for row_y in 0..edge {
        let ty = edge - 1 - row_y;
        for tx in 0..edge {
            let tile = tile_at(data, tx as i32, ty as i32, world_seed);
            let mut color = if tile == WATER_TILE {
                let neighbor_is_land =
                    !is_water(tile_at(data, tx as i32 - 1, ty as i32, world_seed))
                        || !is_water(tile_at(data, tx as i32 + 1, ty as i32, world_seed))
                        || !is_water(tile_at(data, tx as i32, ty as i32 - 1, world_seed))
                        || !is_water(tile_at(data, tx as i32, ty as i32 + 1, world_seed));
                if neighbor_is_land {
                    shallow_water_color()
                } else {
                    tile_color(tile)
                }
            } else {
                tile_color(tile)
            };
            let gx = data.coord.cx * CHUNK_EDGE as i32 + tx as i32;
            let gy = data.coord.cy * CHUNK_EDGE as i32 + ty as i32;
            let jitter = tile_jitter(gx, gy, world_seed, tile);
            color = apply_jitter(color, jitter);
            let resource = resource_at(data, tx as i32, ty as i32);
            if resource.kind != RES_NONE && resource.amount > 0 {
                color = blend_color(color, resource_color(resource.kind), 0.85);
            }
            let placed = placed_at(data, tx as i32, ty as i32);
            if placed.kind != PLACED_NONE {
                color = blend_color(color, placed_color(placed.kind), 0.9);
            }
            pixels.extend_from_slice(&color);
        }
    }

    pixels
}

pub(crate) fn map_unknown_color() -> Color {
    Color::srgb(0.18, 0.18, 0.2)
}

#[cfg(test)]
mod tests;
