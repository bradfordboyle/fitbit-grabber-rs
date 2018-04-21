extern crate chrono;
extern crate clap;
extern crate oauth2;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate tiny_http;
extern crate url;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use chrono::NaiveDate;
use clap::{App, Arg, SubCommand};
use oauth2::{AuthType, Config};
use reqwest::header::{Authorization, Bearer, Headers, UserAgent};
use reqwest::Method;

#[derive(Serialize, Deserialize, Debug)]
struct Token(oauth2::Token);

struct FitbitClient {
    client: reqwest::Client,
    base: url::Url,
}

impl FitbitClient {
    pub fn new(token: Token) -> FitbitClient {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer {
            token: token.0.access_token.to_string(),
        }));
        headers.set(UserAgent::new("fitbit-grabber-rs (0.1.0)"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Unable to build HTTP client");

        FitbitClient {
            client: client,
            base: url::Url::parse("https://api.fitbit.com/1/").unwrap(),
        }
    }

    pub fn user(&self) -> Result<String, String> {
        let url = self.base.join("user/-/profile.json").map_err(stringify)?;
        self.client
            .request(reqwest::Method::Get, url)
            .send()
            .and_then(|mut r| r.text())
            .map_err(stringify)
    }

    pub fn heart(&self, date: NaiveDate) -> Result<String, String> {
        let path = format!(
            "user/-/activities/heart/date/{}/1d.json",
            date.format("%Y-%m-%d")
        );
        let url = self.base.join(&path).map_err(stringify)?;
        self.client
            .request(Method::Get, url)
            .send()
            .and_then(|mut r| r.text())
            .map_err(stringify)
    }

    pub fn step(&self, date: NaiveDate) -> Result<String, String> {
        let path = format!(
            "user/-/activities/steps/date/{}/1d.json",
            date.format("%Y-%m-%d")
        );
        let url = self.base.join(&path).map_err(stringify)?;
        self.client
            .request(Method::Get, url)
            .send()
            .and_then(|mut r| r.text())
            .map_err(stringify)
    }

    pub fn daily_activity_summary(&self, user_id: &str, date: NaiveDate) -> Result<String, String> {
        let path = format!("user/{}/activities/date/{}.json", user_id, date);
        self.do_get(&path)
    }

    fn do_get(&self, path: &str) -> Result<String, String> {
        let url = self.base.join(&path).map_err(stringify)?;
        self.client
            .request(Method::Get, url)
            .send()
            .and_then(|mut r| r.text())
            .map_err(stringify)
    }
}

struct FitbitAuth(oauth2::Config);

impl FitbitAuth {
    fn new(client_id: &str, client_secret: &str) -> FitbitAuth {
        let auth_url = "https://www.fitbit.com/oauth2/authorize";
        let token_url = "https://api.fitbit.com/oauth2/token";
        // let token_url = "http://localhost:8080";

        // Set up the config for the Github OAuth2 process.
        let mut config = Config::new(client_id, client_secret, auth_url, token_url);

        // config = config.set_response_type(ResponseType::Token);
        config = config.set_auth_type(AuthType::BasicAuth);

        // This example is requesting access to the user's public repos and email.
        config = config.add_scope("activity");
        config = config.add_scope("heartrate");
        config = config.add_scope("profile");

        // This example will be running its own server at localhost:8080.
        // See below for the server implementation.
        config = config.set_redirect_url("http://localhost:8080");

        FitbitAuth(config)
    }

    fn get_token(&self) -> Result<oauth2::Token, String> {
        let authorize_url = self.0.authorize_url();

        println!(
            "Open this URL in your browser:\n{}\n",
            authorize_url.to_string()
        );

        // FIXME avoid unwrap here
        let server = tiny_http::Server::http("localhost:8080").unwrap();
        let request = server.recv().map_err(stringify)?;
        let url = request.url().to_string();
        let response = tiny_http::Response::from_string("Go back to your terminal :)");
        request.respond(response).map_err(stringify)?;

        let code = {
            // remove leading '/?'
            let mut parsed = url::form_urlencoded::parse(url[2..].as_bytes());

            let (_, value) = parsed
                .find(|pair| {
                    let &(ref key, _) = pair;
                    key == "code"
                })
                .ok_or("query param `code` not found")?;
            value.to_string()
        };

        // Exchange the code with a token.
        self.0.exchange_code(code).map_err(stringify)
    }

    fn exchange_refresh_token(&self, token: Token) -> Result<oauth2::Token, String> {
        match token.0.refresh_token {
            Some(t) => self.0.exchange_refresh_token(t).map_err(stringify),
            None => Err("No refresh token available".to_string()),
        }
    }
}

