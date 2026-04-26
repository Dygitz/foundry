use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::prelude::ChildSpawnerCommands;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::SystemParam;
use bevy::image::ImageSampler;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseWheel;
use bevy::log::{info, warn};
use bevy::prelude::*;
use bevy::render::camera::{OrthographicProjection, Projection};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::futures_lite::future;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::window::{Window, WindowPlugin};
use persistence::{
    ChunkCodec, ChunkCodecV1, ChunkCoord, ChunkKey, ChunkLayer, ChunkRecordWrite, MapChunkRecord,
    MapChunkRecordWrite, PlayerStateRecordWrite, RecoveryReport, StorageError, WorldId,
    WorldStorage,
};
use simulation_core::{
    CHEST_SLOT_COUNT, CHUNK_EDGE, CHUNK_TILE_COUNT, ChestRecord, ContainerInv, FurnaceRecord,
    FurnaceSlot, FurnaceState, ITEM_CHEST, ITEM_COAL, ITEM_COPPER_ORE, ITEM_COPPER_PLATE,
    ITEM_FURNACE, ITEM_IRON_ORE, ITEM_IRON_PLATE, ITEM_NONE, ITEM_STONE, Inventory, ItemId,
    ObjectId, PLACED_CHEST, PLACED_FURNACE, PLACED_NONE, PlacedCell, PlacedId, RES_COAL,
    RES_COPPER, RES_IRON, RES_NONE, RES_STONE, ResourceCell, ResourceId, SimChunkData,
    SimChunkView, TileId, deposit_to_chest, deposit_to_furnace_fuel, deposit_to_furnace_input,
    take_from_chest, take_from_furnace,
};
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
        .insert_resource(PlayerState::default())
        .insert_resource(PlayerStateSaveState::default())
        .insert_resource(PlayerStateLoadState::default())
        .insert_resource(PlacementState::default())
        .insert_resource(UiState::default())
        .insert_resource(ClickHighlight::default())
        .insert_resource(DebugConfig::default())
        .insert_resource(EvictionStats::default())
        .insert_resource(AutosaveState::default())
        .insert_resource(SaveQueue::default())
        .insert_resource(ChunkLoadState::default())
        .insert_resource(MapState::default())
        .insert_resource(MapLoadState::default())
        .insert_resource(MapSaveState::default())
        .insert_resource(RecoveryState::default())
        .add_event::<ChunkLoadRequest>()
        .add_event::<ChunkLoaded>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .configure_sets(
            Update,
            (UpdateSet::Input, UpdateSet::Ui, UpdateSet::World).chain(),
        )
        .add_systems(Startup, (setup, init_storage))
        .add_systems(
            Update,
            (
                storage_init_pump_system,
                storage_recovery_pump_system,
                player_state_load_pump_system,
                autosave_flush_system,
                storage_status_text_system,
            ),
        )
        .add_systems(
            Update,
            (
                world_runtime_frame_counter_system,
                player_movement_system,
                player_visual_system,
                craft_menu_toggle_system,
                map_toggle_system,
                full_map_input_system,
                placement_select_system,
                ui_close_system,
                inventory_debug_input_system,
                crafting_input_system,
            )
                .in_set(UpdateSet::Input),
        )
        .add_systems(Update, (mining_input_system,).in_set(UpdateSet::Input))
        .add_systems(
            Update,
            (
                ui_visibility_system,
                inventory_text_system,
                placement_text_system,
                craft_menu_text_system,
                chest_ui_system,
                furnace_ui_system,
                minimap_visibility_system,
                map_load_pump_system,
                map_save_system,
                map_ui_render_system,
                chest_button_system,
                furnace_button_system,
            )
                .in_set(UpdateSet::Ui),
        )
        .add_systems(Update, (player_state_save_system,).in_set(UpdateSet::Ui))
        .add_systems(
            Update,
            (
                camera_follow_system,
                camera_zoom_system,
                active_area_chunk_request_system,
                chunk_load_pump_system,
                chunk_loaded_system,
                furnace_smelting_system,
                chunk_eviction_system,
                world_stats_text_system,
            )
                .in_set(UpdateSet::World),
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
        self.last_access_frame
            .insert(key.clone(), self.frame_counter);
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
            keep_radius_chunks: 6,
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
            move_speed: 160.0,
            camera_follow_lerp: 12.0,
        }
    }
}

#[derive(Resource)]
struct WorldRenderConfig {
    tile_size: f32,
    active_radius_chunks: i32,
    layer: ChunkLayer,
    show_chunk_borders: bool,
}

impl Default for WorldRenderConfig {
    fn default() -> Self {
        Self {
            tile_size: 16.0,
            active_radius_chunks: 3,
            layer: 0,
            show_chunk_borders: false,
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
            db_version: 3,
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

struct MapChunk {
    rgba: Vec<u8>,
    resource_kinds: Vec<ResourceId>,
    resource_amounts: Vec<u16>,
    image: Handle<Image>,
    updated_at_ms: u64,
}

#[derive(Resource, Default)]
struct MapState {
    explored: HashMap<ChunkKey, MapChunk>,
    pending_saves: VecDeque<MapChunkRecordWrite>,
    queued_for_save: HashSet<ChunkKey>,
    full_view: FullMapView,
    drag_last_cursor: Option<Vec2>,
}

#[derive(Debug, Clone, Copy)]
struct FullMapView {
    center_tile: Vec2,
    px_per_tile: f32,
}

impl Default for FullMapView {
    fn default() -> Self {
        Self {
            center_tile: Vec2::ZERO,
            px_per_tile: FULL_MAP_DEFAULT_PX_PER_TILE,
        }
    }
}

#[derive(Resource, Default)]
struct MapLoadState {
    task: Option<Task<Result<Vec<MapChunkRecord>, StorageError>>>,
    loaded: bool,
}

#[derive(Resource)]
struct MapSaveState {
    timer: Timer,
    max_per_flush: usize,
    in_flight: Option<MapSaveTask>,
}

impl Default for MapSaveState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            max_per_flush: 64,
            in_flight: None,
        }
    }
}

struct MapSaveTask {
    task: Task<Result<(), StorageError>>,
    pending_count: usize,
    keys: Vec<ChunkKey>,
}

#[derive(Component)]
struct StorageStatusText;

#[derive(Component)]
struct ChunkRenderTag;

#[derive(Component)]
struct WorldStatsText;

#[derive(Component)]
struct InventoryText;

#[derive(Component)]
struct CraftMenuText;

#[derive(Component)]
struct PlacementText;

#[derive(Component)]
struct UiOverlay;

#[derive(Component)]
struct UiPanelRoot;

#[derive(Component)]
struct MinimapRoot;

#[derive(Component, Copy, Clone, PartialEq, Eq)]
enum MapSurfaceKind {
    Minimap,
    Full,
}

