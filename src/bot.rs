use crate::db;
use crate::Result;

use futures::StreamExt;
use guard::guard;
use telegram_bot::{
    prelude::CanSendPhoto,
    types::{InputFileRef, MessageKind, Update, UpdateKind},
    Api,
};

async fn handle_update(database: db::DbPool, api: Api, update: Update) -> Result<()> {
    guard!(let UpdateKind::Message(message) = update.kind else { return Ok(()) });
    guard!(let MessageKind::Text {ref data, ..} = message.kind else { return Ok(())});
    if !data.starts_with("/more") {
        return Ok(());
    }
    let link = database.get().await?.fetch_random_link()?;
    api.send(
        message
            .chat
            .photo(InputFileRef::new(link.link))
            .caption(link.title),
    )
    .await?;
    Ok(())
}

pub async fn start_bot(db_pool: db::DbPool, api: Api) {
    info!("Started the bot");
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = match update {
            Ok(u) => u,
            Err(e) => {
                error!("{}", e);
                continue;
            }
        };
        tokio::spawn({
            let db_pool = db_pool.clone();
            let api = api.clone();
            async move {
                let result = handle_update(db_pool, api, update).await;
                if let Err(e) = result {
                    error!("{}", e);
                }
            }
        });
    }
}
