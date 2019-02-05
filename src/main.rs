pub mod reddit_api;

use failure::Error;

use telebot::bot;
use tokio_core::reactor::Core;                       
use futures::stream::Stream;
use futures::Future;


// import all available functions
use telebot::functions::*;

fn main() -> Result<(), Error> {
    let mut reac = Core::new()?;
    let reddit = reddit_api::Reddit::new("rustTelegramBot.0.1".into())?;

    let bot = bot::RcBot::new(reac.handle(), "626245263:AAHnIxc6IQkL26fzPiKCojW8IXeoedoEuFI")
        .update_interval(200);
    reac.run(reddit.is_connected())?;
    let handle = bot.new_cmd("/top")
        .and_then({let reddit: reddit_api::Reddit = reddit.clone(); move |(bot, msg)| {
            println!("received message");
            reddit.subreddit_posts(
                "wholesomeyuri".into(), 
                reddit_api::Sort::TOP,
                reddit_api::MaxTime::ALL,
                1,
            )
            .then(move |posts| {
                let mut posts = dbg!(posts)?;
                if posts.len() == 0 {
                    Ok(bot.message(msg.chat.id, "no post".into()).send())
                } else {
                    let reply = dbg!(posts.remove(0).url);
                    Ok(bot.message(msg.chat.id, reply).send())
                }
            })
            .and_then(|res| res)
        }});

    bot.register(handle);
    println!("before running");
    bot.run(&mut reac)?;
    Ok(())
}
