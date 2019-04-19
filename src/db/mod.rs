mod schema;
pub mod model;
mod tests;

use failure::Error;
use diesel::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct Database {
    connection : Rc<SqliteConnection>,
}

impl Database {
    pub fn new(path : &str) -> Result<Self, Error> {
        Ok(Database {
            connection: Rc::new(SqliteConnection::establish(path)?)
        })
    }

    pub fn insert_link<'a>(&self, link : &'a str, title : &'a str) -> Result<usize, Error>{
        let new_link = model::NewLink {
            link,
            title,
        };
        diesel::insert_or_ignore_into(schema::links::table)
            .values(new_link)
            .execute(&*self.connection)
            .map_err(|e| e.into())
    }

    pub fn insert_links<'a>(&self, new_links: &[model::NewLink<'a>]) -> Result<usize, Error>{
        diesel::insert_or_ignore_into(schema::links::table)
            .values(new_links)
            .execute(&*self.connection)
            .map_err(|e| e.into())
    }

    pub fn fetch_random_link(&self) -> Result<model::Link, Error> {
        use schema::links::dsl::*;
        no_arg_sql_function!(RANDOM, (), "Represents the sql RANDOM() function");
        links
            .order(RANDOM)
            .limit(1)
            .first(&*self.connection)
            .map_err(|e| e.into())
    }
}