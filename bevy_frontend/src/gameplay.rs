#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, map::*, player::*, rendering::*, resources::*, storage::*,
    ui::*, world::*,
};

pub(crate) struct FoundryGameplayPlugin;

impl Plugin for FoundryGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                placement_select_system,
                hotbar_input_system,
                structure_pickup_input_system,
            )
                .in_set(UpdateSet::Input),
        )
        .add_systems(Update, (mining_input_system,).in_set(UpdateSet::Input))
        .add_systems(Update, (placement_preview_system,).in_set(UpdateSet::World))
        .add_systems(
            Update,
            (
                chest_button_system,
                furnace_button_system,
                drill_button_system,
                inserter_button_system,
            )
                .in_set(UpdateSet::Ui),
        )
        .add_systems(
            Update,
            (
                furnace_smelting_system,
                mining_drill_system,
                inserter_transfer_system,
            )
                .chain()
                .in_set(UpdateSet::World),
        );
    }
}

pub(crate) fn placement_select_system(
    keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    player: Res<PlayerState>,
    mut placement: ResMut<PlacementState>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
    if keys.just_pressed(KeyCode::KeyF) && player.inventory.count(ITEM_FURNACE) > 0 {
        placement.selected = Some(ITEM_FURNACE);
    }
    if keys.just_pressed(KeyCode::KeyC) && player.inventory.count(ITEM_CHEST) > 0 {
        placement.selected = Some(ITEM_CHEST);
    }
    if keys.just_pressed(KeyCode::KeyI) && player.inventory.count(ITEM_INSERTER) > 0 {
        placement.selected = Some(ITEM_INSERTER);
    }
    if keys.just_pressed(KeyCode::KeyD) && player.inventory.count(ITEM_MINING_DRILL) > 0 {
        placement.selected = Some(ITEM_MINING_DRILL);
    }
    if keys.just_pressed(KeyCode::KeyR) && placement.selected == Some(ITEM_INSERTER) {
        placement.inserter_direction = placement.inserter_direction.next_clockwise();
    }
    if keys.just_pressed(KeyCode::Escape) {
        placement.selected = None;
    }
}

pub(crate) fn hotbar_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    player: Res<PlayerState>,
    mut hotbar: ResMut<HotbarState>,
    mut placement: ResMut<PlacementState>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }

    for (index, key) in HOTBAR_KEYS.into_iter().enumerate() {
        if keys.just_pressed(key) {
            select_hotbar_slot(index, &player.inventory, &mut hotbar, &mut placement);
            break;
        }
    }
}

pub(crate) fn placement_preview_system(
    mut commands: Commands,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    placement: Res<PlacementState>,
    ui_state: Res<UiState>,
    player: Res<PlayerState>,
    config: Res<WorldRenderConfig>,
    session: Res<WorldSession>,
    runtime: Res<WorldRuntime>,
    preview_assets: Res<PlacementPreviewAssets>,
    hotbar_interactions: Query<&Interaction, With<HotbarSlotButton>>,
    mut preview_query: Query<
        (&mut Sprite, &mut Transform, &mut Visibility),
        With<PlacementPreview>,
    >,
) {
    let Some(item) = placement.selected else {
        hide_placement_preview(&mut preview_query);
        return;
    };
    if ui_state.mode != UiMode::None
        || player.inventory.count(item) == 0
        || !is_placeable_item(item)
        || hotbar_interactions
            .iter()
            .any(|interaction| *interaction != Interaction::None)
    {
        hide_placement_preview(&mut preview_query);
        return;
    }

    let Ok(window) = windows.single() else {
        hide_placement_preview(&mut preview_query);
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        hide_placement_preview(&mut preview_query);
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        hide_placement_preview(&mut preview_query);
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        hide_placement_preview(&mut preview_query);
        return;
    };

    let tile_x = (world_pos.x / config.tile_size).floor() as i32;
    let tile_y = (world_pos.y / config.tile_size).floor() as i32;
    let center = Vec3::new(
        (tile_x as f32 + 0.5) * config.tile_size,
        (tile_y as f32 + 0.5) * config.tile_size,
        8.0,
    );
    let can_place = can_place_tile(tile_x, tile_y, item, &config, &session, &runtime, &player);
    let color = if can_place {
        Color::srgba(1.0, 1.0, 1.0, 0.68)
    } else {
        Color::srgba(1.0, 0.42, 0.42, 0.62)
    };
    let Some(image) = preview_assets.for_item(item) else {
        hide_placement_preview(&mut preview_query);
        return;
    };
    let size = Some(Vec2::splat(config.tile_size));

    if let Some((mut sprite, mut transform, mut visibility)) = preview_query.iter_mut().next() {
        sprite.image = image;
        sprite.custom_size = size;
        sprite.color = color;
        transform.translation = center;
        *visibility = Visibility::Visible;
    } else {
        commands.spawn((
            Sprite {
                image,
                custom_size: size,
                color,
                ..default()
            },
            Transform::from_translation(center),
            Visibility::Visible,
            PlacementPreview,
        ));
    }
}

fn hide_placement_preview(
    preview_query: &mut Query<
        (&mut Sprite, &mut Transform, &mut Visibility),
        With<PlacementPreview>,
    >,
) {
    for (_, _, mut visibility) in preview_query.iter_mut() {
        *visibility = Visibility::Hidden;
    }
}

#[derive(SystemParam)]
pub(crate) struct MiningParams<'w, 's> {
    config: Res<'w, WorldRenderConfig>,
    session: Res<'w, WorldSession>,
    runtime: ResMut<'w, WorldRuntime>,
    player: ResMut<'w, PlayerState>,
    player_query: Query<'w, 's, &'static Transform, With<Player>>,
    hotbar_interactions: Query<'w, 's, &'static Interaction, With<HotbarSlotButton>>,
    placement: ResMut<'w, PlacementState>,
    ui_state: ResMut<'w, UiState>,
    images: ResMut<'w, Assets<Image>>,
    services: NonSend<'w, StorageServices>,
    time: Res<'w, Time>,
    queue: ResMut<'w, SaveQueue>,
    status: ResMut<'w, StorageStatus>,
    debug: Res<'w, DebugConfig>,
    highlight: ResMut<'w, ClickHighlight>,
    map: ResMut<'w, MapState>,
    pickup_notices: EventWriter<'w, PickupNotice>,
}

