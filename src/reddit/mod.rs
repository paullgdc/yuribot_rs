mod errors;
mod tests;
pub use errors::RedditError;

use futures::{Future, future::IntoFuture, Stream};
use hyper::{
    Client,
    Body,
    Uri,
    Method, 
    Request,
    client::{ResponseFuture, HttpConnector},
    header::{USER_AGENT},
    Chunk,
};
use hyper_tls::HttpsConnector;
use serde_derive::Deserialize;

#[derive(Debug)]
struct Inner {
    user_agent : String,
    client : Client<HttpsConnector<HttpConnector>>,

}

#[derive(Debug)]
pub struct Reddit {
    user_agent : String,
    client : Client<HttpsConnector<HttpConnector>>,
}

impl Reddit {
    pub fn new(user_agent : String) -> Result<Self, RedditError> {
        let client : Client<HttpsConnector<_>, Body> = HttpsConnector::new(4)
            .map_err(|_| RedditError::NetworkError)
            .map(|https| Client::builder().build(https))?;
        Ok(Reddit {
            user_agent,
            client,
        })
    }

    pub fn is_connected(&self) -> impl Future<Item=(), Error=RedditError> {
        use std::str::FromStr;
        let uri : Uri = Uri::from_str("https://www.reddit.com/api/v1/me.json")
            .expect("can't parse https://reddit.com/api/v1/me.json as uri");
        self.api_call(uri)
            .map(|_| ())
    }
 
    fn api_call(&self, uri : Uri) -> impl Future<Item=Chunk, Error=RedditError> {
        let request = Request::builder()
            .method(Method::GET)
            .header(USER_AGENT, self.user_agent.clone())
            .uri(uri)
            .body(Body::empty())
            .expect("couldn't build request request");
        self.client.request(request)
            .map_err(|_| RedditError::NetworkError)
            .and_then(|response| {
                if !response.status().is_success() {
                    return Err(RedditError::ApiError(response.status().as_u16()))
                }
                Ok(response)
            })
            .and_then(|response| {
                response.into_body()
                    .concat2()
                    .map_err(|_| RedditError::NetworkError)
            })
    }

    fn subreddit_posts(&self, subreddit : &str, order : Order) -> impl Future<Item=Vec<Link>, Error=RedditError> {
        Uri::builder()
            .scheme("https")
            .authority("www.reddit.com")
            .path_and_query::<&str>(format!("/r/{}{}.json", subreddit, order.as_str()).as_ref())
            .build()
            .map_err(|_| RedditError::ParsingError)
            .map(|uri| self.api_call(uri))
            .into_future()
            .flatten()
            .and_then(|chunk| {
                let reponse = serde_json::from_slice::<Type>(chunk.as_ref())
                    .map_err(|_| RedditError::ParsingError)?;
                if let Type::Listing(listing) = reponse {
                    return listing.children.into_iter()
                        .map(|child| {
                            if let Type::Link(link) = child {
                                return Ok(link);
                            }
                            Err(RedditError::UnexpectedResponse)
                        })
                        .collect::<Result<Vec<Link>, RedditError>>();
                }
                Err(RedditError::UnexpectedResponse)
            })
    } 
}

#[derive(Deserialize, Debug)]
#[serde(tag = "kind", content = "data")]
pub enum Type {
    #[serde(rename = "t3")]
    Link(Link),
    Listing(Listing),
}

#[derive(Debug, Deserialize)]
pub struct Listing {
    children : Vec<Type>,
    after : Option<String>,
    before : Option<String>
}

#[derive(Deserialize, Debug)]
pub struct Link {
    subreddit : String,
    title : String,
    over_18 : bool,
    pinned : bool,
    url : String,
    spoiler : bool,
    selftext : String,
}

pub struct Order (&'static str);

impl Order {
    pub const NEW : Order = Order("/new");
    pub const BEST :Order = Order("/best");
    pub const TOP :Order = Order("/top");
    pub const CONTROVERSIAL :Order = Order("/controversial");

    fn as_str(&self) -> &'static str {
        self.0
    }
}