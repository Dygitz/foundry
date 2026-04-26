#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, map::*, player::*, rendering::*, resources::*,
    ui::*, world::*,
};

pub(crate) struct FoundryStoragePlugin;

impl Plugin for FoundryStoragePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_storage)
            .add_systems(
                Update,
                (
                    storage_init_pump_system,
                    storage_recovery_pump_system,
                    player_state_load_pump_system,
                    autosave_flush_system,
                ),
            )
            .add_systems(Update, (player_state_save_system,).in_set(UpdateSet::Ui))
            .add_systems(
                Update,
                (
                    chunk_load_pump_system,
                    chunk_loaded_system,
                    chunk_eviction_system,
                )
                    .in_set(UpdateSet::World),
            );
    }
}

pub(crate) fn init_storage(world: &mut World) {
    let (db_name, db_version, game_schema_version) = {
        let config = world.resource::<StorageConfig>();
        (
            config.db_name.clone(),
            config.db_version,
            config.game_schema_version,
        )
    };

    let storage = IndexedDbStorage::new(db_name, db_version);
    let codec = ChunkCodecV1::new(game_schema_version);
    let task = AsyncComputeTaskPool::get().spawn_local({
        let storage = storage.clone();
        async move { storage.init().await }
    });

    world.insert_non_send_resource(StorageServices { storage, codec });
    world.insert_resource(StorageInitTask {
        task: Some(task),
        ready: false,
    });
}

pub(crate) fn storage_init_pump_system(
    mut init_task: ResMut<StorageInitTask>,
    mut status: ResMut<StorageStatus>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut recovery: ResMut<RecoveryState>,
    mut load_state: ResMut<PlayerStateLoadState>,
) {
    if let Some(task) = init_task.task.as_mut() {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            match result {
                Ok(()) => {
                    status.mark_ok();
                    init_task.ready = true;
                }
                Err(error) => status.record_error(&error),
            }
            init_task.task = None;
        }
    }

    if init_task.ready && recovery.task.is_none() && !recovery.completed {
        let storage = services.storage.clone();
        let world_id = session.world_id.clone();
        let task = AsyncComputeTaskPool::get()
            .spawn_local(async move { storage.recover_incomplete_savepoints(&world_id).await });
        recovery.task = Some(task);
    }

    if init_task.ready && recovery.completed && !load_state.loaded && load_state.task.is_none() {
        let storage = services.storage.clone();
        let world_id = session.world_id.clone();
        load_state.task = Some(
            AsyncComputeTaskPool::get()
                .spawn_local(async move { storage.load_player_state(&world_id).await }),
        );
    }
}

pub(crate) fn storage_recovery_pump_system(
    mut recovery: ResMut<RecoveryState>,
    mut status: ResMut<StorageStatus>,
) {
    let Some(task) = recovery.task.as_mut() else {
        return;
    };

    if let Some(result) = future::block_on(future::poll_once(task)) {
        match result {
            Ok(report) => {
                apply_recovery_report(&mut status, &report);
                recovery.completed = true;
            }
            Err(error) => status.record_error(&error),
        }
        recovery.task = None;
    }
}

pub(crate) fn player_state_load_pump_system(
    mut load: ResMut<PlayerStateLoadState>,
    mut player: ResMut<PlayerState>,
    mut status: ResMut<StorageStatus>,
) {
    let Some(task) = load.task.as_mut() else {
        return;
    };

    if let Some(result) = future::block_on(future::poll_once(task)) {
        match result {
            Ok(Some(record)) => match decode_player_state_v1(&record.blob) {
                Ok(inventory) => {
                    player.inventory = inventory;
                    status.mark_ok();
                }
                Err(error) => status.record_error(&StorageError::DecodeFailed(error)),
            },
            Ok(None) => status.mark_ok(),
            Err(error) => status.record_error(&error),
        }
        load.task = None;
        load.loaded = true;
    }
}

pub(crate) fn player_state_save_system(
    time: Res<Time>,
    mut save: ResMut<PlayerStateSaveState>,
    player: Res<PlayerState>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut status: ResMut<StorageStatus>,
) {
    if player.is_changed() {
        save.dirty = true;
    }

    save.timer.tick(time.delta());

    if let Some(task) = save.in_flight.as_mut() {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            match result {
                Ok(()) => status.mark_ok(),
                Err(error) => status.record_error(&error),
            }
            save.in_flight = None;
        }
    }

    if save.in_flight.is_some() {
        return;
    }
    if status.state == StorageState::Paused {
        return;
    }
    if !save.timer.just_finished() {
        return;
    }
    if !save.dirty {
        return;
    }

    let blob = encode_player_state_v1(&player.inventory);
    let record = PlayerStateRecordWrite {
        world_id: session.world_id.clone(),
        blob,
        updated_at_ms: time.elapsed().as_millis() as u64,
    };
    let storage = services.storage.clone();
    save.dirty = false;
    save.in_flight = Some(
        AsyncComputeTaskPool::get()
            .spawn_local(async move { storage.save_player_state(record).await }),
    );
}

