use telegram_types::bot::types::Message;
use worker::{console_log, Env, Error as WorkerError};

use crate::{bot::Bot, bot_store, openai};

pub async fn put_chat_history(
    m: &Message,
    _env: &Env,
    msgs: Vec<openai::Message>,
) -> Result<(), WorkerError> {
    let put = bot_store(&_env)?.put(&format!("INDEX_CHAT_HISTORY:{}", m.chat.id.0), msgs)?;
    console_log!("{:?}", put);
    put.execute().await?;
    Ok(())
}

pub async fn get_chat_history(
    m: &Message,
    _env: &Env,
) -> Result<Vec<openai::Message>, WorkerError> {
    let get = bot_store(&_env)?
        .get(&format!("INDEX_CHAT_HISTORY:{}", m.chat.id.0))
        .json::<Vec<openai::Message>>();
    Ok(get.await?.unwrap_or(vec![]))
}

pub async fn clear_chat_history(m: &Message, _env: &Env) -> Result<(), WorkerError> {
    bot_store(&_env)?
        .delete(&format!("INDEX_CHAT_HISTORY:{}", m.chat.id.0))
        .await?;
    Ok(())
}

const PREFER_CONTEXT_LENGTH: usize = 5;

pub async fn build_message_context(
    m: &Message,
    mut history: Vec<openai::Message>,
    _env: &Env,
    _bot: Bot<'_>,
) -> Result<Vec<openai::Message>, WorkerError> {
    let mut msgs = vec![];
    if let Some(chat_env) = _env
        .kv("FDKEVIN_BOT_STORE")?
        .get(&format!("INDEX_CHAT_ENV:{}", m.chat.id.0))
        .text()
        .await?
    {
        msgs.push(openai::Message::new("system", &chat_env))
    }
    history = history
        .into_iter()
        .filter(|msg| msg.role != "system")
        .collect();
    if history.len() > PREFER_CONTEXT_LENGTH {
        history.drain(..history.len() - PREFER_CONTEXT_LENGTH);
    }

    msgs.append(&mut history);
    Ok(msgs)
}
