mod bot;
mod db;
mod errors;
mod parse_args;
mod reddit_api;
mod scrapper;
mod utils;

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

use errors::{Result, YuribotError};

pub(crate) const VERSION: &'static str = env!("CARGO_PKG_VERSION");

embed_migrations!("./migrations");

#[derive(Debug, Deserialize)]
pub struct Config {
    database_path: String,
    bot_token: Option<String>,
    reddit_user_agent: String,
    log: String,
}

fn read_config(path: &str) -> Result<Config> {
    let settings = (|| -> std::result::Result<config::Config, config::ConfigError> {
        let mut settings = config::Config::new();
        settings
            .set_default("database_path", "yuribot_rs.sqlite3")?
            .set_default("log", "yuribot_rs=info")?
            .set_default("reddit_user_agent", format!("yuribot_rs/{}", VERSION))
            .expect("default config values")
            .merge(config::File::with_name(path).required(false))?
            .merge(config::Environment::with_prefix("YURIBOT"))?;
        Ok(settings)
    })()
    .expect("couldn't build config");
    Ok(settings.try_into()?)
}

async fn inner_main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let conf: Config = read_config("Yuribot")?;
    env_logger::Builder::new().parse_filters(&conf.log).init();
    debug!("version {}", VERSION);

    let action = parse_args::parse_args(args);
    if let Help(usage) = action {
        println!("{}", usage);
        return Ok(());
    }

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
    info!("running migrations");
    embedded_migrations::run(&db_pool.get().await?.connection)?;

    use parse_args::Action::*;
    match action {
        RunBot => {
            let bot_api = telegram_bot::Api::new(
                conf.bot_token
                    .as_ref()
                    .ok_or(YuribotError::NoTelegramTokenError)?,
            );
            let bot_task = bot::start_bot(db_pool.clone(), bot_api).fuse();
            let scrapper_task = scrapper::run_scrapper(db_pool.clone(), rd_pool).fuse();
            pin_mut!(bot_task, scrapper_task);
            select!(
                _ = bot_task => (),
                _ = scrapper_task => (),
            )
        }
        SeedDatabase { limit } => scrapper::seed_database(limit, rd_pool, db_pool).await?,
        Help(_) => unreachable!(),
    };

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = inner_main().await {
        eprintln!("Fatal error: {}", e);
    }
}