#[derive(Component, Copy, Clone)]
struct MapContent {
    kind: MapSurfaceKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct MapResourceCell {
    kind: ResourceId,
    amount: u16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct ResourceNodeSummary {
    kind: ResourceId,
    total: u32,
}

#[derive(Component)]
struct CraftPanelText;

#[derive(Component)]
struct ChestPanel;

#[derive(Component)]
struct FurnacePanel;

#[derive(Component)]
struct ChestSlotButton {
    index: usize,
}

#[derive(Component)]
struct ChestSlotText {
    index: usize,
}

#[derive(Component)]
struct ChestDepositButton {
    item: ItemId,
}

#[derive(Component)]
struct FurnaceSlotButton {
    slot: FurnaceSlot,
}

#[derive(Component)]
struct FurnaceSlotText {
    slot: FurnaceSlot,
}

#[derive(Component)]
struct FurnaceDepositButton {
    slot: FurnaceSlot,
    item: ItemId,
}

#[derive(Component)]
struct FurnaceProgressBar;

#[derive(Component)]
struct Player;

#[derive(Component, Copy, Clone)]
struct Velocity(Vec2);

#[derive(Component, Copy, Clone)]
enum Facing {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Resource, Default)]
struct PlayerState {
    inventory: Inventory,
}

#[derive(Resource, Default)]
struct PlacementState {
    selected: Option<ItemId>,
}

#[derive(Resource, Default)]
struct UiState {
    mode: UiMode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum UiMode {
    None,
    Map,
    Crafting,
    Chest { object_id: ObjectId },
    Furnace { object_id: ObjectId },
}

impl Default for UiMode {
    fn default() -> Self {
        UiMode::None
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum UpdateSet {
    Input,
    Ui,
    World,
}

const MAP_CHUNK_BYTES: usize = CHUNK_TILE_COUNT * 4;
const MINIMAP_SIZE: f32 = 192.0;
const MINIMAP_MARGIN: f32 = 16.0;
const MINIMAP_PX_PER_TILE: f32 = 1.0;
const FULL_MAP_DEFAULT_PX_PER_TILE: f32 = 2.0;
const FULL_MAP_MIN_PX_PER_TILE: f32 = 0.25;
const FULL_MAP_MAX_PX_PER_TILE: f32 = 8.0;
const MAP_TOOLTIP_WIDTH: f32 = 168.0;
const MAP_TOOLTIP_HEIGHT: f32 = 28.0;
const MAP_TOOLTIP_OFFSET: f32 = 12.0;

#[derive(Resource)]
struct PlayerStateSaveState {
    timer: Timer,
    in_flight: Option<Task<Result<(), StorageError>>>,
    dirty: bool,
}

impl Default for PlayerStateSaveState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            in_flight: None,
            dirty: false,
        }
    }
}

#[derive(Resource, Default)]
struct PlayerStateLoadState {
    task: Option<Task<Result<Option<persistence::PlayerStateRecord>, StorageError>>>,
    loaded: bool,
}

#[derive(Resource, Default)]
struct ClickHighlight {
    tile: Option<(i32, i32)>,
}

#[derive(Resource)]
struct DebugConfig {
    log_mining: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self { log_mining: true }
    }
}

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

fn setup(
    mut commands: Commands,
    config: Res<WorldRenderConfig>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut camera = commands.spawn(Camera2d);
    camera.insert(Projection::Orthographic(OrthographicProjection {
        scale: 0.35,
        ..OrthographicProjection::default_2d()
    }));
    let player_texture = images.add(build_player_image());
    commands.spawn((
        Sprite {
            image: player_texture,
            custom_size: Some(Vec2::splat(config.tile_size * 0.95)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        Velocity(Vec2::ZERO),
        Facing::Down,
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
    commands.spawn((
        Text::new(
            "Inventory: Iron Ore 0 | Copper Ore 0 | Coal 0 | Stone 0\n\
Plates: Iron 0 | Copper 0 | Furnace 0 | Chest 0",
        ),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(52.0),
            ..default()
        },
        InventoryText,
    ));
    commands.spawn((
        Text::new("Crafting (E to open)"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
        CraftMenuText,
    ));
    commands.spawn((
        Text::new("Place: None (F furnace, C chest, Esc clear)"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(116.0),
            ..default()
        },
        PlacementText,
    ));
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Visibility::Hidden,
            UiOverlay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                UiPanelRoot,
            ));
        });
    commands
        .spawn((
            Node {
                width: Val::Px(MINIMAP_SIZE),
                height: Val::Px(MINIMAP_SIZE),
                position_type: PositionType::Absolute,
                right: Val::Px(MINIMAP_MARGIN),
                bottom: Val::Px(MINIMAP_MARGIN),
                overflow: Overflow::clip(),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(map_unknown_color()),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.35)),
            MinimapRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                MapContent {
                    kind: MapSurfaceKind::Minimap,
                },
            ));
        });
}

fn spawn_map_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(map_unknown_color()),
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                MapContent {
                    kind: MapSurfaceKind::Full,
                },
            ));
        });
}

fn spawn_crafting_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Px(500.0),
                height: Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.18)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Crafting"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
            ));
            panel.spawn((
                Text::new(
                    "1) Furnace: 10 Stone\n\
2) Chest: 10 Stone",
                ),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                CraftPanelText,
            ));
            panel.spawn((
                Text::new("Press 1-2 to craft. Esc to close."),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn spawn_chest_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Px(520.0),
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.14)),
            ChestPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Chest"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
            ));
            panel
                .spawn(Node {
                    display: Display::Flex,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|grid| {
                    for index in 0..CHEST_SLOT_COUNT {
                        grid.spawn((
                            Button,
                            Node {
                                width: Val::Px(60.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.18, 0.18, 0.2)),
                            ChestSlotButton { index },
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Empty"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                ChestSlotText { index },
                            ));
                        });
                    }
                });
            panel
                .spawn(Node {
                    display: Display::Flex,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    let items = [
                        (ITEM_IRON_ORE, "Deposit Iron"),
                        (ITEM_COPPER_ORE, "Deposit Copper"),
                        (ITEM_COAL, "Deposit Coal"),
                        (ITEM_STONE, "Deposit Stone"),
                        (ITEM_IRON_PLATE, "Deposit Iron Plate"),
                        (ITEM_COPPER_PLATE, "Deposit Copper Plate"),
                    ];
                    for (item, label) in items {
                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.2, 0.22)),
                            ChestDepositButton { item },
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                        });
                    }
                });
        });
}

fn spawn_furnace_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.14)),
            FurnacePanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Furnace"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
            ));
            let slots = [
                (FurnaceSlot::Input, "Input"),
                (FurnaceSlot::Fuel, "Fuel"),
                (FurnaceSlot::Output, "Output"),
            ];
            for (slot, label) in slots {
                panel
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(200.0),
                            height: Val::Px(24.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::new(
                                Val::Px(6.0),
                                Val::Px(6.0),
                                Val::Px(0.0),
                                Val::Px(0.0),
                            ),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.18, 0.18, 0.2)),
                        FurnaceSlotButton { slot },
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ));
                        button.spawn((
                            Text::new("Empty"),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            FurnaceSlotText { slot },
                        ));
                    });
            }
            panel
                .spawn(Node {
                    width: Val::Px(200.0),
                    height: Val::Px(10.0),
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Px(0.0),
                            height: Val::Px(10.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.8, 0.3)),
                        FurnaceProgressBar,
                    ));
                });
            panel
                .spawn(Node {
                    display: Display::Flex,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    let inputs = [
                        (FurnaceSlot::Input, ITEM_IRON_ORE, "Input Iron"),
                        (FurnaceSlot::Input, ITEM_COPPER_ORE, "Input Copper"),
                        (FurnaceSlot::Fuel, ITEM_COAL, "Fuel Coal"),
                    ];
                    for (slot, item, label) in inputs {
                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.2, 0.22)),
                            FurnaceDepositButton { slot, item },
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                        });
                    }
                });
        });
}

