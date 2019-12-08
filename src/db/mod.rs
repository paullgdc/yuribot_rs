pub mod errors;
pub mod model;
mod schema;
mod tests;

use diesel::prelude::*;
use errors::{Result, DatabaseError};
use async_trait::async_trait;

pub struct Database {
    connection: SqliteConnection,
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
        no_arg_sql_function!(RANDOM, (), "Represents the sql RANDOM() function");
        links
            .order(RANDOM)
            .limit(1)
            .first(&self.connection)
            .map_err(|e| e.into())
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