pub(crate) fn mining_input_system(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut params: MiningParams,
) {
    let log_mining = params.debug.log_mining;
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    if params.ui_state.mode != UiMode::None {
        return;
    }
    if params
        .hotbar_interactions
        .iter()
        .any(|interaction| *interaction != Interaction::None)
    {
        return;
    }
    if log_mining {
        info!("mine click");
    }

    let Ok(window) = windows.single() else {
        if log_mining {
            warn!("mine click: missing window");
        }
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        if log_mining {
            warn!("mine click: no cursor position");
        }
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        if log_mining {
            warn!("mine click: missing camera");
        }
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        if log_mining {
            warn!("mine click: viewport_to_world_2d failed");
        }
        return;
    };

    let tile_x = (world_pos.x / params.config.tile_size).floor() as i32;
    let tile_y = (world_pos.y / params.config.tile_size).floor() as i32;
    if params.placement.selected.is_none() {
        if let Some(open_panel) = open_panel_for_tile(
            tile_x,
            tile_y,
            &params.config,
            &params.session,
            &params.runtime,
        ) {
            params.ui_state.mode = open_panel;
            return;
        }
    }
    let previous_highlight = params.highlight.tile;
    params.highlight.tile = Some((tile_x, tile_y));
    if let Some((prev_x, prev_y)) = previous_highlight {
        refresh_highlight_chunk(
            prev_x,
            prev_y,
            &params.config,
            &params.session,
            &params.runtime,
            &mut params.images,
            params.highlight.tile,
        );
    }
    refresh_highlight_chunk(
        tile_x,
        tile_y,
        &params.config,
        &params.session,
        &params.runtime,
        &mut params.images,
        params.highlight.tile,
    );

    if let Some(item) = params.placement.selected {
        let inserter_direction = params.placement.inserter_direction;
        let placed = try_place_at_world_pos(
            world_pos,
            item,
            inserter_direction,
            &params.config,
            &params.session,
            &mut params.runtime,
            &mut params.player,
            &mut params.images,
            &mut params.map,
            &params.services,
            &params.time,
            &mut params.queue,
            &mut params.status,
            params.highlight.tile,
        );
        if placed && params.player.inventory.count(item) == 0 {
            params.placement.selected = None;
        }
        if !placed && params.player.inventory.count(item) == 0 {
            params.placement.selected = None;
        }
        return;
    }

    let mut mined = try_mine_at_world_pos(
        world_pos,
        &params.config,
        &params.session,
        &mut params.runtime,
        &mut params.player,
        &mut params.images,
        &mut params.map,
        &params.services,
        &params.time,
        &mut params.queue,
        &mut params.status,
        &mut params.pickup_notices,
        log_mining,
        params.highlight.tile,
    );

    if !mined {
        if let Ok(player_transform) = params.player_query.single() {
            let player_pos = player_transform.translation.truncate();
            if player_pos.distance(world_pos) <= params.config.tile_size * 0.75 {
                mined = try_mine_at_world_pos(
                    player_pos,
                    &params.config,
                    &params.session,
                    &mut params.runtime,
                    &mut params.player,
                    &mut params.images,
                    &mut params.map,
                    &params.services,
                    &params.time,
                    &mut params.queue,
                    &mut params.status,
                    &mut params.pickup_notices,
                    log_mining,
                    params.highlight.tile,
                );
            }
        }
    }

    if log_mining && !mined {
        info!("mine result: no ore mined");
    }
}

pub(crate) const FURNACE_PROGRESS_BAR_WIDTH: f32 = 252.0;
pub(crate) const MINING_DRILL_PROGRESS_BAR_WIDTH: f32 = 252.0;

pub(crate) fn recipe_detail_label(recipe: &Recipe, inv: &Inventory) -> String {
    let mut label = format!(
        "{} x{}\n\nRequired:",
        item_name(recipe.output),
        recipe.output_amount
    );
    for (item, needed) in recipe.inputs {
        let have = inv.count(*item);
        label.push_str(&format!("\n{} {}/{}", item_name(*item), have, needed));
    }
    label
}

pub(crate) fn item_detail_label(item: ItemId, inv: &Inventory) -> String {
    format!("{}\n\nIn inventory: {}", item_name(item), inv.count(item))
}

#[derive(SystemParam)]
pub(crate) struct StructurePickupParams<'w, 's> {
    config: Res<'w, WorldRenderConfig>,
    session: Res<'w, WorldSession>,
    runtime: ResMut<'w, WorldRuntime>,
    player: ResMut<'w, PlayerState>,
    hotbar_interactions: Query<'w, 's, &'static Interaction, With<HotbarSlotButton>>,
    pickup: ResMut<'w, StructurePickupState>,
    ui_state: Res<'w, UiState>,
    images: ResMut<'w, Assets<Image>>,
    map: ResMut<'w, MapState>,
    services: NonSend<'w, StorageServices>,
    time: Res<'w, Time>,
    queue: ResMut<'w, SaveQueue>,
    status: ResMut<'w, StorageStatus>,
    highlight: Res<'w, ClickHighlight>,
}

pub(crate) fn structure_pickup_input_system(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut params: StructurePickupParams,
) {
    if params.ui_state.mode != UiMode::None || !buttons.pressed(MouseButton::Right) {
        reset_structure_pickup(&mut params.pickup);
        return;
    }
    if params
        .hotbar_interactions
        .iter()
        .any(|interaction| *interaction != Interaction::None)
    {
        reset_structure_pickup(&mut params.pickup);
        return;
    }

    let Ok(window) = windows.single() else {
        reset_structure_pickup(&mut params.pickup);
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        reset_structure_pickup(&mut params.pickup);
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        reset_structure_pickup(&mut params.pickup);
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        reset_structure_pickup(&mut params.pickup);
        return;
    };

    let tile_x = (world_pos.x / params.config.tile_size).floor() as i32;
    let tile_y = (world_pos.y / params.config.tile_size).floor() as i32;
    let Some(target) = pickup_target_at_tile(
        tile_x,
        tile_y,
        &params.config,
        &params.session,
        &params.runtime,
    ) else {
        reset_structure_pickup(&mut params.pickup);
        return;
    };

    if params.pickup.target.as_ref() != Some(&target) {
        params.pickup.target = Some(target.clone());
        params.pickup.elapsed_secs = 0.0;
    }
    params.pickup.elapsed_secs += params.time.delta_secs();

    if params.pickup.elapsed_secs < STRUCTURE_PICKUP_SECONDS {
        return;
    }

    let picked_up = try_pick_up_structure(
        &target,
        &params.config,
        &params.session,
        &mut params.runtime,
        &mut params.player,
        &mut params.images,
        &mut params.map,
        &params.services,
        &params.time,
        &mut params.queue,
        &mut params.status,
        params.highlight.tile,
    );
    reset_structure_pickup(&mut params.pickup);
    if picked_up {
        params.player.set_changed();
    }
}

fn reset_structure_pickup(pickup: &mut StructurePickupState) {
    pickup.target = None;
    pickup.elapsed_secs = 0.0;
}

pub(crate) fn pickup_target_at_tile(
    tile_x: i32,
    tile_y: i32,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &WorldRuntime,
) -> Option<StructurePickupTarget> {
    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
    let loaded = runtime.loaded.get(&key)?;
    let idx = (local_y as usize) * CHUNK_EDGE as usize + local_x as usize;
    let cell = loaded.data.placed.get(idx)?;
    if cell.object_id == 0 || placed_kind_to_item(cell.kind).is_none() {
        return None;
    }
    Some(StructurePickupTarget {
        key,
        tile_x,
        tile_y,
        local_x,
        local_y,
        kind: cell.kind,
        object_id: cell.object_id,
    })
}

pub(crate) fn try_pick_up_structure(
    target: &StructurePickupTarget,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &mut WorldRuntime,
    player: &mut PlayerState,
    images: &mut Assets<Image>,
    map: &mut MapState,
    services: &StorageServices,
    time: &Time,
    queue: &mut SaveQueue,
    status: &mut StorageStatus,
    highlight: Option<(i32, i32)>,
) -> bool {
    let (texture_handle, data_snapshot, pickups) = {
        let Some(loaded) = runtime.loaded.get_mut(&target.key) else {
            return false;
        };
        let idx = (target.local_y as usize) * CHUNK_EDGE as usize + target.local_x as usize;
        let Some(current) = loaded.data.placed.get(idx).copied() else {
            return false;
        };
        if current.kind != target.kind || current.object_id != target.object_id {
            return false;
        }

        let pickups =
            collect_structure_pickup_items(&mut loaded.data, current.kind, current.object_id);
        let Some(cell) = loaded.data.placed.get_mut(idx) else {
            return false;
        };
        cell.kind = PLACED_NONE;
        cell.object_id = 0;

        (loaded.texture_handle.clone(), loaded.data.clone(), pickups)
    };

    for (item, amount) in pickups {
        player.inventory.add(item, amount);
    }

    runtime.touch(&target.key);
    refresh_chunk_texture(
        images,
        &texture_handle,
        &data_snapshot,
        config,
        session.world_seed,
        highlight,
    );
    upsert_map_snapshot(
        map,
        images,
        &target.key,
        &data_snapshot,
        session.world_seed,
        time.elapsed().as_millis() as u64,
    );
    queue_chunk_save(
        &target.key,
        &data_snapshot,
        services,
        session,
        time,
        runtime,
        queue,
        status,
    );
    true
}