fn world_runtime_frame_counter_system(mut runtime: ResMut<WorldRuntime>) {
    runtime.advance_frame();
}

fn player_movement_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<PlayerConfig>,
    render_config: Res<WorldRenderConfig>,
    session: Res<WorldSession>,
    ui_state: Res<UiState>,
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut Facing), With<Player>>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
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

    let raw_direction = direction;
    let direction = direction.normalize_or_zero();
    let velocity = Velocity(direction * config.move_speed);
    let delta = velocity.0 * time.delta_secs();
    for (mut transform, mut current_velocity, mut facing) in &mut player_query {
        *current_velocity = velocity;
        let mut next = transform.translation;
        let next_x = next.x + delta.x;
        if can_walk(
            Vec2::new(next_x, next.y),
            &render_config,
            session.world_seed,
        ) {
            next.x = next_x;
        }
        let next_y = next.y + delta.y;
        if can_walk(
            Vec2::new(next.x, next_y),
            &render_config,
            session.world_seed,
        ) {
            next.y = next_y;
        }
        transform.translation = next;
        if raw_direction != Vec2::ZERO {
            if raw_direction.x.abs() >= raw_direction.y.abs() {
                *facing = if raw_direction.x > 0.0 {
                    Facing::Right
                } else {
                    Facing::Left
                };
            } else {
                *facing = if raw_direction.y > 0.0 {
                    Facing::Up
                } else {
                    Facing::Down
                };
            }
        }
    }
}

fn player_visual_system(mut player_query: Query<(&Facing, &mut Sprite), With<Player>>) {
    for (facing, mut sprite) in &mut player_query {
        match facing {
            Facing::Left => sprite.flip_x = true,
            Facing::Right => sprite.flip_x = false,
            Facing::Up | Facing::Down => {}
        }
    }
}

fn craft_menu_toggle_system(keys: Res<ButtonInput<KeyCode>>, mut ui_state: ResMut<UiState>) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    match ui_state.mode {
        UiMode::None => ui_state.mode = UiMode::Crafting,
        UiMode::Crafting => ui_state.mode = UiMode::None,
        _ => {}
    }
}

fn map_toggle_system(
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

fn full_map_input_system(
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

fn ui_close_system(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut ui_state: ResMut<UiState>,
) {
    if keys.just_pressed(KeyCode::Escape) || buttons.just_pressed(MouseButton::Right) {
        ui_state.mode = UiMode::None;
    }
}

fn ui_visibility_system(
    mut commands: Commands,
    ui_state: Res<UiState>,
    mut overlay_query: Query<&mut Visibility, With<UiOverlay>>,
    panel_query: Query<Entity, With<UiPanelRoot>>,
    children_query: Query<&Children, With<UiPanelRoot>>,
) {
    if !ui_state.is_changed() {
        return;
    }
    let Ok(mut overlay_visibility) = overlay_query.single_mut() else {
        return;
    };
    let Ok(panel_entity) = panel_query.single() else {
        return;
    };

    if let Ok(children) = children_query.get(panel_entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    match ui_state.mode {
        UiMode::None => {
            *overlay_visibility = Visibility::Hidden;
        }
        UiMode::Map => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_map_panel(parent));
        }
        UiMode::Crafting => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_crafting_panel(parent));
        }
        UiMode::Chest { .. } => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_chest_panel(parent));
        }
        UiMode::Furnace { .. } => {
            *overlay_visibility = Visibility::Visible;
            commands
                .entity(panel_entity)
                .with_children(|parent| spawn_furnace_panel(parent));
        }
    }
}

fn placement_select_system(
    keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    mut placement: ResMut<PlacementState>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        placement.selected = Some(ITEM_FURNACE);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        placement.selected = Some(ITEM_CHEST);
    }
    if keys.just_pressed(KeyCode::Escape) {
        placement.selected = None;
    }
}

fn crafting_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    mut player: ResMut<PlayerState>,
) {
    if ui_state.mode != UiMode::Crafting {
        return;
    }

    if keys.just_pressed(KeyCode::Digit1) {
        let _ = try_craft(&mut player.inventory, &RECIPE_FURNACE);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        let _ = try_craft(&mut player.inventory, &RECIPE_CHEST);
    }
}

fn inventory_debug_input_system(
    _keys: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    mut player: ResMut<PlayerState>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
    let _ = &mut player;
}

#[derive(SystemParam)]
struct MiningParams<'w, 's> {
    config: Res<'w, WorldRenderConfig>,
    session: Res<'w, WorldSession>,
    runtime: ResMut<'w, WorldRuntime>,
    player: ResMut<'w, PlayerState>,
    player_query: Query<'w, 's, &'static Transform, With<Player>>,
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
}

fn mining_input_system(
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

fn resource_to_item(kind: ResourceId) -> Option<ItemId> {
    match kind {
        RES_IRON => Some(ITEM_IRON_ORE),
        RES_COPPER => Some(ITEM_COPPER_ORE),
        RES_COAL => Some(ITEM_COAL),
        RES_STONE => Some(ITEM_STONE),
        _ => None,
    }
}

struct Recipe {
    output: ItemId,
    output_amount: u32,
    inputs: &'static [(ItemId, u32)],
}

const RECIPE_FURNACE: Recipe = Recipe {
    output: ITEM_FURNACE,
    output_amount: 1,
    inputs: &[(ITEM_STONE, 10)],
};

const RECIPE_CHEST: Recipe = Recipe {
    output: ITEM_CHEST,
    output_amount: 1,
    inputs: &[(ITEM_STONE, 10)],
};

const FURNACE_PROGRESS_PER_ITEM: u16 = 1000;
const FURNACE_SECONDS_PER_ITEM: f32 = 2.0;
const FURNACE_PROGRESS_PER_SEC: f32 = FURNACE_PROGRESS_PER_ITEM as f32 / FURNACE_SECONDS_PER_ITEM;

fn smelt_output_for_input(item: ItemId) -> Option<ItemId> {
    match item {
        ITEM_IRON_ORE => Some(ITEM_IRON_PLATE),
        ITEM_COPPER_ORE => Some(ITEM_COPPER_PLATE),
        _ => None,
    }
}

fn try_craft(inv: &mut Inventory, recipe: &Recipe) -> bool {
    for (item, amount) in recipe.inputs {
        if inv.count(*item) < *amount {
            return false;
        }
    }
    for (item, amount) in recipe.inputs {
        let _ = inv.try_remove(*item, *amount);
    }
    inv.add(recipe.output, recipe.output_amount);
    true
}

fn item_name(item: ItemId) -> &'static str {
    match item {
        ITEM_IRON_ORE => "Iron Ore",
        ITEM_COPPER_ORE => "Copper Ore",
        ITEM_COAL => "Coal",
        ITEM_STONE => "Stone",
        ITEM_IRON_PLATE => "Iron Plate",
        ITEM_COPPER_PLATE => "Copper Plate",
        ITEM_FURNACE => "Furnace",
        ITEM_CHEST => "Chest",
        _ => "Unknown",
    }
}

