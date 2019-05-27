#[macro_use]
extern crate diesel;

#[macro_use]
extern crate log;

mod db;
mod reddit_api;

use std::time::Duration;

use env_logger;

use failure::{Error, Fail, ResultExt};

use futures::future::Either;
use futures::{Future, IntoFuture, Stream};

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
    send_photo_command: String,
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
    toml::from_slice(&bytes)
        .context(YuribotError::ConfigParseError)
        .map_err(|e| e.into())
}

fn is_image_url(url: &str) -> bool {
    url.ends_with(".png") || url.ends_with(".jpg") || url.ends_with(".jpeg")
}

fn seed_database(
    nb_posts: usize,
    mut reac: Core,
    _conf: Config,
    reddit: reddit_api::Reddit,
    database: db::Database,
) -> Result<(), Error> {
    let fut = reddit
        .subreddit_posts(
            "wholesomeyuri".to_owned(),
            reddit_api::Sort::TOP,
            reddit_api::MaxTime::ALL,
            nb_posts,
        )
        .map_err(|e| e.context(YuribotError::RedditError))
        .and_then(|links| {
            debug!("inserting links in database\n {:?}", links);
            database
                .insert_links(
                    &links
                        .iter()
                        .filter(|link| is_image_url(&link.url))
                        .map(|link| db::model::NewLink {
                            link: &link.url,
                            title: &link.title,
                        })
                        .collect::<Vec<db::model::NewLink>>(),
                )
                .context(YuribotError::DatabaseError)?;
            Ok(())
        });
    reac.run(fut)?;
    Ok(())
}

fn run_bot(
    mut reac: Core,
    conf: Config,
    reddit: reddit_api::Reddit,
    database: db::Database,
) -> Result<(), Error> {
    let bot = bot::RcBot::new(reac.handle(), &conf.bot_token).update_interval(200);

    reac.run(reddit.is_connected())
        .context(YuribotError::RedditError)?;
    let handle = bot
        .new_cmd(&conf.send_photo_command)
        .and_then({
            let database: db::Database = database.clone();
            move |(bot, msg)| {
                database
                    .fetch_random_link()
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
                                bot.message(
                                    msg.chat.id,
                                    "an error happened ¯\\_(ツ)_/¯, maybe retry...".into(),
                                )
                                .send(),
                            ),
                        }
                    })
            }
        })
        .map_err(|e| e.context(YuribotError::TelegramSendError))
        .then(|res| -> Result<(), ()> {
            if let Err(ref e) = res {
                error!("error reponding to /more: {}", e)
            };
            Ok(())
        });
    let pull_link = Interval::new(Duration::from_secs(10), &reac.handle())?
        .then({
            let reddit = reddit.clone();
            move |_| {
                debug!("started fetching reddit links to update database");
                reddit.subreddit_posts(
                    "wholesomeyuri".to_owned(),
                    reddit_api::Sort::HOT,
                    reddit_api::MaxTime::DAY,
                    3,
                )
            }
        })
        .map_err(|e| e.context(YuribotError::RedditError))
        .and_then({
            move |links| {
                debug!("inserting links in database\n {:?}", links);
                database
                    .insert_links(
                        &links
                            .iter()
                            .filter(|link| is_image_url(&link.url))
                            .map(|link| db::model::NewLink {
                                link: &link.url,
                                title: &link.title,
                            })
                            .collect::<Vec<db::model::NewLink>>(),
                    )
                    .context(YuribotError::DatabaseError)?;
                Ok(())
            }
        })
        .then(|res| -> Result<(), ()> {
            if let Err(ref e) = res {
                error!("error while refreshing database : \n {}", e);
            };
            Ok(())
        })
        .for_each(|_| Ok(()));

    reac.handle().spawn(pull_link);
    bot.register(handle);

    info!("yuribot started");
    bot.run(&mut reac)?;
    Ok(())
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    let opts = {
        let mut opts = getopts::Options::new();
        opts.opt(
            "s",
            "seed",
            "initializes the database with pics from /top, defaults to 200 if no number is supplied",
            "N",
            getopts::HasArg::Maybe,
            getopts::Occur::Optional,
        );
        opts.optflag("h", "help", "prints the help");
        opts
    };

    env_logger::Builder::from_env("YURIBOT_LOG").init();

    let conf: Config = read_config_file("Yuribot.toml")?;

    let reac = Core::new()?;

    let reddit = reddit_api::Reddit::new(
        conf.reddit_user_agent.clone(),
        Duration::from_secs(10),
        reac.handle(),
    )
    .context(YuribotError::DatabaseError)?;

    let database = db::Database::new(&conf.database_path).context(YuribotError::DatabaseError)?;

    let matches: getopts::Matches = match opts.parse(args) {
        Ok(m) => m,
        Err(fail) => {
            println!("{}", opts.usage(&format!("{}", fail)));
            return Ok(());
        }
    };
    if matches.opt_present("help") {
        println!("{}", opts.usage(""));
        return Ok(());
    }
    if matches.opt_present("seed") {
        let nb_posts = match matches.opt_get_default::<usize>("seed", 200_usize) {
            Ok(i) => i,
            Err(_) => {
                println!(
                    "{}",
                    opts.usage("failed to parse --seed argument to integer")
                );
                return Ok(());
            }
        };
        seed_database(nb_posts, reac, conf, reddit, database)
    } else {
        run_bot(reac, conf, reddit, database)
    }
}
