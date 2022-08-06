use rand::{seq::SliceRandom, thread_rng};
use reqwest::Client as ReqwestClient;
use serde::{
    ser::{Serialize, SerializeStruct, Serializer},
    Deserialize, Serialize as SerializeDerive,
};
use std::error::Error;
use std::fmt;
use std::time::Instant;
use tracing::debug;

const TOKEN_URL: &str = "https://api.gfycat.com/v1/oauth/token";

#[derive(SerializeDerive, PartialEq)]
#[allow(dead_code)]
pub enum GrantType {
    #[serde(rename = "password")]
    Password,
    #[serde(rename = "client_credentials")]
    ClientCredentials,
    #[serde(rename = "refresh")]
    Refresh,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct GfycatError {
    errorMessage: GfycatErrorMessage,
}

#[derive(Deserialize, Debug)]
struct GfycatErrorMessage {
    code: String,
    description: String,
}

impl fmt::Display for GfycatErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error {}: {}", self.code, self.description)
    }
}

#[derive(Debug)]
pub enum RequestError {
    Gfycat(GfycatError),
    Reqwest(reqwest::Error),
}

impl Error for RequestError {}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let error = match self {
            RequestError::Gfycat(error) => error.errorMessage.to_string(),
            RequestError::Reqwest(error) => error.to_string(),
        };
        write!(f, "{}", error)
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(error: reqwest::Error) -> Self {
        Self::Reqwest(error)
    }
}

/*impl From<RequestError> for CommandError {
    fn from(error: RequestError) -> Self {
        match error {
            RequestError::Gfycat(error) => {
                error.errorMessage
            },
            RequestError::Reqwest(error) => {
                error.into()
            }
        }
    }
}*/

pub struct TokenData {
    client_id: String,
    client_secret: String,
    username: Option<String>,
    password: Option<String>,
    grant_type: GrantType,
}

impl Serialize for TokenData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = match self.grant_type {
            GrantType::Password => serializer.serialize_struct("TokenData", 5),
            GrantType::ClientCredentials => serializer.serialize_struct("TokenData", 3),
            GrantType::Refresh => serializer.serialize_struct("TokenData", 4),
        }?;

        state.serialize_field("client_id", &self.client_id)?;
        state.serialize_field("client_secret", &self.client_secret)?;
        if self.grant_type == GrantType::Password {
            state.serialize_field("username", &self.username)?;
            state.serialize_field("password", &self.password)?;
        }
        state.serialize_field("grant_type", &self.grant_type)?;
        state.end()
    }
}

#[derive(SerializeDerive)]
pub struct RefreshTokenData<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    refresh_token: &'a str,
    grant_type: GrantType,
}

#[derive(Debug)]
pub enum ClientBuilderError {
    GrantMissingError,
}

impl Error for ClientBuilderError {}

impl fmt::Display for ClientBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientBuilderError::GrantMissingError => write!(f, "Grant Missing Error"),
        }
    }
}

pub struct ClientBuilder {
    client_id: String,
    client_secret: String,
    username: Option<String>,
    password: Option<String>,
    grant_type: Option<GrantType>,
    album_id: String,
}

impl ClientBuilder {
    pub fn new(client_id: String, client_secret: String, album_id: String) -> Self {
        Self {
            client_id,
            client_secret,
            username: Option::default(),
            password: Option::default(),
            grant_type: Option::default(),
            album_id,
        }
    }

    #[allow(dead_code)]
    pub fn client_credentials_grant(mut self) -> Self {
        self.username = None;
        self.password = None;
        self.grant_type = Some(GrantType::ClientCredentials);
        self
    }

    pub fn password_grant(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self.grant_type = Some(GrantType::Password);
        self
    }

    pub fn build(self) -> Result<Client, ClientBuilderError> {
        if self.grant_type.is_none() {
            return Err(ClientBuilderError::GrantMissingError);
        }

        let grant_type = self.grant_type.unwrap();

        let token_data = match grant_type {
            GrantType::Password => TokenData {
                client_id: self.client_id,
                client_secret: self.client_secret,
                username: self.username,
                password: self.password,
                grant_type,
            },
            GrantType::ClientCredentials => TokenData {
                client_id: self.client_id,
                client_secret: self.client_secret,
                username: None,
                password: None,
                grant_type,
            },
            _ => panic!("Unknown grant type"),
        };

        Ok(Client {
            token: None,
            token_data,
            client: ReqwestClient::new(),
            album_id: self.album_id,
            gfycats: None,
            time_since_last_request: None,
        })
    }
}

pub struct Client {
    token: Option<Token>,
    token_data: TokenData,
    client: ReqwestClient,
    album_id: String,
    gfycats: Option<GfycatCollection>,
    /// Time since last request
    time_since_last_request: Option<Instant>,
}

