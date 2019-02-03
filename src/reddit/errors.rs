use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum RedditError {
    NetworkError,
    ParsingError,
    ApiError(u16),
    UnexpectedResponse,
}

impl fmt::Display for RedditError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use RedditError::*;
        match self {
            NetworkError => write!(f, "network error while fetching"),
            ParsingError => write!(f, "error while parsing response"),
            ApiError(code) => write!(f, "reddit Api returned a {} code", code),
            UnexpectedResponse => write!(f, "received unexpected result from api call"),
        }
    }
}

impl Error for RedditError {}
