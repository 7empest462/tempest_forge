use thiserror::Error;

#[derive(Error, Debug)]
pub enum GameError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Player entity not found")]
    PlayerNotFound,
}
