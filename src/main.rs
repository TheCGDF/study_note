pub mod config;

use telegram_bot::{UpdateKind, MessageKind, Api, MessageOrChannelPost, ForwardMessage, ChatId, CanReplySendMessage, Message, MessageId, GetChatAdministrators, ChatMember, ToMessageId, DeleteMessage, SendMessage, ParseMode, MessageText};
use futures::StreamExt;
use rand::Rng;

#[tokio::main]
async fn main() {
    let mut config = config::load();
    let mut rng = rand::thread_rng();
    let api = Api::new(&config.token);
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        let update = update.unwrap();
        if let UpdateKind::Message(update_message) = update.kind {
            if let MessageKind::Text { ref data, .. } = &update_message.kind {
                let admins: Vec<ChatMember> = api.send(
                    GetChatAdministrators::new(ChatId::new(config.group))
                ).await.unwrap();
                match data.replace("@study_note_bot", "").trim() {
                    "/id" => {
                        let id: i64 = update_message.chat.id().into();
                        api.send(update_message.text_reply(id.to_string())).await;
                    }
                    "/note" => {
                        let user: i64 = update_message.from.id.into();
                        if config.locks.contains(&user) {
                            api.send(update_message.text_reply("笔记本对你上锁了哦")).await;
                            continue;
                        }
                        if let Some(reply) = &update_message.reply_to_message {
                            let last_message_result = api.send(ForwardMessage::new(
                                reply.to_message_id(),
                                &update_message.chat,
                                ChatId::new(config.group))).await;
                            if last_message_result.is_err() {
                                continue;
                            }
                            let user: i64 = update_message.from.id.into();
                            api.send(SendMessage::new(
                                ChatId::new(config.group),
                                format!("[——](tg://user?id={})", user),
                            ).parse_mode(ParseMode::Markdown)).await;
                            let last_message: Message = last_message_result.unwrap();
                            config.notes.push(last_message.id.into());
                            config.last = last_message.id.into();
                            config.save();
                            api.send(update_message.text_reply("记笔记。。。")).await;
                        } else {
                            api.send(update_message.text_reply("你到底想记什么呢。。。")).await;
                        }
                    }
                    "/review" => {
                        if config.notes.is_empty() {
                            api.send(update_message.text_reply("还没有笔记哦")).await;
                            continue;
                        }
                        api.send(ForwardMessage::new(
                            MessageId::new(config.notes[rng.gen_range(0, config.notes.len())]),
                            ChatId::new(config.group.into()),
                            &update_message.chat,
                        )).await;
                    }
                    "/cram" => {
                        if config.notes.is_empty() {
                            api.send(update_message.text_reply("还没有笔记哦")).await;
                            continue;
                        }
                        for _ in 0..5 {
                            api.send(ForwardMessage::new(
                                MessageId::new(config.notes[rng.gen_range(0, config.notes.len())]),
                                ChatId::new(config.group.into()),
                                &update_message.chat,
                            )).await;
                        }
                    }
                    "/learned" => {
                        let chat: i64 = update_message.chat.id().into();
                        if chat != config.group {
                            continue;
                        }
                        if let Some(reply) = update_message.reply_to_message {
                            let mut note_id: i64 = reply.to_message_id().into();
                            if !config.notes.contains(&note_id) {
                                note_id = note_id - 1;
                            }
                            config.notes.retain(|&note| note != note_id);
                            config.save();
                            api.send(DeleteMessage::new(
                                ChatId::new(config.group),
                                MessageId::new(note_id),
                            )).await;
                            api.send(DeleteMessage::new(
                                ChatId::new(config.group),
                                MessageId::new(note_id + 1),
                            )).await;
                        }
                        api.send(DeleteMessage::new(
                            ChatId::new(config.group),
                            update_message.id)).await;
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
                            let chat: i64 = update_message.chat.id().into();
                            match &**reply {
                                MessageOrChannelPost::ChannelPost(_) => {
                                    api.send(update_message.text_reply("不能对频道上锁。。。")).await;
                                }
                                MessageOrChannelPost::Message(message) => {
                                    if chat != config.group {
                                        config.locks.push(message.from.id.into());
                                        config.save();
                                        api.send(update_message.text_reply("笔记已对其上锁。。。")).await;
                                    } else {
                                        let user_result = message.text().unwrap()
                                            .replace("[——](tg://user?id=", "")
                                            .trim_end_matches(')').parse::<i64>();
                                        if user_result.is_ok() {
                                            let user = user_result.unwrap();
                                            config.locks.push(user);
                                            api.send(DeleteMessage::new(
                                                ChatId::new(config.group),
                                                MessageId::new(user - 1),
                                            )).await;
                                            api.send(DeleteMessage::new(
                                                ChatId::new(config.group),
                                                MessageId::new(user),
                                            )).await;
                                        }
                                        api.send(DeleteMessage::new(
                                            ChatId::new(config.group),
                                            update_message.id)).await;
                                    }
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
                                    api.send(update_message.text_reply("频道无需解锁。。。")).await;
                                }
                                MessageOrChannelPost::Message(message) => {
                                    let user_locked: i64 = message.from.id.into();
                                    config.locks.retain(|&item| item != user_locked);
                                    config.save();
                                    api.send(update_message.text_reply("笔记已对其上锁。。。")).await;
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