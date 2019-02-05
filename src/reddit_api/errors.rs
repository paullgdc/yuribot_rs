use failure::Fail;

#[derive(Debug, Fail)]
pub enum RedditError {
    #[fail(display = "network error while fetching")]
    NetworkError,
    #[fail(display = "error while parsing response")]
    ParsingError,
    #[fail(display = "reddit Api returned a {} code", error_code)]
    ApiError {error_code : u16 },
    #[fail(display = "received unexpected result from api call")]
    UnexpectedResponse,
}
