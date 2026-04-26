use persistence::{
    ChunkCoord, ChunkKey, ChunkLayer, ChunkRecord, ChunkRecordWrite, MapChunkRecord,
    MapChunkRecordWrite, PlayerStateRecord, PlayerStateRecordWrite, RecoveryReport, SavepointId,
    StorageError, StorageFuture, WorldId, WorldMeta, WorldStorage,
};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use web_time::{SystemTime, UNIX_EPOCH};

    use idb::request::OpenDatabaseRequest;
    use idb::{
        Database, DatabaseEvent, Event, Factory, KeyPath, ObjectStore, ObjectStoreParams, Query,
        Request, TransactionMode,
    };
    use serde::{Deserialize, Serialize};
    use serde_wasm_bindgen::{from_value, to_value};
    use wasm_bindgen::JsValue;

    const STORE_WORLDS: &str = "worlds";
    const STORE_CHUNKS: &str = "chunks";
    const STORE_MAP_CHUNKS: &str = "map_chunks";
    const STORE_SAVEPOINTS: &str = "savepoints";
    const STORE_PLAYER_STATE: &str = "player_state";
    const INDEX_BY_WORLD: &str = "by_world";
    const INDEX_BY_WORLD_COORD: &str = "by_world_coord";
    const STATUS_WRITING: &str = "writing";
    const STATUS_COMMITTED: &str = "committed";
    const FIELD_WORLD_ID: &str = "world_id";
    const FIELD_CHUNK_KEY: &str = "chunk_key";
    const FIELD_SAVEPOINT_ID: &str = "savepoint_id";
    const FIELD_CX: &str = "cx";
    const FIELD_CY: &str = "cy";
    const FIELD_LAYER: &str = "layer";

    #[derive(Debug, Serialize, Deserialize)]
    struct WorldMetaRecord {
        world_id: String,
        display_name: String,
        created_at_ms: f64,
        updated_at_ms: f64,
        schema_version: u16,
        seed: Option<f64>,
        last_saved_tick: Option<f64>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ChunkRecordRecord {
        chunk_key: String,
        world_id: String,
        cx: i32,
        cy: i32,
        layer: u8,
        blob: Vec<u8>,
        tick_saved: f64,
        checksum: u32,
        updated_at_ms: f64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct MapChunkRecordRecord {
        chunk_key: String,
        world_id: String,
        cx: i32,
        cy: i32,
        layer: u8,
        rgba: Vec<u8>,
        #[serde(default)]
        resource_kinds: Vec<u8>,
        #[serde(default)]
        resource_amounts: Vec<u16>,
        updated_at_ms: f64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct SavepointRecord {
        savepoint_id: String,
        world_id: String,
        tick: f64,
        created_at_ms: f64,
        status: String,
        chunk_keys: Vec<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct PlayerStateRecordRecord {
        world_id: String,
        blob: Vec<u8>,
        updated_at_ms: f64,
    }

    #[derive(Debug, Clone)]
    pub struct IndexedDbStorage {
        pub db_name: String,
        pub db_version: u32,
        db: Rc<RefCell<Option<Rc<Database>>>>,
    }

    impl IndexedDbStorage {
        pub fn new<S: Into<String>>(db_name: S, db_version: u32) -> Self {
            Self {
                db_name: db_name.into(),
                db_version,
                db: Rc::new(RefCell::new(None)),
            }
        }

        async fn get_db(&self) -> Result<Rc<Database>, StorageError> {
            if let Some(db) = self.db.borrow().as_ref() {
                return Ok(db.clone());
            }

            let db = Rc::new(open_db(&self.db_name, self.db_version).await?);
            *self.db.borrow_mut() = Some(db.clone());
            Ok(db)
        }
    }

    impl WorldStorage for IndexedDbStorage {
        fn init(&self) -> StorageFuture<'_, ()> {
            Box::pin(async move {
                self.get_db().await?;
                Ok(())
            })
        }

        fn create_world(&self, meta: WorldMeta) -> StorageFuture<'_, WorldId> {
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_WORLDS], TransactionMode::ReadWrite)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_WORLDS)
                    .map_err(map_idb_error)?;

                let value = world_meta_to_js(&meta)?;
                store
                    .put(&value, None)
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                Ok(meta.world_id.clone())
            })
        }

        fn list_worlds(&self) -> StorageFuture<'_, Vec<WorldMeta>> {
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_WORLDS], TransactionMode::ReadOnly)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_WORLDS)
                    .map_err(map_idb_error)?;

                let values = store
                    .get_all(None, None)
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction.await.map_err(map_idb_error)?;

                let mut worlds = Vec::with_capacity(values.len());
                for entry in values {
                    worlds.push(world_meta_from_js(&entry)?);
                }

                Ok(worlds)
            })
        }

        fn load_world_meta(&self, world_id: &WorldId) -> StorageFuture<'_, Option<WorldMeta>> {
            let world_id = world_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_WORLDS], TransactionMode::ReadOnly)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_WORLDS)
                    .map_err(map_idb_error)?;
                let value = store
                    .get(JsValue::from_str(world_id.as_str()))
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction.await.map_err(map_idb_error)?;

                Ok(value.map(|value| world_meta_from_js(&value)).transpose()?)
            })
        }

        fn delete_world(&self, world_id: &WorldId) -> StorageFuture<'_, ()> {
            let world_id = world_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                // Best-effort delete: concurrent writes may leave residual records.
                let chunk_keys = fetch_index_keys(&db, STORE_CHUNKS, &world_id).await?;
                let map_chunk_keys = fetch_index_keys(&db, STORE_MAP_CHUNKS, &world_id).await?;
                let savepoint_keys = fetch_index_keys(&db, STORE_SAVEPOINTS, &world_id).await?;

                let transaction = db
                    .transaction(
                        &[
                            STORE_WORLDS,
                            STORE_CHUNKS,
                            STORE_MAP_CHUNKS,
                            STORE_SAVEPOINTS,
                            STORE_PLAYER_STATE,
                        ],
                        TransactionMode::ReadWrite,
                    )
                    .map_err(map_idb_error)?;

                let worlds = transaction
                    .object_store(STORE_WORLDS)
                    .map_err(map_idb_error)?;
                let chunks = transaction
                    .object_store(STORE_CHUNKS)
                    .map_err(map_idb_error)?;
                let map_chunks = transaction
                    .object_store(STORE_MAP_CHUNKS)
                    .map_err(map_idb_error)?;
                let savepoints = transaction
                    .object_store(STORE_SAVEPOINTS)
                    .map_err(map_idb_error)?;
                let player_state = transaction
                    .object_store(STORE_PLAYER_STATE)
                    .map_err(map_idb_error)?;

                worlds
                    .delete(JsValue::from_str(world_id.as_str()))
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                for key in chunk_keys {
                    chunks
                        .delete(key)
                        .map_err(map_idb_error)?
                        .await
                        .map_err(map_idb_error)?;
                }

                for key in map_chunk_keys {
                    map_chunks
                        .delete(key)
                        .map_err(map_idb_error)?
                        .await
                        .map_err(map_idb_error)?;
                }

                for key in savepoint_keys {
                    savepoints
                        .delete(key)
                        .map_err(map_idb_error)?
                        .await
                        .map_err(map_idb_error)?;
                }

                player_state
                    .delete(JsValue::from_str(world_id.as_str()))
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;
                Ok(())
            })
        }

        fn load_player_state(
            &self,
            world_id: &WorldId,
        ) -> StorageFuture<'_, Option<PlayerStateRecord>> {
            let world_id = world_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_PLAYER_STATE], TransactionMode::ReadOnly)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_PLAYER_STATE)
                    .map_err(map_idb_error)?;

                let value = store
                    .get(JsValue::from_str(world_id.as_str()))
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction.await.map_err(map_idb_error)?;

                let Some(value) = value else {
                    return Ok(None);
                };
                let record: PlayerStateRecordRecord =
                    from_value(value).map_err(map_serde_decode_error)?;

                Ok(Some(PlayerStateRecord {
                    world_id: WorldId::from(record.world_id),
                    blob: record.blob,
                    updated_at_ms: f64_to_u64(record.updated_at_ms, "updated_at_ms")?,
                }))
            })
        }

        fn save_player_state(&self, record: PlayerStateRecordWrite) -> StorageFuture<'_, ()> {
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_PLAYER_STATE], TransactionMode::ReadWrite)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_PLAYER_STATE)
                    .map_err(map_idb_error)?;

                let record = PlayerStateRecordRecord {
                    world_id: record.world_id.as_str().to_string(),
                    blob: record.blob,
                    updated_at_ms: u64_to_f64(record.updated_at_ms),
                };
                let value = to_value(&record).map_err(map_serde_encode_error)?;

                store
                    .put(&value, None)
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;
                Ok(())
            })
        }

        fn get_chunk(
            &self,
            world_id: &WorldId,
            coord: ChunkCoord,
            layer: ChunkLayer,
        ) -> StorageFuture<'_, Option<ChunkRecord>> {
            let world_id = world_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_CHUNKS], TransactionMode::ReadOnly)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_CHUNKS)
                    .map_err(map_idb_error)?;

                let key = ChunkKey::new(world_id.clone(), coord, layer).to_key_string();
                let value = store
                    .get(JsValue::from_str(&key))
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction.await.map_err(map_idb_error)?;

                Ok(value
                    .map(|value| chunk_record_from_js(&value))
                    .transpose()?)
            })
        }

        fn put_chunks(
            &self,
            _world_id: &WorldId,
            records: Vec<ChunkRecordWrite>,
        ) -> StorageFuture<'_, ()> {
            Box::pin(async move {
                if records.is_empty() {
                    return Ok(());
                }

                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_CHUNKS], TransactionMode::ReadWrite)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_CHUNKS)
                    .map_err(map_idb_error)?;

                for record in records {
                    let value = chunk_record_write_to_js(&record)?;
                    store
                        .put(&value, None)
                        .map_err(map_idb_error)?
                        .await
                        .map_err(map_idb_error)?;
                }

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;
                Ok(())
            })
        }

        fn delete_chunks(&self, _world_id: &WorldId, keys: Vec<ChunkKey>) -> StorageFuture<'_, ()> {
            Box::pin(async move {
                if keys.is_empty() {
                    return Ok(());
                }

                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_CHUNKS], TransactionMode::ReadWrite)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_CHUNKS)
                    .map_err(map_idb_error)?;

                for key in keys {
                    store
                        .delete(JsValue::from_str(&key.to_key_string()))
                        .map_err(map_idb_error)?
                        .await
                        .map_err(map_idb_error)?;
                }

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;
                Ok(())
            })
        }

        fn load_map_chunks(
            &self,
            world_id: &WorldId,
            layer: ChunkLayer,
        ) -> StorageFuture<'_, Vec<MapChunkRecord>> {
            let world_id = world_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_MAP_CHUNKS], TransactionMode::ReadOnly)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_MAP_CHUNKS)
                    .map_err(map_idb_error)?;
                let index = store.index(INDEX_BY_WORLD).map_err(map_idb_error)?;
                let values = index
                    .get_all(Some(Query::Key(JsValue::from_str(world_id.as_str()))), None)
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction.await.map_err(map_idb_error)?;

                let mut records = Vec::with_capacity(values.len());
                for entry in values {
                    let record = map_chunk_record_from_js(&entry)?;
                    if record.key.layer == layer {
                        records.push(record);
                    }
                }
                Ok(records)
            })
        }

        fn put_map_chunks(
            &self,
            _world_id: &WorldId,
            records: Vec<MapChunkRecordWrite>,
        ) -> StorageFuture<'_, ()> {
            Box::pin(async move {
                if records.is_empty() {
                    return Ok(());
                }

                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_MAP_CHUNKS], TransactionMode::ReadWrite)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_MAP_CHUNKS)
                    .map_err(map_idb_error)?;

                for record in records {
                    let value = map_chunk_record_write_to_js(&record)?;
                    store
                        .put(&value, None)
                        .map_err(map_idb_error)?
                        .await
                        .map_err(map_idb_error)?;
                }

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;
                Ok(())
            })
        }

        fn begin_savepoint(
            &self,
            world_id: &WorldId,
            tick: u64,
            chunk_keys: Vec<ChunkKey>,
        ) -> StorageFuture<'_, SavepointId> {
            let world_id = world_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_SAVEPOINTS], TransactionMode::ReadWrite)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_SAVEPOINTS)
                    .map_err(map_idb_error)?;

                let created_at_ms = now_ms();
                let savepoint_id =
                    SavepointId::from(format!("{}:{}:{}", world_id.as_str(), tick, created_at_ms));
                let value = savepoint_to_js(
                    &savepoint_id,
                    &world_id,
                    tick,
                    created_at_ms,
                    STATUS_WRITING,
                    chunk_keys,
                )?;

                store
                    .put(&value, None)
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                Ok(savepoint_id)
            })
        }

        fn commit_savepoint(&self, savepoint_id: &SavepointId) -> StorageFuture<'_, ()> {
            let savepoint_id = savepoint_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_SAVEPOINTS], TransactionMode::ReadWrite)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_SAVEPOINTS)
                    .map_err(map_idb_error)?;

                let value = store
                    .get(JsValue::from_str(savepoint_id.as_str()))
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                let Some(value) = value else {
                    transaction.await.map_err(map_idb_error)?;
                    return Err(StorageError::NotFound);
                };

                let mut record = savepoint_from_js(value)?;
                record.status = STATUS_COMMITTED.to_string();
                let value = savepoint_record_to_js(&record)?;

                store
                    .put(&value, None)
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction
                    .commit()
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;
                Ok(())
            })
        }

        fn recover_incomplete_savepoints(
            &self,
            world_id: &WorldId,
        ) -> StorageFuture<'_, RecoveryReport> {
            let world_id = world_id.clone();
            Box::pin(async move {
                let db = self.get_db().await?;
                let transaction = db
                    .transaction(&[STORE_SAVEPOINTS], TransactionMode::ReadOnly)
                    .map_err(map_idb_error)?;
                let store = transaction
                    .object_store(STORE_SAVEPOINTS)
                    .map_err(map_idb_error)?;
                let index = store.index(INDEX_BY_WORLD).map_err(map_idb_error)?;
                let values = index
                    .get_all(Some(Query::Key(JsValue::from_str(world_id.as_str()))), None)
                    .map_err(map_idb_error)?
                    .await
                    .map_err(map_idb_error)?;

                transaction.await.map_err(map_idb_error)?;

                let mut incomplete = Vec::new();
                for entry in values {
                    let record = savepoint_from_js(entry)?;
                    if record.status == STATUS_WRITING {
                        incomplete.push(SavepointId::from(record.savepoint_id));
                    }
                }

                Ok(RecoveryReport {
                    incomplete_savepoints: incomplete,
                })
            })
        }
    }

    async fn open_db(name: &str, version: u32) -> Result<Database, StorageError> {
        let factory = Factory::new().map_err(map_idb_error)?;
        let mut request = factory.open(name, Some(version)).map_err(map_idb_error)?;

        let schema_error = Rc::new(RefCell::new(None));
        let schema_error_handle = schema_error.clone();
        request.on_upgrade_needed(move |event| {
            let database = match event.database() {
                Ok(database) => database,
                Err(error) => {
                    *schema_error_handle.borrow_mut() = Some(map_idb_error(error));
                    return;
                }
            };
            let request = match event.target() {
                Ok(request) => request,
                Err(error) => {
                    *schema_error_handle.borrow_mut() = Some(map_idb_error(error));
                    return;
                }
            };

            if let Err(error) = setup_schema(&database, &request) {
                *schema_error_handle.borrow_mut() = Some(error);
            }
        });

        let mut database = request.await.map_err(map_idb_error)?;
        if let Some(error) = schema_error.borrow_mut().take() {
            return Err(error);
        }

        database.on_version_change(|event| {
            if let Ok(database) = event.database() {
                database.close();
            }
        });

        Ok(database)
    }

    fn setup_schema(
        database: &Database,
        request: &OpenDatabaseRequest,
    ) -> Result<(), StorageError> {
        let store_names = database.store_names();

        if !store_names.iter().any(|name| name == STORE_WORLDS) {
            let mut params = ObjectStoreParams::new();
            params.key_path(Some(KeyPath::new_single(FIELD_WORLD_ID)));
            database
                .create_object_store(STORE_WORLDS, params)
                .map_err(map_idb_error)?;
        }

        let chunks_store = get_or_create_store(
            database,
            request,
            &store_names,
            STORE_CHUNKS,
            FIELD_CHUNK_KEY,
        )?;
        ensure_index(
            &chunks_store,
            INDEX_BY_WORLD,
            KeyPath::new_single(FIELD_WORLD_ID),
        )?;
        ensure_index(
            &chunks_store,
            INDEX_BY_WORLD_COORD,
            KeyPath::new_array([FIELD_WORLD_ID, FIELD_CX, FIELD_CY, FIELD_LAYER]),
        )?;

        let map_chunks_store = get_or_create_store(
            database,
            request,
            &store_names,
            STORE_MAP_CHUNKS,
            FIELD_CHUNK_KEY,
        )?;
        ensure_index(
            &map_chunks_store,
            INDEX_BY_WORLD,
            KeyPath::new_single(FIELD_WORLD_ID),
        )?;

        let savepoints_store = get_or_create_store(
            database,
            request,
            &store_names,
            STORE_SAVEPOINTS,
            FIELD_SAVEPOINT_ID,
        )?;
        ensure_index(
            &savepoints_store,
            INDEX_BY_WORLD,
            KeyPath::new_single(FIELD_WORLD_ID),
        )?;

        let _player_state_store = get_or_create_store(
            database,
            request,
            &store_names,
            STORE_PLAYER_STATE,
            FIELD_WORLD_ID,
        )?;

        Ok(())
    }

    fn get_or_create_store(
        database: &Database,
        request: &OpenDatabaseRequest,
        store_names: &[String],
        store_name: &str,
        key_path: &str,
    ) -> Result<ObjectStore, StorageError> {
        if store_names.iter().any(|name| name == store_name) {
            let transaction = request.transaction().ok_or_else(|| {
                StorageError::TransactionFailed("upgrade transaction missing".to_string())
            })?;
            transaction.object_store(store_name).map_err(map_idb_error)
        } else {
            let mut params = ObjectStoreParams::new();
            params.key_path(Some(KeyPath::new_single(key_path)));
            database
                .create_object_store(store_name, params)
                .map_err(map_idb_error)
        }
    }

    fn ensure_index(
        store: &ObjectStore,
        index_name: &str,
        key_path: KeyPath,
    ) -> Result<(), StorageError> {
        if !store.index_names().iter().any(|name| name == index_name) {
            store
                .create_index(index_name, key_path, None)
                .map_err(map_idb_error)?;
        }

        Ok(())
    }

    async fn fetch_index_keys(
        db: &Database,
        store_name: &str,
        world_id: &WorldId,
    ) -> Result<Vec<JsValue>, StorageError> {
        let transaction = db
            .transaction(&[store_name], TransactionMode::ReadOnly)
            .map_err(map_idb_error)?;
        let store = transaction
            .object_store(store_name)
            .map_err(map_idb_error)?;
        let index = store.index(INDEX_BY_WORLD).map_err(map_idb_error)?;
        let keys = index
            .get_all_keys(Some(Query::Key(JsValue::from_str(world_id.as_str()))), None)
            .map_err(map_idb_error)?
            .await
            .map_err(map_idb_error)?;

        transaction.await.map_err(map_idb_error)?;
        Ok(keys)
    }

    fn world_meta_to_js(meta: &WorldMeta) -> Result<JsValue, StorageError> {
        let record = WorldMetaRecord {
            world_id: meta.world_id.as_str().to_string(),
            display_name: meta.display_name.clone(),
            created_at_ms: u64_to_f64(meta.created_at_ms),
            updated_at_ms: u64_to_f64(meta.updated_at_ms),
            schema_version: meta.schema_version,
            seed: option_u64_to_f64(meta.seed),
            last_saved_tick: option_u64_to_f64(meta.last_saved_tick),
        };
        to_value(&record).map_err(map_serde_encode_error)
    }

    fn world_meta_from_js(value: &JsValue) -> Result<WorldMeta, StorageError> {
        let record: WorldMetaRecord = from_value(value.clone()).map_err(map_serde_decode_error)?;
        Ok(WorldMeta {
            world_id: WorldId::from(record.world_id),
            display_name: record.display_name,
            created_at_ms: f64_to_u64(record.created_at_ms, "created_at_ms")?,
            updated_at_ms: f64_to_u64(record.updated_at_ms, "updated_at_ms")?,
            schema_version: record.schema_version,
            seed: option_f64_to_u64(record.seed, "seed")?,
            last_saved_tick: option_f64_to_u64(record.last_saved_tick, "last_saved_tick")?,
        })
    }

    fn chunk_record_write_to_js(record: &ChunkRecordWrite) -> Result<JsValue, StorageError> {
        let record = ChunkRecordRecord {
            chunk_key: record.key.to_key_string(),
            world_id: record.key.world_id.as_str().to_string(),
            cx: record.key.coord.cx,
            cy: record.key.coord.cy,
            layer: record.key.layer,
            blob: record.blob.clone(),
            tick_saved: u64_to_f64(record.tick_saved),
            checksum: record.checksum,
            updated_at_ms: u64_to_f64(record.updated_at_ms),
        };
        to_value(&record).map_err(map_serde_encode_error)
    }

    fn chunk_record_from_js(value: &JsValue) -> Result<ChunkRecord, StorageError> {
        let record: ChunkRecordRecord =
            from_value(value.clone()).map_err(map_serde_decode_error)?;
        let key = ChunkKey::new(
            WorldId::from(record.world_id),
            ChunkCoord {
                cx: record.cx,
                cy: record.cy,
            },
            record.layer,
        );
        Ok(ChunkRecord {
            key,
            blob: record.blob,
            tick_saved: f64_to_u64(record.tick_saved, "tick_saved")?,
            checksum: record.checksum,
            updated_at_ms: f64_to_u64(record.updated_at_ms, "updated_at_ms")?,
        })
    }

    fn map_chunk_record_write_to_js(record: &MapChunkRecordWrite) -> Result<JsValue, StorageError> {
        let record = MapChunkRecordRecord {
            chunk_key: record.key.to_key_string(),
            world_id: record.key.world_id.as_str().to_string(),
            cx: record.key.coord.cx,
            cy: record.key.coord.cy,
            layer: record.key.layer,
            rgba: record.rgba.clone(),
            resource_kinds: record.resource_kinds.clone(),
            resource_amounts: record.resource_amounts.clone(),
            updated_at_ms: u64_to_f64(record.updated_at_ms),
        };
        to_value(&record).map_err(map_serde_encode_error)
    }

    fn map_chunk_record_from_js(value: &JsValue) -> Result<MapChunkRecord, StorageError> {
        let record: MapChunkRecordRecord =
            from_value(value.clone()).map_err(map_serde_decode_error)?;
        let key = ChunkKey::new(
            WorldId::from(record.world_id),
            ChunkCoord {
                cx: record.cx,
                cy: record.cy,
            },
            record.layer,
        );
        Ok(MapChunkRecord {
            key,
            rgba: record.rgba,
            resource_kinds: record.resource_kinds,
            resource_amounts: record.resource_amounts,
            updated_at_ms: f64_to_u64(record.updated_at_ms, "updated_at_ms")?,
        })
    }

    fn savepoint_to_js(
        savepoint_id: &SavepointId,
        world_id: &WorldId,
        tick: u64,
        created_at_ms: u64,
        status: &str,
        chunk_keys: Vec<ChunkKey>,
    ) -> Result<JsValue, StorageError> {
        let record = SavepointRecord {
            savepoint_id: savepoint_id.as_str().to_string(),
            world_id: world_id.as_str().to_string(),
            tick: u64_to_f64(tick),
            created_at_ms: u64_to_f64(created_at_ms),
            status: status.to_string(),
            chunk_keys: chunk_keys
                .into_iter()
                .map(|key| key.to_key_string())
                .collect(),
        };
        savepoint_record_to_js(&record)
    }

    fn savepoint_from_js(value: JsValue) -> Result<SavepointRecord, StorageError> {
        from_value(value).map_err(map_serde_decode_error)
    }

    fn savepoint_record_to_js(record: &SavepointRecord) -> Result<JsValue, StorageError> {
        to_value(record).map_err(map_serde_encode_error)
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn u64_to_f64(value: u64) -> f64 {
        // TODO: values above 2^53 lose precision; migrate to BigInt or string encoding.
        value as f64
    }

    fn option_u64_to_f64(value: Option<u64>) -> Option<f64> {
        value.map(|value| value as f64)
    }

    fn f64_to_u64(value: f64, field: &str) -> Result<u64, StorageError> {
        if value.is_finite() && value >= 0.0 {
            Ok(value as u64)
        } else {
            Err(StorageError::DecodeFailed(format!(
                "expected non-negative number for {field}"
            )))
        }
    }

    fn option_f64_to_u64(value: Option<f64>, field: &str) -> Result<Option<u64>, StorageError> {
        match value {
            Some(value) => Ok(Some(f64_to_u64(value, field)?)),
            None => Ok(None),
        }
    }

    fn map_serde_encode_error(error: serde_wasm_bindgen::Error) -> StorageError {
        StorageError::Other(format!("serde encode failed: {error}"))
    }

    fn map_serde_decode_error(error: serde_wasm_bindgen::Error) -> StorageError {
        StorageError::DecodeFailed(format!("serde decode failed: {error}"))
    }

    fn map_idb_error(error: idb::Error) -> StorageError {
        match error {
            idb::Error::DomException(exception) => match exception.name().as_str() {
                "QuotaExceededError" => StorageError::QuotaExceeded,
                "SecurityError" => StorageError::PermissionDenied,
                _ => StorageError::TransactionFailed(exception.message()),
            },
            idb::Error::IndexedDbNotFound(_) => {
                StorageError::InitFailed("IndexedDB unavailable".to_string())
            }
            idb::Error::IndexedDbOpenFailed(error) => {
                StorageError::InitFailed(js_value_to_string(&error))
            }
            _ => StorageError::TransactionFailed(error.to_string()),
        }
    }

    fn js_value_to_string(value: &JsValue) -> String {
        value.as_string().unwrap_or_else(|| format!("{value:?}"))
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::IndexedDbStorage;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct IndexedDbStorage {
    pub db_name: String,
    pub db_version: u32,
}

#[cfg(not(target_arch = "wasm32"))]
impl IndexedDbStorage {
    pub fn new<S: Into<String>>(db_name: S, db_version: u32) -> Self {
        Self {
            db_name: db_name.into(),
            db_version,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
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
