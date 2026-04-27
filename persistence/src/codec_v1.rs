use crate::errors::{Result, StorageError};
use crate::traits::ChunkCodec;
use simulation_core::{
    CHEST_SLOT_COUNT, CHUNK_EDGE, CHUNK_TILE_COUNT, ChestRecord, ChunkCoord, ContainerInv,
    DrillRecord, DrillState, Entity, FurnaceRecord, FurnaceState, INSERTER_SLOT_COUNT,
    InserterDirection, InserterInv, InserterRecord, PLACED_NONE, PlacedCell, RES_NONE,
    ResourceCell, SimChunkData, SimChunkView, Slot, TileId,
};

const MAGIC: [u8; 4] = *b"CHNK";
const FORMAT_VERSION: u16 = 7;
const FLAGS_NONE: u16 = 0;
const BYTES_PER_ENTITY: usize = 12;
const BYTES_PER_RESOURCE: usize = 3;
const BYTES_PER_PLACED_V3: usize = 1;
const BYTES_PER_PLACED_V4: usize = 9;
const BYTES_PER_SLOT: usize = 6;
const BYTES_PER_CHEST: usize = 8 + (CHEST_SLOT_COUNT * BYTES_PER_SLOT);
const BYTES_PER_FURNACE: usize = 8 + (3 * BYTES_PER_SLOT) + 2;
const BYTES_PER_INSERTER_V5: usize = 8 + (INSERTER_SLOT_COUNT * BYTES_PER_SLOT);
const BYTES_PER_INSERTER: usize = 8 + 1 + (INSERTER_SLOT_COUNT * BYTES_PER_SLOT);
const BYTES_PER_DRILL: usize = 8 + (2 * BYTES_PER_SLOT) + 2;

#[derive(Debug, Clone, Copy)]
pub struct ChunkCodecV1 {
    game_schema_version: u16,
}

impl ChunkCodecV1 {
    pub fn new(game_schema_version: u16) -> Self {
        Self {
            game_schema_version,
        }
    }
}

impl Default for ChunkCodecV1 {
    fn default() -> Self {
        Self::new(1)
    }
}

impl ChunkCodec for ChunkCodecV1 {
    fn encode(&self, chunk: &SimChunkView<'_>, saved_tick: u64) -> Result<Vec<u8>> {
        if chunk.tiles.len() != CHUNK_TILE_COUNT {
            return Err(StorageError::Other(format!(
                "tiles length {} does not match expected {CHUNK_TILE_COUNT}",
                chunk.tiles.len()
            )));
        }
        if chunk.resources.len() != CHUNK_TILE_COUNT {
            return Err(StorageError::Other(format!(
                "resources length {} does not match expected {CHUNK_TILE_COUNT}",
                chunk.resources.len()
            )));
        }
        if chunk.placed.len() != CHUNK_TILE_COUNT {
            return Err(StorageError::Other(format!(
                "placed length {} does not match expected {CHUNK_TILE_COUNT}",
                chunk.placed.len()
            )));
        }

        // NOTE: checksum is computed by the storage layer, not the codec.
        let payload = encode_payload_v7(chunk)?;
        let payload_len = u32::try_from(payload.len())
            .map_err(|_| StorageError::Other("payload length exceeds u32::MAX".to_string()))?;

        let mut buffer = Vec::with_capacity(header_len() + payload.len());
        buffer.extend_from_slice(&MAGIC);
        buffer.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        buffer.extend_from_slice(&self.game_schema_version.to_le_bytes());
        buffer.extend_from_slice(&FLAGS_NONE.to_le_bytes());
        buffer.extend_from_slice(&chunk.coord.cx.to_le_bytes());
        buffer.extend_from_slice(&chunk.coord.cy.to_le_bytes());
        buffer.push(chunk.layer);
        buffer.extend_from_slice(&saved_tick.to_le_bytes());
        buffer.extend_from_slice(&payload_len.to_le_bytes());
        buffer.extend_from_slice(&payload);

        Ok(buffer)
    }

