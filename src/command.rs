use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use serde_json::json;
use telegram_types::bot::{
    methods::{ChatTarget, SendMessage},
    types::Message,
};
use worker::{console_log, Env, Error as WorkerError, Response, Url};

use crate::{
    bot::{Bot, WebhookReply},
    bot_store,
    chat::{build_message_context, clear_chat_history, get_chat_history, put_chat_history},
    openai,
};

pub fn return_reply_message<S: AsRef<str>>(
    message: &Message,
    reply: S,
) -> Result<Response, WorkerError> {
    Response::from_json(&WebhookReply::from(
        SendMessage::new(ChatTarget::Id(message.chat.id), reply.as_ref()).reply(message.message_id),
    ))
}

pub fn return_message<S: AsRef<str>>(message: &Message, reply: S) -> Result<Response, WorkerError> {
    Response::from_json(&WebhookReply::from(SendMessage::new(
        ChatTarget::Id(message.chat.id),
        reply.as_ref(),
    )))
}

pub async fn start(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let reply = format!("FDKevin bot {}", env!("CARGO_PKG_VERSION"));
    console_log!("Replied: {:?}", reply);
    return_reply_message(&m, reply)
}

pub async fn chat_info(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let chat_id = format!("{:#?}", m.chat);
    return_reply_message(&m, chat_id)
}

pub async fn echo(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let text = if let Some(msg) = m.text.clone().unwrap().split_once(' ') {
        msg.1.to_string()
    } else {
        "wut?".to_string()
    };
    return_reply_message(&m, text)
}

pub async fn help(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let mut reply = "Available commands:".to_string();
    for (key, _) in _bot.commands {
        reply = reply + "\n\t/" + &key.clone();
    }
    return_reply_message(&m, reply)
}

pub async fn fetch(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let text = if let Some(msg) = m.text.clone().unwrap().split_once(' ') {
        let url = Url::parse(msg.1).unwrap();
        let mut resp = worker::Fetch::Url(url).send().await?;
        resp.text().await.unwrap()
    } else {
        "You need to input a url".to_string()
    };
    return_reply_message(&m, text)
}

pub async fn set_chat_env(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let text = if let Some(msg) = m.text.clone().unwrap().split_once(' ') {
        if msg.1 == "" {
            "Shoud not be empty"
        } else {
            let put = bot_store(&_env)?.put(&format!("INDEX_CHAT_ENV:{}", m.chat.id.0), msg.1)?;
            console_log!("{:?}", put);
            put.execute().await?;
            "Success"
        }
    } else {
        "Error parse prompt"
    };
    return_reply_message(&m, text)
}

pub async fn get_chat_env(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let get = bot_store(&_env)?.get(&format!("INDEX_CHAT_ENV:{}", m.chat.id.0));
    let text = get.text().await?.unwrap_or("env not set".to_string());
    return_reply_message(&m, text)
}

pub async fn clear_chat_context(
    m: Message,
    _env: Env,
    _bot: Bot<'_>,
) -> Result<Response, WorkerError> {
    let msg = match clear_chat_history(&m, &_env).await {
        Ok(_) => "success".to_string(),
        Err(err) => err.to_string(),
    };
    return_reply_message(&m, msg)
}

// user openai key getter
pub async fn get_user_openai_key(m: &Message, _env: &Env) -> Result<Option<String>, WorkerError> {
    let get = bot_store(&_env)?.get(&format!("USER_OPENAI_KEY:{}", m.chat.id.0));
    Ok(get.text().await?)
}

// user openai key setter
pub async fn set_user_openai_key(
    m: Message,
    _env: Env,
    _bot: Bot<'_>,
) -> Result<Response, WorkerError> {
    let text = if let Some(msg) = m.text.clone().unwrap().split_once(' ') {
        if msg.1 == "" {
            "Shoud not be empty"
        } else {
            let put = bot_store(&_env)?.put(&format!("USER_OPENAI_KEY:{}", m.chat.id.0), msg.1)?;
            console_log!("{:?}", put);
            put.execute().await?;
            "Success"
        }
    } else {
        "Error parse prompt"
    };
    return_reply_message(&m, text)
}

pub async fn get_user_openai_endpoint(
    m: &Message,
    _env: &Env,
) -> Result<Option<String>, WorkerError> {
    let get = bot_store(&_env)?.get(&format!("USER_OPENAI_ENDPOINT:{}", m.chat.id.0));
    Ok(get.text().await?)
}

pub async fn set_user_openai_endpoint(
    m: Message,
    _env: Env,
    _bot: Bot<'_>,
) -> Result<Response, WorkerError> {
    let text = if let Some(msg) = m.text.clone().unwrap().split_once(' ') {
        if msg.1 == "" {
            "Shoud not be empty"
        } else {
            let put =
                bot_store(&_env)?.put(&format!("USER_OPENAI_ENDPOINT:{}", m.chat.id.0), msg.1)?;
            console_log!("{:?}", put);
            put.execute().await?;
            "Success"
        }
    } else {
        "Error parse prompt"
    };
    return_reply_message(&m, text)
}

pub async fn call_chat_api(m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    _bot.send_chat_action(m.chat.id.0, "typing").await?;
    let raw_text = m.text.clone().unwrap();
    let msg = match raw_text.starts_with("/") {
        true => match raw_text.split_once(' ') {
            Some(msg) => msg.1,
            None => return return_reply_message(&m, "invalid input"),
        },
        false => raw_text.as_str(),
    };
    let history = get_chat_history(&m, &_env).await?;
    let mut msgs = build_message_context(&m, history, &_env, _bot).await?;
    msgs.push(openai::Message::new("user", msg));
    let key = match get_user_openai_key(&m, &_env).await? {
        Some(_key) => _key,
        None => _env.secret("OPENAI_KEY")?.to_string(),
    };
    let endpoint = get_user_openai_endpoint(&m, &_env).await?;
    let reply = match openai::call_chat_api(&msgs, key, endpoint).await {
        Ok(reply) => {
            msgs.push(openai::Message::new("assistant", &reply));
            put_chat_history(&m, &_env, msgs).await?;
            reply
        }
        Err(err) => format!("{}", err),
    };
    return_message(&m, reply)
}

#[derive(Serialize, Deserialize, Debug)]
struct Command {
    command: String,
    description: String,
}

pub async fn sync_commands(_m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let mut commands = vec![];
    for cmd in _bot.commands {
        commands.push(Command {
            command: format!("/{}", cmd.0.clone()),
            description: cmd.0,
        })
    }
    commands.sort_by(|a, b| a.command.partial_cmp(&b.command).unwrap_or(Ordering::Equal));
    let body = json!({
        "method": "setMyCommands",
        "commands": commands,
    });
    if let Ok(data) = serde_json::to_string(&body.clone()) {
        console_log!("{}", data)
    }
    Response::from_json(&body)
}

pub async fn list_env(_m: Message, _env: Env, _bot: Bot<'_>) -> Result<Response, WorkerError> {
    let mut commands = vec![];
    for cmd in _bot.commands {
        commands.push(Command {
            command: format!("/{}", cmd.0.clone()),
            description: cmd.0,
        })
    }
    let body = json!({
        "method": "setMyCommands",
        "commands": commands,
    });
    if let Ok(data) = serde_json::to_string(&body.clone()) {
        console_log!("{}", data)
    }
    Response::from_json(&body)
}
