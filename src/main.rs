pub mod config;

use telegram_bot::{UpdateKind, MessageKind, Api, MessageOrChannelPost, ForwardMessage, ChatId, CanReplySendMessage, Message, MessageId};
use futures::StreamExt;
use rand::Rng;

#[tokio::main]
async fn main() {
    let mut config = config::load();
    let mut notes: Vec<i64> = Vec::new();
    let mut rng = rand::thread_rng();
    let api = Api::new(&config.token);
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update.unwrap();
        if let UpdateKind::Message(update_message) = update.kind {
            if let MessageKind::Text { ref data, .. } = &update_message.kind {
                let user: i64 = update_message.from.id.into();
                if config.locks.contains(&user) {
                    api.send(update_message.text_reply("笔记本对你上锁了哦")).await.unwrap();
                    continue;
                }
                //todo get admins
                match data.replace("@study_note_bot", "").trim() {
                    "/id" => {
                        let id: i64 = update_message.chat.id().into();
                        api.send(update_message.text_reply(id.to_string())).await.unwrap();
                    }
                    "/note" => {
                        if let Some(reply) = &update_message.reply_to_message {
                            let last_message: Message = match &**reply {
                                MessageOrChannelPost::ChannelPost(channel_post) => {
                                    api.send(ForwardMessage::new(
                                        channel_post.id,
                                        &channel_post.chat,
                                        ChatId::new(config.group)))
                                }
                                MessageOrChannelPost::Message(message) => {
                                    api.send(ForwardMessage::new(
                                        message.id,
                                        &message.chat,
                                        ChatId::new(config.group),
                                    ))
                                }
                            }.await.unwrap();

                            notes.push(last_message.id.into());
                            config.last = last_message.id.into();
                            config.save();
                            api.send(&update_message.text_reply("记笔记。。。")).await.unwrap();
                        }
                    }
                    "/review" => {
                        api.send(ForwardMessage::new(
                            MessageId::new(notes[rng.gen_range(0, notes.len())]),
                            ChatId::new(config.group.into()),
                            &update_message.chat,
                        )).await.unwrap();
                    }
                    "/cram" => {
                        for _ in 1..5 {
                            api.send(ForwardMessage::new(
                                MessageId::new(notes[rng.gen_range(0, notes.len())]),
                                ChatId::new(config.group.into()),
                                &update_message.chat,
                            )).await.unwrap();
                        }
                    }
                    "/learned" => {
                        let chat: i64 = *&update_message.chat.id().into();
                        if chat != config.group {
                            continue;
                        }
                        //todo delete
                    }
                    "/lock" => {
                        if let Some(reply) = &update_message.reply_to_message {
                            match &**reply {
                                MessageOrChannelPost::ChannelPost(_) => {
                                    api.send(&update_message.text_reply("不能对频道上锁。。。")).await.unwrap();
                                }
                                MessageOrChannelPost::Message(message) => {
                                    config.locks.push(message.from.id.into());
                                    config.save();
                                    api.send(&update_message.text_reply("笔记已对其上锁。。。")).await.unwrap();
                                }
                            }
                        }
                    }
                    "/unlock" => {
                        if let Some(reply) = &update_message.reply_to_message {
                            match &**reply {
                                MessageOrChannelPost::ChannelPost(_) => {
                                    api.send(&update_message.text_reply("频道无需解锁。。。")).await.unwrap();
                                }
                                MessageOrChannelPost::Message(message) => {
                                    let user_locked: i64 = message.from.id.into();
                                    config.locks.retain(|item| *item == user_locked);
                                    config.save();
                                    api.send(&update_message.text_reply("笔记已对其上锁。。。")).await.unwrap();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}