use std::collections::{HashMap, HashSet, VecDeque};

use bevy::image::ImageSampler;
use bevy::input::keyboard::KeyCode;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::tasks::futures_lite::future;
use persistence::{
    ChunkCodec, ChunkCodecV1, ChunkCoord, ChunkKey, ChunkLayer, ChunkRecordWrite, RecoveryReport,
    StorageError, WorldId, WorldStorage,
};
use simulation_core::{SimChunkData, SimChunkView, TileId, CHUNK_EDGE, CHUNK_TILE_COUNT};
use web_storage_indexeddb::IndexedDbStorage;

pub fn run() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.08, 0.75, 0.72)))
        .insert_resource(StorageConfig::default())
        .insert_resource(StorageStatus::default())
        .insert_resource(WorldSession::default())
        .insert_resource(WorldRenderConfig::default())
        .insert_resource(WorldRuntime::default())
        .insert_resource(ChunkCacheConfig::default())
        .insert_resource(PlayerConfig::default())
        .insert_resource(EvictionStats::default())
        .insert_resource(AutosaveState::default())
        .insert_resource(SaveQueue::default())
        .insert_resource(ChunkLoadState::default())
        .insert_resource(RecoveryState::default())
        .add_event::<ChunkLoadRequest>()
        .add_event::<ChunkLoaded>()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
        )
        .add_systems(Startup, (setup, init_storage))
        .add_systems(
            Update,
            (
                storage_init_pump_system,
                storage_recovery_pump_system,
                autosave_flush_system,
                storage_status_text_system,
            ),
        )
        .add_systems(
            Update,
            (
                world_runtime_frame_counter_system,
                player_movement_system,
                camera_follow_system,
                active_area_chunk_request_system,
                chunk_load_pump_system,
                chunk_loaded_system,
                chunk_eviction_system,
                world_stats_text_system,
            )
                .chain(),
        )
        .run();
}

#[derive(Resource)]
struct WorldSession {
    world_id: WorldId,
    world_seed: u64,
    tick: u64,
}

impl Default for WorldSession {
    fn default() -> Self {
        Self {
            world_id: WorldId::from("local-dev"),
            world_seed: 1337,
            tick: 0,
        }
    }
}

#[derive(Resource, Default)]
struct WorldRuntime {
    loaded: HashMap<ChunkKey, LoadedChunk>,
    dirty: HashSet<ChunkKey>,
    last_access_frame: HashMap<ChunkKey, u64>,
    queued_for_save: HashSet<ChunkKey>,
    requested: HashSet<ChunkKey>,
    active_set: HashSet<ChunkKey>,
    keep_set: HashSet<ChunkKey>,
    frame_counter: u64,
}

impl WorldRuntime {
    fn advance_frame(&mut self) {
        self.frame_counter = self.frame_counter.saturating_add(1);
    }

    fn ensure_loaded(&self, key: &ChunkKey) -> bool {
        self.loaded.contains_key(key)
    }

    fn mark_dirty(&mut self, key: ChunkKey) {
        self.dirty.insert(key.clone());
        self.touch(&key);
    }

    fn touch(&mut self, key: &ChunkKey) {
        self.last_access_frame.insert(key.clone(), self.frame_counter);
    }

    fn evictable_candidates(&self) -> Vec<ChunkKey> {
        let mut entries: Vec<(u64, ChunkKey)> = self
            .loaded
            .keys()
            .map(|key| {
                (
                    self.last_access_frame.get(key).copied().unwrap_or(0),
                    key.clone(),
                )
            })
            .collect();
        entries.sort_by_key(|(frame, _)| *frame);
        entries.into_iter().map(|(_, key)| key).collect()
    }
}

struct LoadedChunk {
    data: SimChunkData,
    sprite_entity: Entity,
    texture_handle: Handle<Image>,
}

#[derive(Resource)]
struct ChunkCacheConfig {
    max_loaded_chunks: usize,
    keep_radius_chunks: i32,
    evict_per_frame: usize,
}

impl Default for ChunkCacheConfig {
    fn default() -> Self {
        Self {
            max_loaded_chunks: 512,
            keep_radius_chunks: 4,
            evict_per_frame: 8,
        }
    }
}

#[derive(Resource)]
struct PlayerConfig {
    move_speed: f32,
    camera_follow_lerp: f32,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            move_speed: 240.0,
            camera_follow_lerp: 12.0,
        }
    }
}

