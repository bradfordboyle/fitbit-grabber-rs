extern crate chrono;
extern crate clap;
extern crate fitbit_grabber;
extern crate serde;
extern crate serde_json;
extern crate url;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use chrono::NaiveDate;
use clap::{App, Arg, SubCommand};
use fitbit_grabber::{FitbitAuth, FitbitClient, Token};

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

fn save_token(filename: &str, token: Token) -> Result<(), String> {
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
