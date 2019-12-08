#[cfg(test)]
mod tests {
    use crate::reddit_api::{MaxTime, Reddit, RedditError, Sort, Type};
    use serde_json;
    use std::time::Duration;
    use tokio;

    #[test]
    fn test_deserialize_response() {
        let response = include_str!("response.json");
        assert!(serde_json::from_str::<Type>(response).is_ok());
    }
    #[tokio::test]
    async fn test_reddit_is_connected() {
        let reddit = Reddit::new(
            "rustTest/0.1".into(),
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(reddit.is_connected().await.is_ok())
    }

    #[tokio::test]
    async fn test_subreddit_call() {
        let reddit = Reddit::new(
            "rustTest/0.1".into(),
            Duration::from_secs(10),
        )
        .unwrap();
        let links = reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::ALL, 10)
            .await
            .unwrap();
        assert_eq!(links.len(), 10);
        let links = reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::ALL, 26)
            .await
            .unwrap();
        assert_eq!(links.len(), 26);
    }

    #[tokio::test]
    async fn test_max_time() {
        let reddit = Reddit::new(
            "rustTest/0.1".into(),
            Duration::from_secs(10),
        )
        .unwrap();
        let link_all = reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::ALL, 1)
            .await;
        assert!(link_all.is_ok());
        let link_year =
            reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::YEAR, 1).await;
        assert!(link_year.is_ok());
        let link_month =
            reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::MONTH, 1).await;
        assert!(link_month.is_ok());
        let link_week =
            reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::WEEK, 1).await;
        assert!(link_week.is_ok());
        let link_day =
            reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::DAY, 1).await;
        assert!(link_day.is_ok());
    }

    #[tokio::test]
    async fn test_run_concurrent_query() {
        use futures::future::join_all;
        let reddit = Reddit::new(
            "rustTest/0.1".into(),
            Duration::from_secs(10),
        )
        .unwrap();
        let res = join_all((0..100).map(|_| reddit.is_connected()))
            .await.into_iter()
            .collect::<Result<Vec<()>, RedditError>>();
        assert!(res.is_ok());
    }
}
