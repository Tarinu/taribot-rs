use crate::CatvidConfigContainer;
use log::debug;
use rand::{seq::SliceRandom, thread_rng};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serenity::{
    framework::standard::{macros::command, CommandError, CommandResult},
    model::prelude::*,
    prelude::*,
};
use std::time::Instant;

use std::env;

const TOKEN_URL: &str = "https://api.gfycat.com/v1/oauth/token";

pub struct CatvidConfig {
    token: Option<Token>,
    token_data: TokenData,
    client: Client,
    album_id: String,
    gfycats: Option<GfycatCollection>,
    /// Time since last request
    last_request: Option<Instant>,
}
/// Data that gets sent to the token endpoint
#[derive(Serialize)]
struct TokenData {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
    grant_type: String,
}

/// Data that gets sent to the refresh token endpoint
#[derive(Serialize)]
struct RefreshTokenData<'a> {
    client_id: &'a String,
    client_secret: &'a String,
    refresh_token: &'a String,
    grant_type: String,
}

/// The actual token data returned by the API
#[derive(Deserialize)]
#[allow(dead_code)]
struct Token {
    token_type: String,
    refresh_token_expires_in: u32,
    refresh_token: String,
    scope: String,
    resource_owner: String,
    expires_in: u32,
    access_token: String,
    /// Timestamp when the token was generated, used for checking token validity
    /// This is only Option since serde has no idea how to deal with it, it's safe to assume it will always have value
    #[serde(skip)]
    request_timestamp: Option<Instant>,
}

#[derive(Deserialize)]
#[allow(dead_code, non_snake_case)]
struct AlbumResponse {
    publishedGfys: GfycatCollection,
}
#[derive(Deserialize)]
#[allow(dead_code, non_snake_case)]
struct Gfycat {
    gfyId: String,
    gfyName: String,
    gfyNumber: String,
    avgColor: String,
    userName: String,
    width: String,
    height: String,
    frameRate: String,
    numFrames: String,
    mp4Url: Option<String>,
    webmUrl: Option<String>,
    webpUrl: Option<String>,
    mobileUrl: Option<String>,
    mobilePosterUrl: Option<String>,
    posterUrl: Option<String>,
    thumb360Url: Option<String>,
    thumb360PosterUrl: Option<String>,
    thumb100PosterUrl: Option<String>,
    max5mbGif: Option<String>,
    max2mbGif: Option<String>,
    mjpgUrl: Option<String>,
    miniUrl: Option<String>,
    miniPosterUrl: Option<String>,
    gifUrl: Option<String>,
    gifSize: Option<String>,
    mp4Size: Option<String>,
    webmSize: Option<String>,
    createDate: Option<String>,
    views: u32,
    viewsNewEpoch: Option<String>,
    title: Option<String>,
    extraLemmas: Option<String>,
    md5: Option<String>,
    tags: Option<Vec<String>>,
    userTags: Option<Vec<String>>,
    nsfw: Option<String>,
    sar: Option<String>,
    url: Option<String>,
    source: Option<String>,
    dynamo: Option<String>,
    subreddit: Option<String>,
    redditId: Option<String>,
    redditIdText: Option<String>,
    likes: Option<u32>,
    dislikes: Option<u32>,
    published: Option<String>,
    description: Option<String>,
    copyrightClaimaint: Option<String>,
    languageText: Option<String>,
    gatekeeper: Option<String>,
    fullDomainWhitelist: Vec<String>,
    fullGeoWhitelist: Vec<String>,
    iframeProfileImageVisible: bool,
}

#[derive(Deserialize)]
struct GfycatCollection(Vec<Gfycat>);

