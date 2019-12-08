use thiserror::Error;
use crate::reddit_api;
use crate::db;

#[derive(Debug, Error)]
pub enum YuribotError {
    #[error("failed to parse Yuribot.toml config file: {0}")]
    ConfigParseError(#[from] toml::de::Error),
    #[error("failed to open and read from Yuribot.toml config file: {0}")]
    ConfigFileError(#[from] std::io::Error),
    #[error("error while querying the database: {0}")]
    DatabaseError(#[from] db::errors::DatabaseError),
    #[error("error while sending message to Telegram")]
    TelegramSendError,
    #[error("error with reddit api: {0}")]
    RedditError(#[from] reddit_api::RedditError),
}