fn collect_structure_pickup_items(
    data: &mut SimChunkData,
    kind: PlacedId,
    object_id: ObjectId,
) -> Vec<(ItemId, u32)> {
    let mut pickups = Vec::new();
    if let Some(item) = placed_kind_to_item(kind) {
        pickups.push((item, 1));
    }

    if kind == PLACED_CHEST {
        if let Some(index) = data
            .chests
            .iter()
            .position(|chest| chest.object_id == object_id)
        {
            let chest = data.chests.remove(index);
            for slot in chest.inv.slots {
                push_slot_pickup(&mut pickups, slot);
            }
        }
    } else if kind == PLACED_FURNACE {
        if let Some(index) = data
            .furnaces
            .iter()
            .position(|furnace| furnace.object_id == object_id)
        {
            let furnace = data.furnaces.remove(index);
            push_slot_pickup(&mut pickups, furnace.state.input);
            push_slot_pickup(&mut pickups, furnace.state.fuel);
            push_slot_pickup(&mut pickups, furnace.state.output);
        }
    } else if kind == PLACED_INSERTER {
        if let Some(index) = data
            .inserters
            .iter()
            .position(|inserter| inserter.object_id == object_id)
        {
            let inserter = data.inserters.remove(index);
            for slot in inserter.inv.slots {
                push_slot_pickup(&mut pickups, slot);
            }
        }
    } else if kind == PLACED_MINING_DRILL {
        if let Some(index) = data
            .drills
            .iter()
            .position(|drill| drill.object_id == object_id)
        {
            let drill = data.drills.remove(index);
            push_slot_pickup(&mut pickups, drill.state.fuel);
            push_slot_pickup(&mut pickups, drill.state.output);
        }
    }

    pickups
}

fn push_slot_pickup(pickups: &mut Vec<(ItemId, u32)>, slot: Slot) {
    if !slot.is_empty() {
        pickups.push((slot.item, slot.count));
    }
}

