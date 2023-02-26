pub mod bot;
pub mod command;

use cfg_if::cfg_if;
use log::{error, info};
use sha2::{Digest, Sha256};
use worker::{event, Date, Env, Error as WorkerError, Request, Response, Router};

use bot::Bot;

pub const TELEGRAM_API_TOKEN: &str = "TELEGRAM_API_TOKEN";
const VAR_KV_STORE: &str = "KV_STORE";

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        pub use console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub fn set_panic_hook() {}
    }
}

fn log_request(req: &Request) {
    info!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
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
    bot.register_command("echo", command::echo);
    bot.register_command("start", command::start);
    bot.register_command("chat_info", command::chat_info);
    bot.register_command("help", command::help);
    bot.register_command("fetch", command::fetch);
    // bot.register_command("sync_commands", command::sync_commands);

    let tg_bot_token_sha256 = sha256(env.secret(TELEGRAM_API_TOKEN.as_ref())?.to_string());

    // Router
    let router = Router::with_data(bot)
        .get_async(
            format!("/{}/", tg_bot_token_sha256).as_str(),
            |req, ctx| async move {
                let bot = ctx.data;
                let target = format!("{}updates", req.url()?);
                info!("Setting up webhook, URL: {}", target);
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
    // match bot2::main_inner2(req, env, ctx).await {
    //     Ok(res) => Ok(res),
    //     Err(e) => {
    //         error!("Error occurred: {}", e);
    //         Ok(Response::from_html(format!("Internal Server Error: {}", e))
    //             .expect("Bruh, what just happened?"))
    //     }
    // }
    match main_inner(req, env, ctx).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("Error occurred: {}", e);
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
