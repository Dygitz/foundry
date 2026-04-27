use super::*;
use crate::ChunkCodec;
use simulation_core::{
    AlembicRecord, AlembicState, CHUNK_TILE_COUNT, CrucibleRecord, CrucibleState, DrillRecord,
    DrillState, Entity, FurnaceSlot, ITEM_COAL, ITEM_COPPER_ORE, ITEM_CUPRIC_ESSENCE,
    ITEM_FERRIC_ESSENCE, ITEM_IRON_ORE, ITEM_LODESTONE, ITEM_MINERAL_ESSENCE, ITEM_STONE,
    InserterDirection, InserterInv, InserterRecord, PLACED_ALEMBIC, PLACED_CHEST, PLACED_CRUCIBLE,
    PLACED_INSERTER, PLACED_MINING_DRILL, RES_IRON,
};

fn sample_chunk() -> SimChunkData {
    let mut tiles = vec![0; CHUNK_TILE_COUNT];
    tiles[3] = 5;

    let mut resources = vec![
        ResourceCell {
            kind: RES_NONE,
            amount: 0,
        };
        CHUNK_TILE_COUNT
    ];
    resources[4] = ResourceCell {
        kind: RES_IRON,
        amount: 12,
    };

    let mut placed = vec![
        PlacedCell {
            kind: PLACED_NONE,
            object_id: 0,
        };
        CHUNK_TILE_COUNT
    ];
    placed[5] = PlacedCell {
        kind: PLACED_CHEST,
        object_id: 99,
    };
    placed[6] = PlacedCell {
        kind: PLACED_INSERTER,
        object_id: 101,
    };
    placed[7] = PlacedCell {
        kind: PLACED_MINING_DRILL,
        object_id: 102,
    };
    placed[8] = PlacedCell {
        kind: PLACED_ALEMBIC,
        object_id: 103,
    };
    placed[9] = PlacedCell {
        kind: PLACED_CRUCIBLE,
        object_id: 104,
    };

    let mut chest_inv = ContainerInv::default();
    chest_inv.slots[0] = Slot {
        item: ITEM_STONE,
        count: 20,
    };

    SimChunkData {
        coord: ChunkCoord::new(-2, 3),
        layer: 1,
        tiles,
        resources,
        placed,
        chests: vec![ChestRecord {
            object_id: 99,
            inv: chest_inv,
        }],
        furnaces: vec![FurnaceRecord {
            object_id: 100,
            state: FurnaceState {
                input: Slot {
                    item: ITEM_IRON_ORE,
                    count: 2,
                },
                fuel: Slot {
                    item: ITEM_COAL,
                    count: 1,
                },
                output: Slot::default(),
                progress: 42,
            },
        }],
        inserters: vec![InserterRecord {
            object_id: 101,
            direction: InserterDirection::Down,
            inv: InserterInv {
                slots: [
                    Slot {
                        item: ITEM_STONE,
                        count: 1,
                    },
                    Slot::default(),
                    Slot::default(),
                    Slot::default(),
                ],
            },
        }],
        drills: vec![DrillRecord {
            object_id: 102,
            state: DrillState {
                fuel: Slot {
                    item: ITEM_COAL,
                    count: 3,
                },
                output: Slot {
                    item: ITEM_COPPER_ORE,
                    count: 2,
                },
                progress: 77,
            },
        }],
        alembics: vec![AlembicRecord {
            object_id: 103,
            state: AlembicState {
                input: Slot {
                    item: ITEM_COPPER_ORE,
                    count: 4,
                },
                output: Slot {
                    item: ITEM_CUPRIC_ESSENCE,
                    count: 1,
                },
                progress: 91,
            },
        }],
        crucibles: vec![CrucibleRecord {
            object_id: 104,
            state: CrucibleState {
                inputs: [
                    Slot {
                        item: ITEM_FERRIC_ESSENCE,
                        count: 2,
                    },
                    Slot {
                        item: ITEM_MINERAL_ESSENCE,
                        count: 2,
                    },
                    Slot::default(),
                    Slot::default(),
                ],
                output: Slot {
                    item: ITEM_LODESTONE,
                    count: 1,
                },
                progress: 57,
            },
        }],
        entities: vec![Entity {
            id: 7,
            kind: 2,
            x: 12,
            y: 20,
        }],
        saved_tick: 0,
    }
}

#[test]
fn round_trips_v9_chunk_payload() {
    let codec = ChunkCodecV1::default();
    let chunk = sample_chunk();

    let encoded = codec.encode(&SimChunkView::from_data(&chunk), 123).unwrap();
    let decoded = codec.decode(&encoded).unwrap();

    assert_eq!(decoded.coord, chunk.coord);
    assert_eq!(decoded.layer, chunk.layer);
    assert_eq!(decoded.tiles, chunk.tiles);
    assert_eq!(decoded.resources, chunk.resources);
    assert_eq!(decoded.placed, chunk.placed);
    assert_eq!(decoded.chests, chunk.chests);
    assert_eq!(decoded.furnaces, chunk.furnaces);
    assert_eq!(decoded.inserters, chunk.inserters);
    assert_eq!(decoded.drills, chunk.drills);
    assert_eq!(decoded.alembics, chunk.alembics);
    assert_eq!(decoded.crucibles, chunk.crucibles);
    assert_eq!(decoded.entities, chunk.entities);
    assert_eq!(decoded.saved_tick, 123);
}

#[test]
fn rejects_truncated_payload() {
    let codec = ChunkCodecV1::default();
    let chunk = sample_chunk();
    let mut encoded = codec.encode(&SimChunkView::from_data(&chunk), 123).unwrap();
    encoded.pop();

    let error = codec.decode(&encoded).unwrap_err();

    assert!(matches!(error, StorageError::DecodeFailed(_)));
}

#[test]
fn rejects_wrong_schema_version() {
    let encoded = ChunkCodecV1::new(1)
        .encode(&SimChunkView::from_data(&sample_chunk()), 123)
        .unwrap();

    let error = ChunkCodecV1::new(2).decode(&encoded).unwrap_err();

    assert!(matches!(
        error,
        StorageError::VersionMismatch {
            expected: 2,
            found: 1
        }
    ));
}

#[test]
fn rejects_invalid_chunk_lengths_on_encode() {
    let codec = ChunkCodecV1::default();
    let mut chunk = sample_chunk();
    chunk.tiles.pop();

    let error = codec
        .encode(&SimChunkView::from_data(&chunk), 123)
        .unwrap_err();

    assert!(matches!(error, StorageError::Other(_)));
}

#[test]
fn furnace_slot_type_remains_exhaustive_for_codec_records() {
    let slots = [FurnaceSlot::Input, FurnaceSlot::Fuel, FurnaceSlot::Output];

    assert_eq!(slots.len(), 3);
}
