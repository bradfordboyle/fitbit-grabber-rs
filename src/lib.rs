extern crate chrono;
extern crate oauth2;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tiny_http;
extern crate url;

use std::convert;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use oauth2::{AuthType, Config};
use reqwest::header::{Authorization, Bearer, Headers, UserAgent};
use reqwest::{Client, Method};

pub use self::user::UserService;

pub mod date;
mod user;

#[derive(Debug)]
pub enum FitbitError {
    DateParse(chrono::ParseError),
    UrlParseError(url::ParseError),
    ReqwestError(reqwest::Error),
    IoError(io::Error),
    TokenError(oauth2::TokenError),
    JsonError(serde_json::Error),
    Other(String),
}

impl Error for FitbitError {
    fn description(&self) -> &str {
        "Something bad happened"
    }
}

impl fmt::Display for FitbitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, somthing bad went down")
    }
}

impl convert::From<url::ParseError> for FitbitError {
    fn from(err: url::ParseError) -> Self {
        FitbitError::UrlParseError(err)
    }
}

impl convert::From<reqwest::Error> for FitbitError {
    fn from(err: reqwest::Error) -> Self {
        FitbitError::ReqwestError(err)
    }
}

impl convert::From<io::Error> for FitbitError {
    fn from(err: io::Error) -> Self {
        FitbitError::IoError(err)
    }
}

impl convert::From<oauth2::TokenError> for FitbitError {
    fn from(err: oauth2::TokenError) -> Self {
        FitbitError::TokenError(err)
    }
}

impl convert::From<serde_json::Error> for FitbitError {
    fn from(err: serde_json::Error) -> Self {
        FitbitError::JsonError(err)
    }
}

impl convert::From<chrono::ParseError> for FitbitError {
    fn from(err: chrono::ParseError) -> Self {
        FitbitError::DateParse(err)
    }
}

type Result<T> = std::result::Result<T, FitbitError>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Token(oauth2::Token);

impl Token {
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string(self).unwrap();
        File::create(&path).and_then(|mut file| file.write_all(json.as_bytes()))?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Token> {
        let mut f = File::open(path)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        let token = serde_json::from_str::<Token>(contents.trim())?;

        Ok(token)
    }
}

pub struct FitbitClient {
    client: Client,
    base: url::Url,
}

impl FitbitClient {
    pub fn new(token: Token) -> FitbitClient {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer {
            token: token.0.access_token.to_string(),
        }));
        headers.set(UserAgent::new("fitbit-grabber-rs (0.1.0)"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Unable to build HTTP client");

        FitbitClient {
            client: client,
            base: url::Url::parse("https://api.fitbit.com/").unwrap(),
        }
    }

    pub fn heart(&self, date: &date::Date) -> Result<String> {
        let path = format!("1/user/-/activities/heart/date/{}/1d.json", date);
        self.do_get(&path)
    }

    pub fn step(&self, date: &date::Date) -> Result<String> {
        let path = format!("1/user/-/activities/steps/date/{}/1d.json", date);
        self.do_get(&path)
    }

    pub fn daily_activity_summary(&self, user_id: &str, date: &date::Date) -> Result<String> {
        let path = format!("1/user/{}/activities/date/{}.json", user_id, date);
        self.do_get(&path)
    }

    pub fn get_devices(&self) -> Result<String> {
        let path = format!("1/user/-/devices.json");
        self.do_get(&path)
    }

    pub fn get_alarms(&self, user_id: &str, tracker_id: &str) -> Result<String> {
        let path = format!(
            "1/user/{}/devices/tracker/{}/alarms.json",
            user_id, tracker_id
        );
        self.do_get(&path)
    }

    pub fn get_sleep_logs_for_date(&self, date: &date::Date) -> Result<String> {
        let path = format!("1.2/user/-/sleep/date/{}.json", date);
        self.do_get(&path)
    }

    pub fn get_sleep_logs_list(&self) -> Result<String> {
        let path = format!("1.2/user/-/sleep/list.json");
        self.do_get(&path)
    }

    fn do_get(&self, path: &str) -> Result<String> {
        let url = self.base.join(&path)?;
        self.client
            .request(Method::Get, url)
            .send()
            .and_then(|mut r| r.text())
            .map_err(convert::From::from)
    }
}

pub struct FitbitAuth(oauth2::Config);

impl FitbitAuth {
    pub fn new(client_id: &str, client_secret: &str) -> FitbitAuth {
        let auth_url = "https://www.fitbit.com/oauth2/authorize";
        let token_url = "https://api.fitbit.com/oauth2/token";
        // let token_url = "http://localhost:8080";

        // Set up the config for the Github OAuth2 process.
        let mut config = Config::new(client_id, client_secret, auth_url, token_url);

        // config = config.set_response_type(ResponseType::Token);
        config = config.set_auth_type(AuthType::BasicAuth);

        // This example is requesting access to the user's public repos and email.
        config = config
            .add_scope("activity")
            .add_scope("heartrate")
            .add_scope("location")
            .add_scope("nutrition")
            .add_scope("profile")
            .add_scope("settings")
            .add_scope("sleep")
            .add_scope("social")
            .add_scope("weight");

        // This example will be running its own server at localhost:8080.
        // See below for the server implementation.
        config = config.set_redirect_url("http://localhost:8080");

        FitbitAuth(config)
    }

    pub fn get_token(&self) -> Result<Token> {
        let authorize_url = self.0.authorize_url();

        println!(
            "Open this URL in your browser:\n{}\n",
            authorize_url.to_string()
        );

        // FIXME avoid unwrap here
        let server = tiny_http::Server::http("localhost:8080").unwrap();
        let request = server.recv()?;
        let url = request.url().to_string();
        let response = tiny_http::Response::from_string("Go back to your terminal :)");
        request.respond(response)?;

        let code = {
            // remove leading '/?'
            let mut parsed = url::form_urlencoded::parse(url[2..].as_bytes());

            let (_, value) = parsed
                .find(|pair| {
                    let &(ref key, _) = pair;
                    key == "code"
                })
                .ok_or(FitbitError::Other(
                    "query param `code` not found".to_string(),
                ))?;
            value.to_string()
        };

        // Exchange the code with a token.
        self.0
            .exchange_code(code)
            .map(Token)
            .map_err(convert::From::from)
    }

    pub fn exchange_refresh_token(&self, token: Token) -> Result<Token> {
        match token.0.refresh_token {
            Some(t) => self.0
                .exchange_refresh_token(t)
                .map(Token)
                .map_err(convert::From::from),
            None => Err(FitbitError::Other("no refresh token found".to_string())),
        }
    }
}
