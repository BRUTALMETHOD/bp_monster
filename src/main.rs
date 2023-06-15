use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::model::gateway::{Activity, Ready};
//use serenity::model::id::GuildId;
use serenity::prelude::*;

use reqwest;

#[group]
#[commands(ping)]
struct General;

enum LauncherStatus {
    Up,
    Down,
}

struct Handler {
    is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        println!("{:?}", msg);
        if msg.content == "!ping" {
            // Sending a message can fail, due to a network error, an
            // authentication error, or lack of permissions to post in the
            // channel, so log to stdout when some error happens, with a
            // description of it.
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {:?}", why);
            }
        }
    }
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        // We need to check that the loop is not already running when this event triggers,
        // as this event triggers every time the bot enters or leaves a guild, along every time the
        // ready shard event triggers.
        //
        // An AtomicBool is used because it doesn't require a mutable reference to be changed, as
        // we don't have one due to self being an immutable reference.
        if !self.is_loop_running.load(Ordering::Relaxed) {
            // We have to clone the Arc, as it gets moved into the new thread.
            //let ctx1 = Arc::clone(&ctx);
            // tokio::spawn creates a new green thread that can run in parallel with the rest of
            // the application.
            tokio::spawn(async move {
                loop {
                    // We clone Context again here, because Arc is owned, so it moves to the
                    // new function.
                    //log_system_load(Arc::clone(&ctx1)).await;
                    match get_launcher_status().await.unwrap() {
                        LauncherStatus::Up => {
                            ctx.set_activity(Activity::playing("Blue Protocol is Up!"))
                                .await;
                        }
                        LauncherStatus::Down => {
                            ctx.set_activity(Activity::playing("Blue Protocol is Down!"))
                                .await;
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

async fn get_launcher_status() -> Result<LauncherStatus, reqwest::Error> {
    // check status at url: http://api-bnolauncher.bandainamco-ol.jp
    // maintenance msg : {
    //    "Message": "現在メンテナンス中です。",
    //    "Title": "メンテナンス"
    // }
    println!("Fetching url..");
    let resp = reqwest::get("https://api-bnolauncher.bandainamco-ol.jp").await?;
    eprintln!("{:#?}", resp);
    match resp.status() {
        reqwest::StatusCode::OK => {
            eprintln!("Launcher is OK!: {:#?}", resp);
            Ok(LauncherStatus::Up)
        }
        _ => {
            eprintln!("Launcher is under maintenance: {:#?}", resp);
            Ok(LauncherStatus::Down)
        }
    }
}

// List of urls to check for
// object-bnolauncher-pf.bandainamco-ol.jp
// datastore-main.aws.blue-protocol.com
// masterdata-main.aws.blue-protocol.com
// flg-main.aws.blue-protocol.com
// g-ahpatch-prod.blue-protocol.com
// api-bnolauncher.bandainamco-ol.jp

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}
