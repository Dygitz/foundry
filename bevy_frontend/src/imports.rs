#![allow(unused_imports)]
pub(crate) use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) use bevy::asset::AssetMetaCheck;
pub(crate) use bevy::ecs::prelude::ChildSpawnerCommands;
pub(crate) use bevy::ecs::schedule::IntoScheduleConfigs;
pub(crate) use bevy::ecs::system::SystemParam;
pub(crate) use bevy::image::ImageSampler;
pub(crate) use bevy::input::ButtonInput;
pub(crate) use bevy::input::keyboard::KeyCode;
pub(crate) use bevy::input::mouse::MouseWheel;
pub(crate) use bevy::log::{info, warn};
pub(crate) use bevy::prelude::*;
pub(crate) use bevy::render::camera::{OrthographicProjection, Projection};
pub(crate) use bevy::render::render_asset::RenderAssetUsages;
pub(crate) use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
pub(crate) use bevy::tasks::futures_lite::future;
pub(crate) use bevy::tasks::{AsyncComputeTaskPool, Task};
pub(crate) use bevy::window::{Window, WindowPlugin};
pub(crate) use persistence::{
    ChunkCodec, ChunkCodecV1, ChunkCoord, ChunkKey, ChunkLayer, ChunkRecordWrite, MapChunkRecord,
    MapChunkRecordWrite, PlayerStateRecordWrite, RecoveryReport, StorageError, WorldId,
    WorldStorage,
};
pub(crate) use simulation_core::{
    CHEST_SLOT_COUNT, CHUNK_EDGE, CHUNK_TILE_COUNT, ChestRecord, ContainerInv,
    FURNACE_PROGRESS_PER_ITEM, FURNACE_PROGRESS_PER_SEC, FurnaceRecord, FurnaceSlot, FurnaceState,
    INSERTER_SLOT_COUNT, INVENTORY_ITEMS, ITEM_CHEST, ITEM_COAL, ITEM_COPPER_ORE,
    ITEM_COPPER_PLATE, ITEM_FURNACE, ITEM_INSERTER, ITEM_IRON_ORE, ITEM_IRON_PLATE, ITEM_NONE,
    ITEM_STONE, InserterDirection, InserterInv, InserterRecord, Inventory, ItemId, ObjectId,
    PLACEABLE_ITEMS, PLACED_CHEST, PLACED_FURNACE, PLACED_INSERTER, PLACED_NONE, PlacedCell,
    PlacedId, RECIPES, RES_COAL, RES_COPPER, RES_IRON, RES_NONE, RES_STONE, Recipe, ResourceCell,
    ResourceId, SimChunkData, SimChunkView, Slot, TileId, WATER_TILE, can_craft, deposit_to_chest,
    deposit_to_furnace_fuel, deposit_to_furnace_input, deposit_to_inserter, generate_chunk_data,
    is_placeable_item, is_water, item_name, item_to_placed_kind, object_id_for_tile, placed_at,
    placed_kind_to_item, recipe_for_index, resource_at, resource_to_item, smelt_output_for_input,
    take_from_chest, take_from_furnace, take_from_inserter, terrain_tile_id, tile_at, tile_jitter,
    tile_to_chunk_local, try_craft,
};
pub(crate) use web_storage_indexeddb::IndexedDbStorage;
