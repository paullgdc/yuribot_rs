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

async fn handle_send_image(database: db::DbPool, api: Api, message: Message) -> Result<()> {
    let link = database.get().await?.fetch_random_link()?;
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

fn extract_command<'a>(botname: &str, message: &'a Message) -> Option<(&'a str, bool)> {
    guard!(let MessageKind::Text {ref data, ref entities} = message.kind else { return None });
    for entity in entities {
        debug!("got entity {:?}", entity);
        guard!(let MessageEntityKind::BotCommand = entity.kind else { continue });
        let offset = utf8_pos_from_utf16(data, entity.offset.try_into().ok()?)?;
        let data = &data[offset..];
        let length = utf8_pos_from_utf16(data, entity.length.try_into().ok()?)?;
        let data = &data[..length];
        let command = if data.ends_with(botname) {
            (&data[..(data.len() - botname.len())], true)
        } else {
            (data, false)
        };
        return Some(command);
    }
    return None;
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
        guard!(let Some((command, includes_botname)) = extract_command(&botname, &message) else { continue });
        debug!("extracted command: {:?}", command);
        match command {
            "/more" => {
                tokio::spawn({
                    let db_pool = db_pool.clone();
                    let api = api.clone();
                    async move {
                        let result = handle_send_image(db_pool, api, message).await;
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
