use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::tasks::futures_lite::future;
use persistence::{
    ChunkCodec, ChunkCodecV1, ChunkKey, ChunkRecordWrite, StorageError, WorldId, WorldStorage,
};
use simulation_core::SimChunkData;
use web_storage_indexeddb::IndexedDbStorage;

pub fn run() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.08, 0.75, 0.72)))
        .insert_resource(StorageConfig::default())
        .insert_resource(StorageStatus::default())
        .insert_resource(WorldSession::default())
        .insert_resource(AutosaveState::default())
        .insert_resource(SaveQueue::default())
        .insert_resource(ChunkLoadState::default())
        .add_event::<ChunkLoadRequest>()
        .add_event::<ChunkLoaded>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, init_storage))
        .add_systems(
            Update,
            (
                storage_init_pump_system,
                autosave_flush_system,
                chunk_load_pump_system,
                storage_status_text_system,
            ),
        )
        .run();
}

#[derive(Resource)]
struct WorldSession {
    world_id: WorldId,
    tick: u64,
}

impl Default for WorldSession {
    fn default() -> Self {
        Self {
            world_id: WorldId::from("local-dev"),
            tick: 0,
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
        self.detail = Some(error.to_string());
        self.state = match error {
            StorageError::QuotaExceeded => StorageState::Paused,
            _ => StorageState::Degraded,
        };
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
}

#[derive(Resource)]
struct AutosaveState {
    timer: Timer,
    max_per_flush: usize,
    in_flight: Option<Task<Result<(), StorageError>>>,
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

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
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
    world.insert_resource(StorageInitTask { task: Some(task) });
}

fn storage_init_pump_system(
    mut init_task: ResMut<StorageInitTask>,
    mut status: ResMut<StorageStatus>,
) {
    let Some(task) = init_task.task.as_mut() else {
        return;
    };

    if let Some(result) = future::block_on(future::poll_once(task)) {
        match result {
            Ok(()) => status.mark_ok(),
            Err(error) => status.record_error(&error),
        }
        init_task.task = None;
    }
}

fn autosave_flush_system(
    time: Res<Time>,
    mut state: ResMut<AutosaveState>,
    mut queue: ResMut<SaveQueue>,
    services: NonSend<StorageServices>,
    session: Res<WorldSession>,
    mut status: ResMut<StorageStatus>,
) {
    state.timer.tick(time.delta());

    if let Some(task) = state.in_flight.as_mut() {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            match result {
                Ok(()) => status.mark_ok(),
                Err(error) => status.record_error(&error),
            }
            state.in_flight = None;
        }
    }

    if state.in_flight.is_some()
        || status.state == StorageState::Paused
        || !state.timer.just_finished()
        || queue.pending.is_empty()
    {
        return;
    }

    let mut batch = Vec::new();
    for _ in 0..state.max_per_flush {
        if let Some(record) = queue.pending.pop_front() {
            batch.push(record);
        } else {
            break;
        }
    }

    let storage = services.storage.clone();
    let world_id = session.world_id.clone();
    let task = AsyncComputeTaskPool::get().spawn_local(async move {
        storage.put_chunks(&world_id, batch).await
    });
    state.in_flight = Some(task);
}

fn chunk_load_pump_system(
    mut requests: EventReader<ChunkLoadRequest>,
    mut loaded: EventWriter<ChunkLoaded>,
    services: NonSend<StorageServices>,
    mut state: ResMut<ChunkLoadState>,
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
    }
    state.tasks = remaining;
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
