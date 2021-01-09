use crate::api::gfycat::{Client, ClientBuilder, RequestError};
use crate::CatvidConfigContainer;

use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::*,
    prelude::*,
};
use tracing::debug;

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

    async fn random_video(&mut self) -> Result<String, RequestError> {
        self.client.random_video().await
    }
}

#[command]
pub async fn catvid(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let container = data
        .get::<CatvidConfigContainer>()
        .ok_or("Failed to get CatvidConfig".to_string())?;
    let mut config = container.lock().await;

    let video = config.random_video().await?;
    debug!("Sending {}", video);

    msg.channel_id.say(&ctx.http, video).await?;

    Ok(())
}
