extern crate chrono;
extern crate clap;
extern crate oauth2;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate tiny_http;
extern crate url;
extern crate fitbit;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use chrono::NaiveDate;
use clap::{App, Arg, SubCommand};

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
        // TODO body weight command
        .subcommand(SubCommand::with_name("token").about("request an access token"))
        .subcommand(SubCommand::with_name("refresh-token").about("refresh token"))
        .subcommand(SubCommand::with_name("user").about("get user profile"))
        .get_matches();

    let fitbit_client_id =
        env::var("FITBIT_CLIENT_ID").expect("Missing the FITBIT_CLIENT_ID environment variable.");
    let fitbit_client_secret = env::var("FITBIT_CLIENT_SECRET")
        .expect("Missing the FITBIT_CLIENT_SECRET environment variable.");
    let auth = fitbit::FitbitAuth::new(&fitbit_client_id, &fitbit_client_secret);

    if let Some(_) = matches.subcommand_matches("token") {
        auth.get_token()
            .and_then(|token| save_token(".token", token))
            .expect("unable to obtain token");
    }

    if let Some(_) = matches.subcommand_matches("refresh-token") {
        load_token(".token")
            .and_then(|token| auth.exchange_refresh_token(token))
            .and_then(|token| save_token(".token", token))
            .expect("unable to refresh token");
    }

    let token = load_token(".token").unwrap();
    let f = fitbit::FitbitClient::new(token);

    if let Some(matches) = matches.subcommand_matches("heart") {
        let heart_rate_data = matches
            .value_of("date")
            .ok_or("please give a starting date".to_string())
            .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(stringify))
            .and_then(|date| f.heart(date))
            .expect("unable to fetch heart rate data for given date");
        println!("{}", heart_rate_data);
    }

    if let Some(matches) = matches.subcommand_matches("step") {
        let step_data = matches
            .value_of("date")
            .ok_or("please give a starting date".to_string())
            .and_then(|arg| NaiveDate::parse_from_str(&arg, "%Y-%m-%d").map_err(stringify))
            .and_then(|date| f.step(date))
            .expect("unable to fetch step data for given date");
        println!("{}", step_data);
    }

    if let Some(_) = matches.subcommand_matches("user") {
        let user_profile = f.user().expect("unable to fetch user profile");
        println!("{}", user_profile);
    }
}

fn save_token(filename: &str, token: oauth2::Token) -> Result<(), String> {
    let json = serde_json::to_string(&token).unwrap();
    let path = Path::new(filename);

    File::create(&path)
        .and_then(|mut file| file.write_all(json.as_bytes()))
        .map_err(stringify)
}

fn load_token(filename: &str) -> Result<fitbit::Token, String> {
    let mut f = File::open(filename).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("unable to read file");

    serde_json::from_str::<fitbit::Token>(contents.trim()).map_err(stringify)
}

fn stringify<E: Error>(e: E) -> String {
    format!("{}", e)
}