impl GfycatCollection {
    fn pick_random(&self) -> Option<&Gfycat> {
        self.0.choose(&mut thread_rng())
    }
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct GfycatError {
    errorMessage: GfycatErrorMessage,
}

#[derive(Deserialize, Debug)]
struct GfycatErrorMessage {
    code: String,
    description: String,
}

#[derive(Debug)]
enum RequestError {
    Gfycat(GfycatError),
    Reqwest(reqwest::Error),
}

impl From<reqwest::Error> for RequestError {
    fn from(error: reqwest::Error) -> Self {
        Self::Reqwest(error)
    }
}

impl CatvidConfig {
    pub fn new() -> Self {
        CatvidConfig {
            client: Client::new(),
            token_data: TokenData {
                client_id: env::var("CATVID_CLIENT_ID").expect("CATVID_CLIENT_ID missing"),
                client_secret: env::var("CATVID_CLIENT_SECRET")
                    .expect("CATVID_CLIENT_SECRET missing"),
                username: env::var("CATVID_USERNAME").expect("CATVID_USERNAME missing"),
                password: env::var("CATVID_PASSWORD").expect("CATVID_PASSWORD missing"),
                grant_type: "password".to_string(),
            },
            token: None,
            album_id: env::var("CATVID_ALBUM_ID").expect("CATVID_ALBUM_ID missing"),
            gfycats: None,
            last_request: None,
        }
    }

    fn random_video(&mut self) -> String {
        // Cache is newer than last 24h
        if self.gfycats.is_some() && self.last_request.unwrap().elapsed().as_secs() < 60 * 60 * 24 {
            let collection = self.gfycats.as_ref().unwrap();
            let gfycat = collection.pick_random().unwrap();
            return format!("https://gfycat.com/{}", gfycat.gfyId);
        }

        if self.token.is_none() {
            self.token = Some(self.request_token().unwrap());
        }

        let token = self.token.as_ref().unwrap();
        if !token.is_valid() {
            self.token = Some(self.refresh_token().unwrap());
        }

        let response = self.request_album().unwrap();
        self.last_request = Some(Instant::now());
        self.gfycats = Some(response.publishedGfys);

        let collection = self.gfycats.as_ref().unwrap();
        let gfycat = collection.pick_random().unwrap();
        format!("https://gfycat.com/{}", gfycat.gfyId)
    }

    fn request_token(&self) -> Result<Token, RequestError> {
        let response = self.client.post(TOKEN_URL).json(&self.token_data).send()?;

        if !response.status().is_success() {
            return Err(RequestError::Gfycat(response.json::<GfycatError>()?));
        }
        let mut token: Token = response.json()?;
        token.request_timestamp = Some(Instant::now());

        Ok(token)
    }

    fn refresh_token(&self) -> Result<Token, RequestError> {
        if self.token.is_none() {
            return self.request_token();
        }

        if !self.token.as_ref().unwrap().is_refresh_valid() {
            return self.request_token();
        }

        let data = RefreshTokenData {
            client_id: &self.token_data.client_id,
            client_secret: &self.token_data.client_secret,
            refresh_token: &self.token.as_ref().unwrap().refresh_token,
            grant_type: "refresh".to_string(),
        };

        let response = self.client.post(TOKEN_URL).json(&data).send()?;

        if !response.status().is_success() {
            return Err(RequestError::Gfycat(response.json::<GfycatError>()?));
        }

        let mut token: Token = response.json()?;
        token.request_timestamp = Some(Instant::now());

        Ok(token)
    }

    fn request_album(&self) -> Result<AlbumResponse, reqwest::Error> {
        Ok(self
            .client
            .get(&format!(
                "https://api.gfycat.com/v1/me/albums/{}",
                self.album_id
            ))
            .header(
                "Authorization",
                format!("Bearer {}", self.token.as_ref().unwrap().access_token),
            )
            .send()?
            .json::<AlbumResponse>()?)
    }
}

impl Token {
    fn is_valid(&self) -> bool {
        // Add 5 secs since the request also has to take some time before it reaches the API
        self.request_timestamp.unwrap().elapsed().as_secs() + 5 < self.expires_in as u64
    }

    fn is_refresh_valid(&self) -> bool {
        self.request_timestamp.unwrap().elapsed().as_secs() + 5
            < self.refresh_token_expires_in as u64
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
