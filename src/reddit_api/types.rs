use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "kind", content = "data")]
pub enum Type {
    #[serde(rename = "t3")]
    Link(Link),
    Listing(Listing),
}

#[derive(Debug, Deserialize)]
pub struct Listing {
    pub children: Vec<Type>,
    pub after: Option<String>,
    // pub before: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Link {
    // pub subreddit: String,
    pub title: String,
    // pub name: String,
    // pub over_18: bool,
    // pub pinned: bool,
    pub url: String,
    // pub spoiler: bool,
    // pub selftext: String,
    // pub score: i64,
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

    pub fn as_str(&self) -> &'static str {
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

    pub fn as_str(&self) -> &'static str {
        self.0
    }
}
