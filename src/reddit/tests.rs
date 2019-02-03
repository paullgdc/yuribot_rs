#[cfg(test)]
mod tests {
    use crate::reddit::{Reddit, Sort, Type};
    use serde_json;
    use tokio_core;
    #[test]
    fn test_deserialize_response() {
        let response = include_str!("response.json");
        assert!(dbg!(serde_json::from_str::<Type>(response)).is_ok());
    }
    #[test]
    fn test_reddit_is_connected() {
        let mut reac = tokio_core::reactor::Core::new().unwrap();
        let reddit = Reddit::new("rustTest/0.1 by lunatiks".into()).unwrap();
        assert!(dbg!(reac.run(reddit.is_connected())).is_ok())
    }

    #[test]
    fn test_subreddit_call() {
        let mut reac = tokio_core::reactor::Core::new().unwrap();
        let reddit = Reddit::new("rustTest/0.1 by lunatiks".into()).unwrap();
        let links =
            dbg!(reac.run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, 10))).unwrap();
        assert_eq!(links.len(), 10);
        let links =
            dbg!(reac.run(reddit.subreddit_posts("wholesomeyuri".into(), Sort::TOP, 26))).unwrap();
        assert_eq!(links.len(), 26);
    }
}
