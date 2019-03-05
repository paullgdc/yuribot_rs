mod db;
mod reddit_api;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate log;

use env_logger;

use failure::Error;

use futures::future::Either;
use futures::{Future, Stream};
use telebot::bot;
use tokio_core::reactor::{Core, Interval};

// import all available functions
use telebot::file::File;
use telebot::functions::{FunctionSendMessage, FunctionSendPhoto};

fn is_image_url(url : &str) -> bool {
    url.ends_with(".png") || url.ends_with(".jpg") || url.ends_with(".jpeg")
}

fn main() -> Result<(), Error> {
    env_logger::Builder::from_env("YURIBOT_LOG").init();

    let mut reac = Core::new()?;

    let reddit = reddit_api::Reddit::new("rustTelegramBot.0.1".into())?;

    let database = db::Database::new("backup.sqlite3")?;

    let bot = bot::RcBot::new(
        reac.handle(),
        "626245263:AAHnIxc6IQkL26fzPiKCojW8IXeoedoEuFI",
    )
    .update_interval(200);

    reac.run(reddit.is_connected())?;
    let handle = bot
        .new_cmd("/top")
        .and_then({
            let database: db::Database = database.clone();
            move |(bot, msg)| {
                let response = database.fetch_random_link();
                debug!(
                    "received message : {:?} \n from chat : {:?} \n responding with {:?}",
                    msg.text, msg.chat, response
                );
                match response {
                    Ok(link) => Either::A(
                        bot.photo(msg.chat.id)
                            .file(File::Url(link.link))
                            .caption(link.title)
                            .send(),
                    ),
                    Err(err) => Either::B(
                        bot.message(msg.chat.id, format!("error : {:?}", err))
                            .send(),
                    ),
                }
            }
        })
        .then(|res| -> Result<(), ()> {
            if let Err(ref e) = res {
                error!("error while sending message : {}", e)
            };
            Ok(())
        });
    let pull_link = Interval::new(std::time::Duration::from_secs(10), &reac.handle())?
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
        .map_err(|e| Error::from(e))
        .for_each({
            let db: db::Database = database.clone();
            move |links| {
                for link in links.into_iter().filter(|link| is_image_url(&link.url)) {
                    debug!("inserting link in db : {:?}", link);
                    db.insert_link(&link.url, &link.title)?;
                }
                Ok(())
            }
        })
        .map_err(|e| {
            error!("error while refreshing database : {}", e);
            ()
        });

    reac.handle().spawn(pull_link);
    bot.register(handle);

    info!("yuribot started");
    bot.run(&mut reac)?;
    Ok(())
}
