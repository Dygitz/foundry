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
    pub entities: Vec<Entity>,
    pub saved_tick: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct SimChunkView<'a> {
    pub coord: ChunkCoord,
    pub layer: ChunkLayer,
    pub tiles: &'a [TileId],
    pub entities: &'a [Entity],
}

impl<'a> SimChunkView<'a> {
    pub fn from_data(data: &'a SimChunkData) -> Self {
        Self {
            coord: data.coord,
            layer: data.layer,
            tiles: &data.tiles,
            entities: &data.entities,
        }
    }
}
pub mod inventory;
pub use inventory::{
    Inventory, ItemId, ITEM_COAL, ITEM_COPPER_ORE, ITEM_IRON_ORE, ITEM_NONE, ITEM_STONE,
};
