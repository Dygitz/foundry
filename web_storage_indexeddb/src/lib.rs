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
