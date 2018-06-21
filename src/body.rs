use super::FitbitClient;
use chrono::NaiveDate;
use errors::Error;
use query::DateQuery;
use serializers::naive_date;

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
            _ => unimplemented!(), // TODO: missing an error type?
        };
        let url = self.base.join(&url)?;
        Ok(self.client.get(url).send()?.json()?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WeightSeries {
    #[serde(rename = "dateTime")]
    #[serde(with = "naive_date")]
    pub date: Option<NaiveDate>,
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
}