#[derive(Resource)]
struct WorldRenderConfig {
    tile_size: f32,
    active_radius_chunks: i32,
    layer: ChunkLayer,
}

impl Default for WorldRenderConfig {
    fn default() -> Self {
        Self {
            tile_size: 16.0,
            active_radius_chunks: 2,
            layer: 0,
        }
    }
}

#[derive(Resource)]
struct StorageConfig {
    db_name: String,
    db_version: u32,
    game_schema_version: u16,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_name: "game_worlds".to_string(),
            db_version: 1,
            game_schema_version: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorageState {
    Healthy,
    Degraded,
    Paused,
}

#[derive(Resource, Debug, Clone)]
struct StorageStatus {
    state: StorageState,
    detail: Option<String>,
}

impl Default for StorageStatus {
    fn default() -> Self {
        Self {
            state: StorageState::Degraded,
            detail: Some("initializing".to_string()),
        }
    }
}

impl StorageStatus {
    fn mark_ok(&mut self) {
        if self.state != StorageState::Paused {
            self.state = StorageState::Healthy;
            self.detail = None;
        }
    }

    fn record_error(&mut self, error: &StorageError) {
        if matches!(error, StorageError::QuotaExceeded) {
            self.state = StorageState::Paused;
            self.detail = Some("storage quota exceeded (autosave paused)".to_string());
            return;
        }

        self.detail = Some(error.to_string());
        self.state = StorageState::Degraded;
    }

    fn label(&self) -> String {
        let state = match self.state {
            StorageState::Healthy => "Healthy",
            StorageState::Degraded => "Degraded",
            StorageState::Paused => "Paused",
        };
        match &self.detail {
            Some(detail) => format!("Storage: {state} ({detail})"),
            None => format!("Storage: {state}"),
        }
    }
}

struct StorageServices {
    storage: IndexedDbStorage,
    codec: ChunkCodecV1,
}

#[derive(Resource, Default)]
struct StorageInitTask {
    task: Option<Task<Result<(), StorageError>>>,
    ready: bool,
}

#[derive(Resource)]
struct AutosaveState {
    timer: Timer,
    max_per_flush: usize,
    in_flight: Option<SaveTask>,
}

impl Default for AutosaveState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            max_per_flush: 64,
            in_flight: None,
        }
    }
}

struct SaveTask {
    task: Task<Result<(), StorageError>>,
    pending_count: usize,
    keys: Vec<ChunkKey>,
}

#[derive(Resource, Default)]
struct SaveQueue {
    pending: VecDeque<ChunkRecordWrite>,
}

#[derive(Event, Debug, Clone)]
struct ChunkLoadRequest {
    key: ChunkKey,
}

#[derive(Event, Debug, Clone)]
struct ChunkLoaded {
    key: ChunkKey,
    data: Option<SimChunkData>,
}

struct ChunkLoadTask {
    key: ChunkKey,
    task: Task<Result<Option<SimChunkData>, StorageError>>,
}

#[derive(Resource)]
struct ChunkLoadState {
    queue: VecDeque<ChunkKey>,
    in_flight: HashSet<ChunkKey>,
    tasks: Vec<ChunkLoadTask>,
    max_in_flight: usize,
}

impl Default for ChunkLoadState {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            in_flight: HashSet::new(),
            tasks: Vec::new(),
            max_in_flight: 6,
        }
    }
}

#[derive(Component)]
struct StorageStatusText;

#[derive(Component)]
struct ChunkRenderTag;

#[derive(Component)]
struct WorldStatsText;

#[derive(Component)]
struct Player;

#[derive(Resource, Default)]
struct RecoveryState {
    task: Option<Task<Result<RecoveryReport, StorageError>>>,
    completed: bool,
}

#[derive(Resource)]
struct EvictionStats {
    timer: Timer,
    evicted_this_window: usize,
    evicted_per_second: usize,
}

impl Default for EvictionStats {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            evicted_this_window: 0,
            evicted_per_second: 0,
        }
    }
}

fn setup(mut commands: Commands, config: Res<WorldRenderConfig>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            color: Color::srgb(0.95, 0.9, 0.2),
            custom_size: Some(Vec2::splat(config.tile_size * 0.8)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        Player,
    ));
    commands.spawn((
        Text::new("Storage: initializing"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
        StorageStatusText,
    ));
    commands.spawn((
        Text::new("Chunks: 0 | Dirty: 0 | Evict/s: 0"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(32.0),
            ..default()
        },
        WorldStatsText,
    ));
}

