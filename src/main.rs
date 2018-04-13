extern crate chrono;
extern crate clap;
extern crate oauth2;
extern crate reqwest;
extern crate tiny_http;
extern crate url;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use chrono::NaiveDate;
use clap::{App, Arg, SubCommand};
use oauth2::{AuthType, Config};
use reqwest::header::{Authorization, Bearer, Headers};
use reqwest::Method;

struct FitbitClient {
    client: reqwest::Client,
    base: url::Url,
    user_agent: String,
}

impl FitbitClient {
    pub fn new(token: String) -> FitbitClient {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer { token: token }));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Unable to build HTTP client");

        FitbitClient {
            client: client,
            base: url::Url::parse("https://api.fitbit.com/1/").unwrap(),
            user_agent: "fitbit-grabber-rs (0.1.0)".to_string(),
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
        .get_matches();

    let token = load_token(".token");
    let f = FitbitClient::new(token);

    if let Some(matches) = matches.subcommand_matches("heart") {
        let date = matches
            .value_of("date")
            .ok_or("please give a starting date".to_string())
            .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(stringify))
            .unwrap();
        println!("{}", f.heart(date).unwrap());
    }

    if let Some(matches) = matches.subcommand_matches("step") {
        let date = matches
            .value_of("date")
            .ok_or("please give a starting date".to_string())
            .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(stringify))
            .unwrap();
        println!("{}", f.step(date).unwrap());
    }

    if let Some(_) = matches.subcommand_matches("token") {
        let fitbit_client_id = env::var("FITBIT_CLIENT_ID")
            .expect("Missing the FITBIT_CLIENT_ID environment variable.");
        let fitbit_client_secret = env::var("FITBIT_CLIENT_SECRET")
            .expect("Missing the FITBIT_CLIENT_SECRET environment variable.");

        let token = get_token(&fitbit_client_id, &fitbit_client_secret).unwrap();
        // TODO save the token as JSON
        println!("{}", token.access_token);
    }
}

fn load_token(filename: &str) -> String {
    let mut f = File::open(filename).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("unable to read file");

    contents.trim().to_string()
}

fn new_load_token() -> String {
    let fitbit_client_id =
        env::var("FITBIT_CLIENT_ID").expect("Missing the FITBIT_CLIENT_ID environment variable.");
    let fitbit_client_secret = env::var("FITBIT_CLIENT_SECRET")
        .expect("Missing the FITBIT_CLIENT_SECRET environment variable.");

    let token = get_token(&fitbit_client_id, &fitbit_client_secret).unwrap();
    println!("Token: {:?}", token);
    token.access_token
}

fn get_token(client_id: &str, client_secret: &str) -> Result<oauth2::Token, String> {
    let auth_url = "https://www.fitbit.com/oauth2/authorize";
    let token_url = "https://api.fitbit.com/oauth2/token";

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

    // Generate the authorization URL to which we'll redirect the user.
    let authorize_url = config.authorize_url();

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
    config.exchange_code(code).map_err(stringify)
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
