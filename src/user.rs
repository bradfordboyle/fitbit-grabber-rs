use super::Result;

pub trait UserService {
    fn user_profile(&self) -> Result<String>;

    fn user_badges(&self) -> Result<String>;
}

impl UserService for super::FitbitClient {
    fn user_profile(&self) -> Result<String> {
        let path = "1/user/-/profile.json";
        self.do_get(&path)
    }

    fn user_badges(&self) -> Result<String> {
        let path = "1/user/-/badges.json";
        self.do_get(&path)
    }
}
