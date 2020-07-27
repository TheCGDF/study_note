pub mod config;

use telegram_bot::{UpdateKind, MessageKind, Api, MessageOrChannelPost, ForwardMessage, ChatId, CanReplySendMessage, Message, MessageId, GetChatAdministrators, ChatMember, ToMessageId, DeleteMessage, SendMessage, ParseMode, MessageText, Update, MessageChat};
use futures::StreamExt;
use rand::Rng;
use rand::seq::SliceRandom;
use harsh::Harsh;
use std::cmp::min;

#[tokio::main]
async fn main() {
    let mut config = config::load();
    let harsh = Harsh::builder()
        .salt(config.token.as_bytes())
        .length(8)
        .build()
        .unwrap();
    let mut rng = rand::thread_rng();
    let api = Api::new(&config.token);
    let mut stream = api.stream();
    while let Some(update) = stream.next().await {
        if let Ok(Update { kind: UpdateKind::Message(update_message), .. }) = update {
            if let MessageKind::Text { data, .. } = &update_message.kind {
                let admins: Vec<ChatMember> = api.send(
                    GetChatAdministrators::new(ChatId::new(config.group))
                ).await.unwrap_or(Vec::new());
                let params: Vec<&str> = data.split_whitespace().collect();
                if params.is_empty() {
                    continue;
                }
                match params[0].replace(&config.name, "").as_str() {
                    "/id" => {
                        let id: i64 = update_message.chat.id().into();
                        api.send(update_message.text_reply(id.to_string())).await;
                    }
                    "/note" => {
                        let user: i64 = update_message.from.id.into();
                        if config.locks.contains(&user) {
                            api.send(update_message.text_reply("笔记本对你上锁了哦🔒")).await;
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
                        let noted_id: i64 = last_message.id.into();
                        config.notes.push((noted_id, update_message.from.id.into()));
                        config.save();
                        api.send(update_message.text_reply(
                            format!(
                                "小本本记好了哦📝，id是{}",
                                harsh.encode(&[noted_id as u64])
                            )
                        )).await;
                    }
                    "/review" => {
                        if config.notes.is_empty() {
                            api.send(update_message.text_reply("还没有笔记哦📖")).await;
                            continue;
                        }
                        api.send(ForwardMessage::new(
                            MessageId::new(config.notes[rng.gen_range(0, config.notes.len())].0),
                            ChatId::new(config.group.into()),
                            &update_message.chat,
                        )).await;
                    }
                    "/cram" => {
                        let user: i64 = update_message.from.id.into();
                        if config.silences.contains(&update_message.chat.id().into()) {
                            api.send(update_message.text_reply("安静模式无法发动考前突击🔕")).await;
                            continue;
                        }
                        if config.locks.contains(&user) {
                            api.send(update_message.text_reply("被锁的学渣无法发动考前突击🔒")).await;
                            continue;
                        }
                        if config.notes.is_empty() {
                            api.send(update_message.text_reply("还没有笔记哦📖")).await;
                            continue;
                        }
                        for _ in 0..5 {
                            api.send(ForwardMessage::new(
                                MessageId::new(config.notes[rng.gen_range(0, config.notes.len())].0),
                                ChatId::new(config.group.into()),
                                &update_message.chat,
                            )).await;
                        }
                    }
                    "/my" => {
                        if let MessageChat::Private(user) = &update_message.chat {
                            if params.len() <= 2 {
                                api.send(update_message.text_reply(
                                    "该指令需要参数🔍：第1个参数为notes（笔记）或answers（应答）；第2个参数为页码，每页5个"
                                )).await;
                                continue;
                            }
                            if let Ok(page) = params[2].parse::<usize>() {
                                let user_id: i64 = user.id.into();
                                match params[1] {
                                    "notes" => {
                                        let mut notes: Vec<i64> = config.notes.iter()
                                            .filter_map(|note| if note.1 == user_id {
                                                Some(note.0)
                                            } else {
                                                None
                                            })
                                            .collect();
                                        if page == 0 {
                                            api.send(update_message.text_reply("以1为起始页")).await;
                                            continue;
                                        }
                                        if (page - 1) * 5 >= notes.len() {
                                            notes.clear();
                                        } else if notes.len() > 1 {
                                            notes = notes[(page - 1) * 5..min(page * 5, notes.len())].to_owned();
                                        }
                                        let notes_hash: Vec<String> = notes.iter().map(|&note|
                                            harsh.encode(&[note as u64])
                                        ).collect();
                                        api.send(update_message.text_reply(
                                            format!(
                                                "第{}/{}页：{}",
                                                page,
                                                notes.len() / 5 + 1,
                                                notes_hash.join(" "))
                                        )).await;
                                        for note in notes {
                                            api.send(ForwardMessage::new(
                                                MessageId::new(note),
                                                ChatId::new(config.group.into()),
                                                &update_message.chat,
                                            )).await;
                                        }
                                    }
                                    "answers" => {
                                        let mut answers: Vec<i64> = config.answers.iter()
                                            .filter_map(|answer| if answer.1 == user_id {
                                                Some(answer.0)
                                            } else {
                                                None
                                            })
                                            .collect();
                                        if page == 0 {
                                            api.send(update_message.text_reply("以1为起始页")).await;
                                            continue;
                                        }
                                        if (page - 1) * 5 >= answers.len() {
                                            answers.clear();
                                        } else if answers.len() > 1 {
                                            answers = answers[(page - 1) * 5..min(page * 5, answers.len())].to_owned();
                                        }
                                        let anwsers_hash: Vec<String> = answers.iter().map(|&answer|
                                            harsh.encode(&[answer as u64])
                                        ).collect();
                                        api.send(update_message.text_reply(
                                            format!(
                                                "第{}/{}页：{}",
                                                page,
                                                answers.len() / 5 + 1,
                                                anwsers_hash.join(" "))
                                        )).await;
                                        for answer in answers {
                                            api.send(ForwardMessage::new(
                                                MessageId::new(answer),
                                                ChatId::new(config.group.into()),
                                                &update_message.chat,
                                            )).await;
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                api.send(update_message.text_reply(
                                    "该指令需要参数🔍：第1个参数为notes（笔记）或answers（应答）；第2个参数为页码，每页5个"
                                )).await;
                            }
                        } else {
                            api.send(update_message.text_reply("请私聊小助手使用哦📞")).await;
                        }
                    }
                    "/delete" => {
                        let chat: i64 = update_message.chat.id().into();
                        if chat == config.group {
                            if let Some(reply) = update_message.reply_to_message {
                                let mut delete_id: i64 = reply.to_message_id().into();
                                if let MessageOrChannelPost::Message(message) = *reply {
                                    if message.forward.is_none() {
                                        delete_id = delete_id - 1;
                                    }
                                }
                                config.notes.retain(|&note| note.0 != delete_id);
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
                            }
                            api.send(DeleteMessage::new(
                                ChatId::new(config.group),
                                update_message.id)).await;
                        } else {
                            let user: i64 = update_message.from.id.into();
                            let mut succeed: Vec<&str> = Vec::new();
                            for to_delete in &params[1..] {
                                if let Ok(results) = harsh.decode(to_delete) {
                                    let delete_id = results[0] as i64;
                                    if config.notes.iter().any(|&note| note == (delete_id, user)) ||
                                        config.answers.iter().any(|answer|
                                            answer.0 == delete_id && answer.1 == user
                                        ) {
                                        config.notes.retain(|&note| note.0 != delete_id);
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
                                        succeed.push(to_delete);
                                    }
                                }
                            }
                            if succeed.is_empty() {
                                api.send(update_message.text_reply(
                                    format!("删除失败💬")
                                )).await;
                            } else {
                                api.send(update_message.text_reply(
                                    format!("{}删除成功💭", succeed.join("、"))
                                )).await;
                            }
                        }
                    }
                    "/lock" => {
                        if !admins.iter().any(|admin| admin.user == update_message.from) ||
                            update_message.reply_to_message.is_none() {
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
                                    api.send(update_message.text_reply("笔记已对其上锁🔒")).await;
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
                                    config.notes.retain(|&note| note.0 != message_id - 1);
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
                            api.send(update_message.text_reply("笔记不再对其上锁🔓")).await;
                        }
                    }
                    "/keywords" => {
                        let user: i64 = update_message.from.id.into();
                        if config.locks.contains(&user) {
                            api.send(update_message.text_reply("笔记本对你上锁了哦🔒")).await;
                            continue;
                        }
                        if update_message.reply_to_message.is_none() || params[1..].is_empty() {
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
                        let params_converted: Vec<String> = params[1..].iter().map(|keyword|
                            simplet2s::convert(&keyword.to_lowercase())
                        ).collect();
                        api.send(SendMessage::new(
                            ChatId::new(config.group),
                            format!("——[{0}](tg://user?id={0}) {1}", user, params_converted.join(" ")),
                        ).parse_mode(ParseMode::Markdown)).await;
                        let answer: Message = last_message_result.unwrap();
                        let answer_id: i64 = answer.id.into();
                        config.answers.push((answer_id, user, params_converted));
                        config.save();
                        if config.silences.contains(&update_message.chat.id().into()) {
                            api.send(update_message.text_reply(
                                format!(
                                    "设置完成✅，id是{}，但bot在当前群组中为安静模式，因此不会触发回复🔕",
                                    harsh.encode(&[answer_id as u64])
                                )
                            )).await;
                        } else {
                            api.send(update_message.text_reply(
                                format!(
                                    "设置完成✅，id是{}",
                                    harsh.encode(&[answer_id as u64])
                                )
                            )).await;
                        }
                    }
                    "/silence" => {
                        config.silences.insert(update_message.chat.id().into());
                        config.save();
                        api.send(update_message.text_reply("做一个安静的bot🔕")).await;
                    }
                    "/noisy" => {
                        config.silences.remove(&update_message.chat.id().into());
                        config.save();
                        api.send(update_message.text_reply("奇怪的开关被打开了。。。🔛")).await;
                    }
                    _ => {
                        if config.silences.contains(&update_message.chat.id().into()) ||
                            rng.gen_range(0, 2) != 0 {
                            continue;
                        }
                        let converted = simplet2s::convert(&data.to_lowercase());
                        let mut answers = config.answers.clone();
                        answers.shuffle(&mut rng);
                        for answer in answers {
                            if answer.2.iter().any(|keyword| converted.contains(keyword)) &&
                                rng.gen_range(0, 2) != 0 {
                                api.send(ForwardMessage::new(
                                    MessageId::new(answer.0),
                                    ChatId::new(config.group.into()),
                                    &update_message.chat,
                                )).await;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}