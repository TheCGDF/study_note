use telegram_bot::{Message, CanReplySendMessage, MessageOrChannelPost, User, ToMessageId, ForwardMessage, ChatId, SendMessage, ParseMode, MessageId, MessageChat, DeleteMessage, MessageText, ChatMember, GetChatAdministrators};
use crate::config::Config;
use harsh::Harsh;
use lazy_static::lazy_static;
use crate::API;
use rand::Rng;
use std::cmp::min;
use rand::seq::SliceRandom;
use chrono::{Utc, Duration};

lazy_static! {
    static ref HARSH :Harsh = Harsh::builder()
        .salt(crate::config::load().token)
        .length(8)
        .build()
        .unwrap();
}

impl Config {
    pub async fn command(&mut self, update_message: &Message, data: &String) {
        let chat_id: i64 = update_message.chat.id().into();
        let mut noisy = true;
        let chat_option = self.chats.get(&chat_id);
        if let Some(chat) = chat_option {
            noisy = chat.0;
            if !chat.0 || Utc::now().signed_duration_since(chat.1).num_seconds() < 20 {
                return;
            }
        }
        let mut rng = rand::thread_rng();
        if rng.gen_range(0, 4) != 0 {
            return;
        }
        let converted = simplet2s::convert(&data.to_lowercase());
        let mut answers = self.answers.clone();
        answers.shuffle(&mut rng);
        if let Some(answer) = answers.iter().filter(|answer|
            answer.2.iter().any(|keyword|
                converted.contains(keyword)
            )
        ).next() {
            let _ = API.send(ForwardMessage::new(
                MessageId::new(answer.0),
                ChatId::new(self.group.into()),
                &update_message.chat,
            )).await;
            self.chats.insert(chat_id, (noisy, Utc::now()));
            self.save();
        }
    }

    pub async fn command_cram(&self, update_message: Message) {
        let chat: i64 = update_message.chat.id().into();
        let group_option = self.chats.get(&chat);
        if let Some(group) = group_option {
            if group.0 == false {
                let _ = API.send(update_message.text_reply("安静模式无法发动考前突击🔕")).await;
                return;
            }
        }
        let user: i64 = update_message.from.id.into();
        if self.locks.contains(&user) {
            let _ = API.send(update_message.text_reply("被锁的学渣无法发动考前突击🔒")).await;
            return;
        }
        if self.notes.is_empty() {
            let _ = API.send(update_message.text_reply("还没有笔记哦📖")).await;
            return;
        }
        let mut rng = rand::thread_rng();
        for _ in 0..5 {
            let _ = API.send(ForwardMessage::new(
                MessageId::new(
                    self.notes[rng.gen_range(0, self.notes.len())].0
                ),
                ChatId::new(self.group.into()),
                &update_message.chat,
            )).await;
        }
    }

    pub async fn command_delete(&mut self, update_message: &Message, params: Vec<&str>) {
        let chat: i64 = update_message.chat.id().into();
        if chat == self.group {
            if let Some(ref replied) = update_message.reply_to_message {
                let mut delete_id: i64 = replied.to_message_id().into();
                if let MessageOrChannelPost::Message(message) = &**replied {
                    if message.forward.is_none() {
                        delete_id = delete_id - 1;
                    }
                }
                self.notes.retain(|&note| note.0 != delete_id);
                self.answers.retain(|answer| answer.0 != delete_id);
                self.save();
                let _ = API.send(DeleteMessage::new(
                    ChatId::new(self.group),
                    MessageId::new(delete_id),
                )).await;
                let _ = API.send(DeleteMessage::new(
                    ChatId::new(self.group),
                    MessageId::new(delete_id + 1),
                )).await;
            }
            let _ = API.send(DeleteMessage::new(
                ChatId::new(self.group),
                update_message.id)).await;
            return;
        }

        let user: i64 = update_message.from.id.into();
        let mut succeed: Vec<&str> = Vec::new();
        for to_delete in &params[1..] {
            if let Ok(results) = HARSH.decode(to_delete) {
                let delete_id = results[0] as i64;
                if self.notes.iter().any(|&note| note == (delete_id, user)) ||
                    self.answers.iter().any(|answer|
                        answer.0 == delete_id && answer.1 == user
                    ) {
                    self.notes.retain(|&note| note.0 != delete_id);
                    self.answers.retain(|answer| answer.0 != delete_id);
                    self.save();
                    let _ = API.send(DeleteMessage::new(
                        ChatId::new(self.group),
                        MessageId::new(delete_id),
                    )).await;
                    let _ = API.send(DeleteMessage::new(
                        ChatId::new(self.group),
                        MessageId::new(delete_id + 1),
                    )).await;
                    succeed.push(to_delete);
                }
            }
        }
        if succeed.is_empty() {
            let _ = API.send(update_message.text_reply(
                format!("删除失败💬")
            )).await;
            return;
        }
        let _ = API.send(update_message.text_reply(
            format!("{}删除成功💭", succeed.join("、"))
        )).await;
    }

