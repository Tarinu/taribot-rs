use crate::api::gfycat::{Client, ClientBuilder};
use crate::CatvidConfigContainer;

use log::debug;
use serenity::{
    framework::standard::{macros::command, CommandError, CommandResult},
    model::prelude::*,
    prelude::*,
};

use std::env;

pub struct CatvidConfig {
    client: Client,
}

impl CatvidConfig {
    pub fn new() -> Self {
        let client = ClientBuilder::new(
            env::var("CATVID_CLIENT_ID").expect("CATVID_CLIENT_ID missing"),
            env::var("CATVID_CLIENT_SECRET").expect("CATVID_CLIENT_SECRET missing"),
            env::var("CATVID_ALBUM_ID").expect("CATVID_ALBUM_ID missing"),
        )
        .password_grant(
            env::var("CATVID_USERNAME").expect("CATVID_USERNAME missing"),
            env::var("CATVID_PASSWORD").expect("CATVID_PASSWORD missing"),
        )
        .build()
        .unwrap();

        CatvidConfig { client: client }
    }

    fn random_video(&mut self) -> String {
        self.client.random_video()
    }
}

#[command]
pub fn catvid(ctx: &mut Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read();

    let container = data
        .get::<CatvidConfigContainer>()
        .ok_or(CommandError("Failed to get CatvidConfig".to_string()))?;
    let mut config = container.lock();

    let video = config.random_video();
    debug!("Sending {}", video);

    msg.channel_id.say(&ctx.http, video)?;

    Ok(())
}
