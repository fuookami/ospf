//! Kotlin/Rust 渲染数据的导入边界。 / Import boundary for Kotlin/Rust renderer data.

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let value = Value::deserialize(deserializer)?;
    let number = match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    };
    number
        .filter(|value| value.is_finite())
        .ok_or_else(|| serde::de::Error::custom("expected a finite number or numeric string"))
}

pub fn optional_number<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<f64>, D::Error> {
    let value = Option::<Value>::deserialize(deserializer)?;
    value
        .map(|value| number(value).map_err(serde::de::Error::custom))
        .transpose()
}

pub fn integer<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
    .ok_or_else(|| serde::de::Error::custom("expected an unsigned integer or integer string"))
}

pub fn group<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(Vec::new()),
        Value::String(value) if value.is_empty() => Ok(Vec::new()),
        Value::String(value) => Ok(vec![value]),
        value => serde_json::from_value(value).map_err(serde::de::Error::custom),
    }
}

fn display(value: Value) -> String {
    match value {
        Value::String(value) => value,
        value => value.to_string(),
    }
}

pub fn string_map<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(BTreeMap::new()),
        Value::Object(values) => Ok(values
            .into_iter()
            .map(|(key, value)| (key, display(value)))
            .collect()),
        Value::String(value) => Ok(BTreeMap::from([("info".into(), value)])),
        // Rust 领域 DTO 使用列表；保留重复值和原顺序。 / Rust DTOs use lists; retain duplicates and order.
        Value::Array(values) => Ok(values
            .into_iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), display(value)))
            .collect()),
        _ => Err(serde::de::Error::custom(
            "expected an object, list, text or null",
        )),
    }
}

pub fn optional_string_map<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<BTreeMap<String, String>>, D::Error> {
    let value = Option::<Value>::deserialize(deserializer)?;
    value
        .map(|value| string_map(value).map_err(serde::de::Error::custom))
        .transpose()
}
