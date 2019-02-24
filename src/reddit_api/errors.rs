use failure::Fail;

#[derive(Debug, Fail)]
pub enum RedditError {
    #[fail(display = "network error while fetching from reddit")]
    NetworkError,
    #[fail(display = "error while parsing response from reddit api call")]
    ParsingError,
    #[fail(display = "reddit api returned a {} code", error_code)]
    ApiError {error_code : u16 },
    #[fail(display = "received unexpected result from reddit api call")]
    UnexpectedResponse,
}
