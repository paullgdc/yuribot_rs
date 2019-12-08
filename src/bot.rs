use crate::db;
use crate::Result;

use std::time::Duration;

use futures::StreamExt;
use guard::guard;
use telegram_bot::{
    prelude::{CanReplySendMessage, CanSendPhoto},
    types::{InputFileRef, MessageChat, MessageKind, Update, UpdateKind},
    Api,
};

async fn handle_update(database: db::DbPool, api: Api, update: Update) -> Result<()> {
    guard!(let UpdateKind::Message(message) = update.kind else { return Ok(()) });
    guard!(let MessageKind::Text {ref data, ..} = message.kind else { return Ok(())});
    if !(data.starts_with("/more ") || data == "/more") {
        if let MessageChat::Private(_) = message.chat {
            let response = api
                .send_timeout(
                    message.text_reply("Unrecognized command"),
                    Duration::from_secs(5),
                )
                .await?;
            debug!("Responded with: {:?}", response);
        }
        return Ok(());
    }
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

pub async fn start_bot(db_pool: db::DbPool, api: Api) {
    info!("Started the bot");
    let mut stream = api.stream();
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
        tokio::spawn({
            let db_pool = db_pool.clone();
            let api = api.clone();
            async move {
                let result = handle_update(db_pool, api, update).await;
                if let Err(e) = result {
                    error!("handling update error: {}", e);
                }
            }
        });
    }
}
