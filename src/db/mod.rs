pub mod errors;
pub mod model;
mod schema;

use async_trait::async_trait;
use diesel::{dsl::Asc, prelude::*};
use errors::{DatabaseError, Result};

no_arg_sql_function!(RANDOM, (), "Represents the sql RANDOM() function");

fn escape_fts(search: &str) -> String {
    let mut escaped = String::with_capacity(search.len() + 2);
    escaped.push('"');
    for c in search.chars() {
        if c == '"' {
            escaped.push_str("\"\"");
        } else {
            escaped.push(c);
        }
    }
    escaped.push('"');
    escaped
}

pub struct Database {
    pub connection: SqliteConnection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        Ok(Database {
            connection: SqliteConnection::establish(path)?,
        })
    }

    #[allow(dead_code)]
    pub fn insert_link<'a>(&self, link: &'a str, title: &'a str) -> Result<usize> {
        let new_link = model::NewLink { link, title };
        diesel::insert_or_ignore_into(schema::links::table)
            .values(new_link)
            .execute(&self.connection)
            .map_err(|e| e.into())
    }

    pub fn insert_links<'a>(&self, new_links: &[model::NewLink<'a>]) -> Result<usize> {
        diesel::insert_or_ignore_into(schema::links::table)
            .values(new_links)
            .execute(&self.connection)
            .map_err(|e| e.into())
    }

    pub fn fetch_random_link(&self) -> Result<model::Link> {
        use schema::links::dsl::*;
        links
            .order(RANDOM)
            .limit(1)
            .first(&self.connection)
            .map_err(|e| e.into())
    }

    pub fn count_links(&self) -> Result<i64> {
        use schema::links::dsl::*;
        links.count().first(&self.connection).map_err(|e| e.into())
    }

    pub fn search_random_full_text(&self, search: &str) -> Result<Option<model::Link>> {
        use schema::links_title_idx;
        links_title_idx::table
            .select((
                links_title_idx::id,
                links_title_idx::link,
                links_title_idx::title,
            ))
            .filter(links_title_idx::whole_row.eq(escape_fts(search)))
            .order(RANDOM)
            .limit(1)
            .first(&self.connection)
            .optional()
            .map_err(|e| e.into())
    }

    pub fn count_links_search(&self, search: &str) -> Result<i64> {
        use schema::links_title_idx;
        links_title_idx::table
            .count()
            .filter(links_title_idx::whole_row.eq(escape_fts(search)))
            .first(&self.connection)
            .map_err(|e| e.into())
    }

    pub fn get_all(&self, start_at_id: i32) -> Result<Vec<model::Link>> {
        use schema::links;
        Ok(links::table
            .select((links::id, links::link, links::title))
            .filter(links::id.ge(start_at_id))
            .order(Asc::new(links::id))
            .get_results(&self.connection)?)
    }

    pub fn delete(&self, id: i32) -> Result<()> {
        use schema::links;
        diesel::delete(links::table.filter(links::id.eq(id))).execute(&self.connection)?;
        Ok(())
    }
}

pub struct DatabaseManager {
    pub path: String,
}

#[async_trait]
impl deadpool::Manager<Database, DatabaseError> for DatabaseManager {
    async fn create(&self) -> Result<Database> {
        Database::new(&self.path)
    }

    async fn recycle(&self, db: Database) -> Result<Database> {
        Ok(db)
    }
}

pub type DbPool = deadpool::Pool<Database, DatabaseError>;

#[test]
fn test_escape_fts() {
    assert_eq!("\"test string 132\"", escape_fts("test string 132"));
    assert_eq!(
        "\"test \"\"string\"\" 132\"",
        escape_fts("test \"string\" 132")
    );
}