fn main() {
    let matches = App::new("Fitbit Grabber")
        .subcommand(
            SubCommand::with_name("heart")
                .about("fetch heart data")
                .arg(
                    Arg::with_name("date")
                        .long("date")
                        .required(true)
                        .takes_value(true)
                        .help("date to fetch data for"),
                ),
        )
        .subcommand(
            SubCommand::with_name("step").about("fetch step data").arg(
                Arg::with_name("date")
                    .long("date")
                    .required(true)
                    .takes_value(true)
                    .help("date to fetch data for"),
            ),
        )
        .subcommand(SubCommand::with_name("token").about("request an access token"))
        .subcommand(SubCommand::with_name("refresh-token").about("refresh token"))
        .subcommand(SubCommand::with_name("user").about("get user profile"))
        .subcommand(
            SubCommand::with_name("daily-activity-summary")
                .about("get user profile")
                .arg(
                    Arg::with_name("user-id")
                        .long("user")
                        .required(false)
                        .takes_value(true)
                        .help("user if to fetch summary for"),
                )
                .arg(
                    Arg::with_name("date")
                        .long("date")
                        .required(true)
                        .takes_value(true)
                        .help("date to fetch summary for"),
                ),
        )
        .get_matches();

    let auth = get_auth_from_env();
    let token = load_token(".token");

    match matches.subcommand() {
        ("token", Some(_)) => {
            auth.get_token()
                .and_then(|token| save_token(".token", token))
                .expect("unable to obtain token");
        }
        ("refresh-token", Some(_)) => {
            token
                .and_then(|token| auth.exchange_refresh_token(token))
                .and_then(|token| save_token(".token", token))
                .expect("unable to refresh token");
        }
        ("heart", Some(sub_m)) => {
            let client = token
                .map(|t| FitbitClient::new(t))
                .expect("unable to create Fitbit client");
            let heart_rate_data = sub_m
                .value_of("date")
                .ok_or("please give a starting date".to_string())
                .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(stringify))
                .and_then(|date| client.heart(date))
                .expect("unable to fetch heart rate data for given date");
            println!("{}", heart_rate_data);
        }
        ("step", Some(sub_m)) => {
            let client = token
                .map(|t| FitbitClient::new(t))
                .expect("unable to create Fitbit client");
            let step_data = sub_m
                .value_of("date")
                .ok_or("please give a starting date".to_string())
                .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(stringify))
                .and_then(|date| client.step(date))
                .expect("unable to fetch step data for given date");
            println!("{}", step_data);
        }
        ("user", Some(_)) => {
            let client = token
                .map(|t| FitbitClient::new(t))
                .expect("unable to create Fitbit client");
            let user_profile = client.user().expect("unable to fetch user profile");
            println!("{}", user_profile);
        }
        ("daily-activity-summary", Some(sub_m)) => {
            let client = token
                .map(|t| FitbitClient::new(t))
                .expect("unable to create Fitbit client");
            let summary = sub_m
                .value_of("date")
                .ok_or("please give a starting date".to_string())
                .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(stringify))
                .and_then(|date| client.daily_activity_summary("-", date))
                .expect("unable to fetch summary for given date");
            println!("{}", summary);
        }
        (cmd, _) => {
            panic!("Unknown command: {}", cmd);
        }
    }
}

fn get_auth_from_env() -> FitbitAuth {
    let fitbit_client_id =
        env::var("FITBIT_CLIENT_ID").expect("Missing the FITBIT_CLIENT_ID environment variable.");
    let fitbit_client_secret = env::var("FITBIT_CLIENT_SECRET")
        .expect("Missing the FITBIT_CLIENT_SECRET environment variable.");
    FitbitAuth::new(&fitbit_client_id, &fitbit_client_secret)
}

fn save_token(filename: &str, token: oauth2::Token) -> Result<(), String> {
    let json = serde_json::to_string(&token).unwrap();
    let path = Path::new(filename);

    File::create(&path)
        .and_then(|mut file| file.write_all(json.as_bytes()))
        .map_err(stringify)
}

fn load_token(filename: &str) -> Result<Token, String> {
    let mut f = File::open(filename).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("unable to read file");

    serde_json::from_str::<Token>(contents.trim()).map_err(stringify)
}

fn stringify<E: Error>(e: E) -> String {
    format!("{}", e)
}

#[cfg(test)]
mod tests {
    use DateRange;

    use chrono::{NaiveDate, Utc};

    #[test]
    fn daterange() {
        let d = DateRange::new("2017-09-01", "2017-09-30").unwrap();
        assert_eq!(d.start, NaiveDate::from_ymd(2017, 9, 1));
        assert_eq!(d.end, NaiveDate::from_ymd(2017, 9, 30));
    }

    #[test]
    fn daterange_from() {
        let d = DateRange::from(NaiveDate::from_ymd(2017, 9, 1));
        assert_eq!(d.start, NaiveDate::from_ymd(2017, 9, 1));
        assert_eq!(d.end, Utc::today().naive_utc());
    }

    #[test]
    fn daterange_iter() {
        let d = DateRange::new("2017-09-01", "2017-09-03").unwrap();
        let dates: Vec<NaiveDate> = d.collect();
        assert_eq!(
            dates,
            vec![
                NaiveDate::from_ymd(2017, 9, 1),
                NaiveDate::from_ymd(2017, 9, 2),
                NaiveDate::from_ymd(2017, 9, 3),
            ]
        )
    }
}