fn refresh_highlight_chunk(
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
enum MineAttempt {
    Mined(ResourceId),
    Empty,
    ChunkMissing,
}

impl MineAttempt {
    fn is_mined(self) -> bool {
        matches!(self, MineAttempt::Mined(_))
    }
}

fn try_mine_at_world_pos(
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
    log: bool,
    highlight: Option<(i32, i32)>,
) -> bool {
    let tile_x = (world_pos.x / config.tile_size).floor() as i32;
    let tile_y = (world_pos.y / config.tile_size).floor() as i32;
    let mined = try_mine_tile(
        tile_x, tile_y, config, session, runtime, player, images, map, services, time, queue,
        status, log, highlight,
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

fn try_mine_tile(
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
    MineAttempt::Mined(mined_kind)
}

fn item_to_placed_kind(item: ItemId) -> Option<PlacedId> {
    match item {
        ITEM_FURNACE => Some(PLACED_FURNACE),
        ITEM_CHEST => Some(PLACED_CHEST),
        _ => None,
    }
}

fn try_place_at_world_pos(
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

fn try_place_tile(
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

fn inventory_text_system(
    player: Res<PlayerState>,
    mut query: Query<&mut Text, With<InventoryText>>,
) {
    if !player.is_changed() {
        return;
    }
    let inv = &player.inventory;
    let label = format!(
        "Inventory: Iron Ore {} | Copper Ore {} | Coal {} | Stone {}\nPlates: Iron {} | Copper {} | Furnace {} | Chest {}",
        inv.count(ITEM_IRON_ORE),
        inv.count(ITEM_COPPER_ORE),
        inv.count(ITEM_COAL),
        inv.count(ITEM_STONE),
        inv.count(ITEM_IRON_PLATE),
        inv.count(ITEM_COPPER_PLATE),
        inv.count(ITEM_FURNACE),
        inv.count(ITEM_CHEST),
    );
    for mut text in &mut query {
        *text = Text::new(label.clone());
    }
}

fn craft_menu_text_system(
    ui_state: Res<UiState>,
    mut query: Query<&mut Text, With<CraftMenuText>>,
) {
    if !ui_state.is_changed() {
        return;
    }
    let label = if matches!(ui_state.mode, UiMode::Crafting) {
        "Crafting (E to close)"
    } else {
        "Crafting (E to open)"
    };
    for mut text in &mut query {
        *text = Text::new(label.to_string());
    }
}

fn placement_text_system(
    player: Res<PlayerState>,
    mut placement: ResMut<PlacementState>,
    mut query: Query<&mut Text, With<PlacementText>>,
) {
    if !player.is_changed() && !placement.is_changed() {
        return;
    }
    if let Some(item) = placement.selected {
        if player.inventory.count(item) == 0 {
            placement.selected = None;
        }
    }
    let label = if let Some(item) = placement.selected {
        format!(
            "Place: {} x{} (F furnace, C chest, Esc clear)",
            item_name(item),
            player.inventory.count(item)
        )
    } else {
        "Place: None (F furnace, C chest, Esc clear)".to_string()
    };
    for mut text in &mut query {
        *text = Text::new(label.clone());
    }
}

fn minimap_visibility_system(
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

fn map_load_pump_system(
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

fn map_save_system(
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

fn map_ui_render_system(
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
                    window_size.x - MINIMAP_MARGIN - MINIMAP_SIZE,
                    window_size.y - MINIMAP_MARGIN - MINIMAP_SIZE,
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
            spawn_map_chunk_nodes(parent, &map, center_tile, px_per_tile, viewport);
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

fn chest_ui_system(
    ui_state: Res<UiState>,
    runtime: Res<WorldRuntime>,
    mut slot_texts: Query<(&ChestSlotText, &mut Text)>,
) {
    let UiMode::Chest { object_id } = ui_state.mode else {
        return;
    };
    let chest = find_chest(&runtime, object_id);
    for (slot, mut text) in &mut slot_texts {
        let label = match chest.and_then(|chest| chest.inv.slots.get(slot.index)) {
            Some(slot) if !slot.is_empty() => {
                format!("{} x{}", item_name(slot.item), slot.count)
            }
            _ => "Empty".to_string(),
        };
        *text = Text::new(label);
    }
}

fn furnace_ui_system(
    ui_state: Res<UiState>,
    runtime: Res<WorldRuntime>,
    mut slot_texts: Query<(&FurnaceSlotText, &mut Text)>,
    mut bar_query: Query<&mut Node, With<FurnaceProgressBar>>,
) {
    let UiMode::Furnace { object_id } = ui_state.mode else {
        return;
    };
    let furnace = find_furnace(&runtime, object_id);
    for (slot, mut text) in &mut slot_texts {
        let slot_ref = furnace.map(|furnace| match slot.slot {
            FurnaceSlot::Input => furnace.state.input,
            FurnaceSlot::Fuel => furnace.state.fuel,
            FurnaceSlot::Output => furnace.state.output,
        });
        let label = match slot_ref {
            Some(slot) if !slot.is_empty() => {
                format!("{} x{}", item_name(slot.item), slot.count)
            }
            _ => "Empty".to_string(),
        };
        *text = Text::new(label);
    }
    let width = furnace
        .map(|furnace| {
            200.0
                * (furnace.state.progress as f32 / FURNACE_PROGRESS_PER_ITEM as f32).clamp(0.0, 1.0)
        })
        .unwrap_or(0.0);
    for mut node in &mut bar_query {
        node.width = Val::Px(width);
    }
}

fn chest_button_system(
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

fn furnace_button_system(
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

fn furnace_smelting_system(
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

fn camera_follow_system(
    time: Res<Time>,
    config: Res<PlayerConfig>,
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let mut target = player_transform.translation;
    target.z = camera_transform.translation.z;
    let t = 1.0 - (-config.camera_follow_lerp * time.delta_secs()).exp();
    camera_transform.translation = camera_transform.translation.lerp(target, t);
}

fn camera_zoom_system(
    mut scroll: EventReader<MouseWheel>,
    ui_state: Res<UiState>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if ui_state.mode != UiMode::None {
        return;
    }
    let Ok(mut projection) = camera_query.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = &mut *projection else {
        return;
    };

    for ev in scroll.read() {
        let factor = (1.0 - ev.y * 0.1).clamp(0.7, 1.3);
        ortho.scale = (ortho.scale * factor).clamp(0.25, 0.9);
    }
}

fn active_area_chunk_request_system(
    windows: Query<&Window>,
    camera_query: Query<(&Transform, &Projection, &Camera), With<Camera2d>>,
    config: Res<WorldRenderConfig>,
    cache: Res<ChunkCacheConfig>,
    session: Res<WorldSession>,
    mut runtime: ResMut<WorldRuntime>,
    mut requests: EventWriter<ChunkLoadRequest>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera_transform, projection, camera)) = camera_query.single() else {
        return;
    };

    let chunk_size = chunk_world_size(&config);
    let viewport = camera
        .logical_viewport_size()
        .unwrap_or_else(|| Vec2::new(window.width(), window.height()));
    let scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };
    let camera_pos = camera_transform.translation.truncate();
    let center_coord = world_to_chunk_coord(camera_pos, chunk_size);
    let margin = config.active_radius_chunks.max(1);
    let (rx, ry) = required_radius_chunks(viewport, scale, chunk_size, margin);
    let keep_rx = (rx + cache.keep_radius_chunks).max(rx);
    let keep_ry = (ry + cache.keep_radius_chunks).max(ry);
    let mut active_set = HashSet::new();
    let mut keep_set = HashSet::new();

    for dy in -ry..=ry {
        for dx in -rx..=rx {
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
    for dy in -keep_ry..=keep_ry {
        for dx in -keep_rx..=keep_rx {
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

fn player_state_load_pump_system(
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

fn player_state_save_system(
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

fn queue_chunk_save(
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
    let mut tiles = Vec::with_capacity(CHUNK_TILE_COUNT);
    for y in 0..edge {
        let gy = base_y + y as i32;
        for x in 0..edge {
            let gx = base_x + x as i32;
            let tile = terrain_tile_id(gx, gy, layer, world_seed);
            tiles.push(tile);
        }
    }
    let resources = generate_resources(coord, layer, world_seed, &tiles);
    let placed = vec![
        PlacedCell {
            kind: PLACED_NONE,
            object_id: 0,
        };
        CHUNK_TILE_COUNT
    ];
    SimChunkData {
        coord,
        layer,
        tiles,
        resources,
        placed,
        chests: Vec::new(),
        furnaces: Vec::new(),
        entities: Vec::new(),
        saved_tick,
    }
}

fn generate_resources(
    coord: ChunkCoord,
    layer: ChunkLayer,
    world_seed: u64,
    tiles: &[TileId],
) -> Vec<ResourceCell> {
    let edge = CHUNK_EDGE as usize;
    let base_x = coord.cx * CHUNK_EDGE as i32;
    let base_y = coord.cy * CHUNK_EDGE as i32;
    let mut resources = Vec::with_capacity(CHUNK_TILE_COUNT);

    for y in 0..edge {
        for x in 0..edge {
            let idx = y * edge + x;
            if tiles.get(idx).copied().map(is_water).unwrap_or(false) {
                resources.push(ResourceCell {
                    kind: RES_NONE,
                    amount: 0,
                });
                continue;
            }
            let gx = base_x + x as i32;
            let gy = base_y + y as i32;
            resources.push(resource_at_global(gx, gy, layer, world_seed));
        }
    }

    resources
}

fn pick_resource_kind(value: u32) -> ResourceId {
    let roll = value % 100;
    if roll < 40 {
        RES_IRON
    } else if roll < 65 {
        RES_COPPER
    } else if roll < 85 {
        RES_COAL
    } else {
        RES_STONE
    }
}

fn resource_at_global(gx: i32, gy: i32, layer: ChunkLayer, world_seed: u64) -> ResourceCell {
    const ORE_CELL_SIZE: i32 = 48;
    const ORE_MIN_RADIUS: i32 = 4;
    const ORE_MAX_RADIUS: i32 = 8;
    const ORE_PATCH_CHANCE: u32 = 30;
    const ORE_CELL_MARGIN: i32 = ORE_MAX_RADIUS + 1;

    let max_offset = ORE_CELL_SIZE - (ORE_CELL_MARGIN * 2);
    if max_offset <= 0 {
        return ResourceCell {
            kind: RES_NONE,
            amount: 0,
        };
    }

    let seed = world_seed ^ (layer as u64).wrapping_mul(0x7f4a7c15d14b5b5d);
    let cell_x = gx.div_euclid(ORE_CELL_SIZE);
    let cell_y = gy.div_euclid(ORE_CELL_SIZE);
    let mut best_amount = 0u16;
    let mut best_kind = RES_NONE;

    for cy in (cell_y - 1)..=(cell_y + 1) {
        for cx in (cell_x - 1)..=(cell_x + 1) {
            let cell_seed = mix64(
                seed ^ (cx as i64 as u64).wrapping_mul(0x9e3779b97f4a7c15)
                    ^ (cy as i64 as u64).wrapping_mul(0xc2b2ae3d27d4eb4f),
            );
            if (cell_seed as u32 % 100) >= ORE_PATCH_CHANCE {
                continue;
            }

            let kind = pick_resource_kind(((cell_seed >> 8) as u32) % 100);
            let offset_x = ((cell_seed >> 16) as u32 % max_offset as u32) as i32;
            let offset_y = ((cell_seed >> 24) as u32 % max_offset as u32) as i32;
            let center_x = cx * ORE_CELL_SIZE + ORE_CELL_MARGIN + offset_x;
            let center_y = cy * ORE_CELL_SIZE + ORE_CELL_MARGIN + offset_y;
            let radius_range = (ORE_MAX_RADIUS - ORE_MIN_RADIUS + 1) as u32;
            let radius = ORE_MIN_RADIUS + ((cell_seed >> 32) as u32 % radius_range) as i32;
            let base_amount = 18 + ((cell_seed >> 40) as u32 % 60) as i32;
            let dx = gx - center_x;
            let dy = gy - center_y;
            let dist_sq = dx * dx + dy * dy;
            let radius_sq = radius * radius;
            if dist_sq > radius_sq {
                continue;
            }
            let falloff = (dist_sq * base_amount) / (radius_sq + 1);
            let amount = (base_amount - falloff).max(0) as u16;
            if amount > best_amount {
                best_amount = amount;
                best_kind = kind;
            }
        }
    }

    if best_amount == 0 {
        ResourceCell {
            kind: RES_NONE,
            amount: 0,
        }
    } else {
        ResourceCell {
            kind: best_kind,
            amount: best_amount,
        }
    }
}

fn terrain_tile_id(gx: i32, gy: i32, layer: ChunkLayer, world_seed: u64) -> TileId {
    let seed = world_seed ^ (layer as u64).wrapping_mul(0x9e3779b97f4a7c15);
    let coarse_x = gx >> 4;
    let coarse_y = gy >> 4;
    let h = terrain_hash(coarse_x, coarse_y, seed);
    let v = (h & 0xFFFF) as u16;
    let variant = (terrain_hash(gx, gy, seed ^ 0x5bf03635f7d13d9b) >> 8) as u8;
    if v < 5000 {
        WATER_TILE
    } else if v < 16000 {
        4 + (variant % 2) as TileId
    } else {
        (variant % 4) as TileId
    }
}

fn chunk_world_size(config: &WorldRenderConfig) -> f32 {
    config.tile_size * CHUNK_EDGE as f32
}

fn required_radius_chunks(viewport: Vec2, scale: f32, chunk_size: f32, margin: i32) -> (i32, i32) {
    let half_w = (viewport.x * 0.5) * scale;
    let half_h = (viewport.y * 0.5) * scale;
    let rx = (half_w / chunk_size).ceil() as i32 + margin;
    let ry = (half_h / chunk_size).ceil() as i32 + margin;
    (rx.max(0), ry.max(0))
}

fn world_pos_to_tile_pos(world_pos: Vec2, config: &WorldRenderConfig) -> Vec2 {
    world_pos / config.tile_size
}

fn upsert_map_snapshot(
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

fn queue_map_snapshot_save(
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

fn clear_map_content(
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

fn spawn_map_chunk_nodes(
    parent: &mut ChildSpawnerCommands,
    map: &MapState,
    center_tile: Vec2,
    px_per_tile: f32,
    viewport: Vec2,
) {
    let chunk_tiles = CHUNK_EDGE as f32;
    let chunk_px = chunk_tiles * px_per_tile;
    for (key, chunk) in &map.explored {
        let min_x = key.coord.cx as f32 * chunk_tiles;
        let max_y = (key.coord.cy as f32 + 1.0) * chunk_tiles;
        let left = viewport.x * 0.5 + (min_x - center_tile.x) * px_per_tile;
        let top = viewport.y * 0.5 - (max_y - center_tile.y) * px_per_tile;

        if left > viewport.x || top > viewport.y || left + chunk_px < 0.0 || top + chunk_px < 0.0 {
            continue;
        }

        parent.spawn((
            ImageNode::new(chunk.image.clone()),
            Node {
                width: Val::Px(chunk_px),
                height: Val::Px(chunk_px),
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                ..default()
            },
        ));
    }
}

fn spawn_map_player_marker(
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

fn spawn_map_resource_tooltip(
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

fn map_local_cursor_to_tile(
    cursor: Vec2,
    center_tile: Vec2,
    px_per_tile: f32,
    viewport: Vec2,
) -> (i32, i32) {
    let tile_x = center_tile.x + (cursor.x - viewport.x * 0.5) / px_per_tile;
    let tile_y = center_tile.y + (viewport.y * 0.5 - cursor.y) / px_per_tile;
    (tile_x.floor() as i32, tile_y.floor() as i32)
}

fn map_resource_node_summary(
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

fn map_resource_at(
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

fn map_resource_metadata(data: &SimChunkData) -> (Vec<ResourceId>, Vec<u16>) {
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

fn normalize_map_resource_metadata(
    resource_kinds: Vec<ResourceId>,
    resource_amounts: Vec<u16>,
) -> (Vec<ResourceId>, Vec<u16>) {
    if resource_kinds.len() == CHUNK_TILE_COUNT && resource_amounts.len() == CHUNK_TILE_COUNT {
        return (resource_kinds, resource_amounts);
    }
    (vec![RES_NONE; CHUNK_TILE_COUNT], vec![0; CHUNK_TILE_COUNT])
}

fn resource_display_name(kind: ResourceId) -> &'static str {
    match kind {
        RES_IRON => "Iron Ore",
        RES_COPPER => "Copper Ore",
        RES_COAL => "Coal",
        RES_STONE => "Stone",
        _ => "Resource",
    }
}

fn build_map_chunk_image(rgba: &[u8]) -> Image {
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

fn map_snapshot_pixels(data: &SimChunkData, world_seed: u64) -> Vec<u8> {
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

fn map_unknown_color() -> Color {
    Color::srgb(0.18, 0.18, 0.2)
}

const PS_MAGIC: [u8; 4] = *b"PLR1";
const PS_VERSION: u16 = 1;

fn encode_player_state_v1(inv: &Inventory) -> Vec<u8> {
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

fn decode_player_state_v1(bytes: &[u8]) -> Result<Inventory, String> {
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

fn world_to_chunk_coord(world_pos: Vec2, chunk_size: f32) -> ChunkCoord {
    ChunkCoord::new(
        (world_pos.x / chunk_size).floor() as i32,
        (world_pos.y / chunk_size).floor() as i32,
    )
}

fn tile_to_chunk_local(tile_x: i32, tile_y: i32) -> (ChunkCoord, i32, i32) {
    let edge = CHUNK_EDGE as i32;
    let cx = tile_x.div_euclid(edge);
    let cy = tile_y.div_euclid(edge);
    let local_x = tile_x.rem_euclid(edge);
    let local_y = tile_y.rem_euclid(edge);
    (ChunkCoord::new(cx, cy), local_x, local_y)
}

fn chunk_center_world(coord: ChunkCoord, chunk_size: f32) -> Vec2 {
    Vec2::new(
        (coord.cx as f32 + 0.5) * chunk_size,
        (coord.cy as f32 + 0.5) * chunk_size,
    )
}

fn open_panel_for_tile(
    tile_x: i32,
    tile_y: i32,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &WorldRuntime,
) -> Option<UiMode> {
    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
    let Some(loaded) = runtime.loaded.get(&key) else {
        return None;
    };
    let edge = CHUNK_EDGE as i32;
    let idx = (local_y as usize) * (edge as usize) + (local_x as usize);
    let cell = loaded.data.placed.get(idx)?;
    if cell.kind == PLACED_CHEST && cell.object_id != 0 {
        return Some(UiMode::Chest {
            object_id: cell.object_id,
        });
    }
    if cell.kind == PLACED_FURNACE && cell.object_id != 0 {
        return Some(UiMode::Furnace {
            object_id: cell.object_id,
        });
    }
    None
}

fn find_chest<'a>(runtime: &'a WorldRuntime, object_id: ObjectId) -> Option<&'a ChestRecord> {
    runtime
        .loaded
        .values()
        .find_map(|loaded| loaded.data.chests.iter().find(|c| c.object_id == object_id))
}

fn find_furnace<'a>(runtime: &'a WorldRuntime, object_id: ObjectId) -> Option<&'a FurnaceRecord> {
    runtime.loaded.values().find_map(|loaded| {
        loaded
            .data
            .furnaces
            .iter()
            .find(|f| f.object_id == object_id)
    })
}

fn with_chest_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .chests
            .iter()
            .any(|chest| chest.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}

fn with_furnace_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .furnaces
            .iter()
            .any(|furnace| furnace.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}

fn object_id_for_tile(world_seed: u64, gx: i32, gy: i32, kind: PlacedId) -> ObjectId {
    let mut z = world_seed ^ (kind as u64).wrapping_mul(0x9e3779b97f4a7c15);
    z ^= (gx as i64 as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    z ^= (gy as i64 as u64).wrapping_mul(0x94d049bb133111eb);
    let id = mix64(z);
    if id == 0 { 1 } else { id }
}

fn build_player_image() -> Image {
    let size = 16usize;
    let mut filled = vec![false; size * size];

    let mut fill_rect = |x0: usize, y0: usize, x1: usize, y1: usize| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                filled[y * size + x] = true;
            }
        }
    };

    fill_rect(5, 2, 10, 5); // head
    fill_rect(4, 6, 11, 11); // body
    fill_rect(4, 12, 6, 13); // left foot
    fill_rect(9, 12, 11, 13); // right foot

    let is_filled = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
            return false;
        }
        filled[y as usize * size + x as usize]
    };

    let is_foot =
        |x: usize, y: usize| (y >= 12 && y <= 13) && ((x >= 4 && x <= 6) || (x >= 9 && x <= 11));

    let outline_color = [28, 22, 18, 255];
    let body_color = [226, 205, 124, 255];
    let foot_color = [190, 168, 96, 255];
    let mut pixels = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            if !filled[y * size + x] {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let outline = !is_filled(x as i32 - 1, y as i32)
                || !is_filled(x as i32 + 1, y as i32)
                || !is_filled(x as i32, y as i32 - 1)
                || !is_filled(x as i32, y as i32 + 1);
            let color = if outline {
                outline_color
            } else if is_foot(x, y) {
                foot_color
            } else {
                body_color
            };
            pixels.extend_from_slice(&color);
        }
    }

    let mut image = Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
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

fn build_chunk_image(
    data: &SimChunkData,
    config: &WorldRenderConfig,
    world_seed: u64,
    highlight: Option<(i32, i32)>,
) -> Image {
    let pixels = chunk_pixels(data, config, world_seed, highlight);
    let padded_edge = CHUNK_EDGE as u32 + 2;
    let mut image = Image::new_fill(
        Extent3d {
            width: padded_edge,
            height: padded_edge,
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

fn chunk_sprite_rect() -> Rect {
    let edge = CHUNK_EDGE as f32;
    Rect::from_corners(Vec2::new(1.0, 1.0), Vec2::new(edge + 1.0, edge + 1.0))
}

fn refresh_chunk_texture(
    images: &mut Assets<Image>,
    handle: &Handle<Image>,
    data: &SimChunkData,
    config: &WorldRenderConfig,
    world_seed: u64,
    highlight: Option<(i32, i32)>,
) {
    if let Some(image) = images.get_mut(handle) {
        *image = build_chunk_image(data, config, world_seed, highlight);
    }
}

fn chunk_pixels(
    data: &SimChunkData,
    config: &WorldRenderConfig,
    world_seed: u64,
    highlight: Option<(i32, i32)>,
) -> Vec<u8> {
    let edge = CHUNK_EDGE as usize;
    let padded_edge = edge + 2;
    let mut pixels = Vec::with_capacity(padded_edge * padded_edge * 4);

    for oy in 0..padded_edge {
        let base_ty = if oy == 0 {
            0
        } else if oy > edge {
            edge - 1
        } else {
            oy - 1
        };
        let ty = edge - 1 - base_ty;
        let interior_y = oy as i32 - 1;
        for ox in 0..padded_edge {
            let tx = if ox == 0 {
                0
            } else if ox > edge {
                edge - 1
            } else {
                ox - 1
            };
            let interior_x = ox as i32 - 1;
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
                let overlay = resource_color(resource.kind);
                color = blend_color(color, overlay, 0.85);
            }
            let placed = placed_at(data, tx as i32, ty as i32);
            if placed.kind != PLACED_NONE {
                let overlay = placed_color(placed.kind);
                color = blend_color(color, overlay, 0.9);
            }
            if config.show_chunk_borders
                && interior_x >= 0
                && interior_y >= 0
                && (interior_x == 0 || interior_y == 0)
            {
                color = darken_color(color);
            }
            if let Some((hx, hy)) = highlight {
                if gx == hx && gy == hy {
                    color = [220, 40, 40, 255];
                }
            }
            pixels.extend_from_slice(&color);
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

fn shallow_water_color() -> [u8; 4] {
    [70, 120, 190, 255]
}

fn resource_color(kind: ResourceId) -> [u8; 4] {
    match kind {
        RES_IRON => [180, 180, 190, 255],
        RES_COPPER => [190, 120, 60, 255],
        RES_COAL => [30, 30, 30, 255],
        RES_STONE => [120, 120, 120, 255],
        _ => [0, 0, 0, 0],
    }
}

fn placed_color(kind: PlacedId) -> [u8; 4] {
    match kind {
        PLACED_FURNACE => [90, 90, 100, 255],
        PLACED_CHEST => [150, 95, 55, 255],
        _ => [0, 0, 0, 0],
    }
}

fn darken_color(color: [u8; 4]) -> [u8; 4] {
    let r = (color[0] as u16 * 4 / 5) as u8;
    let g = (color[1] as u16 * 4 / 5) as u8;
    let b = (color[2] as u16 * 4 / 5) as u8;
    [r, g, b, color[3]]
}

fn blend_color(base: [u8; 4], overlay: [u8; 4], overlay_weight: f32) -> [u8; 4] {
    let t = overlay_weight.clamp(0.0, 1.0);
    let blend = |b: u8, o: u8| -> u8 {
        let bf = b as f32;
        let of = o as f32;
        (bf * (1.0 - t) + of * t).round().clamp(0.0, 255.0) as u8
    };
    [
        blend(base[0], overlay[0]),
        blend(base[1], overlay[1]),
        blend(base[2], overlay[2]),
        base[3],
    ]
}

fn apply_jitter(color: [u8; 4], jitter: i8) -> [u8; 4] {
    let adjust = |value: u8| -> u8 {
        let v = value as i16 + jitter as i16;
        v.clamp(0, 255) as u8
    };
    [
        adjust(color[0]),
        adjust(color[1]),
        adjust(color[2]),
        color[3],
    ]
}

fn tile_jitter(gx: i32, gy: i32, world_seed: u64, tile: TileId) -> i8 {
    let seed = world_seed ^ (tile as u64).wrapping_mul(0x94d049bb133111eb);
    let h = terrain_hash(gx, gy, seed);
    let range = if tile == WATER_TILE { 3 } else { 6 };
    let offset = (h % ((range * 2 + 1) as u32)) as i8 - range;
    offset
}

fn tile_at(data: &SimChunkData, tx: i32, ty: i32, world_seed: u64) -> TileId {
    let edge = CHUNK_EDGE as i32;
    if tx >= 0 && tx < edge && ty >= 0 && ty < edge {
        let idx = (ty as usize) * (edge as usize) + (tx as usize);
        return data.tiles.get(idx).copied().unwrap_or(0);
    }
    let gx = data.coord.cx * CHUNK_EDGE as i32 + tx;
    let gy = data.coord.cy * CHUNK_EDGE as i32 + ty;
    terrain_tile_id(gx, gy, data.layer, world_seed)
}

fn resource_at(data: &SimChunkData, tx: i32, ty: i32) -> ResourceCell {
    let edge = CHUNK_EDGE as i32;
    if tx >= 0 && tx < edge && ty >= 0 && ty < edge {
        let idx = (ty as usize) * (edge as usize) + (tx as usize);
        return data.resources.get(idx).copied().unwrap_or(ResourceCell {
            kind: RES_NONE,
            amount: 0,
        });
    }
    ResourceCell {
        kind: RES_NONE,
        amount: 0,
    }
}

fn placed_at(data: &SimChunkData, tx: i32, ty: i32) -> PlacedCell {
    let edge = CHUNK_EDGE as i32;
    if tx >= 0 && tx < edge && ty >= 0 && ty < edge {
        let idx = (ty as usize) * (edge as usize) + (tx as usize);
        return data.placed.get(idx).copied().unwrap_or(PlacedCell {
            kind: PLACED_NONE,
            object_id: 0,
        });
    }
    PlacedCell {
        kind: PLACED_NONE,
        object_id: 0,
    }
}

fn is_water(tile: TileId) -> bool {
    tile == WATER_TILE
}

fn can_walk(world_pos: Vec2, config: &WorldRenderConfig, world_seed: u64) -> bool {
    let tx = (world_pos.x / config.tile_size).floor() as i32;
    let ty = (world_pos.y / config.tile_size).floor() as i32;
    let tile = terrain_tile_id(tx, ty, config.layer, world_seed);
    !is_water(tile)
}

const WATER_TILE: TileId = 6;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_chunk() -> SimChunkData {
        SimChunkData {
            coord: ChunkCoord::new(0, 0),
            layer: 0,
            tiles: vec![0; CHUNK_TILE_COUNT],
            resources: vec![
                ResourceCell {
                    kind: RES_NONE,
                    amount: 0,
                };
                CHUNK_TILE_COUNT
            ],
            placed: vec![
                PlacedCell {
                    kind: PLACED_NONE,
                    object_id: 0,
                };
                CHUNK_TILE_COUNT
            ],
            chests: Vec::new(),
            furnaces: Vec::new(),
            entities: Vec::new(),
            saved_tick: 0,
        }
    }

    fn local_idx(tx: usize, ty: usize) -> usize {
        ty * CHUNK_EDGE as usize + tx
    }

    fn map_pixel(pixels: &[u8], tx: usize, ty: usize) -> [u8; 4] {
        let edge = CHUNK_EDGE as usize;
        let row = edge - 1 - ty;
        let idx = (row * edge + tx) * 4;
        [
            pixels[idx],
            pixels[idx + 1],
            pixels[idx + 2],
            pixels[idx + 3],
        ]
    }

    fn chunk_pixel(pixels: &[u8], tx: usize, ty: usize) -> [u8; 4] {
        let edge = CHUNK_EDGE as usize;
        let padded = edge + 2;
        let ox = tx + 1;
        let oy = edge - ty;
        let idx = (oy * padded + ox) * 4;
        [
            pixels[idx],
            pixels[idx + 1],
            pixels[idx + 2],
            pixels[idx + 3],
        ]
    }

    fn blank_map_chunk() -> MapChunk {
        MapChunk {
            rgba: vec![0; MAP_CHUNK_BYTES],
            resource_kinds: vec![RES_NONE; CHUNK_TILE_COUNT],
            resource_amounts: vec![0; CHUNK_TILE_COUNT],
            image: Handle::<Image>::default(),
            updated_at_ms: 0,
        }
    }

    #[test]
    fn map_snapshot_has_one_rgba_pixel_per_tile() {
        let data = test_chunk();
        let pixels = map_snapshot_pixels(&data, 7);

        assert_eq!(pixels.len(), MAP_CHUNK_BYTES);
        assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn map_snapshot_includes_resource_overlay() {
        let mut data = test_chunk();
        let base = map_snapshot_pixels(&data, 7);
        data.resources[local_idx(5, 6)] = ResourceCell {
            kind: RES_IRON,
            amount: 12,
        };
        let with_resource = map_snapshot_pixels(&data, 7);

        assert_ne!(map_pixel(&base, 5, 6), map_pixel(&with_resource, 5, 6));
    }

    #[test]
    fn map_resource_metadata_tracks_known_resource_amounts() {
        let mut data = test_chunk();
        data.resources[local_idx(3, 4)] = ResourceCell {
            kind: RES_COAL,
            amount: 9,
        };
        data.resources[local_idx(5, 6)] = ResourceCell {
            kind: RES_IRON,
            amount: 0,
        };

        let (resource_kinds, resource_amounts) = map_resource_metadata(&data);

        assert_eq!(resource_kinds.len(), CHUNK_TILE_COUNT);
        assert_eq!(resource_amounts.len(), CHUNK_TILE_COUNT);
        assert_eq!(resource_kinds[local_idx(3, 4)], RES_COAL);
        assert_eq!(resource_amounts[local_idx(3, 4)], 9);
        assert_eq!(resource_kinds[local_idx(5, 6)], RES_NONE);
        assert_eq!(resource_amounts[local_idx(5, 6)], 0);
    }

    #[test]
    fn map_snapshot_includes_placed_overlay() {
        let mut data = test_chunk();
        let base = map_snapshot_pixels(&data, 7);
        data.placed[local_idx(8, 9)] = PlacedCell {
            kind: PLACED_CHEST,
            object_id: 42,
        };
        let with_placed = map_snapshot_pixels(&data, 7);

        assert_ne!(map_pixel(&base, 8, 9), map_pixel(&with_placed, 8, 9));
    }

    #[test]
    fn map_snapshot_orientation_matches_chunk_texture_interior() {
        let data = test_chunk();
        let config = WorldRenderConfig::default();
        let map_pixels = map_snapshot_pixels(&data, 7);
        let chunk_pixels = chunk_pixels(&data, &config, 7, None);

        assert_eq!(
            map_pixel(&map_pixels, 11, 13),
            chunk_pixel(&chunk_pixels, 11, 13)
        );
    }

    #[test]
    fn map_resource_node_summary_sums_connected_explored_tiles_across_chunks() {
        let session = WorldSession::default();
        let layer = 0;
        let mut map = MapState::default();
        let mut left_chunk = blank_map_chunk();
        let mut right_chunk = blank_map_chunk();

        left_chunk.resource_kinds[local_idx(31, 0)] = RES_IRON;
        left_chunk.resource_amounts[local_idx(31, 0)] = 5;
        right_chunk.resource_kinds[local_idx(0, 0)] = RES_IRON;
        right_chunk.resource_amounts[local_idx(0, 0)] = 7;
        right_chunk.resource_kinds[local_idx(2, 0)] = RES_IRON;
        right_chunk.resource_amounts[local_idx(2, 0)] = 100;

        map.explored.insert(
            ChunkKey::new(session.world_id.clone(), ChunkCoord::new(0, 0), layer),
            left_chunk,
        );
        map.explored.insert(
            ChunkKey::new(session.world_id.clone(), ChunkCoord::new(1, 0), layer),
            right_chunk,
        );

        let summary = map_resource_node_summary(&map, &session, layer, 31, 0).unwrap();

        assert_eq!(summary.kind, RES_IRON);
        assert_eq!(summary.total, 12);
    }
}
