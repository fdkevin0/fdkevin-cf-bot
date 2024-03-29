use futures::future::LocalBoxFuture;
use serde::Serialize;
use serde_json::json;
use telegram_types::bot::methods::{
    ApiError, ChatTarget, DeleteWebhook, GetChat, GetChatMember, GetMe, Method, SetWebhook,
    TelegramResult, UpdateTypes,
};
use telegram_types::bot::types::{
    Chat, ChatMember, ChatMemberStatus, Message, Update, UpdateContent, User, UserId,
};
use worker::kv::KvStore;
use worker::wasm_bindgen::JsValue;
use worker::{
    console_debug, console_log, Env, Error as WorkerError, Fetch, Headers, Method as RequestMethod,
    Request, RequestInit, Response, RouteContext,
};

use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;

const ACCEPTED_TYPES: &[UpdateTypes] = &[UpdateTypes::Message];

type CommandFn<'a> =
    Rc<dyn 'a + Fn(Message, Env, Bot<'a>) -> LocalBoxFuture<'a, Result<Response, WorkerError>>>;

#[derive(Clone)]
pub struct Bot<'a> {
    pub token: String,
    kv_store: String,
    pub commands: HashMap<String, CommandFn<'a>>,
    pub default: Option<CommandFn<'a>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WebhookReply<T: Method> {
    pub method: String,
    #[serde(flatten)]
    pub content: T,
}

impl<'a> Bot<'a> {
    pub fn new<S: AsRef<str>>(token: S, kv_store: S) -> Self {
        Self {
            token: token.as_ref().to_string(),
            kv_store: kv_store.as_ref().to_string(),
            commands: HashMap::new(),
            default: None,
        }
    }

    pub async fn send_json_request(
        &self,
        method: RequestMethod,
        url: &str,
        payload: &str,
    ) -> Result<Response, WorkerError> {
        let mut request_builder = RequestInit::new();
        let mut headers = Headers::new();
        if method != RequestMethod::Get {
            headers.set("Content-Type", "application/json")?;
            console_log!("Sending JSON payload: {}", payload);
            request_builder.with_body(Some(JsValue::from_str(payload)));
        }
        request_builder.with_headers(headers).with_method(method);
        Fetch::Request(Request::new_with_init(url, &request_builder)?)
            .send()
            .await
    }

    pub async fn send_method_request<T: Method>(
        &self,
        request: T,
        method: RequestMethod,
    ) -> Result<Response, WorkerError> {
        self.send_json_request(
            method,
            &T::url(&self.token),
            &serde_json::to_string(&request).map_err(Into::<WorkerError>::into)?,
        )
        .await
    }

    pub fn convert_error(e: ApiError) -> WorkerError {
        WorkerError::RustError(e.description)
    }

    pub async fn send_method_get<T: Method>(&self, request: T) -> Result<Response, WorkerError> {
        self.send_method_request(request, RequestMethod::Get).await
    }

    pub async fn get_me(&self) -> Result<User, WorkerError> {
        let mut result = self.send_method_get(GetMe).await?;
        result
            .json::<TelegramResult<User>>()
            .await?
            .into_result()
            .map_err(Bot::convert_error)
    }

    pub async fn get_chat(&self, chat_id: ChatTarget<'_>) -> Result<Chat, WorkerError> {
        let mut result = self
            .send_method_request(GetChat { chat_id }, RequestMethod::Post)
            .await?;
        result
            .json::<TelegramResult<Chat>>()
            .await?
            .into_result()
            .map_err(Bot::convert_error)
    }

    pub async fn send_chat_action(&self, chat_id: i64, action: &str) -> Result<(), WorkerError> {
        self.send_json_request(
            RequestMethod::Post,
            &format!(
                "https://api.telegram.org/bot{}/{}",
                self.token, "sendChatAction"
            ),
            &serde_json::to_string(&json!({
                "chat_id": chat_id,
                "action": action,
            }))
            .map_err(Into::<WorkerError>::into)?,
        )
        .await?;
        Ok(())
    }

    pub async fn is_admin(
        &self,
        chat_id: ChatTarget<'_>,
        user_id: UserId,
    ) -> Result<bool, WorkerError> {
        let chat_member = self
            .send_method_request(GetChatMember { chat_id, user_id }, RequestMethod::Post)
            .await?
            .json::<TelegramResult<ChatMember>>()
            .await?
            .into_result()
            .map_err(Bot::convert_error)?;
        let member_status = chat_member.status;
        console_log!("Member status: {:?}", member_status);
        Ok(member_status == ChatMemberStatus::Creator
            || member_status == ChatMemberStatus::Administrator)
    }

