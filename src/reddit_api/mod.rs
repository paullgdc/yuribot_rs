mod errors;
mod tests;
pub use errors::RedditError;

use std::rc::*;

use futures::{
    future::{loop_fn, IntoFuture, Loop},
    Future, Stream,
};
use hyper::{client::HttpConnector, header::USER_AGENT, Body, Chunk, Client, Method, Request, Uri};
use hyper_tls::HttpsConnector;
use serde_derive::Deserialize;

#[derive(Debug)]
struct Inner {
    user_agent: String,
    client: Client<HttpsConnector<HttpConnector>>,
}

#[derive(Debug, Clone)]
pub struct Reddit {
    inner: Rc<Inner>,
}

impl Reddit {
    pub fn new(user_agent: String) -> Result<Self, RedditError> {
        let client: Client<HttpsConnector<_>, Body> = HttpsConnector::new(4)
            .map_err(|_| RedditError::NetworkError)
            .map(|https| Client::builder().build(https))?;
        Ok(Reddit {
            inner: Rc::new(Inner { user_agent, client }),
        })
    }

    pub fn is_connected(&self) -> impl Future<Item = (), Error = RedditError> {
        use std::str::FromStr;
        let uri: Uri = Uri::from_str("https://www.reddit.com/api/v1/me.json")
            .expect("can't parse https://reddit.com/api/v1/me.json as uri");
        self.api_call(uri).map(|_| ())
    }

    fn api_call(&self, uri: Uri) -> impl Future<Item = Chunk, Error = RedditError> {
        let request = Request::builder()
            .method(Method::GET)
            .header(USER_AGENT, self.inner.user_agent.clone())
            .uri(uri)
            .body(Body::empty())
            .expect("couldn't build request request");
        self.inner
            .client
            .request(request)
            .map_err(|_| RedditError::NetworkError)
            .and_then(|response| {
                if !response.status().is_success() {
                    return Err(RedditError::ApiError{error_code: response.status().as_u16()});
                }
                Ok(response)
            })
            .and_then(|response| {
                response
                    .into_body()
                    .concat2()
                    .map_err(|_| RedditError::NetworkError)
            })
    }

    pub fn subreddit_posts(
        &self,
        subreddit: String,
        sort: Sort,
        max_time: MaxTime,
        limit: usize,
    ) -> impl Future<Item = Vec<Link>, Error = RedditError> {
        let reddit = self.clone();
        loop_fn(
            (limit, None, Vec::with_capacity(limit)),
            move |(limit, after, mut results)| {
                let after = after.unwrap_or(String::new());
                Uri::builder()
                    .scheme("https")
                    .authority("www.reddit.com")
                    .path_and_query::<&str>(
                        format!(
                            "/r/{}{}.json?limit={}&after={}&t={}",
                            subreddit,
                            sort.as_str(),
                            if limit > 15 { 15 } else { limit },
                            after,
                            max_time.as_str(),
                        )
                        .as_ref(),
                    )
                    .build()
                    .map_err(|_| RedditError::ParsingError)
                    .into_future()
                    .and_then({
                        let reddit = reddit.clone();
                        move |uri| reddit.api_call(uri)
                    })
                    .and_then(move |chunk| {
                        let reponse = match serde_json::from_slice::<Type>(chunk.as_ref()) {
                            Ok(response) => response,
                            Err(_) => return Err(RedditError::ParsingError),
                        };
                        if let Type::Listing(listing) = reponse {
                            let after = listing.after;
                            for res_link in listing.children.into_iter().map(|child| {
                                if let Type::Link(link) = child {
                                    return Ok(link);
                                }
                                Err(RedditError::UnexpectedResponse)
                            }) {
                                match res_link {
                                    Ok(link) => results.push(link),
                                    Err(_) => return Err(RedditError::UnexpectedResponse),
                                }
                            }
                            let limit = limit.checked_sub(15).unwrap_or(0);
                            return if limit > 0 {
                                Ok(Loop::Continue((limit, after, results)))
                            } else {
                                Ok(Loop::Break(results))
                            };
                        };
                        Err(RedditError::UnexpectedResponse)
                    })
            },
        )
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
    children: Vec<Type>,
    after: Option<String>,
    before: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Link {
    pub subreddit: String,
    pub title: String,
    pub name: String,
    pub over_18: bool,
    pub pinned: bool,
    pub url: String,
    pub spoiler: bool,
    pub selftext: String,
    pub score: i64,
}

#[derive(Debug)]
pub struct Sort(&'static str);

impl Sort {
    pub const NEW: Sort = Sort("/new");
    pub const BEST: Sort = Sort("/best");
    pub const TOP: Sort = Sort("/top");
    pub const CONTROVERSIAL: Sort = Sort("/controversial");

    fn as_str(&self) -> &'static str {
        self.0
    }
}

#[derive(Debug)]
pub struct MaxTime(&'static str);

impl MaxTime {
    pub const ALL: MaxTime = MaxTime("all");
    pub const YEAR: MaxTime = MaxTime("year");
    pub const MONTH: MaxTime = MaxTime("month");
    pub const WEEK: MaxTime = MaxTime("week");
    pub const DAY: MaxTime = MaxTime("day");

    fn as_str(&self) -> &'static str {
        self.0
    }
}