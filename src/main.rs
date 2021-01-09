mod api;
mod commands;

use dotenv::dotenv;
use serenity::{
    async_trait,
    framework::{
        standard::DispatchError::{CheckFailed, NotEnoughArguments, TooManyArguments},
        standard::Reason,
        standard::{
            macros::{group, hook},
            DispatchError,
        },
        StandardFramework,
    },
    http::Http,
    model::{channel::Message, event::ResumedEvent, gateway::Ready},
    prelude::*,
};
use std::{collections::HashSet, env, sync::Arc};

use commands::cat::*;
use commands::catvid::*;

use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

struct CatvidConfigContainer;

impl TypeMapKey for CatvidConfigContainer {
    type Value = Arc<Mutex<CatvidConfig>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        CheckFailed(_check_name, reason) => match reason {
            Reason::User(message) => {
                if let Err(e) = msg.channel_id.say(&ctx.http, message).await {
                    error!("{}", e);
                }
            }
            Reason::Log(message) => {
                warn!("{}", message);
            }
            Reason::UserAndLog { user, log } => {
                if let Err(e) = msg.channel_id.say(&ctx.http, user).await {
                    error!("{}", e);
                }
                warn!("{}", log);
            }
            _ => (),
        },
        NotEnoughArguments { min, given } => {
            if let Err(e) = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("Need {} arguments, but only got {}.", min, given),
                )
                .await
            {
                error!("{}", e);
            }
        }
        TooManyArguments { max, given } => {
            if let Err(e) = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("Max arguments allowed is {}, but got {}.", max, given),
                )
                .await
            {
                error!("{}", e);
            }
        }
        _ => (),
    }
}

#[group]
#[commands(cat, catvid)]
struct General;

#[tokio::main]
async fn main() {
    if dotenv().is_err() {
        warn!("Failed to load .env file");
    }

    // Initialize the logger to use environment variables.
    //
    // In this case, a good default is setting the environment variable
    // `RUST_LOG` to debug`.
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| {
            c.owners(owners).prefix(
                env::var("PREFIX")
                    .expect("Expected a prefix in the environment")
                    .as_str(),
            )
        })
        .group(&GENERAL_GROUP)
        .on_dispatch_error(dispatch_error);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<CatConfig>(CatConfig::new());
        data.insert::<CatvidConfigContainer>(Arc::new(Mutex::new(CatvidConfig::new())));
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
