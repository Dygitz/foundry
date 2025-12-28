use std::future::Future;
use std::pin::Pin;

use crate::errors::Result;
use crate::types::{
    ChunkCoord, ChunkKey, ChunkLayer, ChunkRecord, ChunkRecordWrite, RecoveryReport, SavepointId,
    WorldId, WorldMeta,
};
use simulation_core::{SimChunkData, SimChunkView};

pub type StorageFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

pub trait WorldStorage {
    fn init(&self) -> StorageFuture<'_, ()>;

    fn create_world(&self, meta: WorldMeta) -> StorageFuture<'_, WorldId>;
    fn list_worlds(&self) -> StorageFuture<'_, Vec<WorldMeta>>;
    fn load_world_meta(&self, world_id: &WorldId) -> StorageFuture<'_, Option<WorldMeta>>;
    fn delete_world(&self, world_id: &WorldId) -> StorageFuture<'_, ()>;

    fn get_chunk(
        &self,
        world_id: &WorldId,
        coord: ChunkCoord,
        layer: ChunkLayer,
    ) -> StorageFuture<'_, Option<ChunkRecord>>;
    fn put_chunks(
        &self,
        world_id: &WorldId,
        records: Vec<ChunkRecordWrite>,
    ) -> StorageFuture<'_, ()>;
    fn delete_chunks(&self, world_id: &WorldId, keys: Vec<ChunkKey>) -> StorageFuture<'_, ()>;

    fn begin_savepoint(
        &self,
        world_id: &WorldId,
        tick: u64,
        chunk_keys: Vec<ChunkKey>,
    ) -> StorageFuture<'_, SavepointId>;
    fn commit_savepoint(&self, savepoint_id: &SavepointId) -> StorageFuture<'_, ()>;
    fn recover_incomplete_savepoints(
        &self,
        world_id: &WorldId,
    ) -> StorageFuture<'_, RecoveryReport>;
}

pub trait ChunkCodec {
    fn encode(&self, chunk: &SimChunkView<'_>, tick: u64) -> Result<Vec<u8>>;
    fn decode(&self, bytes: &[u8]) -> Result<SimChunkData>;
}

pub trait ChunkStore {
    fn request_chunk(&mut self, key: ChunkKey) -> StorageFuture<'_, Option<SimChunkData>>;
    fn flush_dirty(&mut self, max: usize) -> StorageFuture<'_, ()>;
}