fn world_runtime_frame_counter_system(mut runtime: ResMut<WorldRuntime>) {
    runtime.advance_frame();
}

fn player_movement_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<PlayerConfig>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO {
        return;
    }

    let delta = direction * config.move_speed * time.delta_secs();
    for mut transform in &mut player_query {
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

fn camera_follow_system(
    time: Res<Time>,
    config: Res<PlayerConfig>,
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };

    let mut target = player_transform.translation;
    target.z = camera_transform.translation.z;
    let t = 1.0 - (-config.camera_follow_lerp * time.delta_secs()).exp();
    camera_transform.translation = camera_transform.translation.lerp(target, t);
}

fn active_area_chunk_request_system(
    camera_query: Query<&Transform, With<Camera2d>>,
    config: Res<WorldRenderConfig>,
    cache: Res<ChunkCacheConfig>,
    session: Res<WorldSession>,
    mut runtime: ResMut<WorldRuntime>,
    mut requests: EventWriter<ChunkLoadRequest>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let chunk_size = chunk_world_size(&config);
    let camera_pos = camera_transform.translation.truncate();
    let center_coord = world_to_chunk_coord(camera_pos, chunk_size);
    let radius = config.active_radius_chunks;
    let keep_radius = cache.keep_radius_chunks.max(radius);
    let mut active_set = HashSet::new();
    let mut keep_set = HashSet::new();

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let coord = ChunkCoord::new(center_coord.cx + dx, center_coord.cy + dy);
            let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
            active_set.insert(key.clone());
            if runtime.ensure_loaded(&key) {
                runtime.touch(&key);
            } else if runtime.requested.insert(key.clone()) {
                requests.write(ChunkLoadRequest { key });
            }
        }
    }
    for dy in -keep_radius..=keep_radius {
        for dx in -keep_radius..=keep_radius {
            let coord = ChunkCoord::new(center_coord.cx + dx, center_coord.cy + dy);
            let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
            keep_set.insert(key);
        }
    }
    runtime.active_set = active_set;
    runtime.keep_set = keep_set;
}

fn init_storage(world: &mut World) {
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

fn storage_init_pump_system(
    mut init_task: ResMut<StorageInitTask>,
    mut status: ResMut<StorageStatus>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut recovery: ResMut<RecoveryState>,
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
        let task = AsyncComputeTaskPool::get().spawn_local(async move {
            storage.recover_incomplete_savepoints(&world_id).await
        });
        recovery.task = Some(task);
    }
}

