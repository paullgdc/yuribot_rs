mod errors;
mod tests;
mod types;

pub use errors::RedditError;
pub use types::*;
use errors::Result;

use std::time::Duration;

use async_trait::async_trait;
use deadpool;
use guard::guard;
use hyper::{client::HttpConnector, header::USER_AGENT, Body, Client, Method, Request, Uri};
use hyper_tls::HttpsConnector;
use tokio::timer::Timeout;

#[derive(Debug)]
struct Inner {
    user_agent: String,
    client: Client<HttpsConnector<HttpConnector>>,
    timeout: Duration,
}

#[derive(Debug)]
pub struct Reddit {
    inner: Inner,
}

impl Reddit {
    pub fn new(user_agent: String, timeout: Duration) -> Result<Self> {
        let client: Client<HttpsConnector<_>, Body> = HttpsConnector::new()
            .map_err(|_| RedditError::NetworkError)
            .map(|https| Client::builder().build(https))?;
        Ok(Reddit {
            inner: Inner {
                user_agent,
                client,
                timeout,
            },
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
                let mut body = response.into_body();
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
            after = match listing.after {
                Some(after) => after,
                None => break,
            };
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

pub struct RedditManager {
    pub user_agent: String,
    pub timeout: Duration,
}

#[async_trait]
impl deadpool::Manager<Reddit, RedditError> for RedditManager {
    async fn create(&self) -> Result<Reddit> {
        Reddit::new(self.user_agent.clone(), self.timeout)
    }

    async fn recycle(&self, reddit: Reddit) -> Result<Reddit> {
        reddit.is_connected().await?;
        Ok(reddit)
    }
}

pub type RdPool = deadpool::Pool<Reddit, RedditError>;