    pub async fn command_id(&mut self, update_message: Message) {
        let id: i64 = update_message.chat.id().into();
        let _ = API.send(update_message.text_reply(id.to_string())).await;
    }

    pub async fn command_keywords(&mut self, update_message: &Message, params: Vec<&str>) {
        let user: i64 = update_message.from.id.into();
        if self.locks.contains(&user) {
            let _ = API.send(update_message.text_reply("笔记本对你上锁了哦🔒")).await;
            return;
        }
        if update_message.reply_to_message.is_none() || params[1..].is_empty() {
            let _ = API.send(update_message.text_reply("你到底想设置什么呢。。。")).await;
            return;
        }
        let replied = update_message.reply_to_message.clone().unwrap();
        if let MessageOrChannelPost::Message(Message { from: User { username: Some(ref username), .. }, .. }) = *replied {
            if username == self.name.trim_start_matches('@') {
                let _ = API.send(update_message.text_reply("傻逼，给爷爬🔪")).await;
                return;
            }
        }
        let last_message_result = API.send(ForwardMessage::new(
            replied.to_message_id(),
            &update_message.chat,
            ChatId::new(self.group))).await;
        if last_message_result.is_err() {
            return;
        }
        let params_converted: Vec<String> = params[1..].iter().map(|keyword|
            simplet2s::convert(&keyword.to_lowercase())
        ).collect();
        let _ = API.send(SendMessage::new(
            ChatId::new(self.group),
            format!("——[{0}](tg://user?id={0}) {1}", user, params_converted.join(" ")),
        ).parse_mode(ParseMode::Markdown)).await;
        let answer: Message = last_message_result.unwrap();
        let answer_id: i64 = answer.id.into();
        self.answers.push((answer_id, user, params_converted));
        self.save();
        let group_option = self.chats.get(&update_message.chat.id().into());
        if let Some(group) = group_option {
            if !group.0 {
                let _ = API.send(update_message.text_reply(
                    format!(
                        "设置完成✅，id是{}，但bot在当前群组中为安静模式，因此不会触发回复🔕",
                        HARSH.encode(&[answer_id as u64])
                    )
                )).await;
                return;
            }
        }
        let _ = API.send(update_message.text_reply(
            format!("设置完成✅，id是{}", HARSH.encode(&[answer_id as u64])
            )
        )).await;
    }