const INSERTER_FUEL_BUFFER: u32 = 4;
const DRILL_FUEL_BUFFER: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InserterTile {
    pub(crate) world_id: WorldId,
    pub(crate) layer: ChunkLayer,
    pub(crate) tile_x: i32,
    pub(crate) tile_y: i32,
    pub(crate) object_id: ObjectId,
    pub(crate) direction: InserterDirection,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TransferEndpoint {
    Chest { object_id: ObjectId },
    Furnace { object_id: ObjectId },
    Drill { object_id: ObjectId },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TransferSource {
    ChestSlot {
        object_id: ObjectId,
        slot_idx: usize,
    },
    FurnaceOutput {
        object_id: ObjectId,
    },
    DrillOutput {
        object_id: ObjectId,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TransferTarget {
    Chest { object_id: ObjectId },
    FurnaceInput { object_id: ObjectId },
    FurnaceFuel { object_id: ObjectId },
    DrillFuel { object_id: ObjectId },
}

pub(crate) fn inserter_transfer_system(
    time: Res<Time>,
    mut inserters: ResMut<InserterState>,
    mut runtime: ResMut<WorldRuntime>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
) {
    inserters.timer.tick(time.delta());
    if !inserters.timer.just_finished() {
        return;
    }

    let inserter_tiles = collect_inserter_tiles(&runtime);
    let mut save_keys = HashSet::new();

    for tile in inserter_tiles {
        let (source_endpoint, target_endpoint) = directional_transfer_endpoints(&runtime, &tile);
        let mut moved_keys = execute_inserter_push(&mut runtime, &tile, target_endpoint);
        if moved_keys.is_empty() {
            moved_keys =
                execute_inserter_pull(&mut runtime, &tile, source_endpoint, target_endpoint);
        }
        for key in moved_keys {
            save_keys.insert(key);
        }
    }

    for key in save_keys {
        let snapshot = runtime.loaded.get(&key).map(|loaded| loaded.data.clone());
        if let Some(snapshot) = snapshot {
            queue_chunk_save(
                &key,
                &snapshot,
                &services,
                &session,
                &time,
                &mut runtime,
                &mut queue,
                &mut status,
            );
        }
    }
}

pub(crate) fn collect_inserter_tiles(runtime: &WorldRuntime) -> Vec<InserterTile> {
    let mut tiles = Vec::new();
    let edge = CHUNK_EDGE as usize;

    for (key, loaded) in &runtime.loaded {
        for (idx, placed) in loaded.data.placed.iter().enumerate() {
            if placed.kind != PLACED_INSERTER || placed.object_id == 0 {
                continue;
            }
            let local_x = (idx % edge) as i32;
            let local_y = (idx / edge) as i32;
            tiles.push(InserterTile {
                world_id: key.world_id.clone(),
                layer: key.layer,
                tile_x: loaded.data.coord.cx * CHUNK_EDGE as i32 + local_x,
                tile_y: loaded.data.coord.cy * CHUNK_EDGE as i32 + local_y,
                object_id: placed.object_id,
                direction: loaded
                    .data
                    .inserters
                    .iter()
                    .find(|inserter| inserter.object_id == placed.object_id)
                    .map(|inserter| inserter.direction)
                    .unwrap_or_default(),
            });
        }
    }

    tiles.sort_by_key(|tile| (tile.layer, tile.tile_y, tile.tile_x));
    tiles
}

pub(crate) fn directional_transfer_endpoints(
    runtime: &WorldRuntime,
    tile: &InserterTile,
) -> (Option<TransferEndpoint>, Option<TransferEndpoint>) {
    let (source_dx, source_dy) = tile.direction.back_offset();
    let (target_dx, target_dy) = tile.direction.forward_offset();
    (
        transfer_endpoint_at(
            runtime,
            &tile.world_id,
            tile.layer,
            tile.tile_x + source_dx,
            tile.tile_y + source_dy,
        ),
        transfer_endpoint_at(
            runtime,
            &tile.world_id,
            tile.layer,
            tile.tile_x + target_dx,
            tile.tile_y + target_dy,
        ),
    )
}

pub(crate) fn transfer_endpoint_at(
    runtime: &WorldRuntime,
    world_id: &WorldId,
    layer: ChunkLayer,
    tile_x: i32,
    tile_y: i32,
) -> Option<TransferEndpoint> {
    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(world_id.clone(), coord, layer);
    let loaded = runtime.loaded.get(&key)?;
    let idx = (local_y as usize) * CHUNK_EDGE as usize + local_x as usize;
    let cell = loaded.data.placed.get(idx)?;
    match cell.kind {
        PLACED_CHEST if cell.object_id != 0 => Some(TransferEndpoint::Chest {
            object_id: cell.object_id,
        }),
        PLACED_FURNACE if cell.object_id != 0 => Some(TransferEndpoint::Furnace {
            object_id: cell.object_id,
        }),
        PLACED_MINING_DRILL if cell.object_id != 0 => Some(TransferEndpoint::Drill {
            object_id: cell.object_id,
        }),
        _ => None,
    }
}

fn first_chest_slot_matching(
    runtime: &WorldRuntime,
    object_id: ObjectId,
    predicate: impl Fn(ItemId) -> bool,
) -> Option<(usize, ItemId)> {
    let chest = find_chest(runtime, object_id)?;
    chest
        .inv
        .slots
        .iter()
        .enumerate()
        .find(|(_, slot)| !slot.is_empty() && predicate(slot.item))
        .map(|(idx, slot)| (idx, slot.item))
}

fn first_inserter_slot_matching(
    runtime: &WorldRuntime,
    object_id: ObjectId,
    predicate: impl Fn(ItemId) -> bool,
) -> Option<(usize, ItemId)> {
    let inserter = find_inserter(runtime, object_id)?;
    inserter
        .inv
        .slots
        .iter()
        .enumerate()
        .find(|(_, slot)| !slot.is_empty() && predicate(slot.item))
        .map(|(idx, slot)| (idx, slot.item))
}

fn inserter_can_accept(runtime: &WorldRuntime, object_id: ObjectId, item: ItemId) -> bool {
    find_inserter(runtime, object_id)
        .map(|inserter| {
            inserter
                .inv
                .slots
                .iter()
                .any(|slot| slot_can_accept(*slot, item))
        })
        .unwrap_or(false)
}

fn best_target_for_item(
    runtime: &WorldRuntime,
    endpoints: &[TransferEndpoint],
    item: ItemId,
    exclude: Option<TransferEndpoint>,
) -> Option<TransferTarget> {
    if item == ITEM_COAL {
        for endpoint in endpoints {
            if Some(*endpoint) == exclude {
                continue;
            }
            let TransferEndpoint::Furnace { object_id } = *endpoint else {
                continue;
            };
            let target = TransferTarget::FurnaceFuel { object_id };
            if target_can_accept(runtime, target, item) {
                return Some(target);
            }
        }
        for endpoint in endpoints {
            if Some(*endpoint) == exclude {
                continue;
            }
            let TransferEndpoint::Drill { object_id } = *endpoint else {
                continue;
            };
            let target = TransferTarget::DrillFuel { object_id };
            if target_can_accept(runtime, target, item) {
                return Some(target);
            }
        }
    }

    if smelt_output_for_input(item).is_some() {
        for endpoint in endpoints {
            if Some(*endpoint) == exclude {
                continue;
            }
            let TransferEndpoint::Furnace { object_id } = *endpoint else {
                continue;
            };
            let target = TransferTarget::FurnaceInput { object_id };
            if target_can_accept(runtime, target, item) {
                return Some(target);
            }
        }
    }

    for endpoint in endpoints {
        if Some(*endpoint) == exclude {
            continue;
        }
        let TransferEndpoint::Chest { object_id } = *endpoint else {
            continue;
        };
        let target = TransferTarget::Chest { object_id };
        if target_can_accept(runtime, target, item) {
            return Some(target);
        }
    }

    None
}

fn best_pull_source(
    runtime: &WorldRuntime,
    inserter_id: ObjectId,
    source_endpoint: TransferEndpoint,
    target_endpoint: TransferEndpoint,
) -> Option<TransferSource> {
    let target_endpoints = [target_endpoint];

    if let TransferEndpoint::Furnace { object_id } = source_endpoint {
        if let Some(furnace) = find_furnace(runtime, object_id) {
            let output = furnace.state.output;
            if !output.is_empty()
                && inserter_can_accept(runtime, inserter_id, output.item)
                && best_target_for_item(runtime, &target_endpoints, output.item, None).is_some()
            {
                return Some(TransferSource::FurnaceOutput { object_id });
            }
        }
    }

    if let TransferEndpoint::Drill { object_id } = source_endpoint {
        if let Some(drill) = find_drill(runtime, object_id) {
            let output = drill.state.output;
            if !output.is_empty()
                && inserter_can_accept(runtime, inserter_id, output.item)
                && best_target_for_item(runtime, &target_endpoints, output.item, None).is_some()
            {
                return Some(TransferSource::DrillOutput { object_id });
            }
        }
    }

    let TransferEndpoint::Chest { object_id } = source_endpoint else {
        return None;
    };

    if let Some((slot_idx, _)) = first_chest_slot_matching(runtime, object_id, |item| {
        item == ITEM_COAL
            && inserter_can_accept(runtime, inserter_id, item)
            && best_target_for_item(runtime, &target_endpoints, item, None).is_some()
    }) {
        return Some(TransferSource::ChestSlot {
            object_id,
            slot_idx,
        });
    }

    if let Some((slot_idx, _)) = first_chest_slot_matching(runtime, object_id, |item| {
        smelt_output_for_input(item).is_some()
            && inserter_can_accept(runtime, inserter_id, item)
            && best_target_for_item(runtime, &target_endpoints, item, None).is_some()
    }) {
        return Some(TransferSource::ChestSlot {
            object_id,
            slot_idx,
        });
    }

    if let Some((slot_idx, _)) = first_chest_slot_matching(runtime, object_id, |item| {
        inserter_can_accept(runtime, inserter_id, item)
            && best_target_for_item(runtime, &target_endpoints, item, None).is_some()
    }) {
        return Some(TransferSource::ChestSlot {
            object_id,
            slot_idx,
        });
    }

    None
}

fn target_can_accept(runtime: &WorldRuntime, target: TransferTarget, item: ItemId) -> bool {
    match target {
        TransferTarget::Chest { object_id } => find_chest(runtime, object_id)
            .map(|chest| {
                chest
                    .inv
                    .slots
                    .iter()
                    .any(|slot| slot_can_accept(*slot, item))
            })
            .unwrap_or(false),
        TransferTarget::FurnaceInput { object_id } => {
            if smelt_output_for_input(item).is_none() {
                return false;
            }
            find_furnace(runtime, object_id)
                .map(|furnace| slot_can_accept(furnace.state.input, item))
                .unwrap_or(false)
        }
        TransferTarget::FurnaceFuel { object_id } => {
            if item != ITEM_COAL {
                return false;
            }
            find_furnace(runtime, object_id)
                .map(|furnace| {
                    slot_can_accept(furnace.state.fuel, item)
                        && (furnace.state.fuel.is_empty()
                            || furnace.state.fuel.count < INSERTER_FUEL_BUFFER)
                })
                .unwrap_or(false)
        }
        TransferTarget::DrillFuel { object_id } => {
            if item != ITEM_COAL {
                return false;
            }
            find_drill(runtime, object_id)
                .map(|drill| {
                    slot_can_accept(drill.state.fuel, item)
                        && (drill.state.fuel.is_empty()
                            || drill.state.fuel.count < DRILL_FUEL_BUFFER)
                })
                .unwrap_or(false)
        }
    }
}

fn slot_can_accept(slot: Slot, item: ItemId) -> bool {
    item != ITEM_NONE && (slot.is_empty() || slot.item == item)
}

fn execute_inserter_push(
    runtime: &mut WorldRuntime,
    tile: &InserterTile,
    target_endpoint: Option<TransferEndpoint>,
) -> Vec<ChunkKey> {
    let Some(target_endpoint) = target_endpoint else {
        return Vec::new();
    };
    let Some((slot_idx, item)) = first_inserter_slot_matching(runtime, tile.object_id, |_| true)
    else {
        return Vec::new();
    };
    let target_endpoints = [target_endpoint];
    let Some(target) = best_target_for_item(runtime, &target_endpoints, item, None) else {
        return Vec::new();
    };

    let Some((inserter_key, slot)) = take_from_inserter_source(runtime, tile.object_id, slot_idx)
    else {
        return Vec::new();
    };

    let Some((target_key, moved)) =
        deposit_to_transfer_target(runtime, target, slot.item, slot.count)
    else {
        restore_inserter_source(runtime, tile.object_id, slot);
        return Vec::new();
    };
    if moved != slot.count {
        restore_inserter_source(runtime, tile.object_id, slot);
        return Vec::new();
    }

    if inserter_key == target_key {
        vec![inserter_key]
    } else {
        vec![inserter_key, target_key]
    }
}

fn execute_inserter_pull(
    runtime: &mut WorldRuntime,
    tile: &InserterTile,
    source_endpoint: Option<TransferEndpoint>,
    target_endpoint: Option<TransferEndpoint>,
) -> Vec<ChunkKey> {
    let (Some(source_endpoint), Some(target_endpoint)) = (source_endpoint, target_endpoint) else {
        return Vec::new();
    };
    let Some(source) = best_pull_source(runtime, tile.object_id, source_endpoint, target_endpoint)
    else {
        return Vec::new();
    };
    let Some((source_key, slot)) = take_from_transfer_source(runtime, source) else {
        return Vec::new();
    };
    let Some((inserter_key, moved)) =
        deposit_to_inserter_target(runtime, tile.object_id, slot.item, slot.count)
    else {
        restore_transfer_source(runtime, source, slot);
        return Vec::new();
    };
    if moved != slot.count {
        restore_transfer_source(runtime, source, slot);
        return Vec::new();
    }

    if source_key == inserter_key {
        vec![source_key]
    } else {
        vec![source_key, inserter_key]
    }
}

fn take_from_inserter_source(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    slot_idx: usize,
) -> Option<(ChunkKey, Slot)> {
    with_inserter_mut(runtime, object_id, |data| {
        take_from_inserter(&mut data.inserters, object_id, slot_idx, 1)
    })
    .and_then(|(key, _, slot)| slot.map(|slot| (key, slot)))
}

fn take_from_transfer_source(
    runtime: &mut WorldRuntime,
    source: TransferSource,
) -> Option<(ChunkKey, Slot)> {
    match source {
        TransferSource::ChestSlot {
            object_id,
            slot_idx,
        } => with_chest_mut(runtime, object_id, |data| {
            take_from_chest(&mut data.chests, object_id, slot_idx, 1)
        })
        .and_then(|(key, _, slot)| slot.map(|slot| (key, slot))),
        TransferSource::FurnaceOutput { object_id } => {
            with_furnace_mut(runtime, object_id, |data| {
                take_from_furnace(&mut data.furnaces, object_id, FurnaceSlot::Output, 1)
            })
            .and_then(|(key, _, slot)| slot.map(|slot| (key, slot)))
        }
        TransferSource::DrillOutput { object_id } => with_drill_mut(runtime, object_id, |data| {
            take_from_drill(&mut data.drills, object_id, DrillSlot::Output, 1)
        })
        .and_then(|(key, _, slot)| slot.map(|slot| (key, slot))),
    }
}

fn deposit_to_transfer_target(
    runtime: &mut WorldRuntime,
    target: TransferTarget,
    item: ItemId,
    amount: u32,
) -> Option<(ChunkKey, u32)> {
    match target {
        TransferTarget::Chest { object_id } => with_chest_mut(runtime, object_id, |data| {
            deposit_to_chest(&mut data.chests, object_id, item, amount)
        })
        .map(|(key, _, moved)| (key, moved)),
        TransferTarget::FurnaceInput { object_id } => {
            if smelt_output_for_input(item).is_none() {
                return None;
            }
            with_furnace_mut(runtime, object_id, |data| {
                deposit_to_furnace_input(&mut data.furnaces, object_id, item, amount)
            })
            .map(|(key, _, moved)| (key, moved))
        }
        TransferTarget::FurnaceFuel { object_id } => {
            if item != ITEM_COAL {
                return None;
            }
            with_furnace_mut(runtime, object_id, |data| {
                deposit_to_furnace_fuel(&mut data.furnaces, object_id, item, amount)
            })
            .map(|(key, _, moved)| (key, moved))
        }
        TransferTarget::DrillFuel { object_id } => {
            if item != ITEM_COAL {
                return None;
            }
            with_drill_mut(runtime, object_id, |data| {
                deposit_to_drill_fuel(&mut data.drills, object_id, item, amount)
            })
            .map(|(key, _, moved)| (key, moved))
        }
    }
}

fn deposit_to_inserter_target(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    item: ItemId,
    amount: u32,
) -> Option<(ChunkKey, u32)> {
    with_inserter_mut(runtime, object_id, |data| {
        deposit_to_inserter(&mut data.inserters, object_id, item, amount)
    })
    .map(|(key, _, moved)| (key, moved))
}

fn restore_inserter_source(runtime: &mut WorldRuntime, object_id: ObjectId, slot: Slot) {
    if slot.is_empty() {
        return;
    }
    let _ = with_inserter_mut(runtime, object_id, |data| {
        deposit_to_inserter(&mut data.inserters, object_id, slot.item, slot.count)
    });
}

fn restore_transfer_source(runtime: &mut WorldRuntime, source: TransferSource, slot: Slot) {
    if slot.is_empty() {
        return;
    }
    match source {
        TransferSource::ChestSlot { object_id, .. } => {
            let _ = with_chest_mut(runtime, object_id, |data| {
                deposit_to_chest(&mut data.chests, object_id, slot.item, slot.count)
            });
        }
        TransferSource::FurnaceOutput { object_id } => {
            let _ = with_furnace_mut(runtime, object_id, |data| {
                let Some(furnace) = data
                    .furnaces
                    .iter_mut()
                    .find(|furnace| furnace.object_id == object_id)
                else {
                    return 0;
                };
                deposit_to_slot_unchecked(&mut furnace.state.output, slot.item, slot.count)
            });
        }
        TransferSource::DrillOutput { object_id } => {
            let _ = with_drill_mut(runtime, object_id, |data| {
                let Some(drill) = data
                    .drills
                    .iter_mut()
                    .find(|drill| drill.object_id == object_id)
                else {
                    return 0;
                };
                deposit_to_slot_unchecked(&mut drill.state.output, slot.item, slot.count)
            });
        }
    }
}

fn deposit_to_slot_unchecked(slot: &mut Slot, item: ItemId, amount: u32) -> u32 {
    if item == ITEM_NONE || amount == 0 {
        return 0;
    }
    if slot.is_empty() {
        slot.item = item;
        slot.count = amount;
        return amount;
    }
    if slot.item == item {
        slot.count = slot.count.saturating_add(amount);
        return amount;
    }
    0
}

pub(crate) fn refresh_highlight_chunk(
    tile_x: i32,
    tile_y: i32,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &WorldRuntime,
    images: &mut Assets<Image>,
    highlight: Option<(i32, i32)>,
) {
    let (coord, _, _) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
    if let Some(loaded) = runtime.loaded.get(&key) {
        refresh_chunk_texture(
            images,
            &loaded.texture_handle,
            &loaded.data,
            config,
            session.world_seed,
            highlight,
        );
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum MineAttempt {
    Mined,
    Empty,
    ChunkMissing,
}

impl MineAttempt {
    fn is_mined(self) -> bool {
        matches!(self, MineAttempt::Mined)
    }
}

pub(crate) fn try_mine_at_world_pos(
    world_pos: Vec2,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &mut WorldRuntime,
    player: &mut PlayerState,
    images: &mut Assets<Image>,
    map: &mut MapState,
    services: &StorageServices,
    time: &Time,
    queue: &mut SaveQueue,
    status: &mut StorageStatus,
    pickup_notices: &mut EventWriter<PickupNotice>,
    log: bool,
    highlight: Option<(i32, i32)>,
) -> bool {
    let tile_x = (world_pos.x / config.tile_size).floor() as i32;
    let tile_y = (world_pos.y / config.tile_size).floor() as i32;
    let mined = try_mine_tile(
        tile_x,
        tile_y,
        config,
        session,
        runtime,
        player,
        images,
        map,
        services,
        time,
        queue,
        status,
        pickup_notices,
        log,
        highlight,
    )
    .is_mined();
    if log {
        info!(
            "mine attempt: world=({:.2},{:.2}) tile=({},{}) mined={}",
            world_pos.x, world_pos.y, tile_x, tile_y, mined
        );
    }
    mined
}

pub(crate) fn try_mine_tile(
    tile_x: i32,
    tile_y: i32,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &mut WorldRuntime,
    player: &mut PlayerState,
    images: &mut Assets<Image>,
    map: &mut MapState,
    services: &StorageServices,
    time: &Time,
    queue: &mut SaveQueue,
    status: &mut StorageStatus,
    pickup_notices: &mut EventWriter<PickupNotice>,
    log: bool,
    highlight: Option<(i32, i32)>,
) -> MineAttempt {
    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
    let (texture_handle, data_snapshot, mined_kind) = {
        let Some(loaded) = runtime.loaded.get_mut(&key) else {
            if log {
                info!("mine miss: chunk not loaded {:?}", key);
            }
            return MineAttempt::ChunkMissing;
        };
        let edge = CHUNK_EDGE as i32;
        let idx = (local_y as usize) * (edge as usize) + (local_x as usize);
        let Some(cell) = loaded.data.resources.get_mut(idx) else {
            if log {
                info!("mine miss: no resource cell at {}/{}", local_x, local_y);
            }
            return MineAttempt::Empty;
        };
        if cell.amount == 0 || cell.kind == RES_NONE {
            if log {
                info!(
                    "mine miss: empty ore at chunk=({},{}) local=({},{}) kind={} amt={}",
                    coord.cx, coord.cy, local_x, local_y, cell.kind, cell.amount
                );
            }
            return MineAttempt::Empty;
        }

        let mined_kind = cell.kind;
        cell.amount = cell.amount.saturating_sub(1);
        if cell.amount == 0 {
            cell.kind = RES_NONE;
        }

        (
            loaded.texture_handle.clone(),
            loaded.data.clone(),
            mined_kind,
        )
    };

    if let Some(item) = resource_to_item(mined_kind) {
        player.inventory.add(item, 1);
        pickup_notices.write(PickupNotice {
            item,
            amount: 1,
            total: player.inventory.count(item),
        });
    }
    runtime.touch(&key);
    refresh_chunk_texture(
        images,
        &texture_handle,
        &data_snapshot,
        config,
        session.world_seed,
        highlight,
    );
    upsert_map_snapshot(
        map,
        images,
        &key,
        &data_snapshot,
        session.world_seed,
        time.elapsed().as_millis() as u64,
    );
    queue_chunk_save(
        &key,
        &data_snapshot,
        services,
        session,
        time,
        runtime,
        queue,
        status,
    );
    if log {
        info!(
            "mine hit: chunk=({},{}) local=({},{}) kind={}",
            coord.cx, coord.cy, local_x, local_y, mined_kind
        );
    }
    MineAttempt::Mined
}

pub(crate) fn can_place_tile(
    tile_x: i32,
    tile_y: i32,
    item: ItemId,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &WorldRuntime,
    player: &PlayerState,
) -> bool {
    let Some(placed_kind) = item_to_placed_kind(item) else {
        return false;
    };
    if player.inventory.count(item) == 0 {
        return false;
    }

    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
    let Some(loaded) = runtime.loaded.get(&key) else {
        return false;
    };
    let edge = CHUNK_EDGE as i32;
    let idx = (local_y as usize) * (edge as usize) + (local_x as usize);
    let tile = tile_at(&loaded.data, local_x, local_y, session.world_seed);
    if is_water(tile) {
        return false;
    }
    if placed_kind == PLACED_MINING_DRILL {
        let resource = loaded
            .data
            .resources
            .get(idx)
            .copied()
            .unwrap_or(ResourceCell {
                kind: RES_NONE,
                amount: 0,
            });
        if resource.kind == RES_NONE || resource.amount == 0 {
            return false;
        }
    }

    loaded
        .data
        .placed
        .get(idx)
        .is_some_and(|cell| cell.kind == PLACED_NONE)
}

pub(crate) fn try_place_at_world_pos(
    world_pos: Vec2,
    item: ItemId,
    inserter_direction: InserterDirection,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &mut WorldRuntime,
    player: &mut PlayerState,
    images: &mut Assets<Image>,
    map: &mut MapState,
    services: &StorageServices,
    time: &Time,
    queue: &mut SaveQueue,
    status: &mut StorageStatus,
    highlight: Option<(i32, i32)>,
) -> bool {
    let tile_x = (world_pos.x / config.tile_size).floor() as i32;
    let tile_y = (world_pos.y / config.tile_size).floor() as i32;
    try_place_tile(
        tile_x,
        tile_y,
        item,
        inserter_direction,
        config,
        session,
        runtime,
        player,
        images,
        map,
        services,
        time,
        queue,
        status,
        highlight,
    )
}

pub(crate) fn try_place_tile(
    tile_x: i32,
    tile_y: i32,
    item: ItemId,
    inserter_direction: InserterDirection,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &mut WorldRuntime,
    player: &mut PlayerState,
    images: &mut Assets<Image>,
    map: &mut MapState,
    services: &StorageServices,
    time: &Time,
    queue: &mut SaveQueue,
    status: &mut StorageStatus,
    highlight: Option<(i32, i32)>,
) -> bool {
    let Some(placed_kind) = item_to_placed_kind(item) else {
        return false;
    };
    if player.inventory.count(item) == 0 {
        return false;
    }

    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
    let (texture_handle, data_snapshot) = {
        let Some(loaded) = runtime.loaded.get_mut(&key) else {
            return false;
        };
        let edge = CHUNK_EDGE as i32;
        let idx = (local_y as usize) * (edge as usize) + (local_x as usize);
        let tile = tile_at(&loaded.data, local_x, local_y, session.world_seed);
        if is_water(tile) {
            return false;
        }
        if placed_kind == PLACED_MINING_DRILL {
            let resource = loaded
                .data
                .resources
                .get(idx)
                .copied()
                .unwrap_or(ResourceCell {
                    kind: RES_NONE,
                    amount: 0,
                });
            if resource.kind == RES_NONE || resource.amount == 0 {
                return false;
            }
        }
        let Some(cell) = loaded.data.placed.get_mut(idx) else {
            return false;
        };
        if cell.kind != PLACED_NONE {
            return false;
        }
        let object_id = object_id_for_tile(session.world_seed, tile_x, tile_y, placed_kind);
        if !player.inventory.try_remove(item, 1) {
            return false;
        }
        cell.kind = placed_kind;
        cell.object_id = object_id;
        if placed_kind == PLACED_CHEST {
            if !loaded
                .data
                .chests
                .iter()
                .any(|chest| chest.object_id == object_id)
            {
                loaded.data.chests.push(ChestRecord {
                    object_id,
                    inv: ContainerInv::default(),
                });
            }
        } else if placed_kind == PLACED_FURNACE {
            if !loaded
                .data
                .furnaces
                .iter()
                .any(|furnace| furnace.object_id == object_id)
            {
                loaded.data.furnaces.push(FurnaceRecord {
                    object_id,
                    state: FurnaceState::default(),
                });
            }
        } else if placed_kind == PLACED_INSERTER {
            if !loaded
                .data
                .inserters
                .iter()
                .any(|inserter| inserter.object_id == object_id)
            {
                loaded.data.inserters.push(InserterRecord {
                    object_id,
                    direction: inserter_direction,
                    inv: InserterInv::default(),
                });
            }
        } else if placed_kind == PLACED_MINING_DRILL {
            if !loaded
                .data
                .drills
                .iter()
                .any(|drill| drill.object_id == object_id)
            {
                loaded.data.drills.push(DrillRecord {
                    object_id,
                    state: DrillState::default(),
                });
            }
        }
        (loaded.texture_handle.clone(), loaded.data.clone())
    };

    runtime.touch(&key);
    refresh_chunk_texture(
        images,
        &texture_handle,
        &data_snapshot,
        config,
        session.world_seed,
        highlight,
    );
    upsert_map_snapshot(
        map,
        images,
        &key,
        &data_snapshot,
        session.world_seed,
        time.elapsed().as_millis() as u64,
    );
    queue_chunk_save(
        &key,
        &data_snapshot,
        services,
        session,
        time,
        runtime,
        queue,
        status,
    );
    true
}

pub(crate) fn select_hotbar_slot(
    index: usize,
    inventory: &Inventory,
    hotbar: &mut HotbarState,
    placement: &mut PlacementState,
) {
    let Some(item) = hotbar.slots.get(index).and_then(|slot| *slot) else {
        hotbar.selected_slot = None;
        placement.selected = None;
        return;
    };
    if inventory.count(item) == 0 || !is_placeable_item(item) {
        hotbar.selected_slot = None;
        placement.selected = None;
        return;
    }

    if hotbar.selected_slot == Some(index) {
        hotbar.selected_slot = None;
        placement.selected = None;
    } else {
        hotbar.selected_slot = Some(index);
        placement.selected = Some(item);
    }
}

pub(crate) fn chest_button_system(
    ui_state: Res<UiState>,
    mut runtime: ResMut<WorldRuntime>,
    mut player: ResMut<PlayerState>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    time: Res<Time>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
    mut slot_buttons: Query<(&Interaction, &ChestSlotButton), Changed<Interaction>>,
    mut deposit_buttons: Query<(&Interaction, &ChestDepositButton), Changed<Interaction>>,
) {
    let UiMode::Chest { object_id } = ui_state.mode else {
        return;
    };

    for (interaction, button) in &mut slot_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let result = with_chest_mut(&mut runtime, object_id, |data| {
            take_from_chest(&mut data.chests, object_id, button.index, u32::MAX)
        });
        if let Some((key, snapshot, Some(slot))) = result {
            if slot.item != ITEM_NONE && slot.count > 0 {
                player.inventory.add(slot.item, slot.count);
            }
            runtime.touch(&key);
            queue_chunk_save(
                &key,
                &snapshot,
                &services,
                &session,
                &time,
                &mut runtime,
                &mut queue,
                &mut status,
            );
        }
    }

    for (interaction, button) in &mut deposit_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let amount = player.inventory.count(button.item);
        if amount == 0 {
            continue;
        }
        let result = with_chest_mut(&mut runtime, object_id, |data| {
            deposit_to_chest(&mut data.chests, object_id, button.item, amount)
        });
        if let Some((key, snapshot, moved)) = result {
            if moved > 0 {
                let _ = player.inventory.try_remove(button.item, moved);
                runtime.touch(&key);
                queue_chunk_save(
                    &key,
                    &snapshot,
                    &services,
                    &session,
                    &time,
                    &mut runtime,
                    &mut queue,
                    &mut status,
                );
            }
        }
    }
}

pub(crate) fn furnace_button_system(
    ui_state: Res<UiState>,
    mut runtime: ResMut<WorldRuntime>,
    mut player: ResMut<PlayerState>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    time: Res<Time>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
    mut slot_buttons: Query<(&Interaction, &FurnaceSlotButton), Changed<Interaction>>,
    mut deposit_buttons: Query<(&Interaction, &FurnaceDepositButton), Changed<Interaction>>,
) {
    let UiMode::Furnace { object_id } = ui_state.mode else {
        return;
    };

    for (interaction, button) in &mut slot_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let result = with_furnace_mut(&mut runtime, object_id, |data| {
            take_from_furnace(&mut data.furnaces, object_id, button.slot, u32::MAX)
        });
        if let Some((key, snapshot, Some(slot))) = result {
            if slot.item != ITEM_NONE && slot.count > 0 {
                player.inventory.add(slot.item, slot.count);
            }
            runtime.touch(&key);
            queue_chunk_save(
                &key,
                &snapshot,
                &services,
                &session,
                &time,
                &mut runtime,
                &mut queue,
                &mut status,
            );
        }
    }

    for (interaction, button) in &mut deposit_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let amount = player.inventory.count(button.item);
        if amount == 0 {
            continue;
        }
        let result = with_furnace_mut(&mut runtime, object_id, |data| match button.slot {
            FurnaceSlot::Input => {
                deposit_to_furnace_input(&mut data.furnaces, object_id, button.item, amount)
            }
            FurnaceSlot::Fuel => {
                deposit_to_furnace_fuel(&mut data.furnaces, object_id, button.item, amount)
            }
            FurnaceSlot::Output => 0,
        });
        if let Some((key, snapshot, moved)) = result {
            if moved > 0 {
                let _ = player.inventory.try_remove(button.item, moved);
                runtime.touch(&key);
                queue_chunk_save(
                    &key,
                    &snapshot,
                    &services,
                    &session,
                    &time,
                    &mut runtime,
                    &mut queue,
                    &mut status,
                );
            }
        }
    }
}

pub(crate) fn drill_button_system(
    ui_state: Res<UiState>,
    mut runtime: ResMut<WorldRuntime>,
    mut player: ResMut<PlayerState>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    time: Res<Time>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
    mut slot_buttons: Query<(&Interaction, &DrillSlotButton), Changed<Interaction>>,
    mut deposit_buttons: Query<(&Interaction, &DrillDepositButton), Changed<Interaction>>,
) {
    let UiMode::MiningDrill { object_id } = ui_state.mode else {
        return;
    };

    for (interaction, button) in &mut slot_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let result = with_drill_mut(&mut runtime, object_id, |data| {
            take_from_drill(&mut data.drills, object_id, button.slot, u32::MAX)
        });
        if let Some((key, snapshot, Some(slot))) = result {
            if slot.item != ITEM_NONE && slot.count > 0 {
                player.inventory.add(slot.item, slot.count);
            }
            runtime.touch(&key);
            queue_chunk_save(
                &key,
                &snapshot,
                &services,
                &session,
                &time,
                &mut runtime,
                &mut queue,
                &mut status,
            );
        }
    }

    for (interaction, button) in &mut deposit_buttons {
        if *interaction != Interaction::Pressed || button.item != ITEM_COAL {
            continue;
        }
        let amount = player.inventory.count(button.item);
        if amount == 0 {
            continue;
        }
        let result = with_drill_mut(&mut runtime, object_id, |data| {
            deposit_to_drill_fuel(&mut data.drills, object_id, button.item, amount)
        });
        if let Some((key, snapshot, moved)) = result {
            if moved > 0 {
                let _ = player.inventory.try_remove(button.item, moved);
                runtime.touch(&key);
                queue_chunk_save(
                    &key,
                    &snapshot,
                    &services,
                    &session,
                    &time,
                    &mut runtime,
                    &mut queue,
                    &mut status,
                );
            }
        }
    }
}

pub(crate) fn inserter_button_system(
    ui_state: Res<UiState>,
    mut runtime: ResMut<WorldRuntime>,
    mut player: ResMut<PlayerState>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    time: Res<Time>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
    mut slot_buttons: Query<(&Interaction, &InserterSlotButton), Changed<Interaction>>,
) {
    let UiMode::Inserter { object_id } = ui_state.mode else {
        return;
    };

    for (interaction, button) in &mut slot_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let result = with_inserter_mut(&mut runtime, object_id, |data| {
            take_from_inserter(&mut data.inserters, object_id, button.index, u32::MAX)
        });
        if let Some((key, snapshot, Some(slot))) = result {
            if slot.item != ITEM_NONE && slot.count > 0 {
                player.inventory.add(slot.item, slot.count);
            }
            runtime.touch(&key);
            queue_chunk_save(
                &key,
                &snapshot,
                &services,
                &session,
                &time,
                &mut runtime,
                &mut queue,
                &mut status,
            );
        }
    }
}

