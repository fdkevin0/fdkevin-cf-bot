pub mod bot;
pub mod chat;
pub mod command;
pub mod openai;

use cfg_if::cfg_if;
use sha2::{Digest, Sha256};
use worker::{
    console_error, console_log, event, Date, Env, Error as WorkerError, Request, Response, Router,
};

use bot::Bot;

pub const TELEGRAM_API_TOKEN: &str = "TELEGRAM_API_TOKEN";
const VAR_KV_STORE: &str = "KV_STORE";

// cloudflare workers log api workarround
cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        pub use self::console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub fn set_panic_hook() {}
    }
}

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

pub async fn main_inner(
    req: Request,
    env: Env,
    _ctx: worker::Context,
) -> Result<Response, WorkerError> {
    log_request(&req);
    set_panic_hook();

    // Bot
    let mut bot = Bot::new_with_env(&env, TELEGRAM_API_TOKEN, VAR_KV_STORE)?;

    // Commands
    bot.register_command("echo", command::echo);
    bot.register_command("start", command::start);
    bot.register_command("chat_info", command::chat_info);
    bot.register_command("help", command::help);
    bot.register_command("fetch", command::fetch);
    bot.register_command("chat", command::call_chat_api);
    bot.register_command("sync_commands", command::sync_commands);
    bot.register_command("set_chat_env", command::set_chat_env);
    bot.register_command("get_chat_env", command::get_chat_env);
    bot.register_command("clear", command::clear_chat_context);
    bot.register_command("set_openai_key", command::set_user_openai_key);
    bot.register_command("set_openai_endpoint", command::set_user_openai_endpoint);

    bot.with_default(command::call_chat_api);

    let tg_bot_token_sha256 = sha256(env.secret(TELEGRAM_API_TOKEN.as_ref())?.to_string());

    // Router
    let router = Router::with_data(bot)
        .get_async(
            format!("/{}/", tg_bot_token_sha256).as_str(),
            |req, ctx| async move {
                let bot = ctx.data;
                let target = format!("{}updates", req.url()?);
                console_log!("Setting up webhook, URL: {}", target);
                bot.setup_webhook(target).await?;
                Response::from_json(&bot.get_me().await?)
            },
        )
        .post_async(
            format!("/{}/updates", tg_bot_token_sha256).as_str(),
            |mut req, ctx| async move { Bot::process_update(&mut req, ctx).await },
        );

    // Run
    router.run(req, env).await
}

fn sha256(token: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token);
    format!("{:x}", hasher.finalize())
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: worker::Context) -> Result<Response, WorkerError> {
    match main_inner(req, env, ctx).await {
        Ok(res) => Ok(res),
        Err(e) => {
            console_error!("Error occurred: {}", e);
            Ok(Response::from_html(format!("Internal Server Error: {}", e))
                .expect("Bruh, what just happened?"))
        }
    }
}

#[test]
fn test_sha256() {
    let token = "476884080:AAH-qyccfEpbCh8Pr1bw-wXL67EWGTW337I";
    println!("{}", sha256(token.to_string()))
}

pub fn bot_store(_env: &Env) -> Result<worker::kv::KvStore, WorkerError> {
    _env.kv("FDKEVIN_BOT_STORE")
}