pub(crate) fn autosave_flush_system(
    time: Res<Time>,
    mut state: ResMut<AutosaveState>,
    mut queue: ResMut<SaveQueue>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut runtime: ResMut<WorldRuntime>,
    mut status: ResMut<StorageStatus>,
) {
    state.timer.tick(time.delta());

    if let Some(mut save_task) = state.in_flight.take() {
        if let Some(result) = future::block_on(future::poll_once(&mut save_task.task)) {
            match result {
                Ok(()) => {
                    for _ in 0..save_task.pending_count {
                        queue.pending.pop_front();
                    }
                    for key in save_task.keys {
                        let still_pending = queue.pending.iter().any(|record| record.key == key);
                        if still_pending {
                            runtime.queued_for_save.insert(key.clone());
                            runtime.dirty.insert(key.clone());
                        } else {
                            runtime.queued_for_save.remove(&key);
                            runtime.dirty.remove(&key);
                        }
                    }
                    status.mark_ok();
                }
                Err(error) => {
                    status.record_error(&error);
                }
            }
        } else {
            state.in_flight = Some(save_task);
            return;
        }
    }

    if state.in_flight.is_some()
        || status.state == StorageState::Paused
        || !state.timer.just_finished()
        || queue.pending.is_empty()
    {
        return;
    }

    let batch: Vec<ChunkRecordWrite> = queue
        .pending
        .iter()
        .take(state.max_per_flush)
        .cloned()
        .collect();
    let pending_count = batch.len();
    if pending_count == 0 {
        return;
    }

    let storage = services.storage.clone();
    let world_id = session.world_id.clone();
    let tick = session.tick;
    let chunk_keys: Vec<ChunkKey> = batch.iter().map(|record| record.key.clone()).collect();
    let save_keys = chunk_keys.clone();
    let task = AsyncComputeTaskPool::get().spawn_local(async move {
        let savepoint_id = storage.begin_savepoint(&world_id, tick, chunk_keys).await?;
        storage.put_chunks(&world_id, batch).await?;
        storage.commit_savepoint(&savepoint_id).await?;
        Ok(())
    });
    state.in_flight = Some(SaveTask {
        task,
        pending_count,
        keys: save_keys,
    });
}

pub(crate) fn chunk_load_pump_system(
    mut requests: EventReader<ChunkLoadRequest>,
    mut loaded: EventWriter<ChunkLoaded>,
    services: NonSend<StorageServices>,
    mut state: ResMut<ChunkLoadState>,
    mut runtime: ResMut<WorldRuntime>,
    mut status: ResMut<StorageStatus>,
) {
    for request in requests.read() {
        if state.in_flight.contains(&request.key) || state.queue.contains(&request.key) {
            continue;
        }
        state.queue.push_back(request.key.clone());
    }

    let pool = AsyncComputeTaskPool::get();
    while state.in_flight.len() < state.max_in_flight {
        let Some(key) = state.queue.pop_front() else {
            break;
        };

        let storage = services.storage.clone();
        let codec = services.codec;
        let world_id = key.world_id.clone();
        let coord = key.coord;
        let layer = key.layer;
        let task = pool.spawn_local(async move {
            let record = storage.get_chunk(&world_id, coord, layer).await?;
            match record {
                Some(record) => codec.decode(&record.blob).map(Some),
                None => Ok(None),
            }
        });

        state.in_flight.insert(key.clone());
        state.tasks.push(ChunkLoadTask { key, task });
    }

    let mut remaining = Vec::with_capacity(state.tasks.len());
    let mut completed_keys = Vec::new();
    for mut task in state.tasks.drain(..) {
        match future::block_on(future::poll_once(&mut task.task)) {
            Some(Ok(data)) => {
                completed_keys.push(task.key.clone());
                loaded.write(ChunkLoaded {
                    key: task.key,
                    data,
                });
                status.mark_ok();
            }
            Some(Err(error)) => {
                if matches!(error, StorageError::NotFound) {
                    completed_keys.push(task.key.clone());
                    loaded.write(ChunkLoaded {
                        key: task.key,
                        data: None,
                    });
                } else {
                    completed_keys.push(task.key);
                    status.record_error(&error);
                }
            }
            None => remaining.push(task),
        }
    }
    for key in completed_keys {
        state.in_flight.remove(&key);
        runtime.requested.remove(&key);
    }
    state.tasks = remaining;
}

