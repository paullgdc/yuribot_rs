#[macro_use]
extern crate diesel;

#[macro_use]
extern crate log;

mod db;
mod reddit_api;

use env_logger;

use failure::{Error, Fail, ResultExt};

use futures::future::Either;
use futures::{Future, Stream, IntoFuture};

use tokio_core::reactor::{Core, Interval};

use toml;

use serde_derive::Deserialize;

use telebot::bot;
use telebot::file::File;
use telebot::functions::{FunctionSendMessage, FunctionSendPhoto};

#[derive(Debug, Deserialize)]
struct Config {
    database_path: String,
    bot_token: String,
    reddit_user_agent: String,
}

#[derive(Debug, Fail)]
enum YuribotError {
    #[fail(display = "failed to parse Yuribot.toml config file")]
    ConfigParseError,
    #[fail(display = "failed to open and read from Yuribot.toml config file")]
    ConfigFileError,
    #[fail(display = "error while querying the database")]
    DatabaseError,
    #[fail(display = "error while sending message to Telegram")]
    TelegramSendError,
    #[fail(display = "error with reddit api")]
    RedditError,
}

fn read_config_file(path: &str) -> Result<Config, Error> {
    let bytes = std::fs::read(path).context(YuribotError::ConfigFileError)?;
    toml::from_slice(&bytes).context(YuribotError::ConfigParseError)
        .map_err(|e| e.into())
}

fn is_image_url(url: &str) -> bool {
    url.ends_with(".png") || url.ends_with(".jpg") || url.ends_with(".jpeg")
}

fn main() -> Result<(), Error> {
    env_logger::Builder::from_env("YURIBOT_LOG").init();

    let conf: Config = read_config_file("Yuribot.toml")?;

    let mut reac = Core::new()?;

    let reddit = reddit_api::Reddit::new(conf.reddit_user_agent.clone())
        .context(YuribotError::DatabaseError)?;

    let database = db::Database::new(&conf.database_path)
        .context(YuribotError::DatabaseError)?;

    let bot = bot::RcBot::new(
        reac.handle(),
        &conf.bot_token,
    )
    .update_interval(200);

    reac.run(reddit.is_connected())
        .context(YuribotError::RedditError)?;
    let handle = bot
        .new_cmd("/top")
        .and_then({
            let database: db::Database = database.clone();
            move |(bot, msg)| {
                database.fetch_random_link()
                    .context(YuribotError::DatabaseError)
                    .into_future()
                    .then(move |maybe_link| {
                        debug!(
                            "received message : {:?} \n from chat : {:?} \n responding with {:?}",
                            msg.text, msg.chat, maybe_link
                        );
                        match maybe_link {
                            Ok(link) => Either::A(
                                bot.photo(msg.chat.id)
                                    .file(File::Url(link.link))
                                    .caption(link.title)
                                    .send(),
                            ),
                            Err(_) => Either::B(
                                bot.message(msg.chat.id, "an error happened ¯\\_(ツ)_/¯, maybe retry...".into())
                                    .send(),
                            ),
                        }
                    })
            }
        })
        .map_err(|e| e.context(YuribotError::TelegramSendError))
        .then(|res| -> Result<(), ()> {
            if let Err(ref e) = res {
                error!("{}", e)
            };
            Ok(())
        });
    let pull_link = Interval::new(std::time::Duration::from_secs(60 * 30), &reac.handle())?
        .then({
            let reddit = reddit.clone();
            move |_| {
                reddit.subreddit_posts(
                    "wholesomeyuri".to_owned(),
                    reddit_api::Sort::HOT,
                    reddit_api::MaxTime::DAY,
                    3,
                )
            }
        })
        .map_err(|e| e.context(YuribotError::RedditError))
        .for_each({
            let db: db::Database = database.clone();
            move |links| {
                for link in links.into_iter().filter(|link| is_image_url(&link.url)) {
                    debug!("inserting link in db : {:?}", link);
                    db.insert_link(&link.url, &link.title)
                        .context(YuribotError::DatabaseError)?;
                }
                Ok(())
            }
        })
        .map_err(|e| {
            error!("error while refreshing database : \n {}", e);
            ()
        });

    reac.handle().spawn(pull_link);
    bot.register(handle);

    info!("yuribot started");
    bot.run(&mut reac)?;
    Ok(())
}
