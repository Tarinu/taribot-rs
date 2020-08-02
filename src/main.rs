mod commands;

use dotenv::dotenv;
use log::{debug, error, info, warn};
use serenity::{
    client::bridge::gateway::ShardManager,
    framework::{
        standard::macros::group,
        standard::DispatchError::{CheckFailed, NotEnoughArguments, TooManyArguments},
        standard::Reason,
        StandardFramework,
    },
    model::{event::ResumedEvent, gateway::Ready},
    prelude::*,
};
use std::{collections::HashSet, env, sync::Arc};

use commands::cat::*;
struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(cat)]
struct General;

struct CatConfig {
    max_images: u8,
}

impl TypeMapKey for CatConfig {
    type Value = CatConfig;
}

fn main() {
    if dotenv().is_err() {
        warn!("Failed to load .env file");
    }

    env_logger::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    {
        let mut data = client.data.write();
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
        let mut cat_count = 1;
        match env::var("CAT_MAX_IMAGES") {
            Ok(count) => {
                cat_count = count.parse::<u8>().unwrap();
            }
            Err(_) => {
                warn!(
                    "Failed to load Cat command count from env, defaulting to {}",
                    cat_count
                );
            }
        }
        debug!("Cat count set to: {}", cat_count);
        data.insert::<CatConfig>(CatConfig {
            max_images: cat_count,
        });
    }

    let owners = match client.cache_and_http.http.get_current_application_info() {
        Ok(info) => {
            let mut set = HashSet::new();
            set.insert(info.owner.id);

            set
        }
        Err(reason) => panic!("Couldn't get application info: {:?}", reason),
    };

    client.with_framework(
        StandardFramework::new()
            .configure(|c| {
                c.owners(owners).prefix(
                    env::var("PREFIX")
                        .expect("Expected a prefix in the environment")
                        .as_str(),
                )
            })
            .group(&GENERAL_GROUP)
            .on_dispatch_error(|ctx, msg, error| match error {
                CheckFailed(_check_name, reason) => match reason {
                    Reason::User(message) => {
                        if let Err(e) = msg.channel_id.say(&ctx.http, message) {
                            error!("{}", e);
                        }
                    }
                    Reason::Log(message) => {
                        warn!("{}", message);
                    }
                    Reason::UserAndLog { user, log } => {
                        if let Err(e) = msg.channel_id.say(&ctx.http, user) {
                            error!("{}", e);
                        }
                        warn!("{}", log);
                    }
                    _ => (),
                },
                NotEnoughArguments { min, given } => {
                    if let Err(e) = msg.channel_id.say(
                        &ctx.http,
                        format!("Need {} arguments, but only got {}.", min, given),
                    ) {
                        error!("{}", e);
                    }
                }
                TooManyArguments { max, given } => {
                    if let Err(e) = msg.channel_id.say(
                        &ctx.http,
                        format!("Max arguments allowed is {}, but got {}.", max, given),
                    ) {
                        error!("{}", e);
                    }
                }
                _ => (),
            }),
    );

    if let Err(reason) = client.start() {
        error!("Client error: {:?}", reason);
    }
}
