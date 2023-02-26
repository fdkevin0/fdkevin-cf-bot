use serde_json::json;
use teloxide::{dispatching::dialogue::GetChatId, prelude::*, utils::command::BotCommands};

pub async fn main_inner2(
    req: worker::Request,
    env: worker::Env,
    _ctx: worker::Context,
) -> Result<worker::Response, worker::Error> {
    // log_request(&req);
    // set_panic_hook();

    // Bot
    // let mut bot = Bot::new_with_env(&env, TELEGRAM_API_TOKEN, VAR_KV_STORE)?;
    // bot.register_command("echo", command::echo);
    // bot.register_command("start", command::start);
    // bot.register_command("chat_info", command::chat_info);
    // bot.register_command("help", command::help);
    let bot = Bot::new(crate::TELEGRAM_API_TOKEN);
    let tg_bot_token_sha256 =
        crate::sha256(env.secret(crate::TELEGRAM_API_TOKEN.as_ref())?.to_string());

    // Router
    let router = worker::Router::with_data(bot)
        // .get_async(
        //     format!("/{}/", tg_bot_token_sha256).as_str(),
        //     |req, ctx| async move {
        //         let bot = ctx.data;
        //         let target = format!("{}updates", req.url()?);
        //         info!("Setting up webhook, URL: {}", target);
        //         bot.setup_webhook(target).await?;
        //         Response::from_json(&bot.get_me().await?)
        //     },
        // )
        .post_async(
            format!("/{}/updates", tg_bot_token_sha256).as_str(),
            |mut req, ctx| async move { process_update(&mut req, ctx).await },
        );

    // Run
    router.run(req, env).await
}

pub async fn process_update(
    req: &mut worker::Request,
    ctx: worker::RouteContext<Bot>,
) -> Result<worker::Response, worker::Error> {
    let update = req.json::<Message>().await?;
    // debug!("Received update: {:?}", update);
    // if update.content.is_none() {
    // debug!("No content found, ignoring...");
    //     return worker::Response::from_json(&json!({}));
    // }
    // let update_content = update.content.unwrap();
    let bot = ctx.data;
    if let Some(text) = update.text() {
        bot.send_message(update.chat.id, text).await.unwrap();
    } else {
        bot.send_message(update.chat.id, "sth").await.unwrap();
    }
    // if let UpdateContent::Message(m) = update_content {
    //     // debug!("Got message: {:#?}", m);
    //     if m.text.is_none() {
    //         // debug!("No text found, ignoring...");
    //         return worker::Response::from_json(&json!({}));
    //     }
    //     let bot = ctx.data;
    //     let env = ctx.env;
    //     bot.run_commands(m, env).await
    // } else {
    //     // info!("Not a message, ignoring...");
    //     worker::Response::from_json(&json!({}))
    // }
    worker::Response::from_json(&json!({}))
}

async fn main2() {
    let bot = Bot::new("");
    // bot.set_webhook("");

    // teloxide::repl(bot, |bot: Bot, msg: Message| async move {
    //     bot.send_dice(msg.chat.id).await?;
    //     Ok(())
    // })
    // .await;
    // let listener = webhooks::axum(bot.clone(), webhooks::Options::new(addr, url))
    //     .await
    //     .expect("Couldn't setup webhook");

    // teloxide::repl_with_listener(
    //     bot,
    //     |bot: Bot, msg: Message| async move {
    //         bot.send_message(msg.chat.id, "pong").await?;
    //         Ok(())
    //     },
    //     listener,
    // )
    // .await;
    // answer(bot, msg, cmd);
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "handle a username.")]
    Username(String),
    #[command(description = "handle a username and an age.", parse_with = "split")]
    UsernameAndAge { username: String, age: u8 },
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Username(username) => {
            bot.send_message(msg.chat.id, format!("Your username is @{username}."))
                .await?
        }
        Command::UsernameAndAge { username, age } => {
            bot.send_message(
                msg.chat.id,
                format!("Your username is @{username} and age is {age}."),
            )
            .await?
        }
    };

    Ok(())
}
