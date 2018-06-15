pub mod naive_date {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<NaiveDate>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref d) = *date {
            return s.serialize_str(&d.format("%Y-%m-%d").to_string());
        }
        s.serialize_none()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        if let Some(s) = s {
            return Ok(Some(
                NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(serde::de::Error::custom)?,
            ));
        }

        Ok(None)
    }
}
