use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    InitFailed(String),
    NotFound,
    QuotaExceeded,
    TransactionFailed(String),
    DecodeFailed(String),
    VersionMismatch { expected: u16, found: u16 },
    PermissionDenied,
    Other(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::InitFailed(message) => write!(f, "storage init failed: {message}"),
            StorageError::NotFound => write!(f, "not found"),
            StorageError::QuotaExceeded => write!(f, "storage quota exceeded"),
            StorageError::TransactionFailed(message) => {
                write!(f, "storage transaction failed: {message}")
            }
            StorageError::DecodeFailed(message) => write!(f, "decode failed: {message}"),
            StorageError::VersionMismatch { expected, found } => {
                write!(f, "version mismatch (expected {expected}, found {found})")
            }
            StorageError::PermissionDenied => write!(f, "storage permission denied"),
            StorageError::Other(message) => write!(f, "storage error: {message}"),
        }
    }
}

impl std::error::Error for StorageError {}
