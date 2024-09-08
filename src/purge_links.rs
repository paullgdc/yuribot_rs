use crate::db::{self, model};
use std::convert::{TryFrom, TryInto};

use thiserror::Error;

type Client = hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>;

#[derive(Debug, Error)]
pub(crate) enum PurgeError {
    #[error("error while querying the database: {0}")]
    Database(#[from] db::errors::DatabaseError),
    #[error("error while deleting id {0}: (1")]
    DatabasDelete(i32, db::errors::DatabaseError),
    #[error("error while looking for link: {0}")]
    LinkCheckBuildReq(#[from] hyper::http::Error),
    #[error("error while looking for link: {0}")]
    LinkCheck(#[from] hyper::Error),
    #[error("error following links, invalid uri: {0}")]
    InvalidUri(#[from] hyper::http::uri::InvalidUri),
    #[error("error following links, invalid uri: {0}")]
    InvalidUriParts(#[from] hyper::http::uri::InvalidUriParts),
    #[error("received status code {0} looking for link {1} at index {2}")]
    UnexpectedStatusCode(u16, String, i32),
    #[error("failed to cast start_id to i32")]
    InvalidStartIdValue(#[from] std::num::TryFromIntError),
}

pub async fn purge_links(
    db_pool: db::DbPool,
    dry_run: bool,
    start_at_id: usize,
) -> Result<(), PurgeError> {
    let https = hyper_tls::HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);

    let db = db_pool.get().await?;
    let links = db.get_all(start_at_id.try_into()?)?;
    log::info!("checking {} links", links.len());
    let mut removed_count = 0;
    for (i, link) in links.iter().enumerate() {
        if i != 0 && i % 100 == 0 {
            log::info!(
                "link {}/{}, at id={}, removed {}",
                i,
                links.len(),
                link.id,
                removed_count
            );
        }
        let found = match check_link(&client, link).await {
            Ok(b) => b,
            Err(e) => {
                log::error!("error while fetching link {e}");
                match e {
                    PurgeError::UnexpectedStatusCode(521, url, _)
                        if url.starts_with("http://cdn.awwni.me/169qt.jpg") =>
                    {
                        false
                    }
                    _ => continue,
                }
            }
        };
        if !found {
            removed_count += 1;
            if !dry_run {
                db.delete(link.id)
                    .map_err(|e| PurgeError::DatabasDelete(link.id, e))?;
            }
        }
    }
    log::info!("Removed {} links", removed_count);
    Ok(())
}

const MAX_ATTEMPTS: u32 = 5;

async fn check_link(client: &Client, link: &model::Link) -> Result<bool, PurgeError> {
    let mut status = 0;
    for _ in 0..MAX_ATTEMPTS {
        let res = request_follow_redirects(client, hyper::Uri::try_from(&link.link)?, 10).await?;
        status = res.status().as_u16();
        if status == 404 {
            return Ok(false);
        } else if 200 <= status && status < 300 {
            return Ok(true);
        }
    }
    return Err(PurgeError::UnexpectedStatusCode(
        status,
        link.link.to_owned(),
        link.id,
    ));
}

async fn request_follow_redirects(
    client: &Client,
    uri: hyper::Uri,
    max_redirects: u32,
) -> Result<hyper::Response<hyper::Body>, PurgeError> {
    let mut uri = uri;
    for _ in 0..max_redirects {
        let req: hyper::Request<hyper::Body> = hyper::Request::builder()
            .method("HEAD")
            .uri(uri.clone())
            .body(hyper::Body::empty())?;

        let res = client.request(req).await?;
        let status = res.status().as_u16();

        if !(300 <= status && status < 400) {
            return Ok(res);
        }
        let Some(next) = res.headers().get(hyper::http::header::LOCATION) else {
            return Ok(res);
        };
        let next_uri = hyper::Uri::try_from(next.as_bytes())?;
        if uri.authority().is_none() {
            let mut parts = uri.into_parts();
            parts.path_and_query = next_uri.into_parts().path_and_query;
            uri = hyper::Uri::from_parts(parts)?;
        } else {
            uri = next_uri;
        }
    }
    todo!()
}
