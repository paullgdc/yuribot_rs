use crate::db;
use crate::reddit_api;
use crate::YuribotError;

use std::time::Duration;

use futures::StreamExt;
use tokio::time;

fn is_image_url(url: &str) -> bool {
    url.ends_with(".png") || url.ends_with(".jpg") || url.ends_with(".jpeg")
}
async fn pull_links(
    database: &db::Database,
    reddit: &reddit_api::Reddit,
    link_number: usize,
    time: reddit_api::MaxTime,
) -> Result<(), YuribotError> {
    let links = reddit
        .subreddit_posts(
            "wholesomeyuri".to_owned(),
            reddit_api::Sort::HOT,
            time,
            link_number,
        )
        .await?;
    let insert_count = database.insert_links(
        &links
            .iter()
            .filter(|link| is_image_url(&link.url))
            .map(|link| db::model::NewLink {
                link: &link.url,
                title: &link.title,
            })
            .collect::<Vec<db::model::NewLink>>(),
    )?;
    info!("inserted {} new links in database", insert_count);
    Ok(())
}

pub async fn run_scrapper(db_pool: db::DbPool, rd_pool: reddit_api::RdPool) {
    let database = db_pool.get().await.expect("can't get database connection");
    let reddit = rd_pool
        .get()
        .await
        .expect("can't get reddit api connection");
    let mut interval = time::interval(Duration::from_secs(30 * 60));
    while let Some(_) = interval.next().await {
        if let Err(e) = pull_links(&database, &reddit, 3, reddit_api::MaxTime::DAY).await {
            error!("{}", e);
        }
    }
}

pub async fn seed_database(
    nb_posts: usize,
    rd_pool: reddit_api::RdPool,
    db_pool: db::DbPool,
) -> Result<(), YuribotError> {
    let reddit = rd_pool.get().await?;
    let database = db_pool.get().await?;
    pull_links(&database, &reddit, nb_posts, reddit_api::MaxTime::ALL).await?;
    Ok(())
}
