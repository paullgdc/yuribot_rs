use diesel::prelude::*;
use super::schema::links;

#[derive(Queryable, Debug)]
pub struct Link {
    #[allow(dead_code)]
    id: i32,
    pub link: String,
    pub title: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = links)]
pub struct NewLink<'a> {
    pub link: &'a str,
    pub title: &'a str,
}
