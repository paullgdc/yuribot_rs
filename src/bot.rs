use crate::db;
use crate::utils::utf8_pos_from_utf16;
use crate::Result;

use std::convert::TryInto;
use std::time::Duration;

use futures::StreamExt;
use guard::guard;
use telegram_bot::{
    prelude::{CanReplySendMessage, CanSendPhoto},
    types::{
        GetMe, InputFileRef, Message, MessageChat, MessageEntityKind, MessageKind, UpdateKind,
    },
    Api,
};

trait IndexMessageContent {
    fn get_content(&self, range: ArgRange) -> Option<&str>;
}

impl IndexMessageContent for Message {
    fn get_content(&self, range: ArgRange) -> Option<&str> {
        guard!(let MessageKind::Text {ref data, ref entities} = self.kind else { return None });
        data.get(range)
    }
}

type ArgRange = std::ops::Range<usize>;

#[derive(Debug)]
enum Command {
    More { arg: ArgRange },
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
            _ => Command::Unrecognized,
        };
        Some((command, is_directed))
    }
}

async fn handle_send_image(
    database: db::DbPool,
    api: Api,
    message: Message,
    arg_range: ArgRange,
) -> Result<()> {
    let arg = message
        .get_content(arg_range)
        .ok_or(crate::YuribotError::CommandArgParseError)?
        .trim();
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
                tokio::spawn({
                    let resp = handle_send_image(db_pool.clone(), api.clone(), message, arg);
                    async {
                        let result = resp.await;
                        if let Err(e) = result {
                            error!("handling update error: {}", e);
                        }
                    }
                });
            }
            _ => {
                tokio::spawn({
                    let api = api.clone();
                    let is_directed = includes_botname
                        || if let MessageChat::Private(_) = message.chat {
                            true
                        } else {
                            false
                        };
                    async move {
                        let result = handle_unrecognized(is_directed, api, message).await;
                        if let Err(e) = result {
                            error!("handling update error: {}", e);
                        }
                    }
                });
            }
        }
    }
}
