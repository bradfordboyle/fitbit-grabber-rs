extern crate chrono;
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

pub mod errors;

use errors::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct Token(oauth2::Token);

pub struct FitbitClient {
    client: reqwest::Client,
    base: url::Url,
}

pub enum DateQuery {
    ForDate(NaiveDate),
    PeriodicSince(NaiveDate, Period),
    Range(NaiveDate, NaiveDate),
}

/// UserProfile is a partial serialization struct of the Fitbit API profile. See:
/// https://dev.fitbit.com/build/reference/web-api/user/
#[derive(Serialize, Deserialize, Debug)]
pub struct UserProfile {
    age: i64,
    #[serde(rename = "offsetFromUTCMillis")]
    utc_offset: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserProfileResult {
    user: UserProfile,
}

pub trait User {
    fn get_profile(&self) -> Result<UserProfileResult, Error>;
}

impl User for FitbitClient {
    fn get_profile(&self) -> Result<UserProfileResult, Error> {
        let url = self.base.join("user/-/profile.json")?;
        Ok(self
            .client
            .request(Method::Get, url)
            .send()
            .and_then(|mut resp| {
                //println!("debuggin': {:?}", resp);
                Ok(resp.json::<UserProfileResult>()?)
            })?)
    }
}

pub trait Body {
    fn get_body_time_series(&self, DateQuery) -> Result<WeightSeriesResult, Error>;
    // etc.
}

impl Body for FitbitClient {
    fn get_body_time_series(&self, q: DateQuery) -> Result<WeightSeriesResult, Error> {
        let url: String = match q {
            DateQuery::PeriodicSince(date, period) => format!(
                "user/-/body/weight/date/{}/{}.json",
                date.format("%Y-%m-%d"),
                period.string()
            ),
            //GET /1/user/[user-id]/body/[resource-path]/date/[base-date]/[end-date].json
            DateQuery::Range(from, to) => format!(
                "user/-/body/weight/date/{}/{}.json",
                from.format("%Y-%m-%d"),
                to.format("%Y-%m-%d")
            ),
            _ => unimplemented!(),
        };
        Ok(self
            .client
            .request(Method::Get, self.base.join(&url)?)
            .send()
            .and_then(|mut resp| {
                //println!("debuggin': {:?}", resp);
                Ok(resp.json::<WeightSeriesResult>()?)
            })?)
    }
}

impl FitbitClient {
    pub fn new(token: Token) -> Result<FitbitClient, Error> {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer {
            token: token.0.access_token,
        }));
        headers.set(UserAgent::new("fitbit-grabber-rs (0.1.0)"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| Error::Http(e))?;

        Ok(FitbitClient {
            client: client,
            base: url::Url::parse("https://api.fitbit.com/1/").unwrap(),
        })
    }

    pub fn user(&self) -> Result<String, Error> {
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

    pub fn heart(&self, date: NaiveDate) -> Result<String, Error> {
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

    pub fn step(&self, date: NaiveDate) -> Result<String, Error> {
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

    pub fn weight(&self, date: NaiveDate) -> Result<WeightResult, Error> {
        let path = format!(
            "user/-/body/log/weight/date/{}/1d.json",
            date.format("%Y-%m-%d")
        );
        let url = self.base.join(&path).map_err(|e| Error::Url(e))?;
        Ok(self
            .client
            .request(Method::Get, url)
            .send()
            .and_then(|mut resp| {
                //println!("debuggin': {:?}", resp);
                Ok(resp.json::<WeightResult>()?)
            })?)
    }
}

/// Variants are 1d, 7d, 30d, 1w, 1m, 3m, 6m, 1y, or max.
pub enum Period {
    Day,
    Week,
}

impl Period {
    pub fn string(&self) -> &'static str {
        match *self {
            Period::Day => "1d",
            Period::Week => "1w",
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct WeightSeries {
    #[serde(rename = "dateTime")]
    pub date: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WeightSeriesResult {
    #[serde(rename = "body-weight")]
    pub body_weight: Vec<WeightSeries>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WeightResult {
    pub weight: Vec<Weight>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Weight {
    pub bmi: f64,
    pub date: String,
    #[serde(rename = "logId")]
    pub log_id: i64,
    pub time: String,
    pub weight: f64,
    pub source: String,
    /*
     * {
     *    "bmi":23.57,
     *    "date":"2015-03-05",
     *    "logId":1330991999000,
     *    "time":"23:59:59",
     *    "weight":73,
     *    "source": "API"
     *  }
     */
}

pub struct FitbitAuth(OAuth2Config);

impl FitbitAuth {
    pub fn new(client_id: &str, client_secret: &str) -> FitbitAuth {
        let auth_url = "https://www.fitbit.com/oauth2/authorize";
        let token_url = "https://api.fitbit.com/oauth2/token";
        // let token_url = "http://localhost:8080";

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
        config = config.set_redirect_url("http://localhost:8080");

        FitbitAuth(config)
    }

    pub fn get_token(&self) -> Result<oauth2::Token, Error> {
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

    pub fn exchange_refresh_token(&self, token: Token) -> Result<oauth2::Token, Error> {
        match token.0.refresh_token {
            Some(t) => self
                .0
                .exchange_refresh_token(t)
                .map_err(|e| Error::AuthToken(e)),
            None => Err(Error::RefreshTokenMissing),
        }
    }
}

#[cfg(test)]
mod tests {
    // NOTE: where's this from? Tests don't compile.
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
