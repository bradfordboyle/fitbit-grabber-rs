extern crate clap;
extern crate fitbit_grabber;
extern crate serde;
extern crate serde_json;
extern crate url;

use std::env;
use std::error::Error;
use std::fmt;
use std::io;
use std::result;
use std::str::FromStr;

use clap::{App, Arg, SubCommand};
use fitbit_grabber::date;
use fitbit_grabber::{FitbitAuth, FitbitClient, FitbitError, Token, UserService};

#[derive(Debug)]
enum CliError {
    Fitbit(FitbitError),
    Io(io::Error),
    Json(serde_json::Error),
    MissingArg(String),
}

impl Error for CliError {
    fn description(&self) -> &str {
        match *self {
            CliError::Fitbit(ref err) => err.description(),
            _ => "Something bad happened",
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::Fitbit(ref err) => write!(f, "Date Parse Error: {}", err),
            _ => write!(f, "Oh no, something bad went down"),
        }
    }
}

impl From<fitbit_grabber::FitbitError> for CliError {
    fn from(err: FitbitError) -> CliError {
        CliError::Fitbit(err)
    }
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError {
        CliError::Io(err)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> CliError {
        CliError::Json(err)
    }
}

type Result<T> = result::Result<T, CliError>;

fn main() {
    let matches = App::new("Fitbit Grabber")
        .subcommand(SubCommand::with_name("get-devices").about("list devices connected to account"))
        .subcommand(
            SubCommand::with_name("get-alarms")
                .about("list alarms connected to device")
                .arg(
                    Arg::with_name("tracker-id")
                        .long("tracker-id")
                        .required(true)
                        .takes_value(true)
                        .help("the ID of the tracker for which data is returned"),
                ),
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
        .subcommand(SubCommand::with_name("token").about("request an access token"))
        .subcommand(SubCommand::with_name("refresh-token").about("refresh token"))
        .subcommand(SubCommand::with_name("user-profile").about("get user profile"))
        .subcommand(SubCommand::with_name("user-badges").about("get user badges"))
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
        .subcommand(
            SubCommand::with_name("sleep-log")
                .about("fetch sleep logs")
                .arg(
                    Arg::with_name("date")
                        .long("date")
                        .required(true)
                        .takes_value(true)
                        .help("date to fetch data for"),
                ),
        )
        .subcommand(SubCommand::with_name("sleep-log-list").about("get sleep logs list"))
        .get_matches();

    let auth = get_auth_from_env();

    match matches.subcommand() {
        ("token", Some(_)) => {
            auth.get_token()
                .and_then(|token| token.save(".token"))
                .expect("unable to obtain token");
        }
        ("refresh-token", Some(_)) => {
            Token::load(".token")
                .and_then(|old_token| auth.exchange_refresh_token(old_token))
                .and_then(|token| token.save(".token"))
                .expect("unable to refresh token");
        }
        ("heart", Some(sub_m)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");
            let heart_rate_data = parse_date_from(sub_m)
                .and_then(|ref date| client.heart(date).map_err(From::from))
                .expect("unable to fetch step data for given date");
            println!("{}", heart_rate_data);
        }
        ("step", Some(sub_m)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let step_data = parse_date_from(sub_m)
                .and_then(|ref date| client.step(date).map_err(From::from))
                .expect("unable to fetch step data for given date");
            println!("{}", step_data);
        }
        ("user-profile", Some(_)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let user_profile = client.user_profile().expect("unable to fetch user profile");

            println!("{}", user_profile);
        }
        ("user-badges", Some(_)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let user_badges = client.user_badges().expect("unable to fetch user badges");

            println!("{}", user_badges);
        }
        ("daily-activity-summary", Some(sub_m)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let summary = parse_date_from(sub_m)
                .and_then(|ref date| client.daily_activity_summary("-", date).map_err(From::from))
                .expect("unable to fetch summary");

            println!("{}", summary);
        }
        ("get-devices", Some(_)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let devices = client.get_devices().expect("unable to fetch devices");

            println!("{}", devices);
        }
        ("get-alarms", Some(sub_m)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let tracker_id = sub_m
                .value_of("tracker-id")
                .expect("please give tracker device id");

            let alarms = client
                .get_alarms("-", tracker_id)
                .expect("unable to fetch alarms for given device");

            println!("{}", alarms);
        }
        ("sleep-log", Some(sub_m)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let summary = parse_date_from(sub_m)
                .and_then(|ref date| client.get_sleep_logs_for_date(date).map_err(From::from))
                .expect("unable to fetch summary");

            println!("{}", summary);
        }
        ("sleep-log-list", Some(_)) => {
            let client = Token::load(".token")
                .map(|token| FitbitClient::new(token))
                .expect("unable to create Fitbit client");

            let summary = client
                .get_sleep_logs_list()
                .expect("unable to fetch summary");

            println!("{}", summary);
        }
        (cmd, _) => {
            panic!("Unknown command: {}", cmd);
        }
    }
}

fn parse_date_from(matches: &clap::ArgMatches) -> Result<date::Date> {
    matches
        .value_of("date")
        .ok_or(CliError::MissingArg("date".to_string()))
        .and_then(|arg| date::Date::from_str(arg).map_err(From::from))
}

fn get_auth_from_env() -> FitbitAuth {
    let fitbit_client_id =
        env::var("FITBIT_CLIENT_ID").expect("Missing the FITBIT_CLIENT_ID environment variable.");
    let fitbit_client_secret = env::var("FITBIT_CLIENT_SECRET")
        .expect("Missing the FITBIT_CLIENT_SECRET environment variable.");
    FitbitAuth::new(&fitbit_client_id, &fitbit_client_secret)
}
