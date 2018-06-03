use reqwest;
use std::convert::From;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "http err: {}", _0)]
    Http(#[cause] ::reqwest::Error),

    #[fail(display = "io error: {}", _0)]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "url err: {}", _0)]
    Url(#[cause] ::url::ParseError),

    #[fail(display = "oauth token err: {}", _0)]
    AuthToken(#[cause] ::oauth2::TokenError),

    #[fail(display = "refresh token missing")]
    RefreshTokenMissing,

    #[fail(display = "missing code param ")]
    OAuthCodeMissing,

    #[fail(display = "bad date format: {}", _0)]
    DateParse(#[cause] ::chrono::ParseError),
}

impl From<reqwest::Error> for Error {
    fn from(kind: reqwest::Error) -> Error {
        Error::Http(kind)
    }
}

impl From<::std::io::Error> for Error {
    fn from(kind: ::std::io::Error) -> Error {
        Error::Io(kind)
    }
}
