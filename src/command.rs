use log::info;
use telegram_types::bot::{
    methods::{ChatTarget, SendMessage},
    types::Message,
};
use worker::{Env, Error as WorkerError, Response, Url};

use crate::bot::{Bot, WebhookReply};

pub fn return_message<S: AsRef<str>>(message: &Message, reply: S) -> Result<Response, WorkerError> {
    Response::from_json(&WebhookReply::from(
        SendMessage::new(ChatTarget::Id(message.chat.id), reply.as_ref()).reply(message.message_id),
    ))
}

pub async fn start(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let reply = format!("FDKevin bot {}", env!("CARGO_PKG_VERSION"));
    info!("Replied: {:?}", reply);
    return_message(&m, reply)
}

pub async fn chat_info(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let chat_id = format!("{:#?}", m.chat);
    return_message(&m, chat_id)
}

pub async fn echo(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let text = if let Some(msg) = m.text.clone().unwrap().split_once(' ') {
        msg.1.to_string()
    } else {
        "wut?".to_string()
    };
    return_message(&m, text)
}

pub async fn help(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let mut reply = "Available commands:".to_string();
    for (key, _) in _bot.commands {
        reply = reply + "\n\t/" + &key.clone();
    }
    return_message(&m, reply)
}

pub async fn fetch(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let text = if let Some(msg) = m.text.clone().unwrap().split_once(' ') {
        let url = Url::parse(msg.1).unwrap();
        let mut resp = worker::Fetch::Url(url).send().await?;
        resp.text().await.unwrap()
    } else {
        "You need to input a url".to_string()
    };
    return_message(&m, text)
}

// pub async fn sync_commands(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
//     // let telebot_api = telbot_cf_worker::Api::new(_bot.token);
//     let commands = Vec::new();
//     _bot.send_json_post(&telbot_types::bot::SetMyCommands::new(commands))
//         .await
//         .unwrap();
//     return_message(&m, "success".to_string())
// }
