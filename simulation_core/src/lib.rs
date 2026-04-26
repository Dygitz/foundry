#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub cx: i32,
    pub cy: i32,
}

impl ChunkCoord {
    pub fn new(cx: i32, cy: i32) -> Self {
        Self { cx, cy }
    }
}

pub type ChunkLayer = u8;

pub const CHUNK_EDGE: u16 = 32;
pub const CHUNK_TILE_COUNT: usize = (CHUNK_EDGE as usize) * (CHUNK_EDGE as usize);

pub type TileId = u16;
pub type EntityKind = u16;
pub type ResourceId = u8;
pub type PlacedId = u8;
pub type ObjectId = u64;

pub const RES_NONE: ResourceId = 0;
pub const RES_IRON: ResourceId = 1;
pub const RES_COPPER: ResourceId = 2;
pub const RES_COAL: ResourceId = 3;
pub const RES_STONE: ResourceId = 4;

pub const PLACED_NONE: PlacedId = 0;
pub const PLACED_FURNACE: PlacedId = 1;
pub const PLACED_CHEST: PlacedId = 2;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ResourceCell {
    pub kind: ResourceId,
    pub amount: u16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlacedCell {
    pub kind: PlacedId,
    pub object_id: ObjectId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Entity {
    pub id: u32,
    pub kind: EntityKind,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone)]
pub struct SimChunkData {
    pub coord: ChunkCoord,
    pub layer: ChunkLayer,
    pub tiles: Vec<TileId>,
    pub resources: Vec<ResourceCell>,
    pub placed: Vec<PlacedCell>,
    pub chests: Vec<ChestRecord>,
    pub furnaces: Vec<FurnaceRecord>,
    pub entities: Vec<Entity>,
    pub saved_tick: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct SimChunkView<'a> {
    pub coord: ChunkCoord,
    pub layer: ChunkLayer,
    pub tiles: &'a [TileId],
    pub resources: &'a [ResourceCell],
    pub placed: &'a [PlacedCell],
    pub chests: &'a [ChestRecord],
    pub furnaces: &'a [FurnaceRecord],
    pub entities: &'a [Entity],
}

impl<'a> SimChunkView<'a> {
    pub fn from_data(data: &'a SimChunkData) -> Self {
        Self {
            coord: data.coord,
            layer: data.layer,
            tiles: &data.tiles,
            resources: &data.resources,
            placed: &data.placed,
            chests: &data.chests,
            furnaces: &data.furnaces,
            entities: &data.entities,
        }
    }
}
pub mod containers;
pub mod inventory;
pub use containers::{
    CHEST_SLOT_COUNT, ChestRecord, ContainerInv, FurnaceRecord, FurnaceSlot, FurnaceState, Slot,
    deposit_to_chest, deposit_to_furnace_fuel, deposit_to_furnace_input, take_from_chest,
    take_from_furnace,
};
pub use inventory::{
    ITEM_CHEST, ITEM_COAL, ITEM_COPPER_ORE, ITEM_COPPER_PLATE, ITEM_FURNACE, ITEM_IRON_ORE,
    ITEM_IRON_PLATE, ITEM_NONE, ITEM_STONE, Inventory, ItemId,
};
