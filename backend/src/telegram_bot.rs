use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands};
use dotenv::dotenv;

use crate::app;


// ---------------------------------------------------------------------------
// Telegram Bot Logic
// ---------------------------------------------------------------------------

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Supported commands:")]
enum Command {
    #[command(description = "Start authentication")]
    Start(String), // captures /start <arg>
    #[command(description = "Show help")]
    Help,
}

async fn bot_handler(
    bot: Bot, 
    msg: Message,
    cmd: Command, 
    state: app::AppState
) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, "Send /start <uuid> to log in.").await?;
        }
        Command::Start(args) => {
            // Args will be the UUID provided in the deep link: "t.me/bot?start=UUID"
            let uuid = args.trim();

            if uuid.is_empty() {
                bot.send_message(msg.chat.id, "Please use the login link provided on the website.").await?;
                return Ok(());
            }

            // 1. "Find or Create" User Logic
            // In a real app, you would use sqlx here to upsert the user into Postgres
            let user = msg.from.unwrap();
            let telegram_id = user.id.0;

            println!("🔑 Auth attempt for UUID: {uuid} by User: {telegram_id}");

            // 2. Check if a WebSocket is waiting for this UUID
            if let Some(value) = state.waiting_clients.get(uuid) {
                let tx_channel = &value.client;
                let pin = &value.pin;

                // 4. Send to WebSocket
                // We use a channel here because we can't hold the WS stream directly in the map easily
                if tx_channel.is_closed() {
                    bot.send_message(msg.chat.id, "The browser session seems to have disconnected.").await?;
                } else {
                    bot
                    .parse_mode(ParseMode::MarkdownV2)
                    .send_message(msg.chat.id, format!("Your login code is `{pin}`\\. Enter this in your browser\\."))
                    .await?;
                    // bot.send_message(msg.chat.id, format!("✅ You've been logged in as {}! You can return back now.", username.as_deref().unwrap_or("Unknown"))).await?;
                }
            } else {
                bot.send_message(msg.chat.id, "⚠️ Login session expired or invalid. Please refresh the webpage.").await?;
            }
        }
    };
    Ok(())
}


pub fn spawn_bot_task(state: &app::AppState) -> tokio::task::JoinHandle<()> {
    dotenv().ok();
    
    // Setup Bot
    let bot = Bot::from_env();
    let bot_state = state.clone();
    
    // Spawn Bot Handler
    let handler = Update::filter_message().branch(
        dptree::entry()
            .filter_command::<Command>()
            .endpoint(bot_handler),
    );
    
    return tokio::spawn(async move {
        Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![bot_state])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    });
}