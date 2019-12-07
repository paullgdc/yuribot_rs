mod errors;
mod tests;
pub use errors::RedditError;
use errors::Result;

use std::rc::*;
use std::time::Duration;

use guard::guard;
use hyper::{client::HttpConnector, header::USER_AGENT, Body, Client, Method, Request, Uri};
use hyper_tls::HttpsConnector;
use serde_derive::Deserialize;
use tokio::timer::Timeout;

#[derive(Debug)]
struct Inner {
    user_agent: String,
    client: Client<HttpsConnector<HttpConnector>>,
    timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct Reddit {
    inner: Rc<Inner>,
}

impl Reddit {
    pub fn new(user_agent: String, timeout: Duration) -> Result<Self> {
        let client: Client<HttpsConnector<_>, Body> = HttpsConnector::new()
            .map_err(|_| RedditError::NetworkError)
            .map(|https| Client::builder().build(https))?;
        Ok(Reddit {
            inner: Rc::new(Inner {
                user_agent,
                client,
                timeout,
            }),
        })
    }

    pub async fn is_connected(&self) -> Result<()> {
        use std::str::FromStr;
        let uri: Uri = Uri::from_str("https://www.reddit.com/api/v1/me.json")
            .expect("can't parse https://reddit.com/api/v1/me.json as uri");
        self.api_call(uri).await.map(|_| ())
    }

    async fn api_call(&self, uri: Uri) -> Result<Vec<u8>> {
        let request = Request::builder()
            .method(Method::GET)
            .header(USER_AGENT, self.inner.user_agent.clone())
            .uri(uri)
            .body(Body::empty())
            .expect("couldn't build request request");
        let response = self
            .inner
            .client
            .request(request)
            .await
            .map_err(|_| RedditError::NetworkError)?;
        if !response.status().is_success() {
            return Err(RedditError::ApiError {
                error_code: response.status().as_u16(),
            });
        }
        Timeout::new(
            async {
                let body = response.into_body();
                let mut bytes = Vec::new();
                while let Some(next) = body.next().await {
                    let chunk = next.map_err(|_| RedditError::NetworkError)?;
                    bytes.extend(chunk);
                }
                Ok(bytes)
            },
            self.inner.timeout,
        )
        .await
        .map_err(|_| RedditError::Timeout)?
    }

    pub async fn subreddit_posts(
        &self,
        subreddit: String,
        sort: Sort,
        max_time: MaxTime,
        limit: usize,
    ) -> Result<Vec<Link>> {
        let reddit = self.clone();
        let mut posts = Vec::with_capacity(limit);
        let mut after = String::new();
        let mut left = limit;
        while left > 0 {
            let uri = Uri::builder()
                .scheme("https")
                .authority("www.reddit.com")
                .path_and_query::<&str>(
                    format!(
                        "/r/{}{}.json?limit={}&after={}&t={}",
                        subreddit,
                        sort.as_str(),
                        if left > 25 { 25 } else { left },
                        after,
                        max_time.as_str(),
                    )
                    .as_ref(),
                )
                .build()
                .map_err(|_| RedditError::ParsingError)?;
            let data = self.api_call(uri).await?;
            let response =
                serde_json::from_slice::<Type>(&data).map_err(|_| RedditError::ParsingError)?;
            guard!(let Type::Listing(listing) = response else {
                return Err(RedditError::UnexpectedResponse)
            });
            let after = listing.after;
            for child in listing.children {
                guard!(let Type::Link(link) = child else {
                    return Err(RedditError::UnexpectedResponse)
                });
                posts.push(link);
            }
            left = left.checked_sub(25).unwrap_or(0);
        }
        Ok(posts)
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

#[allow(dead_code)]
impl Sort {
    pub const NEW: Sort = Sort("/new");
    pub const BEST: Sort = Sort("/best");
    pub const TOP: Sort = Sort("/top");
    pub const CONTROVERSIAL: Sort = Sort("/controversial");
    pub const HOT: Sort = Sort("/hot");

    fn as_str(&self) -> &'static str {
        self.0
    }
}

#[derive(Debug)]
pub struct MaxTime(&'static str);

#[allow(dead_code)]
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
