#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, map::*, player::*, rendering::*, storage::*,
    ui::*, world::*,
};

#[derive(Resource)]
pub(crate) struct WorldSession {
    pub(crate) world_id: WorldId,
    pub(crate) world_seed: u64,
    pub(crate) tick: u64,
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
pub(crate) struct WorldRuntime {
    pub(crate) loaded: HashMap<ChunkKey, LoadedChunk>,
    pub(crate) dirty: HashSet<ChunkKey>,
    pub(crate) last_access_frame: HashMap<ChunkKey, u64>,
    pub(crate) queued_for_save: HashSet<ChunkKey>,
    pub(crate) requested: HashSet<ChunkKey>,
    pub(crate) active_set: HashSet<ChunkKey>,
    pub(crate) keep_set: HashSet<ChunkKey>,
    pub(crate) frame_counter: u64,
}

impl WorldRuntime {
    pub(crate) fn advance_frame(&mut self) {
        self.frame_counter = self.frame_counter.saturating_add(1);
    }

    pub(crate) fn ensure_loaded(&self, key: &ChunkKey) -> bool {
        self.loaded.contains_key(key)
    }

    pub(crate) fn mark_dirty(&mut self, key: ChunkKey) {
        self.dirty.insert(key.clone());
        self.touch(&key);
    }

    pub(crate) fn touch(&mut self, key: &ChunkKey) {
        self.last_access_frame
            .insert(key.clone(), self.frame_counter);
    }