    pub async fn command_lock(&mut self, update_message: Message) {
        let admins: Vec<ChatMember> = API.send(
            GetChatAdministrators::new(ChatId::new(self.group))
        ).await.unwrap_or(Vec::new());
        let replied = update_message.reply_to_message.clone().unwrap();
        let chat: i64 = update_message.chat.id().into();
        match *replied {
            MessageOrChannelPost::ChannelPost(_) => {
                let _ = API.send(update_message.text_reply("不能对频道上锁。。。")).await;
            }
            MessageOrChannelPost::Message(message) => {
                if chat != self.group {
                    if !admins.iter().any(|admin| admin.user == update_message.from) ||
                        update_message.reply_to_message.is_none() {
                        return;
                    }
                    self.locks.insert(message.from.id.into());
                    self.save();
                    let _ = API.send(update_message.text_reply("笔记已对其上锁🔒")).await;
                    return;
                }
                if admins.iter().any(|admin| admin.user == update_message.from) &&
                    update_message.reply_to_message.is_none() {
                    let user_result = message.text().unwrap()["——".len()..]
                        .split_whitespace().collect::<Vec<&str>>()[0]
                        .parse::<i64>();
                    if user_result.is_ok() {
                        self.locks.insert(user_result.unwrap());
                        let message_id: i64 = message.id.into();
                        let _ = API.send(DeleteMessage::new(
                            ChatId::new(self.group),
                            MessageId::new(message_id - 1),
                        )).await;
                        let _ = API.send(DeleteMessage::new(
                            ChatId::new(self.group),
                            MessageId::new(message_id),
                        )).await;
                        self.notes.retain(|&note| note.0 != message_id - 1);
                        self.answers.retain(|answer| answer.0 != message_id - 1);
                        self.save()
                    }
                }
                let _ = API.send(DeleteMessage::new(
                    ChatId::new(self.group),
                    update_message.id)).await;
            }
        }
    }

    pub async fn command_my(&self, update_message: &Message, params: Vec<&str>) {
        if let MessageChat::Private(user) = &update_message.chat {
            if params.len() <= 2 {
                let _ = API.send(update_message.text_reply(
                    "该指令需要参数🔍：第1个参数为notes（笔记）或answers（应答）；第2个参数为页码，每页5个"
                )).await;
                return;
            }
            let page_result = params[2].parse::<usize>();
            if page_result.is_err() {
                let _ = API.send(update_message.text_reply(
                    "该指令需要参数🔍：第1个参数为notes（笔记）或answers（应答）；第2个参数为页码，每页5个"
                )).await;
                return;
            }
            let page = page_result.unwrap();
            let user_id: i64 = user.id.into();
            match params[1] {
                "notes" => {
                    let mut notes: Vec<i64> = self.notes.iter()
                        .filter_map(|note| if note.1 == user_id {
                            Some(note.0)
                        } else {
                            None
                        })
                        .collect();
                    if page == 0 {
                        let _ = API.send(update_message.text_reply("以1为起始页")).await;
                        return;
                    }
                    let pages = (notes.len() as i64 - 1) / 5 + 1;
                    if (page - 1) * 5 >= notes.len() {
                        notes.clear();
                    } else if notes.len() > 1 {
                        notes = notes[(page - 1) * 5..min(page * 5, notes.len())].to_owned();
                    }
                    let notes_hash: Vec<String> = notes.iter().map(|&note|
                        HARSH.encode(&[note as u64])
                    ).collect();
                    let _ = API.send(update_message.text_reply(
                        format!(
                            "第{}/{}页：{}",
                            page,
                            pages,
                            notes_hash.join(" "))
                    )).await;
                    for note in notes {
                        let _ = API.send(ForwardMessage::new(
                            MessageId::new(note),
                            ChatId::new(self.group.into()),
                            &update_message.chat,
                        )).await;
                    }
                }
                "answers" => {
                    let mut answers: Vec<i64> = self.answers.iter()
                        .filter_map(|answer| if answer.1 == user_id {
                            Some(answer.0)
                        } else {
                            None
                        })
                        .collect();
                    if page == 0 {
                        let _ = API.send(update_message.text_reply("以1为起始页")).await;
                        return;
                    }
                    let pages = (answers.len() as i64 - 1) / 5 + 1;
                    if (page - 1) * 5 >= answers.len() {
                        answers.clear();
                    } else if answers.len() > 1 {
                        answers = answers[(page - 1) * 5..min(page * 5, answers.len())].to_owned();
                    }
                    let answers_hash: Vec<String> = answers.iter().map(|&answer|
                        HARSH.encode(&[answer as u64])
                    ).collect();
                    let _ = API.send(update_message.text_reply(
                        format!(
                            "第{}/{}页：{}",
                            page,
                            pages,
                            answers_hash.join(" "))
                    )).await;
                    for answer in answers {
                        let _ = API.send(ForwardMessage::new(
                            MessageId::new(answer),
                            ChatId::new(self.group.into()),
                            &update_message.chat,
                        )).await;
                    }
                }
                _ => {
                    let _ = API.send(update_message.text_reply(
                        "该指令需要参数🔍：第1个参数为notes（笔记）或answers（应答）；第2个参数为页码，每页5个"
                    )).await;
                }
            }
        } else {
            let _ = API.send(update_message.text_reply("请私聊小助手使用哦📞")).await;
        }
    }

