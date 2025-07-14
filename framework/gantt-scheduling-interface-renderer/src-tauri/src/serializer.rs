use chrono::NaiveDateTime;
use serde::{de, Serializer, Deserialize, Deserializer};

pub(crate) fn native_date_time_to_str<S>(
    time: &NaiveDateTime, 
    s: S
) -> Result<S::Ok, S::Error>
    where S: Serializer,
{
    s.serialize_str(&time.format("%Y-%m-%d %H:%M:%S").to_string())
}

pub(crate) fn naive_date_time_from_str<'de, D>(
    deserializer: D
) -> Result<NaiveDateTime, D::Error>
    where D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(de::Error::custom)
}

pub(crate) fn optional_naive_date_time_to_str<S>(
    time: &Option<NaiveDateTime>, 
    s: S
) -> Result<S::Ok, S::Error>
    where S: Serializer,
{
    match time {
        Some(time) => s.serialize_str(&time.format("%Y-%m-%d %H:%M:%S").to_string()),
        None => s.serialize_none(),
    }
}

pub(crate) fn optional_naive_date_time_from_str<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error>
    where D: Deserializer<'de>,
{
    let s: Option<String> = Deserialize::deserialize(deserializer)?;
    match s {
        Some(s) => NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .map(|result| Some(result))
            .map_err(de::Error::custom),

        None => Ok(None),
    }
}