pub(crate) fn chunk_loaded_system(
    mut commands: Commands,
    mut runtime: ResMut<WorldRuntime>,
    mut loaded_events: EventReader<ChunkLoaded>,
    mut images: ResMut<Assets<Image>>,
    mut map: ResMut<MapState>,
    config: Res<WorldRenderConfig>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    time: Res<Time>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
    highlight: Res<ClickHighlight>,
) {
    for event in loaded_events.read() {
        let data = match &event.data {
            Some(data) => data.clone(),
            None => {
                let generated = generate_chunk_data(
                    event.key.coord,
                    event.key.layer,
                    session.world_seed,
                    session.tick,
                );
                let mut queued = false;
                if !runtime.queued_for_save.contains(&event.key) {
                    let view = SimChunkView::from_data(&generated);
                    match services.codec.encode(&view, session.tick) {
                        Ok(blob) => {
                            let updated_at_ms = time.elapsed().as_millis() as u64;
                            runtime.queued_for_save.insert(event.key.clone());
                            queue.pending.push_back(ChunkRecordWrite {
                                key: event.key.clone(),
                                blob,
                                tick_saved: session.tick,
                                checksum: 0,
                                updated_at_ms,
                            });
                            queued = true;
                        }
                        Err(error) => status.record_error(&error),
                    }
                }
                if queued {
                    runtime.mark_dirty(event.key.clone());
                }
                generated
            }
        };

        if let Some(existing) = runtime.loaded.get_mut(&event.key) {
            existing.data = data;
            runtime.touch(&event.key);
            if let Some(existing) = runtime.loaded.get(&event.key) {
                upsert_map_snapshot(
                    &mut map,
                    &mut images,
                    &event.key,
                    &existing.data,
                    session.world_seed,
                    time.elapsed().as_millis() as u64,
                );
            }
            // Texture refresh will use existing.texture_handle once chunk diffing is wired up.
            continue;
        }

        let texture_handle = images.add(build_chunk_image(
            &data,
            &config,
            session.world_seed,
            highlight.tile,
        ));
        let chunk_size = chunk_world_size(&config);
        let center = chunk_center_world(data.coord, chunk_size);
        let entity = commands
            .spawn((
                Sprite {
                    image: texture_handle.clone(),
                    custom_size: Some(Vec2::splat(chunk_size)),
                    rect: Some(chunk_sprite_rect()),
                    ..default()
                },
                Transform::from_translation(Vec3::new(center.x, center.y, 0.0)),
                ChunkRenderTag,
            ))
            .id();

        runtime.loaded.insert(
            event.key.clone(),
            LoadedChunk {
                data,
                sprite_entity: entity,
                texture_handle,
            },
        );
        if let Some(loaded) = runtime.loaded.get(&event.key) {
            upsert_map_snapshot(
                &mut map,
                &mut images,
                &event.key,
                &loaded.data,
                session.world_seed,
                time.elapsed().as_millis() as u64,
            );
        }
        runtime.touch(&event.key);
    }
}

pub(crate) fn queue_chunk_save(
    key: &ChunkKey,
    data: &SimChunkData,
    services: &StorageServices,
    session: &WorldSession,
    time: &Time,
    runtime: &mut WorldRuntime,
    queue: &mut SaveQueue,
    status: &mut StorageStatus,
) {
    let view = SimChunkView::from_data(data);
    match services.codec.encode(&view, session.tick) {
        Ok(blob) => {
            let updated_at_ms = time.elapsed().as_millis() as u64;
            if let Some(existing) = queue.pending.iter_mut().find(|record| record.key == *key) {
                existing.blob = blob;
                existing.tick_saved = session.tick;
                existing.checksum = 0;
                existing.updated_at_ms = updated_at_ms;
            } else {
                queue.pending.push_back(ChunkRecordWrite {
                    key: key.clone(),
                    blob,
                    tick_saved: session.tick,
                    checksum: 0,
                    updated_at_ms,
                });
            }
            runtime.queued_for_save.insert(key.clone());
            runtime.mark_dirty(key.clone());
        }
        Err(error) => status.record_error(&error),
    }
}

