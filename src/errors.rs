use std::io;

use chrono;
use oauth2;
use reqwest;
use std::convert::From;
use url;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "http err: {}", _0)]
    Http(#[cause] reqwest::Error),

    #[fail(display = "io error: {}", _0)]
    Io(#[cause] io::Error),

    #[fail(display = "url err: {}", _0)]
    Url(#[cause] url::ParseError),

    #[fail(display = "oauth token err: {}", _0)]
    AuthToken(#[cause] oauth2::TokenError),

    #[fail(display = "refresh token missing")]
    RefreshTokenMissing,

    #[fail(display = "missing code param ")]
    OAuthCodeMissing,

    #[fail(display = "bad date format: {}", _0)]
    DateParse(#[cause] chrono::ParseError),
}

impl From<chrono::ParseError> for Error {
    fn from(kind: chrono::ParseError) -> Self {
        Error::DateParse(kind)
    }
}

impl From<reqwest::Error> for Error {
    fn from(kind: reqwest::Error) -> Self {
        Error::Http(kind)
    }
}

impl From<::std::io::Error> for Error {
    fn from(kind: ::std::io::Error) -> Self {
        Error::Io(kind)
    }
}

impl From<url::ParseError> for Error {
    fn from(kind: url::ParseError) -> Self {
        Error::Url(kind)
    }
}
