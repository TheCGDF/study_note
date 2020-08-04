pub mod command;
pub mod config;

use telegram_bot::{UpdateKind, MessageKind, Api, Update, GetMe};
use futures::StreamExt;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref API:Api=Api::new(config::load().token);
}

#[tokio::main]
async fn main() {
    let mut config = config::CONFIG.lock().unwrap();
    let mut stream = API.stream();
    while let Some(update) = stream.next().await {
        if let Ok(Update { kind: UpdateKind::Message(update_message), .. }) = update {
            if let MessageKind::Text { data, .. } = &update_message.kind {
                let params: Vec<&str> = data.split_whitespace().collect();
                if params.is_empty() {
                    continue;
                }
                let username = API.send(GetMe {}).await.unwrap().username.unwrap();
                match params[0].replace(&format!("@{}", username), "").as_str() {
                    "/id" => config.command_id(update_message).await,
                    "/note" => config.command_note(update_message).await,
                    "/review" => config.command_review(update_message).await,
                    "/cram" => config.command_cram(update_message).await,
                    "/my" => config.command_my(&update_message, params).await,
                    "/delete" => config.command_delete(&update_message, params).await,
                    "/lock" => config.command_lock(update_message).await,
                    "/unlock" => config.command_unlock(update_message).await,
                    "/keywords" => config.command_keywords(&update_message, params).await,
                    "/silence" => config.command_silence(update_message).await,
                    "/noisy" => config.command_noisy(update_message).await,
                    _ => config.command(&update_message, data).await,
                }
            }
        }
    }
}