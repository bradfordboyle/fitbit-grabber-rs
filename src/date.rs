extern crate chrono;

use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct Date(chrono::NaiveDate);

impl FromStr for Date {
    type Err = super::FitbitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map(|d| Date(d)).map_err(From::from)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_valid_date() {
        let d = Date::from_str("2018-05-03");
        assert!(d.is_ok());
        assert_eq!(d.unwrap().to_string(), "2018-05-03");
    }
}
