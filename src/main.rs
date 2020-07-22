pub mod config;

use telegram_bot::{UpdateKind, MessageKind, Api, MessageOrChannelPost, ForwardMessage, ChatId, CanReplySendMessage, Message, MessageId, GetChatAdministrators, ChatMember, ToMessageId, DeleteMessage, SendMessage, ParseMode, MessageText, Update};
use futures::StreamExt;
use rand::Rng;
use rand::seq::SliceRandom;

#[tokio::main]
async fn main() {
    let mut config = config::load();
    let mut rng = rand::thread_rng();
    let api = Api::new(&config.token);
    let mut stream = api.stream();
    while let Some(Ok(
                       Update { kind: UpdateKind::Message(update_message), .. })
    ) = stream.next().await {
        if let MessageKind::Text { data, .. } = &update_message.kind {
            let admins: Vec<ChatMember> = api.send(
                GetChatAdministrators::new(ChatId::new(config.group))
            ).await.unwrap_or(Vec::new());
            let mut datas: Vec<&str> = data.split_whitespace().collect();
            if datas.is_empty() {
                continue;
            }
            match datas.swap_remove(0).replace(&config.name, "").as_str() {
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
                    if update_message.reply_to_message.is_none() {
                        api.send(update_message.text_reply("你到底想记什么呢。。。")).await;
                        continue;
                    }
                    let reply = update_message.reply_to_message.clone().unwrap();

                    let last_message_result = api.send(ForwardMessage::new(
                        reply.to_message_id(),
                        &update_message.chat,
                        ChatId::new(config.group))).await;
                    if last_message_result.is_err() {
                        continue;
                    }
                    api.send(SendMessage::new(
                        ChatId::new(config.group),
                        format!("——[{0}](tg://user?id={0})", user),
                    ).parse_mode(ParseMode::Markdown)).await;
                    let last_message: Message = last_message_result.unwrap();
                    config.notes.push(last_message.id.into());
                    config.save();
                    api.send(update_message.text_reply("记笔记。。。")).await;
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
                    let user: i64 = update_message.from.id.into();
                    if config.locks.contains(&user) {
                        api.send(update_message.text_reply("被锁的学渣无法使用考前突击")).await;
                        continue;
                    }
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
                "/delete" => {
                    let chat: i64 = update_message.chat.id().into();
                    if chat != config.group || update_message.reply_to_message.is_none() {
                        continue;
                    }
                    let reply = update_message.reply_to_message.unwrap();
                    let mut delete_id: i64 = reply.to_message_id().into();
                    if let MessageOrChannelPost::Message(message) = *reply {
                        if message.forward.is_none() {
                            delete_id = delete_id - 1;
                        }
                    }
                    config.notes.retain(|&note| note != delete_id);
                    config.answers.retain(|answer| answer.0 != delete_id);
                    config.save();
                    api.send(DeleteMessage::new(
                        ChatId::new(config.group),
                        MessageId::new(delete_id),
                    )).await;
                    api.send(DeleteMessage::new(
                        ChatId::new(config.group),
                        MessageId::new(delete_id + 1),
                    )).await;
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
                    if !is_admin || update_message.reply_to_message.is_none() {
                        continue;
                    }
                    let reply = update_message.reply_to_message.clone().unwrap();
                    let chat: i64 = update_message.chat.id().into();
                    match *reply {
                        MessageOrChannelPost::ChannelPost(_) => {
                            api.send(update_message.text_reply("不能对频道上锁。。。")).await;
                        }
                        MessageOrChannelPost::Message(message) => {
                            if chat != config.group {
                                config.locks.insert(message.from.id.into());
                                config.save();
                                api.send(update_message.text_reply("笔记已对其上锁。。。")).await;
                                continue;
                            }
                            let user_result = message.text().unwrap()["——".len()..]
                                .split_whitespace().collect::<Vec<&str>>()[0]
                                .parse::<i64>();
                            if user_result.is_ok() {
                                config.locks.insert(user_result.unwrap());
                                let message_id: i64 = message.id.into();
                                api.send(DeleteMessage::new(
                                    ChatId::new(config.group),
                                    MessageId::new(message_id - 1),
                                )).await;
                                api.send(DeleteMessage::new(
                                    ChatId::new(config.group),
                                    MessageId::new(message_id),
                                )).await;
                                config.notes.retain(|&note| note != message_id - 1);
                                config.answers.retain(|answer| answer.0 != message_id - 1);
                                config.save()
                            }
                            api.send(DeleteMessage::new(
                                ChatId::new(config.group),
                                update_message.id)).await;
                        }
                    }
                }
                "/unlock" => {
                    if !admins.iter().any(|admin| admin.user == update_message.from) ||
                        update_message.reply_to_message.is_none() {
                        continue;
                    }
                    let reply = update_message.reply_to_message.clone().unwrap();
                    if let MessageOrChannelPost::Message(message) = *reply {
                        config.locks.remove(&message.from.id.into());
                        config.save();
                        api.send(update_message.text_reply("笔记不再对其上锁。。。")).await;
                    }
                }
                "/keywords" => {
                    let user: i64 = update_message.from.id.into();
                    if config.locks.contains(&user) {
                        api.send(update_message.text_reply("笔记本对你上锁了哦")).await;
                        continue;
                    }
                    if update_message.reply_to_message.is_none() || datas.is_empty() {
                        api.send(update_message.text_reply("你到底想设置什么呢。。。")).await;
                        continue;
                    }
                    let reply = update_message.reply_to_message.clone().unwrap();
                    let last_message_result = api.send(ForwardMessage::new(
                        reply.to_message_id(),
                        &update_message.chat,
                        ChatId::new(config.group))).await;
                    if last_message_result.is_err() {
                        continue;
                    }
                    let datas_converted: Vec<String> = datas.iter().map(|keyword|
                        simplet2s::convert(&keyword.to_lowercase())
                    ).collect();
                    api.send(SendMessage::new(
                        ChatId::new(config.group),
                        format!("——[{0}](tg://user?id={0}) {1}", user, datas_converted.join(" ")),
                    ).parse_mode(ParseMode::Markdown)).await;
                    let last_message: Message = last_message_result.unwrap();
                    config.answers.push((last_message.id.into(), datas_converted));
                    config.save();
                    api.send(update_message.text_reply("设置完成。。。")).await;
                }
                _ => {
                    if rng.gen_range(0, 2) != 0 {
                        continue;
                    }
                    let converted = simplet2s::convert(&data.to_lowercase());
                    let mut answers = config.answers.clone();
                    answers.shuffle(&mut rng);
                    for answer in answers {
                        if answer.1.iter().any(|keyword| converted.contains(keyword)) &&
                            rng.gen_range(0, 2) != 0 {
                            api.send(ForwardMessage::new(
                                MessageId::new(answer.0),
                                ChatId::new(config.group.into()),
                                &update_message.chat,
                            )).await;
                        }
                    }
                }
            }
        }
    }
}