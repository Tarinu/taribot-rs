mod api;
mod commands;

use dotenv::dotenv;
use serenity::{
    async_trait,
    framework::{
        standard::DispatchError::{CheckFailed, NotEnoughArguments, TooManyArguments},
        standard::{
            help_commands,
            macros::{group, help, hook},
            CommandResult, DispatchError,
        },
        standard::{Args, CommandGroup, HelpOptions, Reason},
        StandardFramework,
    },
    http::Http,
    model::{channel::Message, event::ResumedEvent, gateway::Ready, id::UserId},
    prelude::*,
};
use std::{collections::HashSet, env, sync::Arc};

use commands::cat::*;
use commands::catvid::*;

use tracing::{error, info, warn};

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
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, _command_name: &str) {
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

#[help]
#[max_levenshtein_distance(3)]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await?;
    Ok(())
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
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new(&token);

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
        .help(&HELP)
        .group(&GENERAL_GROUP)
        .on_dispatch_error(dispatch_error);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
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
