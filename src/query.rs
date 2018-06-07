use chrono::NaiveDate;

pub enum DateQuery {
    ForDate(NaiveDate),
    PeriodicSince(NaiveDate, Period),
    Range(NaiveDate, NaiveDate),
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