    // fn get_kv(&self) -> Result<KvStore, WorkerError> {
    //     self.env.kv(&self.env.var(VAR_KV_STORE)?.to_string())
    // }

    // pub async fn update_username(&mut self) -> Result<(), WorkerError> {
    //     let user = self.get_me().await?;
    //     self.username = user.username;
    //     let kv = self.get_kv()?;
    //     kv.put(KEY_USERNAME, self.username.clone().expect("WTF"))?
    //         .execute()
    //         .await?;
    //     Ok(())
    // }

    pub fn new_with_env<S: AsRef<str>>(
        env: &Env,
        var_token: S,
        var_kv_store: S,
    ) -> Result<Self, WorkerError> {
        Ok(Self::new(
            env.secret(var_token.as_ref())?.to_string(),
            env.var(var_kv_store.as_ref())?.to_string(),
        ))
    }

    pub async fn setup_webhook<S: AsRef<str>>(&self, url: S) -> Result<(), WorkerError> {
        let payload = DeleteWebhook;
        let mut result = self
            .send_method_request(payload, RequestMethod::Post)
            .await?;
        console_log!(
            "Trying to delete previously set webhooks: {}",
            result.text().await?
        );
        let mut payload = SetWebhook::new(url.as_ref());
        payload.allowed_updates = Some(Cow::from(ACCEPTED_TYPES));
        let mut result = self
            .send_method_request(payload, RequestMethod::Post)
            .await?;
        console_log!("Set new webhook: {}", result.text().await?);
        Ok(())
    }

    pub fn with_default<F: 'a + Future<Output = Result<Response, WorkerError>>>(
        &mut self,
        func: fn(Message, Env, Bot<'a>) -> F,
    ) {
        self.default = Some(Rc::new(move |msg, env, bot| Box::pin(func(msg, env, bot))))
    }

    pub fn register_command<
        S: AsRef<str>,
        F: 'a + Future<Output = Result<Response, WorkerError>>,
    >(
        &mut self,
        command: S,
        // description: S,
        func: fn(Message, Env, Bot<'a>) -> F,
    ) {
        self.commands.insert(
            command.as_ref().to_string(),
            Rc::new(move |msg, env, bot| Box::pin(func(msg, env, bot))),
        );
    }

    pub async fn run_commands(&self, m: Message, env: Env) -> Result<Response, WorkerError> {
        let message_text = m.text.clone().unwrap_or_default();
        console_log!(
            "Non empty message text from {:?} : {}",
            m.chat.clone(),
            message_text
        );
        let message_command = message_text.split(' ').collect::<Vec<&str>>()[0]
            .trim()
            .to_ascii_lowercase();
        console_debug!("First phrase extracted from text: {}", message_command);
        for (command, func) in &self.commands {
            // `/start bruh` and `/start@blablabot bruh`
            let command_prefix = format!("/{}", command.to_ascii_lowercase());
            let command_prefix_extended = format!("{}@THE_BOT_USERNAME", command_prefix,);
            if (message_command == command_prefix) || (message_command == command_prefix_extended) {
                console_log!("Command matched: {}", command);
                return func(m, env, self.clone()).await;
            }
        }
        if let Some(cmd) = &self.default {
            return cmd(m, env, self.clone()).await;
        }
        console_log!("No command matched, ignoring...");
        Response::empty()
    }

    pub async fn process_update(
        req: &mut Request,
        ctx: RouteContext<Bot<'a>>,
    ) -> Result<Response, WorkerError> {
        let update = req.json::<Update>().await?;
        console_debug!("Received update: {:?}", update);
        if update.content.is_none() {
            console_debug!("No content found, ignoring...");
            return Response::from_json(&json!({}));
        }
        let update_content = update.content.unwrap();
        if let UpdateContent::Message(m) = update_content {
            // console_debug!("Got message: {:#?}", m);
            if m.text.is_none() {
                console_debug!("No text found, ignoring...");
                return Response::from_json(&json!({}));
            }
            if m.chat.id.0 != 374506773 {
                return Response::from_json(&json!({}));
            }
            let bot = ctx.data;
            let env = ctx.env;
            bot.run_commands(m, env).await
        } else {
            console_log!("Not a message, ignoring...");
            Response::from_json(&json!({}))
        }
    }

    pub fn get_kv(&self, env: &Env) -> Result<KvStore, WorkerError> {
        env.kv(&self.kv_store)
    }
}

impl<T: Method> From<T> for WebhookReply<T> {
    fn from(method: T) -> WebhookReply<T> {
        WebhookReply {
            method: <T>::NAME.to_string(),
            content: method,
        }
    }
}
