extern crate chrono;
#[macro_use]
extern crate log;
extern crate oauth2;
extern crate reqwest;
extern crate url;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate tiny_http;
#[macro_use]
extern crate failure;

use chrono::NaiveDate;
use oauth2::{AuthType, Config as OAuth2Config};
use reqwest::header::{Authorization, Bearer, Headers, UserAgent};
use reqwest::Method;

// TODO: how to re-export public names?
pub mod activities;
pub mod body;
pub mod date;
pub mod errors;
pub mod query;
pub mod serializers;
pub mod user;

use errors::Error;

type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Token(oauth2::Token);

pub struct FitbitClient {
    client: reqwest::Client,
    base: url::Url,
}

impl FitbitClient {
    pub fn new(token: Token) -> Result<FitbitClient> {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer {
            token: token.0.access_token,
        }));
        headers.set(UserAgent::new("fitbit-rs (0.1.0)"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| Error::Http(e))?;

        Ok(FitbitClient {
            client: client,
            base: url::Url::parse("https://api.fitbit.com/1/").unwrap(),
        })
    }

    pub fn user(&self) -> Result<String> {
        let url = self
            .base
            .join("user/-/profile.json")
            .map_err(|e| Error::Url(e))?;
        Ok(self
            .client
            .request(reqwest::Method::Get, url)
            .send()
            .and_then(|mut r| r.text())?)
    }

    pub fn heart(&self, date: NaiveDate) -> Result<String> {
        let path = format!(
            "user/-/activities/heart/date/{}/1d.json",
            date.format("%Y-%m-%d")
        );
        let url = self.base.join(&path).map_err(|e| Error::Url(e))?;
        self.client
            .request(Method::Get, url)
            .send()
            .and_then(|mut r| r.text())
            .map_err(|e| Error::Http(e))
    }

    pub fn step(&self, date: NaiveDate) -> Result<String> {
        let path = format!(
            "user/-/activities/steps/date/{}/1d.json",
            date.format("%Y-%m-%d")
        );
        let url = self.base.join(&path).map_err(|e| Error::Url(e))?;
        Ok(self
            .client
            .request(Method::Get, url)
            .send()
            .and_then(|mut r| r.text())
            .map_err(|e| Error::Http(e))?)
    }

    fn do_get(&self, path: &str) -> Result<String> {
        let url = self.base.join(&path)?;
        debug!("GET - {:?}", url);
        Ok(self.client.get(url).send()?.text()?)
    }
}

pub struct FitbitAuth(OAuth2Config);

impl FitbitAuth {
    pub fn new(client_id: &str, client_secret: &str) -> FitbitAuth {
        let auth_url = "https://www.fitbit.com/oauth2/authorize";
        let token_url = "https://api.fitbit.com/oauth2/token";

        // Set up the config for the Github OAuth2 process.
        let mut config = OAuth2Config::new(client_id, client_secret, auth_url, token_url);

        // config = config.set_response_type(ResponseType::Token);
        config = config.set_auth_type(AuthType::BasicAuth);

        // This example is requesting access to the user's public repos and email.
        config = config.add_scope("activity");
        config = config.add_scope("heartrate");
        config = config.add_scope("profile");
        config = config.add_scope("weight");

        // This example will be running its own server at localhost:8080.
        // See below for the server implementation.
        // TODO configurable redirect?
        config = config.set_redirect_url("http://localhost:8080");

        FitbitAuth(config)
    }

    pub fn get_token(&self) -> Result<oauth2::Token> {
        let authorize_url = self.0.authorize_url();

        use std::process::Command;

        #[cfg(target_os = "linux")]
        {
            let mut cmd = Command::new("xdg-open");
            cmd.arg(authorize_url.as_str());
            let mut child = cmd.spawn()?;
            child.wait()?;
        }

        #[cfg(target_os = "macos")]
        {
            let mut cmd = Command::new("open");
            cmd.arg(authorize_url.as_str());
            let mut child = cmd.spawn()?;
            child.wait()?;
        }

        println!(
            "Your browser should open automatically. If not, open this URL in your browser:\n{}\n",
            authorize_url.to_string()
        );

        // FIXME avoid unwrap here
        let server =
            tiny_http::Server::http("localhost:8080").expect("could not start http listener");
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
                .ok_or(Error::OAuthCodeMissing)?;
            value.to_string()
        };

        // Exchange the code with a token.
        self.0.exchange_code(code).map_err(|e| Error::AuthToken(e))
    }

    pub fn exchange_refresh_token(&self, token: Token) -> Result<oauth2::Token> {
        match token.0.refresh_token {
            Some(t) => self
                .0
                .exchange_refresh_token(t)
                .map_err(|e| Error::AuthToken(e)),
            None => Err(Error::RefreshTokenMissing),
        }
    }
}