fn storage_recovery_pump_system(
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

fn autosave_flush_system(
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
                        runtime.queued_for_save.remove(&key);
                        runtime.dirty.remove(&key);
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
        let savepoint_id = storage
            .begin_savepoint(&world_id, tick, chunk_keys)
            .await?;
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

fn chunk_load_pump_system(
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

fn chunk_loaded_system(
    mut commands: Commands,
    mut runtime: ResMut<WorldRuntime>,
    mut loaded_events: EventReader<ChunkLoaded>,
    mut images: ResMut<Assets<Image>>,
    config: Res<WorldRenderConfig>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    time: Res<Time>,
    mut queue: ResMut<SaveQueue>,
    mut status: ResMut<StorageStatus>,
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
            // Texture refresh will use existing.texture_handle once chunk diffing is wired up.
            continue;
        }

        let texture_handle = images.add(build_chunk_image(&data));
        let chunk_size = chunk_world_size(&config);
        let center = chunk_center_world(data.coord, chunk_size);
        let entity = commands
            .spawn((
                Sprite {
                    image: texture_handle.clone(),
                    custom_size: Some(Vec2::splat(chunk_size)),
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
        runtime.touch(&event.key);
    }
}

fn chunk_eviction_system(
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
                commands.entity(loaded.sprite_entity).despawn_recursive();
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

fn storage_status_text_system(
    status: Res<StorageStatus>,
    mut query: Query<&mut Text, With<StorageStatusText>>,
) {
    if !status.is_changed() {
        return;
    }
    let label = status.label();
    for mut text in &mut query {
        *text = Text::new(label.clone());
    }
}

fn world_stats_text_system(
    runtime: Res<WorldRuntime>,
    stats: Res<EvictionStats>,
    mut query: Query<&mut Text, With<WorldStatsText>>,
) {
    if !runtime.is_changed() && !stats.is_changed() {
        return;
    }
    let label = format!(
        "Chunks: {} | Dirty: {} | Evict/s: {}",
        runtime.loaded.len(),
        runtime.dirty.len(),
        stats.evicted_per_second
    );
    for mut text in &mut query {
        *text = Text::new(label.clone());
    }
}

fn generate_chunk_data(
    coord: ChunkCoord,
    layer: ChunkLayer,
    world_seed: u64,
    saved_tick: u64,
) -> SimChunkData {
    let edge = CHUNK_EDGE as usize;
    let base_x = coord.cx * CHUNK_EDGE as i32;
    let base_y = coord.cy * CHUNK_EDGE as i32;
    let seed = world_seed ^ (layer as u64).wrapping_mul(0x9e3779b97f4a7c15);
    let mut tiles = Vec::with_capacity(CHUNK_TILE_COUNT);
    for y in 0..edge {
        let gy = base_y + y as i32;
        for x in 0..edge {
            let gx = base_x + x as i32;
            let coarse_x = gx >> 3;
            let coarse_y = gy >> 3;
            let h = terrain_hash(coarse_x, coarse_y, seed);
            let v = (h & 0xFFFF) as u16;
            let variant = (terrain_hash(gx, gy, seed ^ 0x5bf03635f7d13d9b) >> 8) as u8;
            let tile = if v < 5000 {
                6
            } else if v < 16000 {
                4 + (variant % 2) as TileId
            } else {
                (variant % 4) as TileId
            };
            tiles.push(tile);
        }
    }
    SimChunkData {
        coord,
        layer,
        tiles,
        entities: Vec::new(),
        saved_tick,
    }
}

fn chunk_world_size(config: &WorldRenderConfig) -> f32 {
    config.tile_size * CHUNK_EDGE as f32
}

fn world_to_chunk_coord(world_pos: Vec2, chunk_size: f32) -> ChunkCoord {
    ChunkCoord::new(
        (world_pos.x / chunk_size).floor() as i32,
        (world_pos.y / chunk_size).floor() as i32,
    )
}

fn chunk_center_world(coord: ChunkCoord, chunk_size: f32) -> Vec2 {
    Vec2::new(
        (coord.cx as f32 + 0.5) * chunk_size,
        (coord.cy as f32 + 0.5) * chunk_size,
    )
}

fn build_chunk_image(data: &SimChunkData) -> Image {
    let pixels = chunk_pixels(data);
    let mut image = Image::new_fill(
        Extent3d {
            width: CHUNK_EDGE as u32,
            height: CHUNK_EDGE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn chunk_pixels(data: &SimChunkData) -> Vec<u8> {
    let edge = CHUNK_EDGE as usize;
    let mut pixels = Vec::with_capacity(CHUNK_TILE_COUNT * 4);
    let mut idx = 0usize;
    for y in 0..edge {
        for x in 0..edge {
            let tile = data.tiles.get(idx).copied().unwrap_or(0);
            let mut color = tile_color(tile);
            if x == 0 || y == 0 {
                color = darken_color(color);
            }
            pixels.extend_from_slice(&color);
            idx += 1;
        }
    }
    pixels
}

fn tile_color(tile: TileId) -> [u8; 4] {
    match tile {
        0 => [58, 123, 70, 255],
        1 => [66, 132, 78, 255],
        2 => [72, 140, 82, 255],
        3 => [80, 148, 90, 255],
        4 => [132, 96, 60, 255],
        5 => [146, 106, 66, 255],
        6 => [46, 92, 166, 255],
        _ => [110, 110, 110, 255],
    }
}

fn darken_color(color: [u8; 4]) -> [u8; 4] {
    let r = (color[0] as u16 * 4 / 5) as u8;
    let g = (color[1] as u16 * 4 / 5) as u8;
    let b = (color[2] as u16 * 4 / 5) as u8;
    [r, g, b, color[3]]
}

fn terrain_hash(x: i32, y: i32, seed: u64) -> u32 {
    let mut z = seed;
    z ^= (x as i64 as u64).wrapping_mul(0x9e3779b97f4a7c15);
    z ^= (y as i64 as u64).wrapping_mul(0xc2b2ae3d27d4eb4f);
    mix64(z) as u32
}

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn apply_recovery_report(status: &mut StorageStatus, report: &RecoveryReport) {
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
