use crate::db;
use crate::utils::utf8_pos_from_utf16;
use crate::Result;

use std::convert::TryInto;
use std::time::Duration;

use futures::StreamExt;
use guard::guard;
use telegram_bot::{
    prelude::{CanReplySendMessage, CanSendPhoto},
    types::{GetMe, InputFileRef, Message, MessageEntityKind, MessageKind, UpdateKind},
    Api,
};

mod message {
    use guard::guard;
    use telegram_bot::types::{Message, MessageChat, MessageKind};
    pub type ArgRange = std::ops::Range<usize>;
    pub fn get_arg(message: &Message, range: ArgRange) -> Option<&str> {
        guard!(let MessageKind::Text {ref data, ref entities} = message.kind else { return None });
        Some(data[range].trim())
    }
    pub fn is_private(message: &Message) -> bool {
        if let MessageChat::Private(_) = message.chat {
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
enum Command {
    More { arg: message::ArgRange },
    Count { arg: message::ArgRange },
    Version,
    Unrecognized,
}

impl Command {
    fn from_message(botname: &str, message: &Message) -> Option<(Self, bool)> {
        guard!(let MessageKind::Text {ref data, ref entities} = message.kind else { return None });
        let entity = entities.get(0)?;
        guard!(let MessageEntityKind::BotCommand = entity.kind else { return None });
        if entity.offset != 0 {
            return None;
        }
        let length = utf8_pos_from_utf16(data, entity.length.try_into().ok()?)?;
        let (command, is_directed) = if data[..length].ends_with(botname) {
            (&data[..(length - botname.len())], true)
        } else {
            (&data[..length], false)
        };

        let command = match command {
            "/more" => Command::More {
                arg: (length..data.len()),
            },
            "/count" => Command::Count {
                arg: (length..data.len()),
            },
            "/version" => Command::Version,
            _ => Command::Unrecognized,
        };
        Some((command, is_directed))
    }
}

async fn handle_more(
    database: db::DbPool,
    api: Api,
    message: Message,
    arg_range: message::ArgRange,
) -> Result<()> {
    let arg =
        message::get_arg(&message, arg_range).ok_or(crate::YuribotError::CommandArgParseError)?;
    let link = if arg == "" {
        Some(database.get().await?.fetch_random_link()?)
    } else {
        database.get().await?.search_random_full_text(arg)?
    };

    let link = match link {
        Some(l) => l,
        None => {
            api.send_timeout(
                message.text_reply("There is no image in the database for this. Sorry :("),
                Duration::from_secs(5),
            )
            .await?;
            return Ok(());
        }
    };
    info!(
        "Sending image\n\t{}: {}\n\tUser: {:?}\n\tChat: {:?}",
        link.title, link.link, message.from.username, message.chat
    );
    let response = api
        .send_timeout(
            message
                .chat
                .photo(InputFileRef::new(link.link))
                .caption(link.title),
            Duration::from_secs(5),
        )
        .await?;
    debug!("responded with: {:?}", response);
    Ok(())
}

async fn handle_count(
    database: db::DbPool,
    api: Api,
    message: Message,
    arg_range: message::ArgRange,
) -> Result<()> {
    let arg =
        message::get_arg(&message, arg_range).ok_or(crate::YuribotError::CommandArgParseError)?;
    let link_count = if arg == "" {
        database.get().await?.count_links()?
    } else {
        database.get().await?.count_links_search(arg)?
    };
    api.send_timeout(
        message.text_reply(format!(
            "There are {} links in the database for this query",
            link_count
        )),
        Duration::from_secs(5),
    )
    .await?;
    Ok(())
}

async fn handle_unrecognized(is_directed_to_bot: bool, api: Api, message: Message) -> Result<()> {
    if !is_directed_to_bot {
        return Ok(());
    }
    api.send_timeout(
        message.text_reply("Unrecognized command"),
        Duration::from_secs(5),
    )
    .await?;
    Ok(())
}

async fn handle_version(api: Api, message: Message) -> Result<()> {
    api.send_timeout(message.text_reply(crate::VERSION), Duration::from_secs(5))
        .await?;
    Ok(())
}

fn spawn_response<T, E>(fut: T)
where
    T: std::future::Future<Output = Result<E>> + Send + 'static,
    E: Send,
{
    tokio::spawn(async {
        if let Err(e) = fut.await {
            error!("telegram message handling error: {}", e);
        }
    });
}

pub async fn start_bot(db_pool: db::DbPool, api: Api) {
    info!("started the bot");
    let mut stream = api.stream();
    let botname = match api.send_timeout(GetMe, Duration::from_secs(5)).await {
        Ok(user) => user
            .and_then(|u| u.username)
            .map(|u| {
                let mut prefix = "@".to_owned();
                prefix.push_str(&u);
                prefix
            })
            .unwrap_or(String::new()),
        Err(e) => {
            error!("fatal: couldn't get bot name: {}", e);
            return;
        }
    };
    info!("bot running as {}", botname);
    stream.error_delay(Duration::from_secs(5));
    while let Some(update) = stream.next().await {
        let update = match update {
            Ok(u) => u,
            Err(e) => {
                debug!("update error: {}", e);
                continue;
            }
        };
        debug!("received update: {:?}", update);
        guard!(let UpdateKind::Message(message) = update.kind else { continue });
        guard!(let Some((command, includes_botname)) = Command::from_message(&botname, &message) else { continue });

        debug!("extracted command: {:?}", command);
        match command {
            Command::More { arg } => {
                spawn_response(handle_more(db_pool.clone(), api.clone(), message, arg));
            }
            Command::Count { arg } => {
                spawn_response(handle_count(db_pool.clone(), api.clone(), message, arg));
            }
            Command::Unrecognized => {
                let is_directed = includes_botname || message::is_private(&message);
                spawn_response(handle_unrecognized(is_directed, api.clone(), message));
            }
            Command::Version => {
                spawn_response(handle_version(api.clone(), message));
            }
        }
    }
}
