use std::fmt;

pub use simulation_core::{ChunkCoord, ChunkLayer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorldId(String);

impl WorldId {
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for WorldId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorldId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChunkKey {
    pub world_id: WorldId,
    pub coord: ChunkCoord,
    pub layer: ChunkLayer,
}

impl ChunkKey {
    pub fn new(world_id: WorldId, coord: ChunkCoord, layer: ChunkLayer) -> Self {
        Self {
            world_id,
            coord,
            layer,
        }
    }

    pub fn to_key_string(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.world_id, self.coord.cx, self.coord.cy, self.layer
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMeta {
    pub world_id: WorldId,
    pub display_name: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub schema_version: u16,
    pub seed: Option<u64>,
    pub last_saved_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRecord {
    pub key: ChunkKey,
    pub blob: Vec<u8>,
    pub tick_saved: u64,
    pub checksum: u32,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRecordWrite {
    pub key: ChunkKey,
    pub blob: Vec<u8>,
    pub tick_saved: u64,
    pub checksum: u32,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapChunkRecord {
    pub key: ChunkKey,
    pub rgba: Vec<u8>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapChunkRecordWrite {
    pub key: ChunkKey,
    pub rgba: Vec<u8>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStateRecord {
    pub world_id: WorldId,
    pub blob: Vec<u8>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStateRecordWrite {
    pub world_id: WorldId,
    pub blob: Vec<u8>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SavepointId(String);

impl SavepointId {
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SavepointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for SavepointId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SavepointId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavepointStatus {
    Writing,
    Committed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Savepoint {
    pub id: SavepointId,
    pub world_id: WorldId,
    pub tick: u64,
    pub created_at_ms: u64,
    pub status: SavepointStatus,
    pub chunk_keys: Vec<ChunkKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub incomplete_savepoints: Vec<SavepointId>,
}