pub(crate) fn furnace_smelting_system(
    time: Res<Time>,
    mut runtime: ResMut<WorldRuntime>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
) {
    let delta = time.delta_secs();
    if delta <= 0.0 {
        return;
    }

    let mut save_keys = Vec::new();

    for (key, loaded) in runtime.loaded.iter_mut() {
        let mut smelted_in_chunk = false;

        for furnace in &mut loaded.data.furnaces {
            let output_item = smelt_output_for_input(furnace.state.input.item);
            let can_smelt = output_item.is_some()
                && !furnace.state.input.is_empty()
                && furnace.state.fuel.item == ITEM_COAL
                && furnace.state.fuel.count > 0
                && (furnace.state.output.is_empty()
                    || furnace.state.output.item == output_item.unwrap());

            if !can_smelt {
                if furnace.state.progress != 0 {
                    furnace.state.progress = 0;
                }
                continue;
            }

            let output_item = output_item.unwrap();
            let mut progress = furnace.state.progress as f32 + delta * FURNACE_PROGRESS_PER_SEC;

            while progress >= FURNACE_PROGRESS_PER_ITEM as f32 {
                if furnace.state.input.is_empty()
                    || furnace.state.fuel.is_empty()
                    || (!furnace.state.output.is_empty()
                        && furnace.state.output.item != output_item)
                {
                    break;
                }

                furnace.state.input.count = furnace.state.input.count.saturating_sub(1);
                if furnace.state.input.count == 0 {
                    furnace.state.input.clear();
                }
                furnace.state.fuel.count = furnace.state.fuel.count.saturating_sub(1);
                if furnace.state.fuel.count == 0 {
                    furnace.state.fuel.clear();
                }
                if furnace.state.output.is_empty() {
                    furnace.state.output.item = output_item;
                    furnace.state.output.count = 1;
                } else {
                    furnace.state.output.count = furnace.state.output.count.saturating_add(1);
                }

                smelted_in_chunk = true;
                progress -= FURNACE_PROGRESS_PER_ITEM as f32;
            }

            let still_can_smelt = !furnace.state.input.is_empty()
                && furnace.state.fuel.item == ITEM_COAL
                && furnace.state.fuel.count > 0
                && (furnace.state.output.is_empty() || furnace.state.output.item == output_item);

            furnace.state.progress = if still_can_smelt {
                progress.min(FURNACE_PROGRESS_PER_ITEM as f32) as u16
            } else {
                0
            };
        }

        if smelted_in_chunk {
            save_keys.push(key.clone());
        }
    }

    for key in save_keys {
        let snapshot = runtime.loaded.get(&key).map(|loaded| loaded.data.clone());
        if let Some(snapshot) = snapshot {
            queue_chunk_save(
                &key,
                &snapshot,
                &services,
                &session,
                &time,
                &mut runtime,
                &mut queue,
                &mut status,
            );
        }
    }
}

