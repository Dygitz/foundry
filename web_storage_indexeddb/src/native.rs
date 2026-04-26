use persistence::{
    ChunkCoord, ChunkKey, ChunkLayer, ChunkRecord, ChunkRecordWrite, MapChunkRecord,
    MapChunkRecordWrite, PlayerStateRecord, PlayerStateRecordWrite, RecoveryReport, SavepointId,
    StorageError, StorageFuture, WorldId, WorldMeta, WorldStorage,
};

#[derive(Debug, Clone)]
pub struct IndexedDbStorage {
    pub db_name: String,
    pub db_version: u32,
}

impl IndexedDbStorage {
    pub fn new<S: Into<String>>(db_name: S, db_version: u32) -> Self {
        Self {
            db_name: db_name.into(),
            db_version,
        }
    }
}

impl WorldStorage for IndexedDbStorage {
    fn init(&self) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn create_world(&self, _meta: WorldMeta) -> StorageFuture<'_, WorldId> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn list_worlds(&self) -> StorageFuture<'_, Vec<WorldMeta>> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn load_world_meta(&self, _world_id: &WorldId) -> StorageFuture<'_, Option<WorldMeta>> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn delete_world(&self, _world_id: &WorldId) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn load_player_state(
        &self,
        _world_id: &WorldId,
    ) -> StorageFuture<'_, Option<PlayerStateRecord>> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn save_player_state(&self, _record: PlayerStateRecordWrite) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn get_chunk(
        &self,
        _world_id: &WorldId,
        _coord: ChunkCoord,
        _layer: ChunkLayer,
    ) -> StorageFuture<'_, Option<ChunkRecord>> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn put_chunks(
        &self,
        _world_id: &WorldId,
        _records: Vec<ChunkRecordWrite>,
    ) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn delete_chunks(&self, _world_id: &WorldId, _keys: Vec<ChunkKey>) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn load_map_chunks(
        &self,
        _world_id: &WorldId,
        _layer: ChunkLayer,
    ) -> StorageFuture<'_, Vec<MapChunkRecord>> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn put_map_chunks(
        &self,
        _world_id: &WorldId,
        _records: Vec<MapChunkRecordWrite>,
    ) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn begin_savepoint(
        &self,
        _world_id: &WorldId,
        _tick: u64,
        _chunk_keys: Vec<ChunkKey>,
    ) -> StorageFuture<'_, SavepointId> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn commit_savepoint(&self, _savepoint_id: &SavepointId) -> StorageFuture<'_, ()> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }

    fn recover_incomplete_savepoints(
        &self,
        _world_id: &WorldId,
    ) -> StorageFuture<'_, RecoveryReport> {
        Box::pin(async move {
            Err(StorageError::InitFailed(
                "IndexedDB is only available for wasm32 targets".to_string(),
            ))
        })
    }
}
