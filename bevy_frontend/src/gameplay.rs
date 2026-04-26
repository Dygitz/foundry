#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, map::*, player::*, rendering::*, resources::*, storage::*,
    ui::*, world::*,
};

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
        let placed = try_place_at_world_pos(
            world_pos,
            item,
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

pub(crate) fn try_place_at_world_pos(
    world_pos: Vec2,
    item: ItemId,
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
        tile_x, tile_y, item, config, session, runtime, player, images, map, services, time, queue,
        status, highlight,
    )
}

pub(crate) fn try_place_tile(
    tile_x: i32,
    tile_y: i32,
    item: ItemId,
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