impl Client {
    pub async fn random_video(&mut self) -> Result<String, RequestError> {
        // Cache is newer than last 24h
        if self.gfycats.is_some()
            && self.time_since_last_request.unwrap().elapsed().as_secs() < 60 * 60 * 24
        {
            let collection = self.gfycats.as_ref().unwrap();
            let gfycat = collection.pick_random().unwrap();
            return Ok(format!("https://gfycat.com/{}", gfycat.gfyId));
        }

        if self.token.is_none() {
            self.token = Some(self.request_token().await?);
        }

        let token = self.token.as_ref().unwrap();
        if !token.is_valid() {
            self.token = Some(self.refresh_token().await?);
        }

        let response = self.request_album().await?;
        self.time_since_last_request = Some(Instant::now());
        self.gfycats = Some(response.publishedGfys);

        let collection = self.gfycats.as_ref().unwrap();
        let gfycat = collection.pick_random().unwrap();

        Ok(format!("https://gfycat.com/{}", gfycat.gfyId))
    }

    async fn request_token(&self) -> Result<Token, RequestError> {
        debug!("Requesting new gfycat token");
        let response = self
            .client
            .post(TOKEN_URL)
            .json(&self.token_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RequestError::Gfycat(response.json::<GfycatError>().await?));
        }
        let mut token: Token = response.json().await?;
        token.time_since_request = Some(Instant::now());

        Ok(token)
    }

    async fn refresh_token(&self) -> Result<Token, RequestError> {
        debug!("Refreshing gfycat token");
        if self.token.is_none() {
            return self.request_token().await;
        }

        if !self.token.as_ref().unwrap().is_refresh_valid() {
            return self.request_token().await;
        }

        let data = RefreshTokenData {
            client_id: &self.token_data.client_id,
            client_secret: &self.token_data.client_secret,
            refresh_token: &self.token.as_ref().unwrap().refresh_token,
            grant_type: GrantType::Refresh,
        };

        let response = self.client.post(TOKEN_URL).json(&data).send().await?;

        if !response.status().is_success() {
            return Err(RequestError::Gfycat(response.json::<GfycatError>().await?));
        }

        let mut token: Token = response.json().await?;
        token.time_since_request = Some(Instant::now());

        Ok(token)
    }

    async fn request_album(&self) -> Result<AlbumResponse, reqwest::Error> {
        self.client
            .get(&format!(
                "https://api.gfycat.com/v1/me/albums/{}",
                self.album_id
            ))
            .header(
                "Authorization",
                format!("Bearer {}", self.token.as_ref().unwrap().access_token),
            )
            .send()
            .await?
            .json::<AlbumResponse>()
            .await
    }
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
    time_since_request: Option<Instant>,
}

impl Token {
    fn is_valid(&self) -> bool {
        // Add 5 secs since the request also has to take some time before it reaches the API
        self.time_since_request.unwrap().elapsed().as_secs() + 5 < self.expires_in as u64
    }

    fn is_refresh_valid(&self) -> bool {
        self.time_since_request.unwrap().elapsed().as_secs() + 5
            < self.refresh_token_expires_in as u64
    }
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
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
    likes: Option<String>,
    dislikes: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    mod token_data {
        use super::{GrantType, TokenData};

        #[test]
        fn serialize_password_grant() {
            let data = TokenData {
                client_id: "foo".to_string(),
                client_secret: "bar".to_string(),
                username: Some("baz".to_string()),
                password: Some("bas".to_string()),
                grant_type: GrantType::Password,
            };

            let json = serde_json::to_string(&data);
            assert!(json.is_ok());
            let json = json.unwrap();
            assert_eq!(
                json,
                r#"{"client_id":"foo","client_secret":"bar","username":"baz","password":"bas","grant_type":"password"}"#
            );
        }

        #[test]
        fn serialize_client_credentials_grant() {
            let data = TokenData {
                client_id: "foo".to_string(),
                client_secret: "bar".to_string(),
                username: None,
                password: None,
                grant_type: GrantType::ClientCredentials,
            };

            let json = serde_json::to_string(&data);
            assert!(json.is_ok());
            let json = json.unwrap();
            assert_eq!(
                json,
                r#"{"client_id":"foo","client_secret":"bar","grant_type":"client_credentials"}"#
            );
        }
    }

    #[test]
    fn serialize_refresh_token_data() {
        let data = RefreshTokenData {
            client_id: "foo",
            client_secret: "bar",
            refresh_token: "baz",
            grant_type: GrantType::Refresh,
        };

        let json = serde_json::to_string(&data);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert_eq!(
            json,
            r#"{"client_id":"foo","client_secret":"bar","refresh_token":"baz","grant_type":"refresh"}"#
        );
    }
}
