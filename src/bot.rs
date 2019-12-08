use crate::db;
use futures::StreamExt;
use guard::guard;
use telegram_bot::{
    prelude::CanSendPhoto,
    types::{InputFileRef, MessageKind, Update, UpdateKind},
    Api,
};

async fn handle_update(database: db::DbPool, api: Api, update: Update) {
    guard!(let UpdateKind::Message(message) = update.kind else { return });
    guard!(let MessageKind::Text {ref data, ..} = message.kind else { return });
    if !data.starts_with("/more") {
        return;
    }
    let link = database
        .get()
        .await
        .expect("Couldn't get database from pool")
        .fetch_random_link();
    let link = match link {
        Ok(link) => link,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
    api.send(
        message
            .chat
            .photo(InputFileRef::new(link.link))
            .caption(link.title),
    )
    .await
    .err()
    .map(|e| error!("error while responding tom message: {}", e));
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
        tokio::spawn(handle_update(db_pool.clone(), api.clone(), update));
    }
}
