use log::info;
use telegram_types::bot::{types::Message, methods::{ChatTarget, SendMessage}};
use worker::{Env, Error as WorkerError, Response};

use crate::{bot::{Bot, WebhookReply}};

pub fn return_message<S: AsRef<str>>(message: &Message, reply: S) -> Result<Response, WorkerError> {
    Response::from_json(&WebhookReply::from(
        SendMessage::new(ChatTarget::Id(message.chat.id), reply.as_ref()).reply(message.message_id),
    ))
}

pub async fn start(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let reply = format!("Title bot {}", env!("CARGO_PKG_VERSION"));
    info!("Replied: {:?}", reply);
    return_message(&m, reply)
}

pub async fn chat_info(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let chat_id = format!("{}", m.chat.id.0);
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
    let reply = "Available commands:";
    return_message(&m, reply)
}