pub(crate) fn chunk_eviction_system(
    mut commands: Commands,
    mut runtime: ResMut<WorldRuntime>,
    cache: Res<ChunkCacheConfig>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    time: Res<Time>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
    mut stats: ResMut<EvictionStats>,
) {
    stats.timer.tick(time.delta());

    let allow_eviction = queue.pending.len() <= 512;
    let allow_dirty_eviction = queue.pending.is_empty();
    let mut evicted = 0usize;

    if allow_eviction
        && runtime.loaded.len() > cache.max_loaded_chunks
        && !runtime.keep_set.is_empty()
    {
        let mut candidates: Vec<(u64, ChunkKey)> = Vec::new();
        for key in runtime.loaded.keys() {
            if runtime.keep_set.contains(key) {
                continue;
            }
            let last_access = runtime.last_access_frame.get(key).copied().unwrap_or(0);
            candidates.push((last_access, key.clone()));
        }
        candidates.sort_by_key(|(frame, _)| *frame);

        for (_, key) in candidates {
            if runtime.loaded.len() <= cache.max_loaded_chunks || evicted >= cache.evict_per_frame {
                break;
            }

            if runtime.dirty.contains(&key) {
                if !allow_dirty_eviction {
                    continue;
                }
                if !runtime.queued_for_save.contains(&key) {
                    let Some(loaded) = runtime.loaded.get(&key) else {
                        continue;
                    };
                    let view = SimChunkView::from_data(&loaded.data);
                    match services.codec.encode(&view, session.tick) {
                        Ok(blob) => {
                            let updated_at_ms = time.elapsed().as_millis() as u64;
                            runtime.queued_for_save.insert(key.clone());
                            queue.pending.push_back(ChunkRecordWrite {
                                key: key.clone(),
                                blob,
                                tick_saved: session.tick,
                                checksum: 0,
                                updated_at_ms,
                            });
                        }
                        Err(error) => {
                            status.record_error(&error);
                            continue;
                        }
                    }
                }
            }

            if let Some(loaded) = runtime.loaded.remove(&key) {
                commands.entity(loaded.sprite_entity).despawn();
            }
            runtime.last_access_frame.remove(&key);
            evicted += 1;
        }
    }

    stats.evicted_this_window += evicted;
    if stats.timer.just_finished() {
        stats.evicted_per_second = stats.evicted_this_window;
        stats.evicted_this_window = 0;
    }
}

pub(crate) const PS_MAGIC: [u8; 4] = *b"PLR1";
pub(crate) const PS_VERSION: u16 = 1;

pub(crate) fn encode_player_state_v1(inv: &Inventory) -> Vec<u8> {
    let entries: Vec<(simulation_core::ItemId, u32)> = inv.entries().collect();
    let count_u16: u16 = entries.len().min(u16::MAX as usize) as u16;

    let mut out = Vec::with_capacity(4 + 2 + 2 + (count_u16 as usize) * 6);
    out.extend_from_slice(&PS_MAGIC);
    out.extend_from_slice(&PS_VERSION.to_le_bytes());
    out.extend_from_slice(&count_u16.to_le_bytes());
    for (item, qty) in entries.into_iter().take(count_u16 as usize) {
        out.extend_from_slice(&item.to_le_bytes());
        out.extend_from_slice(&qty.to_le_bytes());
    }
    out
}

pub(crate) fn decode_player_state_v1(bytes: &[u8]) -> Result<Inventory, String> {
    if bytes.len() < 8 {
        return Err("player_state blob too short".to_string());
    }
    if bytes[0..4] != PS_MAGIC {
        return Err("player_state magic mismatch".to_string());
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != PS_VERSION {
        return Err(format!("player_state version {version} unsupported"));
    }

    let count = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let mut offset = 8usize;
    let mut inv = Inventory::default();

    for _ in 0..count {
        if offset + 6 > bytes.len() {
            return Err("player_state truncated".to_string());
        }
        let item = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let qty = u32::from_le_bytes([
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
        ]);
        offset += 6;
        if item != ITEM_NONE && qty > 0 {
            inv.add(item, qty);
        }
    }

    if offset != bytes.len() {
        return Err("player_state trailing bytes".to_string());
    }
    Ok(inv)
}

pub(crate) fn apply_recovery_report(status: &mut StorageStatus, report: &RecoveryReport) {
    if report.incomplete_savepoints.is_empty() {
        status.mark_ok();
        return;
    }

    if status.state != StorageState::Paused {
        status.state = StorageState::Healthy;
        status.detail = Some(format!(
            "ignored {} incomplete savepoints",
            report.incomplete_savepoints.len()
        ));
    }
}

#[cfg(test)]
mod tests;
