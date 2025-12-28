use crate::errors::{Result, StorageError};
use crate::traits::ChunkCodec;
use simulation_core::{
    ChunkCoord, CHUNK_EDGE, CHUNK_TILE_COUNT, Entity, SimChunkData, SimChunkView, TileId,
};

const MAGIC: [u8; 4] = *b"CHNK";
const FORMAT_VERSION: u16 = 1;
const FLAGS_NONE: u16 = 0;
const BYTES_PER_ENTITY: usize = 12;

#[derive(Debug, Clone, Copy)]
pub struct ChunkCodecV1 {
    game_schema_version: u16,
}

impl ChunkCodecV1 {
    pub fn new(game_schema_version: u16) -> Self {
        Self { game_schema_version }
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

        // NOTE: checksum is computed by the storage layer, not the codec.
        let payload = encode_payload_v1(chunk)?;
        let payload_len = u32::try_from(payload.len()).map_err(|_| {
            StorageError::Other("payload length exceeds u32::MAX".to_string())
        })?;

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
        if format_version != FORMAT_VERSION {
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
        let (tiles, entities) = decode_payload_v1(payload_bytes)?;

        Ok(SimChunkData {
            coord: ChunkCoord { cx, cy },
            layer,
            tiles,
            entities,
            saved_tick,
        })
    }
}

fn encode_payload_v1(chunk: &SimChunkView<'_>) -> Result<Vec<u8>> {
    let tiles_len = u32::try_from(chunk.tiles.len()).map_err(|_| {
        StorageError::Other("tiles length exceeds u32::MAX".to_string())
    })?;
    let entity_count = u32::try_from(chunk.entities.len()).map_err(|_| {
        StorageError::Other("entity count exceeds u32::MAX".to_string())
    })?;

    let mut buffer = Vec::with_capacity(
        4 + (chunk.tiles.len() * 2) + 4 + (chunk.entities.len() * BYTES_PER_ENTITY),
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
        let slice = self.bytes.get(self.offset..end).ok_or_else(|| {
            StorageError::DecodeFailed("unexpected end of data".to_string())
        })?;
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
