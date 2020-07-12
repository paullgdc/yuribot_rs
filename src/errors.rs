use crate::db;
use crate::reddit_api;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum YuribotError {
    #[error("failed to parse config: {0}")]
    ConfigParseError(#[from] config::ConfigError),
    #[error("failed to open and read from config file: {0}")]
    ConfigFileError(#[from] std::io::Error),
    #[error("error while querying the database: {0}")]
    DatabaseError(#[from] db::errors::DatabaseError),
    #[error("error while sending message to Telegram: {0}")]
    TelegramSendError(#[from] telegram_bot::Error),
    #[error("error with reddit api: {0}")]
    RedditError(#[from] reddit_api::RedditError),
    #[error("migration error: {0}")]
    MigrationError(#[from] diesel_migrations::RunMigrationsError),
    #[error("no telegram bot token has been provided. Provide one either as an env variable YURIBOT_BOT_TOKEN, or as a variable in the Yuribot.toml")]
    NoTelegramTokenError,
    #[error("Unable to parse the command passed to the bot")]
    CommandArgParseError,
}

pub type Result<T> = std::result::Result<T, YuribotError>;