pub(crate) fn mining_drill_system(
    time: Res<Time>,
    mut runtime: ResMut<WorldRuntime>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    config: Res<WorldRenderConfig>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
    mut images: ResMut<Assets<Image>>,
    mut map: ResMut<MapState>,
    highlight: Res<ClickHighlight>,
) {
    let delta = time.delta_secs();
    if delta <= 0.0 {
        return;
    }

    let mut changed_chunks = Vec::new();

    for (key, loaded) in runtime.loaded.iter_mut() {
        let drill_tiles = drill_tiles_in_chunk(&loaded.data);
        if drill_tiles.is_empty() {
            continue;
        }

        let mut mined_in_chunk = false;
        for (resource_idx, drill_idx) in drill_tiles {
            mined_in_chunk |=
                advance_mining_drill(&mut loaded.data, resource_idx, drill_idx, delta);
        }

        if mined_in_chunk {
            changed_chunks.push((
                key.clone(),
                loaded.texture_handle.clone(),
                loaded.data.clone(),
            ));
        }
    }

    for (key, texture_handle, snapshot) in changed_chunks {
        runtime.touch(&key);
        refresh_chunk_texture(
            &mut images,
            &texture_handle,
            &snapshot,
            &config,
            session.world_seed,
            highlight.tile,
        );
        upsert_map_snapshot(
            &mut map,
            &mut images,
            &key,
            &snapshot,
            session.world_seed,
            time.elapsed().as_millis() as u64,
        );
        queue_chunk_save(
            &key,
            &snapshot,
            &services,
            &session,
            &time,
            &mut runtime,
            &mut queue,
            &mut status,
        );
    }
}

