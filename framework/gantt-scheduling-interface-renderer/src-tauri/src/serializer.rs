use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use serde::{de, Deserialize, Deserializer, Serializer};
use serde_json::Value;

pub(crate) fn native_date_time_to_str<S: Serializer>(
    time: &NaiveDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&time.format("%Y-%m-%d %H:%M:%S%.9f").to_string())
}

fn parse_time(value: Value) -> Result<NaiveDateTime, String> {
    if let Value::String(value) = value {
        if let Ok(time) = DateTime::parse_from_rfc3339(&value) {
            return Ok(time.naive_utc());
        }
        if let Ok(time) = DateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f %:z") {
            return Ok(time.naive_utc());
        }
        // chrono 不解析带秒的 UTC 偏移量。 / chrono cannot parse UTC offsets containing seconds.
        if let Some((local, offset)) = value.rsplit_once(' ') {
            if offset.starts_with('+') || offset.starts_with('-') {
                let parts = offset[1..]
                    .split(':')
                    .map(str::parse::<u32>)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())?;
                if parts.len() != 3 || parts[0] > 23 || parts[1] > 59 || parts[2] > 59 {
                    return Err("invalid UTC offset".into());
                }
                let sign = if offset.starts_with('-') { -1 } else { 1 };
                let seconds = sign * (parts[0] * 3600 + parts[1] * 60 + parts[2]) as i32;
                let offset = FixedOffset::east_opt(seconds).ok_or("invalid UTC offset")?;
                let local = NaiveDateTime::parse_from_str(local, "%Y-%m-%d %H:%M:%S%.f")
                    .map_err(|error| error.to_string())?;
                return local
                    .checked_sub_offset(offset)
                    .ok_or_else(|| "date-time out of range".into());
            }
        }
        for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
            if let Ok(time) = NaiveDateTime::parse_from_str(&value, format) {
                return Ok(time);
            }
        }
        return Err(format!("unsupported date-time: {value}"));
    }
    // time::OffsetDateTime 的默认 serde 元组。 / Default serde tuple of time::OffsetDateTime.
    let tuple: [i32; 9] = serde_json::from_value(value).map_err(|error| error.to_string())?;
    let [year, ordinal, hour, minute, second, nanos, offset_hour, offset_minute, offset_second] =
        tuple;
    let unsigned =
        |value: i32| u32::try_from(value).map_err(|_| "negative date-time component".to_string());
    let date = NaiveDate::from_yo_opt(year, unsigned(ordinal)?).ok_or("invalid year or ordinal")?;
    if !(0..1_000_000_000).contains(&nanos) {
        return Err("invalid nanoseconds".into());
    }
    let local = date
        .and_hms_nano_opt(
            unsigned(hour)?,
            unsigned(minute)?,
            unsigned(second)?,
            unsigned(nanos)?,
        )
        .ok_or("invalid time component")?;
    if !(-23..=23).contains(&offset_hour)
        || !(-59..=59).contains(&offset_minute)
        || !(-59..=59).contains(&offset_second)
    {
        return Err("invalid UTC offset".into());
    }
    let offset = FixedOffset::east_opt(offset_hour * 3600 + offset_minute * 60 + offset_second)
        .ok_or("invalid UTC offset")?;
    local
        .checked_sub_offset(offset)
        .ok_or_else(|| "date-time out of range".into())
}

pub(crate) fn optional_naive_date_time_to_str<S: Serializer>(
    time: &Option<NaiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match time {
        Some(time) => native_date_time_to_str(time, serializer),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn optional_naive_date_time_from_str<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error> {
    Option::<Value>::deserialize(deserializer)?
        .map(parse_time)
        .transpose()
        .map_err(de::Error::custom)
}
