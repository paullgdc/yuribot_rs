use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RedditError {
    #[error("network error while fetching from reddit")]
    NetworkError,
    #[error("network query timed out")]
    Timeout,
    #[error("io error {}", 0)]
    IoError(#[from] io::Error),
    #[error("error while parsing response from reddit api call")]
    ParsingError,
    #[error("reddit api returned a {error_code} code")]
    ApiError {error_code : u16 },
    #[error("received unexpected result from reddit api call")]
    UnexpectedResponse,
}

pub type Result<T> = std::result::Result<T, RedditError>;