fn drill_tiles_in_chunk(data: &SimChunkData) -> Vec<(usize, usize)> {
    data.placed
        .iter()
        .enumerate()
        .filter_map(|(resource_idx, placed)| {
            if placed.kind != PLACED_MINING_DRILL || placed.object_id == 0 {
                return None;
            }
            let drill_idx = data
                .drills
                .iter()
                .position(|drill| drill.object_id == placed.object_id)?;
            Some((resource_idx, drill_idx))
        })
        .collect()
}

fn advance_mining_drill(
    data: &mut SimChunkData,
    resource_idx: usize,
    drill_idx: usize,
    delta: f32,
) -> bool {
    let Some(resource) = data.resources.get_mut(resource_idx) else {
        return false;
    };
    let Some(drill) = data.drills.get_mut(drill_idx) else {
        return false;
    };
    let Some(output_item) = resource_to_item(resource.kind) else {
        drill.state.progress = 0;
        return false;
    };

    let can_mine = resource.amount > 0
        && drill.state.fuel.item == ITEM_COAL
        && drill.state.fuel.count > 0
        && (drill.state.output.is_empty() || drill.state.output.item == output_item);
    if !can_mine {
        drill.state.progress = 0;
        return false;
    }

    let mut mined = false;
    let mut progress = drill.state.progress as f32 + delta * MINING_DRILL_PROGRESS_PER_SEC;

    while progress >= MINING_DRILL_PROGRESS_PER_ITEM as f32 {
        if resource.amount == 0
            || drill.state.fuel.is_empty()
            || (!drill.state.output.is_empty() && drill.state.output.item != output_item)
        {
            break;
        }

        drill.state.fuel.count = drill.state.fuel.count.saturating_sub(1);
        if drill.state.fuel.count == 0 {
            drill.state.fuel.clear();
        }

        resource.amount = resource.amount.saturating_sub(1);
        if resource.amount == 0 {
            resource.kind = RES_NONE;
        }

        if drill.state.output.is_empty() {
            drill.state.output.item = output_item;
            drill.state.output.count = 1;
        } else {
            drill.state.output.count = drill.state.output.count.saturating_add(1);
        }

        mined = true;
        progress -= MINING_DRILL_PROGRESS_PER_ITEM as f32;
    }

    let still_can_mine = resource.amount > 0
        && drill.state.fuel.item == ITEM_COAL
        && drill.state.fuel.count > 0
        && (drill.state.output.is_empty() || drill.state.output.item == output_item);
    drill.state.progress = if still_can_mine {
        progress.min(MINING_DRILL_PROGRESS_PER_ITEM as f32) as u16
    } else {
        0
    };

    mined
}
