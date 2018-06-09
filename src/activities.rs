use super::FitbitClient;
use date;

pub trait Activities {
    fn get_daily_activity_summary(&self, user_id: &str, date: &date::Date) -> ::Result<String>;
}

impl Activities for FitbitClient {
    fn get_daily_activity_summary(&self, user_id: &str, date: &date::Date) -> ::Result<String> {
        let path = format!("user/{}/activities/date/{}.json", user_id, date);
        self.do_get(&path)
    }
}
