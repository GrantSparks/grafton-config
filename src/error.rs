use {serde_json::Value, thiserror::Error};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid config: {0}")]
    ConfigError(String),
    #[error("Error serializing config: {0}")]
    SerializationError(String),
    #[error("Error deserializing config: {0}")]
    DeserializationError(String),
    #[error("Token resolve recursion detected at depth {depth}. Current path: {path}, Current value: {value:?}")]
    TokenRecursionLimitExceeded {
        depth: usize,
        path: String,
        value: Value,
    },
}
