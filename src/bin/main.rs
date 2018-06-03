extern crate chrono;
extern crate clap;
extern crate fitbit;
extern crate oauth2;
extern crate reqwest;
extern crate serde_json;
extern crate tiny_http;
extern crate toml;
extern crate url;
#[macro_use]
extern crate failure;

extern crate serde;
#[macro_use]
extern crate serde_derive;

use failure::Error;

use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use chrono::NaiveDate;
use clap::{App, Arg, SubCommand};

mod config;
use config::Config;

fn main() -> Result<(), Error> {
    //    let home_dir = ;
    let default_config = Path::new(&env::var("HOME")?).join(".config/fitbit-grabber/conf.toml");

    let matches = App::new("Fitbit Grabber")
        .arg(
            Arg::with_name("config")
            .help("path to config file")
            .short("c")
            .long("config")
            .default_value(default_config.to_str().unwrap())
            )
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

    let conf = Config::load(matches.value_of("config"))?;
    let config::FitbitConfig {
        client_id,
        client_secret,
    } = conf.fitbit.unwrap();

    let auth = fitbit::FitbitAuth::new(&client_id.unwrap(), &client_secret.unwrap());

    if let Some(_) = matches.subcommand_matches("token") {
        let token = auth.get_token()?;
        save_token(".token", token)?;
    }

    if let Some(_) = matches.subcommand_matches("refresh-token") {
        let token = load_token(".token")?;
        let exchanged = auth.exchange_refresh_token(token)?;
        save_token(".token", exchanged)?;
    }

    let token = load_token(".token").unwrap();
    let f = fitbit::FitbitClient::new(token)?;

    if let Some(matches) = matches.subcommand_matches("heart") {
        let raw_date = matches
            .value_of("date")
            .ok_or(format_err!("please give a starting date"))?;
        let date = NaiveDate::parse_from_str(&raw_date, "%Y-%m-%d")?;
        let heart_rate_data = f.heart(date)?;
        println!("{}", heart_rate_data);
    }

    if let Some(matches) = matches.subcommand_matches("step") {
        let raw_date = matches
            .value_of("date")
            .ok_or(format_err!("please give a starting date"))?;
        let date = NaiveDate::parse_from_str(&raw_date, "%Y-%m-%d")
            .map_err(|e| format_err!("could not parse date {}", e))?;
        let step_data = f.step(date)?;
        println!("{}", step_data);
    }

    if let Some(_) = matches.subcommand_matches("user") {
        let user_profile = f.user()?;
        println!("{}", user_profile);
    }

    Ok(()) // ok!
}

fn save_token(filename: &str, token: oauth2::Token) -> Result<(), Error> {
    let json = serde_json::to_string(&token).unwrap();
    let path = Path::new(filename);

    Ok(File::create(&path).and_then(|mut file| file.write_all(json.as_bytes()))?)
}

fn load_token(filename: &str) -> Result<fitbit::Token, Error> {
    let mut f = File::open(filename).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("unable to read file");

    Ok(serde_json::from_str::<fitbit::Token>(contents.trim())?)
}
