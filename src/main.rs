pub mod config;

use telegram_bot::{UpdateKind, MessageKind, Api, MessageOrChannelPost, ForwardMessage, ChatId, CanReplySendMessage, Message, MessageId, GetChatAdministrators, ChatMember, ToMessageId, DeleteMessage};
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
                let admins: Vec<ChatMember> = api.send(
                    GetChatAdministrators::new(ChatId::new(config.group))
                ).await.unwrap();
                match data.replace("@study_note_bot", "").trim() {
                    "/id" => {
                        let id: i64 = update_message.chat.id().into();
                        api.send(update_message.text_reply(id.to_string())).await.unwrap();
                    }
                    "/note" => {
                        if let Some(reply) = &update_message.reply_to_message {
                            let last_message: Message = api.send(ForwardMessage::new(
                                reply.to_message_id(),
                                &update_message.chat,
                                ChatId::new(config.group))).await.unwrap();
                            notes.push(last_message.id.into());
                            config.last = last_message.id.into();
                            config.save();
                            api.send(&update_message.text_reply("记笔记。。。")).await.unwrap();
                        }
                    }
                    "/review" => {
                        if notes.is_empty() {
                            api.send(&update_message.text_reply("还没有笔记哦")).await.unwrap();
                        }
                        api.send(ForwardMessage::new(
                            MessageId::new(notes[rng.gen_range(0, notes.len())]),
                            ChatId::new(config.group.into()),
                            &update_message.chat,
                        )).await.unwrap();
                    }
                    "/cram" => {
                        if notes.is_empty() {
                            api.send(&update_message.text_reply("还没有笔记哦")).await.unwrap();
                        }
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
                        if let Some(reply) = &update_message.reply_to_message {
                            let reply_id: i64 = reply.to_message_id().into();
                            notes.retain(|&note| note != reply_id);
                            api.send(
                                DeleteMessage::new(ChatId::new(config.group), MessageId::new(reply_id))
                            ).await.unwrap();
                        }
                        api.send(
                            DeleteMessage::new(ChatId::new(config.group), update_message.id)
                        ).await.unwrap();
                    }
                    "/lock" => {
                        let mut is_admin = false;
                        for admin in admins {
                            if admin.user == update_message.from {
                                is_admin = true;
                                break;
                            }
                        }
                        if !is_admin {
                            continue;
                        }
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
                        let mut is_admin = false;
                        for admin in admins {
                            if admin.user == update_message.from {
                                is_admin = true;
                                break;
                            }
                        }
                        if !is_admin {
                            continue;
                        }
                        if let Some(reply) = &update_message.reply_to_message {
                            match &**reply {
                                MessageOrChannelPost::ChannelPost(_) => {
                                    api.send(&update_message.text_reply("频道无需解锁。。。")).await.unwrap();
                                }
                                MessageOrChannelPost::Message(message) => {
                                    let user_locked: i64 = message.from.id.into();
                                    config.locks.retain(|&item| item != user_locked);
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