    pub async fn command_noisy(&mut self, update_message: Message) {
        let chat_id: i64 = update_message.chat.id().into();
        let chat_option = self.chats.get(&chat_id);
        if chat_option.is_none() {
            self.chats.insert(chat_id, (true, Utc::now() - Duration::seconds(20)));
        } else {
            let mut chat = *chat_option.unwrap();
            chat.0 = true;
            self.chats.insert(chat_id, chat);
        }
        self.save();
        let _ = API.send(update_message.text_reply("奇怪的开关被打开了。。。🔛")).await;
    }

    pub async fn command_note(&mut self, update_message: Message) {
        let user: i64 = update_message.from.id.into();
        if self.locks.contains(&user) {
            let _ = API.send(update_message.text_reply("笔记本对你上锁了哦🔒")).await;
            return;
        }
        if update_message.reply_to_message.is_none() {
            let _ = API.send(update_message.text_reply("你到底想记什么呢。。。")).await;
            return;
        }
        let replied = update_message.reply_to_message.clone().unwrap();
        if let MessageOrChannelPost::Message(Message { from: User { username: Some(ref username), .. }, .. }) = *replied {
            if username == self.name.trim_start_matches('@') {
                let _ = API.send(update_message.text_reply("傻逼，给爷爬🔪")).await;
                return;
            }
        }
        let last_message_result = API.send(ForwardMessage::new(
            replied.to_message_id(),
            &update_message.chat,
            ChatId::new(self.group))).await;
        if last_message_result.is_err() {
            return;
        }
        let _ = API.send(SendMessage::new(
            ChatId::new(self.group),
            format!("——[{0}](tg://user?id={0})", user),
        ).parse_mode(ParseMode::Markdown)).await;
        let last_message: Message = last_message_result.unwrap();
        let noted_id: i64 = last_message.id.into();
        self.notes.push((noted_id, update_message.from.id.into()));
        self.save();
        let _ = API.send(update_message.text_reply(
            format!(
                "小本本记好了哦📝，id是{}",
                HARSH.encode(&[noted_id as u64])
            )
        )).await;
    }

    pub async fn command_review(&self, update_message: Message) {
        if self.notes.is_empty() {
            let _ = API.send(update_message.text_reply("还没有笔记哦📖")).await;
            return;
        }
        let mut rng = rand::thread_rng();
        let _ = API.send(ForwardMessage::new(
            MessageId::new(
                self.notes[rng.gen_range(0, self.notes.len())].0
            ),
            ChatId::new(self.group.into()),
            &update_message.chat,
        )).await;
    }

    pub async fn command_silence(&mut self, update_message: Message) {
        let chat_id: i64 = update_message.chat.id().into();
        let chat_option = self.chats.get(&chat_id);
        if chat_option.is_none() {
            self.chats.insert(chat_id, (true, Utc::now() - Duration::seconds(20)));
        } else {
            let mut chat = *chat_option.unwrap();
            chat.0 = false;
            self.chats.insert(chat_id, chat);
        }
        self.save();
        let _ = API.send(update_message.text_reply("做一个安静的bot🔕")).await;
    }

    pub async fn command_unlock(&mut self, update_message: Message) {
        let admins: Vec<ChatMember> = API.send(
            GetChatAdministrators::new(ChatId::new(self.group))
        ).await.unwrap_or(Vec::new());
        if !admins.iter().any(|admin| admin.user == update_message.from) ||
            update_message.reply_to_message.is_none() {
            return;
        }
        let replied = update_message.reply_to_message.clone().unwrap();
        if let MessageOrChannelPost::Message(message) = *replied {
            self.locks.remove(&message.from.id.into());
            self.save();
            let _ = API.send(update_message.text_reply("笔记不再对其上锁🔓")).await;
        }
    }
}