    fn decode(&self, bytes: &[u8]) -> Result<SimChunkData> {
        let mut reader = Reader::new(bytes);

        let magic = reader.take(4)?;
        if magic != MAGIC {
            return Err(StorageError::DecodeFailed(
                "chunk magic mismatch".to_string(),
            ));
        }

        let format_version = reader.read_u16()?;
        if format_version != FORMAT_VERSION
            && format_version != 1
            && format_version != 2
            && format_version != 3
            && format_version != 4
            && format_version != 5
            && format_version != 6
        {
            return Err(StorageError::DecodeFailed(format!(
                "unsupported chunk format version {format_version}"
            )));
        }

        let game_schema_version = reader.read_u16()?;
        if game_schema_version != self.game_schema_version {
            return Err(StorageError::VersionMismatch {
                expected: self.game_schema_version,
                found: game_schema_version,
            });
        }

        let flags = reader.read_u16()?;
        if flags != FLAGS_NONE {
            return Err(StorageError::DecodeFailed(format!(
                "unsupported chunk flags {flags}"
            )));
        }

        let cx = reader.read_i32()?;
        let cy = reader.read_i32()?;
        let layer = reader.read_u8()?;
        let saved_tick = reader.read_u64()?;
        let payload_len = reader.read_u32()? as usize;

        if reader.remaining() != payload_len {
            return Err(StorageError::DecodeFailed(
                "payload length mismatch".to_string(),
            ));
        }

        let payload_bytes = reader.take(payload_len)?;
        let (tiles, entities, resources, placed, chests, furnaces, inserters, drills) =
            if format_version == 1 {
                let (tiles, entities) = decode_payload_v1(payload_bytes)?;
                let resources = vec![
                    ResourceCell {
                        kind: RES_NONE,
                        amount: 0
                    };
                    CHUNK_TILE_COUNT
                ];
                let placed = vec![
                    PlacedCell {
                        kind: PLACED_NONE,
                        object_id: 0,
                    };
                    CHUNK_TILE_COUNT
                ];
                (
                    tiles,
                    entities,
                    resources,
                    placed,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
            } else if format_version == 2 {
                let (tiles, entities, resources) = decode_payload_v2(payload_bytes)?;
                let placed = vec![
                    PlacedCell {
                        kind: PLACED_NONE,
                        object_id: 0,
                    };
                    CHUNK_TILE_COUNT
                ];
                (
                    tiles,
                    entities,
                    resources,
                    placed,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
            } else if format_version == 3 {
                let (tiles, entities, resources, placed) = decode_payload_v3(payload_bytes)?;
                (
                    tiles,
                    entities,
                    resources,
                    placed,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
            } else if format_version == 4 {
                let (tiles, entities, resources, placed, chests, furnaces) =
                    decode_payload_v4(payload_bytes)?;
                (
                    tiles,
                    entities,
                    resources,
                    placed,
                    chests,
                    furnaces,
                    Vec::new(),
                    Vec::new(),
                )
            } else if format_version == 5 {
                decode_payload_v5_to_v7(payload_bytes, false, false)?
            } else if format_version == 6 {
                decode_payload_v5_to_v7(payload_bytes, true, false)?
            } else {
                decode_payload_v5_to_v7(payload_bytes, true, true)?
            };

        Ok(SimChunkData {
            coord: ChunkCoord { cx, cy },
            layer,
            tiles,
            resources,
            placed,
            chests,
            furnaces,
            inserters,
            drills,
            entities,
            saved_tick,
        })
    }
}

fn encode_payload_v4(chunk: &SimChunkView<'_>) -> Result<Vec<u8>> {
    let tiles_len = u32::try_from(chunk.tiles.len())
        .map_err(|_| StorageError::Other("tiles length exceeds u32::MAX".to_string()))?;
    let entity_count = u32::try_from(chunk.entities.len())
        .map_err(|_| StorageError::Other("entity count exceeds u32::MAX".to_string()))?;
    let resource_len = u32::try_from(chunk.resources.len())
        .map_err(|_| StorageError::Other("resources length exceeds u32::MAX".to_string()))?;
    let placed_len = u32::try_from(chunk.placed.len())
        .map_err(|_| StorageError::Other("placed length exceeds u32::MAX".to_string()))?;
    let chest_count = u32::try_from(chunk.chests.len())
        .map_err(|_| StorageError::Other("chest count exceeds u32::MAX".to_string()))?;
    let furnace_count = u32::try_from(chunk.furnaces.len())
        .map_err(|_| StorageError::Other("furnace count exceeds u32::MAX".to_string()))?;

    let mut buffer = Vec::with_capacity(
        4 + (chunk.tiles.len() * 2)
            + 4
            + (chunk.entities.len() * BYTES_PER_ENTITY)
            + 4
            + (chunk.resources.len() * BYTES_PER_RESOURCE)
            + 4
            + (chunk.placed.len() * BYTES_PER_PLACED_V4)
            + 4
            + (chunk.chests.len() * BYTES_PER_CHEST)
            + 4
            + (chunk.furnaces.len() * BYTES_PER_FURNACE),
    );
    buffer.extend_from_slice(&tiles_len.to_le_bytes());
    for &tile in chunk.tiles {
        buffer.extend_from_slice(&tile.to_le_bytes());
    }
    buffer.extend_from_slice(&entity_count.to_le_bytes());
    for entity in chunk.entities {
        buffer.extend_from_slice(&entity.id.to_le_bytes());
        buffer.extend_from_slice(&entity.kind.to_le_bytes());
        buffer.extend_from_slice(&entity.x.to_le_bytes());
        buffer.extend_from_slice(&entity.y.to_le_bytes());
    }
    buffer.extend_from_slice(&resource_len.to_le_bytes());
    for resource in chunk.resources {
        buffer.push(resource.kind);
        buffer.extend_from_slice(&resource.amount.to_le_bytes());
    }
    buffer.extend_from_slice(&placed_len.to_le_bytes());
    for placed in chunk.placed {
        buffer.push(placed.kind);
        buffer.extend_from_slice(&placed.object_id.to_le_bytes());
    }
    buffer.extend_from_slice(&chest_count.to_le_bytes());
    for chest in chunk.chests {
        buffer.extend_from_slice(&chest.object_id.to_le_bytes());
        encode_slots(&mut buffer, &chest.inv);
    }
    buffer.extend_from_slice(&furnace_count.to_le_bytes());
    for furnace in chunk.furnaces {
        buffer.extend_from_slice(&furnace.object_id.to_le_bytes());
        encode_slot(&mut buffer, &furnace.state.input);
        encode_slot(&mut buffer, &furnace.state.fuel);
        encode_slot(&mut buffer, &furnace.state.output);
        buffer.extend_from_slice(&furnace.state.progress.to_le_bytes());
    }

    Ok(buffer)
}

fn encode_payload_v6(chunk: &SimChunkView<'_>) -> Result<Vec<u8>> {
    let mut buffer = encode_payload_v4(chunk)?;
    let inserter_count = u32::try_from(chunk.inserters.len())
        .map_err(|_| StorageError::Other("inserter count exceeds u32::MAX".to_string()))?;
    buffer.reserve(4 + (chunk.inserters.len() * BYTES_PER_INSERTER));
    buffer.extend_from_slice(&inserter_count.to_le_bytes());
    for inserter in chunk.inserters {
        buffer.extend_from_slice(&inserter.object_id.to_le_bytes());
        buffer.push(inserter.direction.to_u8());
        encode_inserter_slots(&mut buffer, &inserter.inv);
    }
    Ok(buffer)
}

fn encode_payload_v7(chunk: &SimChunkView<'_>) -> Result<Vec<u8>> {
    let mut buffer = encode_payload_v6(chunk)?;
    let drill_count = u32::try_from(chunk.drills.len())
        .map_err(|_| StorageError::Other("drill count exceeds u32::MAX".to_string()))?;
    buffer.reserve(4 + (chunk.drills.len() * BYTES_PER_DRILL));
    buffer.extend_from_slice(&drill_count.to_le_bytes());
    for drill in chunk.drills {
        buffer.extend_from_slice(&drill.object_id.to_le_bytes());
        encode_slot(&mut buffer, &drill.state.fuel);
        encode_slot(&mut buffer, &drill.state.output);
        buffer.extend_from_slice(&drill.state.progress.to_le_bytes());
    }
    Ok(buffer)
}

fn decode_payload_v1(bytes: &[u8]) -> Result<(Vec<TileId>, Vec<Entity>)> {
    let mut reader = Reader::new(bytes);

    let tiles_len = reader.read_u32()? as usize;
    if tiles_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "tile count {tiles_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }

    let mut tiles = Vec::with_capacity(tiles_len);
    for _ in 0..tiles_len {
        tiles.push(reader.read_u16()?);
    }

    let entity_count = reader.read_u32()? as usize;
    if entity_count > 0 && entity_count > reader.remaining() / BYTES_PER_ENTITY {
        return Err(StorageError::DecodeFailed(
            "entity count exceeds payload size".to_string(),
        ));
    }

    let mut entities = Vec::with_capacity(entity_count);
    for _ in 0..entity_count {
        let id = reader.read_u32()?;
        let kind = reader.read_u16()?;
        let x = reader.read_u16()?;
        let y = reader.read_u16()?;
        if x >= CHUNK_EDGE || y >= CHUNK_EDGE {
            return Err(StorageError::DecodeFailed(
                "entity position out of chunk bounds".to_string(),
            ));
        }
        entities.push(Entity { id, kind, x, y });
    }

    if reader.remaining() != 0 {
        return Err(StorageError::DecodeFailed(
            "payload has trailing bytes".to_string(),
        ));
    }

    Ok((tiles, entities))
}

fn decode_payload_v2(bytes: &[u8]) -> Result<(Vec<TileId>, Vec<Entity>, Vec<ResourceCell>)> {
    let mut reader = Reader::new(bytes);

    let tiles_len = reader.read_u32()? as usize;
    if tiles_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "tile count {tiles_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }

    let mut tiles = Vec::with_capacity(tiles_len);
    for _ in 0..tiles_len {
        tiles.push(reader.read_u16()?);
    }

    let entity_count = reader.read_u32()? as usize;
    if entity_count > 0 && entity_count > reader.remaining() / BYTES_PER_ENTITY {
        return Err(StorageError::DecodeFailed(
            "entity count exceeds payload size".to_string(),
        ));
    }

    let mut entities = Vec::with_capacity(entity_count);
    for _ in 0..entity_count {
        let id = reader.read_u32()?;
        let kind = reader.read_u16()?;
        let x = reader.read_u16()?;
        let y = reader.read_u16()?;
        if x >= CHUNK_EDGE || y >= CHUNK_EDGE {
            return Err(StorageError::DecodeFailed(
                "entity position out of chunk bounds".to_string(),
            ));
        }
        entities.push(Entity { id, kind, x, y });
    }

    let resource_len = reader.read_u32()? as usize;
    if resource_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "resource count {resource_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }
    if resource_len > 0 && resource_len > reader.remaining() / BYTES_PER_RESOURCE {
        return Err(StorageError::DecodeFailed(
            "resource count exceeds payload size".to_string(),
        ));
    }

    let mut resources = Vec::with_capacity(resource_len);
    for _ in 0..resource_len {
        let kind = reader.read_u8()?;
        let amount = reader.read_u16()?;
        resources.push(ResourceCell { kind, amount });
    }

    if reader.remaining() != 0 {
        return Err(StorageError::DecodeFailed(
            "payload has trailing bytes".to_string(),
        ));
    }

    Ok((tiles, entities, resources))
}

fn decode_payload_v3(
    bytes: &[u8],
) -> Result<(Vec<TileId>, Vec<Entity>, Vec<ResourceCell>, Vec<PlacedCell>)> {
    let mut reader = Reader::new(bytes);

    let tiles_len = reader.read_u32()? as usize;
    if tiles_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "tile count {tiles_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }

    let mut tiles = Vec::with_capacity(tiles_len);
    for _ in 0..tiles_len {
        tiles.push(reader.read_u16()?);
    }

    let entity_count = reader.read_u32()? as usize;
    if entity_count > 0 && entity_count > reader.remaining() / BYTES_PER_ENTITY {
        return Err(StorageError::DecodeFailed(
            "entity count exceeds payload size".to_string(),
        ));
    }

    let mut entities = Vec::with_capacity(entity_count);
    for _ in 0..entity_count {
        let id = reader.read_u32()?;
        let kind = reader.read_u16()?;
        let x = reader.read_u16()?;
        let y = reader.read_u16()?;
        if x >= CHUNK_EDGE || y >= CHUNK_EDGE {
            return Err(StorageError::DecodeFailed(
                "entity position out of chunk bounds".to_string(),
            ));
        }
        entities.push(Entity { id, kind, x, y });
    }

    let resource_len = reader.read_u32()? as usize;
    if resource_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "resource count {resource_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }
    if resource_len > 0 && resource_len > reader.remaining() / BYTES_PER_RESOURCE {
        return Err(StorageError::DecodeFailed(
            "resource count exceeds payload size".to_string(),
        ));
    }

    let mut resources = Vec::with_capacity(resource_len);
    for _ in 0..resource_len {
        let kind = reader.read_u8()?;
        let amount = reader.read_u16()?;
        resources.push(ResourceCell { kind, amount });
    }

    let placed_len = reader.read_u32()? as usize;
    if placed_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "placed count {placed_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }
    if placed_len > 0 && placed_len > reader.remaining() / BYTES_PER_PLACED_V3 {
        return Err(StorageError::DecodeFailed(
            "placed count exceeds payload size".to_string(),
        ));
    }

    let mut placed = Vec::with_capacity(placed_len);
    for _ in 0..placed_len {
        let kind = reader.read_u8()?;
        placed.push(PlacedCell { kind, object_id: 0 });
    }

    if reader.remaining() != 0 {
        return Err(StorageError::DecodeFailed(
            "payload has trailing bytes".to_string(),
        ));
    }

    Ok((tiles, entities, resources, placed))
}

fn decode_payload_v4(
    bytes: &[u8],
) -> Result<(
    Vec<TileId>,
    Vec<Entity>,
    Vec<ResourceCell>,
    Vec<PlacedCell>,
    Vec<ChestRecord>,
    Vec<FurnaceRecord>,
)> {
    let mut reader = Reader::new(bytes);

    let tiles_len = reader.read_u32()? as usize;
    if tiles_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "tile count {tiles_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }

    let mut tiles = Vec::with_capacity(tiles_len);
    for _ in 0..tiles_len {
        tiles.push(reader.read_u16()?);
    }

    let entity_count = reader.read_u32()? as usize;
    if entity_count > 0 && entity_count > reader.remaining() / BYTES_PER_ENTITY {
        return Err(StorageError::DecodeFailed(
            "entity count exceeds payload size".to_string(),
        ));
    }

    let mut entities = Vec::with_capacity(entity_count);
    for _ in 0..entity_count {
        let id = reader.read_u32()?;
        let kind = reader.read_u16()?;
        let x = reader.read_u16()?;
        let y = reader.read_u16()?;
        if x >= CHUNK_EDGE || y >= CHUNK_EDGE {
            return Err(StorageError::DecodeFailed(
                "entity position out of chunk bounds".to_string(),
            ));
        }
        entities.push(Entity { id, kind, x, y });
    }

    let resource_len = reader.read_u32()? as usize;
    if resource_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "resource count {resource_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }
    if resource_len > 0 && resource_len > reader.remaining() / BYTES_PER_RESOURCE {
        return Err(StorageError::DecodeFailed(
            "resource count exceeds payload size".to_string(),
        ));
    }

    let mut resources = Vec::with_capacity(resource_len);
    for _ in 0..resource_len {
        let kind = reader.read_u8()?;
        let amount = reader.read_u16()?;
        resources.push(ResourceCell { kind, amount });
    }

    let placed_len = reader.read_u32()? as usize;
    if placed_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "placed count {placed_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }
    if placed_len > 0 && placed_len > reader.remaining() / BYTES_PER_PLACED_V4 {
        return Err(StorageError::DecodeFailed(
            "placed count exceeds payload size".to_string(),
        ));
    }

    let mut placed = Vec::with_capacity(placed_len);
    for _ in 0..placed_len {
        let kind = reader.read_u8()?;
        let object_id = reader.read_u64()?;
        placed.push(PlacedCell { kind, object_id });
    }

    let chest_count = reader.read_u32()? as usize;
    if chest_count > 0 && chest_count > reader.remaining() / BYTES_PER_CHEST {
        return Err(StorageError::DecodeFailed(
            "chest count exceeds payload size".to_string(),
        ));
    }
    let mut chests = Vec::with_capacity(chest_count);
    for _ in 0..chest_count {
        let object_id = reader.read_u64()?;
        let inv = decode_slots(&mut reader)?;
        chests.push(ChestRecord { object_id, inv });
    }

    let furnace_count = reader.read_u32()? as usize;
    if furnace_count > 0 && furnace_count > reader.remaining() / BYTES_PER_FURNACE {
        return Err(StorageError::DecodeFailed(
            "furnace count exceeds payload size".to_string(),
        ));
    }
    let mut furnaces = Vec::with_capacity(furnace_count);
    for _ in 0..furnace_count {
        let object_id = reader.read_u64()?;
        let input = decode_slot(&mut reader)?;
        let fuel = decode_slot(&mut reader)?;
        let output = decode_slot(&mut reader)?;
        let progress = reader.read_u16()?;
        furnaces.push(FurnaceRecord {
            object_id,
            state: FurnaceState {
                input,
                fuel,
                output,
                progress,
            },
        });
    }

    if reader.remaining() != 0 {
        return Err(StorageError::DecodeFailed(
            "payload has trailing bytes".to_string(),
        ));
    }

    Ok((tiles, entities, resources, placed, chests, furnaces))
}

fn decode_payload_v5_to_v7(
    bytes: &[u8],
    has_inserter_direction: bool,
    has_drills: bool,
) -> Result<(
    Vec<TileId>,
    Vec<Entity>,
    Vec<ResourceCell>,
    Vec<PlacedCell>,
    Vec<ChestRecord>,
    Vec<FurnaceRecord>,
    Vec<InserterRecord>,
    Vec<DrillRecord>,
)> {
    let mut reader = Reader::new(bytes);

    let tiles_len = reader.read_u32()? as usize;
    if tiles_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "tile count {tiles_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }

    let mut tiles = Vec::with_capacity(tiles_len);
    for _ in 0..tiles_len {
        tiles.push(reader.read_u16()?);
    }

    let entity_count = reader.read_u32()? as usize;
    if entity_count > 0 && entity_count > reader.remaining() / BYTES_PER_ENTITY {
        return Err(StorageError::DecodeFailed(
            "entity count exceeds payload size".to_string(),
        ));
    }

    let mut entities = Vec::with_capacity(entity_count);
    for _ in 0..entity_count {
        let id = reader.read_u32()?;
        let kind = reader.read_u16()?;
        let x = reader.read_u16()?;
        let y = reader.read_u16()?;
        if x >= CHUNK_EDGE || y >= CHUNK_EDGE {
            return Err(StorageError::DecodeFailed(
                "entity position out of chunk bounds".to_string(),
            ));
        }
        entities.push(Entity { id, kind, x, y });
    }

    let resource_len = reader.read_u32()? as usize;
    if resource_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "resource count {resource_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }
    if resource_len > 0 && resource_len > reader.remaining() / BYTES_PER_RESOURCE {
        return Err(StorageError::DecodeFailed(
            "resource count exceeds payload size".to_string(),
        ));
    }

    let mut resources = Vec::with_capacity(resource_len);
    for _ in 0..resource_len {
        let kind = reader.read_u8()?;
        let amount = reader.read_u16()?;
        resources.push(ResourceCell { kind, amount });
    }

    let placed_len = reader.read_u32()? as usize;
    if placed_len != CHUNK_TILE_COUNT {
        return Err(StorageError::DecodeFailed(format!(
            "placed count {placed_len} does not match expected {CHUNK_TILE_COUNT}"
        )));
    }
    if placed_len > 0 && placed_len > reader.remaining() / BYTES_PER_PLACED_V4 {
        return Err(StorageError::DecodeFailed(
            "placed count exceeds payload size".to_string(),
        ));
    }

    let mut placed = Vec::with_capacity(placed_len);
    for _ in 0..placed_len {
        let kind = reader.read_u8()?;
        let object_id = reader.read_u64()?;
        placed.push(PlacedCell { kind, object_id });
    }

    let chest_count = reader.read_u32()? as usize;
    if chest_count > 0 && chest_count > reader.remaining() / BYTES_PER_CHEST {
        return Err(StorageError::DecodeFailed(
            "chest count exceeds payload size".to_string(),
        ));
    }
    let mut chests = Vec::with_capacity(chest_count);
    for _ in 0..chest_count {
        let object_id = reader.read_u64()?;
        let inv = decode_slots(&mut reader)?;
        chests.push(ChestRecord { object_id, inv });
    }

    let furnace_count = reader.read_u32()? as usize;
    if furnace_count > 0 && furnace_count > reader.remaining() / BYTES_PER_FURNACE {
        return Err(StorageError::DecodeFailed(
            "furnace count exceeds payload size".to_string(),
        ));
    }
    let mut furnaces = Vec::with_capacity(furnace_count);
    for _ in 0..furnace_count {
        let object_id = reader.read_u64()?;
        let input = decode_slot(&mut reader)?;
        let fuel = decode_slot(&mut reader)?;
        let output = decode_slot(&mut reader)?;
        let progress = reader.read_u16()?;
        furnaces.push(FurnaceRecord {
            object_id,
            state: FurnaceState {
                input,
                fuel,
                output,
                progress,
            },
        });
    }

    let inserter_count = reader.read_u32()? as usize;
    let bytes_per_inserter = if has_inserter_direction {
        BYTES_PER_INSERTER
    } else {
        BYTES_PER_INSERTER_V5
    };
    if inserter_count > 0 && inserter_count > reader.remaining() / bytes_per_inserter {
        return Err(StorageError::DecodeFailed(
            "inserter count exceeds payload size".to_string(),
        ));
    }
    let mut inserters = Vec::with_capacity(inserter_count);
    for _ in 0..inserter_count {
        let object_id = reader.read_u64()?;
        let direction = if has_inserter_direction {
            let raw_direction = reader.read_u8()?;
            InserterDirection::from_u8(raw_direction).ok_or_else(|| {
                StorageError::DecodeFailed(format!("invalid inserter direction {raw_direction}"))
            })?
        } else {
            InserterDirection::default()
        };
        let inv = decode_inserter_slots(&mut reader)?;
        inserters.push(InserterRecord {
            object_id,
            direction,
            inv,
        });
    }

    let drills = if has_drills {
        let drill_count = reader.read_u32()? as usize;
        if drill_count > 0 && drill_count > reader.remaining() / BYTES_PER_DRILL {
            return Err(StorageError::DecodeFailed(
                "drill count exceeds payload size".to_string(),
            ));
        }
        let mut drills = Vec::with_capacity(drill_count);
        for _ in 0..drill_count {
            let object_id = reader.read_u64()?;
            let fuel = decode_slot(&mut reader)?;
            let output = decode_slot(&mut reader)?;
            let progress = reader.read_u16()?;
            drills.push(DrillRecord {
                object_id,
                state: DrillState {
                    fuel,
                    output,
                    progress,
                },
            });
        }
        drills
    } else {
        Vec::new()
    };

    if reader.remaining() != 0 {
        return Err(StorageError::DecodeFailed(
            "payload has trailing bytes".to_string(),
        ));
    }

    Ok((
        tiles, entities, resources, placed, chests, furnaces, inserters, drills,
    ))
}

fn encode_slot(buffer: &mut Vec<u8>, slot: &Slot) {
    buffer.extend_from_slice(&slot.item.to_le_bytes());
    buffer.extend_from_slice(&slot.count.to_le_bytes());
}

fn encode_slots(buffer: &mut Vec<u8>, inv: &ContainerInv) {
    for slot in &inv.slots {
        encode_slot(buffer, slot);
    }
}

fn encode_inserter_slots(buffer: &mut Vec<u8>, inv: &InserterInv) {
    for slot in &inv.slots {
        encode_slot(buffer, slot);
    }
}

fn decode_slot(reader: &mut Reader<'_>) -> Result<Slot> {
    let item = reader.read_u16()?;
    let count = reader.read_u32()?;
    Ok(Slot { item, count })
}

fn decode_slots(reader: &mut Reader<'_>) -> Result<ContainerInv> {
    let mut inv = ContainerInv::default();
    for slot in &mut inv.slots {
        *slot = decode_slot(reader)?;
    }
    Ok(inv)
}

fn decode_inserter_slots(reader: &mut Reader<'_>) -> Result<InserterInv> {
    let mut inv = InserterInv::default();
    for slot in &mut inv.slots {
        *slot = decode_slot(reader)?;
    }
    Ok(inv)
}

fn header_len() -> usize {
    4 + 2 + 2 + 2 + 4 + 4 + 1 + 8 + 4
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| StorageError::DecodeFailed("read overflow".to_string()))?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| StorageError::DecodeFailed("unexpected end of data".to_string()))?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8> {
        let bytes = self.take(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_i32(&mut self) -> Result<i32> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

#[cfg(test)]
mod tests;