    #[allow(dead_code)]
    pub(crate) fn evictable_candidates(&self) -> Vec<ChunkKey> {
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

pub(crate) struct LoadedChunk {
    pub(crate) data: SimChunkData,
    pub(crate) sprite_entity: Entity,
    pub(crate) texture_handle: Handle<Image>,
}

#[derive(Resource)]
pub(crate) struct ChunkCacheConfig {
    pub(crate) max_loaded_chunks: usize,
    pub(crate) keep_radius_chunks: i32,
    pub(crate) evict_per_frame: usize,
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
pub(crate) struct PlayerConfig {
    pub(crate) move_speed: f32,
    pub(crate) camera_follow_lerp: f32,
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
pub(crate) struct WorldRenderConfig {
    pub(crate) tile_size: f32,
    pub(crate) active_radius_chunks: i32,
    pub(crate) layer: ChunkLayer,
    pub(crate) show_chunk_borders: bool,
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
pub(crate) struct StorageConfig {
    pub(crate) db_name: String,
    pub(crate) db_version: u32,
    pub(crate) game_schema_version: u16,
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
pub(crate) enum StorageState {
    Healthy,
    Degraded,
    Paused,
}

#[derive(Resource, Debug, Clone)]
pub(crate) struct StorageStatus {
    pub(crate) state: StorageState,
    pub(crate) detail: Option<String>,
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
    pub(crate) fn mark_ok(&mut self) {
        if self.state != StorageState::Paused {
            self.state = StorageState::Healthy;
            self.detail = None;
        }
    }

    pub(crate) fn record_error(&mut self, error: &StorageError) {
        if matches!(error, StorageError::QuotaExceeded) {
            self.state = StorageState::Paused;
            self.detail = Some("storage quota exceeded (autosave paused)".to_string());
            return;
        }

        self.detail = Some(error.to_string());
        self.state = StorageState::Degraded;
    }

    pub(crate) fn label(&self) -> String {
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

pub(crate) struct StorageServices {
    pub(crate) storage: IndexedDbStorage,
    pub(crate) codec: ChunkCodecV1,
}

#[derive(Resource, Default)]
pub(crate) struct StorageInitTask {
    pub(crate) task: Option<Task<Result<(), StorageError>>>,
    pub(crate) ready: bool,
}

#[derive(Resource)]
pub(crate) struct AutosaveState {
    pub(crate) timer: Timer,
    pub(crate) max_per_flush: usize,
    pub(crate) in_flight: Option<SaveTask>,
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

pub(crate) struct SaveTask {
    pub(crate) task: Task<Result<(), StorageError>>,
    pub(crate) pending_count: usize,
    pub(crate) keys: Vec<ChunkKey>,
}

#[derive(Resource, Default)]
pub(crate) struct SaveQueue {
    pub(crate) pending: VecDeque<ChunkRecordWrite>,
}

#[derive(Event, Debug, Clone)]
pub(crate) struct ChunkLoadRequest {
    pub(crate) key: ChunkKey,
}

#[derive(Event, Debug, Clone)]
pub(crate) struct ChunkLoaded {
    pub(crate) key: ChunkKey,
    pub(crate) data: Option<SimChunkData>,
}

#[derive(Event, Debug, Clone, Copy)]
pub(crate) struct PickupNotice {
    pub(crate) item: ItemId,
    pub(crate) amount: u32,
    pub(crate) total: u32,
}

pub(crate) struct ChunkLoadTask {
    pub(crate) key: ChunkKey,
    pub(crate) task: Task<Result<Option<SimChunkData>, StorageError>>,
}

#[derive(Resource)]
pub(crate) struct ChunkLoadState {
    pub(crate) queue: VecDeque<ChunkKey>,
    pub(crate) in_flight: HashSet<ChunkKey>,
    pub(crate) tasks: Vec<ChunkLoadTask>,
    pub(crate) max_in_flight: usize,
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

pub(crate) struct MapChunk {
    pub(crate) rgba: Vec<u8>,
    pub(crate) resource_kinds: Vec<ResourceId>,
    pub(crate) resource_amounts: Vec<u16>,
    pub(crate) image: Handle<Image>,
    pub(crate) updated_at_ms: u64,
}

#[derive(Resource, Default)]
pub(crate) struct MapState {
    pub(crate) explored: HashMap<ChunkKey, MapChunk>,
    pub(crate) pending_saves: VecDeque<MapChunkRecordWrite>,
    pub(crate) queued_for_save: HashSet<ChunkKey>,
    pub(crate) full_view: FullMapView,
    pub(crate) drag_last_cursor: Option<Vec2>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FullMapView {
    pub(crate) center_tile: Vec2,
    pub(crate) px_per_tile: f32,
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
pub(crate) struct MapLoadState {
    pub(crate) task: Option<Task<Result<Vec<MapChunkRecord>, StorageError>>>,
    pub(crate) loaded: bool,
}

#[derive(Resource)]
pub(crate) struct MapSaveState {
    pub(crate) timer: Timer,
    pub(crate) max_per_flush: usize,
    pub(crate) in_flight: Option<MapSaveTask>,
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

pub(crate) struct MapSaveTask {
    pub(crate) task: Task<Result<(), StorageError>>,
    pub(crate) pending_count: usize,
    pub(crate) keys: Vec<ChunkKey>,
}

#[derive(Resource, Default)]
pub(crate) struct PlayerState {
    pub(crate) inventory: Inventory,
}

#[derive(Resource, Clone)]
pub(crate) struct PlayerSpriteAssets {
    pub(crate) down: Handle<Image>,
    pub(crate) up: Handle<Image>,
    pub(crate) side: Handle<Image>,
}

#[derive(Resource)]
pub(crate) struct PlacementState {
    pub(crate) selected: Option<ItemId>,
    pub(crate) inserter_direction: InserterDirection,
}

impl Default for PlacementState {
    fn default() -> Self {
        Self {
            selected: None,
            inserter_direction: InserterDirection::default(),
        }
    }
}

pub(crate) const STRUCTURE_PICKUP_SECONDS: f32 = 1.25;

#[derive(Resource, Default)]
pub(crate) struct StructurePickupState {
    pub(crate) target: Option<StructurePickupTarget>,
    pub(crate) elapsed_secs: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructurePickupTarget {
    pub(crate) key: ChunkKey,
    pub(crate) tile_x: i32,
    pub(crate) tile_y: i32,
    pub(crate) local_x: i32,
    pub(crate) local_y: i32,
    pub(crate) kind: PlacedId,
    pub(crate) object_id: ObjectId,
}

#[derive(Resource)]
pub(crate) struct InserterState {
    pub(crate) timer: Timer,
}

impl Default for InserterState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

pub(crate) const HOTBAR_SLOT_COUNT: usize = 10;
pub(crate) const HOTBAR_KEYS: [KeyCode; HOTBAR_SLOT_COUNT] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::Digit0,
];

#[derive(Resource)]
pub(crate) struct HotbarState {
    pub(crate) slots: [Option<ItemId>; HOTBAR_SLOT_COUNT],
    pub(crate) selected_slot: Option<usize>,
}

impl Default for HotbarState {
    fn default() -> Self {
        Self {
            slots: [None; HOTBAR_SLOT_COUNT],
            selected_slot: None,
        }
    }
}

impl HotbarState {
    pub(crate) fn assign_item(&mut self, item: ItemId) {
        if item == ITEM_NONE || !is_placeable_item(item) {
            return;
        }
        if self.slots.iter().any(|slot| *slot == Some(item)) {
            return;
        }
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(item);
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum CraftingTab {
    Crafting,
    Tips,
}

#[derive(Resource)]
pub(crate) struct CraftingUiState {
    pub(crate) active_tab: CraftingTab,
    pub(crate) focused_recipe: usize,
    pub(crate) hovered_item: Option<ItemId>,
}

impl Default for CraftingUiState {
    fn default() -> Self {
        Self {
            active_tab: CraftingTab::Crafting,
            focused_recipe: 0,
            hovered_item: None,
        }
    }
}

#[derive(Resource, Clone)]
pub(crate) struct UiIconAssets {
    pub(crate) empty: Handle<Image>,
    pub(crate) heart: Handle<Image>,
    pub(crate) stone: Handle<Image>,
    pub(crate) copper_ore: Handle<Image>,
    pub(crate) coal: Handle<Image>,
    pub(crate) iron_ore: Handle<Image>,
    pub(crate) iron_plate: Handle<Image>,
    pub(crate) copper_plate: Handle<Image>,
    pub(crate) furnace: Handle<Image>,
    pub(crate) chest: Handle<Image>,
    pub(crate) inserter: Handle<Image>,
    pub(crate) mining_drill: Handle<Image>,
    pub(crate) alembic: Handle<Image>,
    pub(crate) crucible: Handle<Image>,
    pub(crate) ferric_essence: Handle<Image>,
    pub(crate) cupric_essence: Handle<Image>,
    pub(crate) umbral_essence: Handle<Image>,
    pub(crate) mineral_essence: Handle<Image>,
    pub(crate) lodestone: Handle<Image>,
    pub(crate) brass_core: Handle<Image>,
    pub(crate) cinder_glass: Handle<Image>,
    pub(crate) quintessence: Handle<Image>,
    pub(crate) crafting: Handle<Image>,
    pub(crate) tips: Handle<Image>,
}

impl UiIconAssets {
    pub(crate) fn for_item(&self, item: ItemId) -> Handle<Image> {
        match item {
            ITEM_STONE => self.stone.clone(),
            ITEM_COPPER_ORE => self.copper_ore.clone(),
            ITEM_COAL => self.coal.clone(),
            ITEM_IRON_ORE => self.iron_ore.clone(),
            ITEM_IRON_PLATE => self.iron_plate.clone(),
            ITEM_COPPER_PLATE => self.copper_plate.clone(),
            ITEM_FERRIC_ESSENCE => self.ferric_essence.clone(),
            ITEM_CUPRIC_ESSENCE => self.cupric_essence.clone(),
            ITEM_UMBRAL_ESSENCE => self.umbral_essence.clone(),
            ITEM_MINERAL_ESSENCE => self.mineral_essence.clone(),
            ITEM_LODESTONE => self.lodestone.clone(),
            ITEM_BRASS_CORE => self.brass_core.clone(),
            ITEM_CINDER_GLASS => self.cinder_glass.clone(),
            ITEM_QUINTESSENCE => self.quintessence.clone(),
            ITEM_FURNACE => self.furnace.clone(),
            ITEM_CHEST => self.chest.clone(),
            ITEM_INSERTER => self.inserter.clone(),
            ITEM_MINING_DRILL => self.mining_drill.clone(),
            ITEM_ALEMBIC => self.alembic.clone(),
            ITEM_CRUCIBLE => self.crucible.clone(),
            _ => self.empty.clone(),
        }
    }
}

#[derive(Resource, Clone)]
pub(crate) struct PlacementPreviewAssets {
    pub(crate) furnace: Handle<Image>,
    pub(crate) chest: Handle<Image>,
    pub(crate) inserter: Handle<Image>,
    pub(crate) mining_drill: Handle<Image>,
    pub(crate) alembic: Handle<Image>,
    pub(crate) crucible: Handle<Image>,
}

impl PlacementPreviewAssets {
    pub(crate) fn for_item(&self, item: ItemId) -> Option<Handle<Image>> {
        match item {
            ITEM_FURNACE => Some(self.furnace.clone()),
            ITEM_CHEST => Some(self.chest.clone()),
            ITEM_INSERTER => Some(self.inserter.clone()),
            ITEM_MINING_DRILL => Some(self.mining_drill.clone()),
            ITEM_ALEMBIC => Some(self.alembic.clone()),
            ITEM_CRUCIBLE => Some(self.crucible.clone()),
            _ => None,
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct UiState {
    pub(crate) mode: UiMode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum UiMode {
    None,
    Map,
    Crafting,
    Chest { object_id: ObjectId },
    Furnace { object_id: ObjectId },
    Inserter { object_id: ObjectId },
    MiningDrill { object_id: ObjectId },
    Alembic { object_id: ObjectId },
    Crucible { object_id: ObjectId },
}

impl Default for UiMode {
    fn default() -> Self {
        UiMode::None
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum UpdateSet {
    Input,
    Ui,
    World,
}

pub(crate) const MAP_CHUNK_BYTES: usize = CHUNK_TILE_COUNT * 4;
pub(crate) const MINIMAP_SIZE: f32 = 238.0;
pub(crate) const MINIMAP_FRAME: f32 = 3.0;
pub(crate) const MINIMAP_OUTER_SIZE: f32 = MINIMAP_SIZE + MINIMAP_FRAME * 2.0;
pub(crate) const MINIMAP_MARGIN: f32 = 12.0;
pub(crate) const MINIMAP_PX_PER_TILE: f32 = 1.0;
pub(crate) const FULL_MAP_DEFAULT_PX_PER_TILE: f32 = 2.0;
pub(crate) const FULL_MAP_MIN_PX_PER_TILE: f32 = 0.25;
pub(crate) const FULL_MAP_MAX_PX_PER_TILE: f32 = 8.0;
pub(crate) const MAP_TOOLTIP_WIDTH: f32 = 168.0;
pub(crate) const MAP_TOOLTIP_HEIGHT: f32 = 28.0;
pub(crate) const MAP_TOOLTIP_OFFSET: f32 = 12.0;

#[derive(Resource)]
pub(crate) struct PlayerStateSaveState {
    pub(crate) timer: Timer,
    pub(crate) in_flight: Option<Task<Result<(), StorageError>>>,
    pub(crate) dirty: bool,
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
pub(crate) struct PlayerStateLoadState {
    pub(crate) task: Option<Task<Result<Option<persistence::PlayerStateRecord>, StorageError>>>,
    pub(crate) loaded: bool,
}

#[derive(Resource, Default)]
pub(crate) struct ClickHighlight {
    pub(crate) tile: Option<(i32, i32)>,
}

#[derive(Resource, Default)]
pub(crate) struct MiningFeedbackState {
    pub(crate) tracked_tile: Option<(i32, i32)>,
    pub(crate) tracked_resource_kind: ResourceId,
    pub(crate) tracked_max_amount: u16,
}

#[derive(Resource)]
pub(crate) struct DebugConfig {
    pub(crate) log_mining: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self { log_mining: true }
    }
}

#[derive(Resource, Default)]
pub(crate) struct RecoveryState {
    pub(crate) task: Option<Task<Result<RecoveryReport, StorageError>>>,
    pub(crate) completed: bool,
}

#[derive(Resource)]
pub(crate) struct EvictionStats {
    pub(crate) timer: Timer,
    pub(crate) evicted_this_window: usize,
    pub(crate) evicted_per_second: usize,
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
