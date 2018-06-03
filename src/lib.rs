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
            .and_then(|mut r| r.text())
            .map_err(|e| Error::Http(e))?)
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

    pub fn weight(&self, date: NaiveDate) -> Result<Vec<Weight>, Error> {
        let url = format!(
            "user/-/body/weight/date/{}/1d.json",
            date.format("%Y-%m-%d")
        );
        Ok(self
            .client
            .request(Method::Get, &url)
            .send()
            .and_then(|mut resp| Ok(resp.json::<Vec<Weight>>()?))?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Weight {
    pub bmi: f32,
    pub date: String,
    pub log_id: i32,
    pub time: String,
    pub weight: f32,
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

        // This example will be running its own server at localhost:8080.
        // See below for the server implementation.
        config = config.set_redirect_url("http://localhost:8080");

        FitbitAuth(config)
    }

    pub fn get_token(&self) -> Result<oauth2::Token, Error> {
        let authorize_url = self.0.authorize_url();

        println!(
            "Open this URL in your browser:\n{}\n",
            authorize_url.to_string()
        );

        // FIXME avoid unwrap here
        let server = tiny_http::Server::http("localhost:8080").unwrap();
        let request = server.recv().map_err(|e| Error::Io(e))?;
        let url = request.url().to_string();
        let response = tiny_http::Response::from_string("Go back to your terminal :)");
        request.respond(response).map_err(|e| Error::Io(e))?;

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
