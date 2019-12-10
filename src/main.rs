mod bot;
mod db;
mod errors;
mod parse_args;
mod reddit_api;
mod scrapper;

use std::time::Duration;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel_migrations;
use env_logger;
use futures::{pin_mut, select, FutureExt};
use serde::Deserialize;
use toml;

use errors::{YuribotError, Result};

embed_migrations!("./migrations");

#[derive(Debug, Deserialize)]
struct Config {
    database_path: String,
    bot_token: String,
    reddit_user_agent: String,
    send_photo_command: String,
}

fn read_config_file(path: &str) -> Result<Config> {
    let bytes = std::fs::read(path)?;
    Ok(toml::from_slice(&bytes)?)
}

async fn inner_main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    env_logger::Builder::from_env("YURIBOT_LOG").init();
    let conf: Config = read_config_file("Yuribot.toml")?;
    let rd_pool = deadpool::Pool::new(
        reddit_api::RedditManager {
            user_agent: conf.reddit_user_agent.clone(),
            timeout: Duration::from_secs(10),
        },
        16,
    );
    let db_pool = deadpool::Pool::new(
        db::DatabaseManager {
            path: conf.database_path.clone(),
        },
        4,
    );
    let bot_api = telegram_bot::Api::new(&conf.bot_token);

    info!("running migrations");
    embedded_migrations::run(&db_pool.get().await?.connection)?;

    use parse_args::Action::*;
    match parse_args::parse_args(args) {
        RunBot => {
            let bot_task = bot::start_bot(db_pool.clone(), bot_api).fuse();
            let scrapper_task = scrapper::run_scrapper(db_pool.clone(), rd_pool).fuse();
            pin_mut!(bot_task, scrapper_task);
            select!(
                _ = bot_task => (),
                _ = scrapper_task => (),
            )
        }
        SeedDatabase { limit } => scrapper::seed_database(limit, rd_pool, db_pool).await?,
        Help(usage) => println!("{}", usage),
    };

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = inner_main().await {
        error!("Fatal: {}", e);
    }
}
