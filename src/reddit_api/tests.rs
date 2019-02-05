#[cfg(test)]
mod tests {
    use crate::reddit_api::{Reddit, Sort, Type, MaxTime};
    use serde_json;
    use tokio_core;
    #[test]
    fn test_deserialize_response() {
        let response = include_str!("response.json");
        assert!(serde_json::from_str::<Type>(response).is_ok());
    }
    #[test]
    fn test_reddit_is_connected() {
        let mut reac = tokio_core::reactor::Core::new().unwrap();
        let reddit = Reddit::new("rustTest/0.1".into()).unwrap();
        assert!(reac.run(reddit.is_connected()).is_ok())
    }

    #[test]
    fn test_subreddit_call() {
        let mut reac = tokio_core::reactor::Core::new().unwrap();
        let reddit = Reddit::new("rustTest/0.1".into()).unwrap();
        let links = reac
            .run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::ALL, 10))
            .unwrap();
        assert_eq!(links.len(), 10);
        let links = reac
            .run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::ALL, 26))
            .unwrap();
        assert_eq!(links.len(), 26);
    }

    #[test]
    fn test_max_time() {
        let mut reac = tokio_core::reactor::Core::new().unwrap();
        let reddit = Reddit::new("rustTest/0.1".into()).unwrap();
        let link_all =
            reac.run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::ALL, 1));
        assert!(link_all.is_ok());
        let link_year =
            reac.run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::YEAR, 1));
        assert!(link_year.is_ok());
        let link_month =
            reac.run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::MONTH, 1));
        assert!(link_month.is_ok());
        let link_week =
            reac.run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::WEEK, 1));
        assert!(link_week.is_ok());
        let link_day =
            reac.run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, MaxTime::DAY, 1));
        assert!(link_day.is_ok());

    }

    #[test]
    fn test_run_concurrent_query() {
        use futures::future::join_all;
        let mut reac = tokio_core::reactor::Core::new().unwrap();
        let reddit = Reddit::new("rustTest/0.1".into()).unwrap();
        let res = reac.run(join_all((0..100).map(|_| reddit.is_connected())));
        assert!(res.is_ok());
    }
}
