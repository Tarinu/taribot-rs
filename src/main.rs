mod commands;

use std::{
    collections::HashSet,
    env,
    sync::Arc,
};
use serenity::{
    client::bridge::gateway::ShardManager,
    framework::{
        StandardFramework,
        standard::macros::group,
    },
    model::{event::ResumedEvent, gateway::Ready},
    prelude::*,
};
use log::{error, warn, info};
use dotenv::dotenv;

use commands::{
    cat::*,
};
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

fn main() {
    if dotenv().is_err() {
        warn!("Failed to load .env file");
    }

    env_logger::init();

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    {
        let mut data = client.data.write();
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    let owners = match client.cache_and_http.http.get_current_application_info() {
        Ok(info) => {
            let mut set = HashSet::new();
            set.insert(info.owner.id);

            set
        },
        Err(reason) => panic!("Couldn't get application info: {:?}", reason),
    };

    client.with_framework(StandardFramework::new()
        .configure(|c| c
            .owners(owners)
            .prefix(env::var("PREFIX").expect("Expected a prefix in the environment").as_str()))
        .group(&GENERAL_GROUP));

    if let Err(reason) = client.start() {
        error!("Client error: {:?}", reason);
    }
}
