//! JSON 状态载体 / JSON state carrier
//!
//! 会话快照以 JSON 承载状态，与 Kotlin 用 `kotlinx.serialization` 的 `JsonElement` 对应。Rust 侧直接使用
//! [`serde_json::Value`]，因此快照可以落盘、跨进程传输，并与其他 serde 生态互通。
//! Session snapshots carry state as JSON, matching Kotlin's use of `kotlinx.serialization`'s `JsonElement`. The Rust side uses
//! [`serde_json::Value`] directly, so snapshots can be persisted, transferred across processes, and interoperate with the wider
//! serde ecosystem.

use std::fmt::Display;

use serde_json::{Map, Number, Value};

use crate::solver::heuristic::result::HeuristicError;

/// 协议结果别名 / Result alias of the protocol
pub type Result<T> = std::result::Result<T, HeuristicError>;

/// JSON 值，直接复用 serde 的数据模型 / JSON value, reusing serde's data model directly
pub type JsonValue = Value;

/// 构造数值数组 / Build a numeric array
pub fn numbers(values: impl IntoIterator<Item = f64>) -> JsonValue {
    JsonValue::Array(
        values
            .into_iter()
            .map(|value| JsonValue::Number(Number::from_f64(value).unwrap_or_else(|| Number::from(0))))
            .collect(),
    )
}

/// 构造对象，字段顺序即插入顺序 / Build an object whose field order is the insertion order
pub fn object(entries: impl IntoIterator<Item = (impl Into<String>, JsonValue)>) -> JsonValue {
    let mut map = Map::new();
    for (key, value) in entries {
        map.insert(key.into(), value);
    }
    JsonValue::Object(map)
}

/// 读取数值 / Read a number
pub fn as_f64(value: &JsonValue) -> Option<f64> {
    value.as_f64().filter(|item| item.is_finite())
}

/// 读取数值数组 / Read a numeric array
pub fn as_numbers(value: &JsonValue) -> Option<Vec<f64>> {
    value
        .as_array()?
        .iter()
        .map(as_f64)
        .collect::<Option<Vec<f64>>>()
}

/// 读取二维数值数组 / Read a nested numeric array
pub fn as_number_matrix(value: &JsonValue) -> Option<Vec<Vec<f64>>> {
    value
        .as_array()?
        .iter()
        .map(as_numbers)
        .collect::<Option<Vec<Vec<f64>>>>()
}

/// 读取对象字段 / Read an object field
pub fn field<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    value.get(name)
}

/// 非法状态错误 / Invalid state error
pub(crate) fn invalid_state(message: impl Display) -> HeuristicError {
    HeuristicError::InvalidArgument(format!(
        "invalid search configuration or state / 搜索配置或状态无效: {message}"
    ))
}

/// 无损状态编码协议 / Lossless state codec
///
/// 与 Kotlin `SearchCodec` 一致：编码为 JSON 值，解码失败必须显式返回错误。
/// Matches the Kotlin `SearchCodec`: encode into a JSON value, and decode failures must be returned explicitly.
pub trait SearchCodec<T> {
    /// 编码状态 / Encode state
    fn encode(&self, value: &T) -> Result<JsonValue>;

    /// 解码状态 / Decode state
    fn decode(&self, value: &JsonValue) -> Result<T>;
}

impl<T, C> SearchCodec<T> for &C
where
    C: SearchCodec<T> + ?Sized,
{
    fn encode(&self, value: &T) -> Result<JsonValue> {
        (**self).encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<T> {
        (**self).decode(value)
    }
}

impl<T, C> SearchCodec<T> for Box<C>
where
    C: SearchCodec<T> + ?Sized,
{
    fn encode(&self, value: &T) -> Result<JsonValue> {
        (**self).encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<T> {
        (**self).decode(value)
    }
}

/// 列表编解码 / List codec
pub struct ListCodec<C> {
    element: C,
}

impl<C> ListCodec<C> {
    /// 以元素编解码创建 / Create from an element codec
    pub fn new(element: C) -> Self {
        Self { element }
    }
}

impl<T, C> SearchCodec<Vec<T>> for ListCodec<C>
where
    C: SearchCodec<T>,
{
    fn encode(&self, value: &Vec<T>) -> Result<JsonValue> {
        let mut encoded = Vec::with_capacity(value.len());
        for item in value {
            encoded.push(self.element.encode(item)?);
        }
        Ok(JsonValue::Array(encoded))
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<T>> {
        let array = value
            .as_array()
            .ok_or_else(|| invalid_state("expected array / 期望数组"))?;
        let mut decoded = Vec::with_capacity(array.len());
        for item in array {
            decoded.push(self.element.decode(item)?);
        }
        Ok(decoded)
    }
}

/// 可空编解码 / Nullable codec
pub struct NullableCodec<C> {
    element: C,
}

impl<C> NullableCodec<C> {
    /// 以元素编解码创建 / Create from an element codec
    pub fn new(element: C) -> Self {
        Self { element }
    }
}

impl<T, C> SearchCodec<Option<T>> for NullableCodec<C>
where
    C: SearchCodec<T>,
{
    fn encode(&self, value: &Option<T>) -> Result<JsonValue> {
        match value {
            None => Ok(JsonValue::Null),
            Some(value) => self.element.encode(value),
        }
    }

    fn decode(&self, value: &JsonValue) -> Result<Option<T>> {
        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(self.element.decode(value)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_values_round_trip_through_codecs() {
        let list = ListCodec::new(crate::solver::heuristic::session::codecs::TextNumberCodec);
        let values = vec![1.0, -2.5, 3.25];
        let encoded = list.encode(&values).expect("encodable");
        assert_eq!(list.decode(&encoded).expect("decodable"), values);
        assert!(list.encode(&vec![f64::NAN]).is_err());
        assert!(list.decode(&JsonValue::Null).is_err());
    }

    #[test]
    fn nullable_codec_preserves_null() {
        let codec = NullableCodec::new(crate::solver::heuristic::session::codecs::TextNumberCodec);
        let encoded = codec.encode(&None).expect("encodable");
        assert!(encoded.is_null());
        assert_eq!(codec.decode(&encoded).expect("decodable"), None);
        assert_eq!(
            codec
                .decode(&serde_json::json!("2.0"))
                .expect("decodable"),
            Some(2.0)
        );
    }

    #[test]
    fn objects_keep_insertion_order() {
        let value = object([("b", JsonValue::from(2.0)), ("a", JsonValue::Null)]);
        assert_eq!(value.to_string(), r#"{"b":2.0,"a":null}"#);
    }

    #[test]
    fn snapshots_can_be_serialized_to_text() {
        let value = object([
            ("version", JsonValue::from(1)),
            ("state", numbers([1.0, 2.0])),
        ]);
        let text = serde_json::to_string(&value).expect("serializable");
        assert_eq!(text, r#"{"version":1,"state":[1.0,2.0]}"#);
        let restored: JsonValue = serde_json::from_str(&text).expect("deserializable");
        assert_eq!(restored, value);
    }
}
