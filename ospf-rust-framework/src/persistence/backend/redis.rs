//! Redis 持久化后端
//! Redis persistence backend

use serde_json::Value;

/// Redis 后端标记类型。
/// Redis backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RedisBackend;

/// Redis 配置。
/// Redis configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RedisConfig {
    pub url: String,
    pub key_prefix: Option<String>,
}

impl RedisConfig {
    /// 创建配置。
    /// Create configuration.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            key_prefix: None,
        }
    }

    /// 设置 key 前缀。
    /// Set key prefix.
    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(key_prefix.into());
        self
    }

    /// 格式化 key。
    /// Format a key.
    pub fn key(&self, key: impl AsRef<str>) -> String {
        match &self.key_prefix {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{}", key.as_ref()),
            _ => key.as_ref().to_string(),
        }
    }
}

/// Redis client handle。
/// Redis client handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisClientHandle<C> {
    pub config: RedisConfig,
    pub client: C,
}

/// Redis 命令描述。
/// Redis command descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisCommand {
    pub name: String,
    pub args: Vec<String>,
}

impl RedisCommand {
    /// 创建命令。
    /// Create a command.
    pub fn new(name: impl Into<String>, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

/// Redis key/value helper。
/// Redis key/value helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisKeyValueHelper {
    config: RedisConfig,
}

impl RedisKeyValueHelper {
    /// 创建 helper。
    /// Create a helper.
    pub fn new(config: RedisConfig) -> Self {
        Self { config }
    }

    /// 字符串读取。
    /// String get.
    pub fn get(&self, key: impl AsRef<str>) -> RedisCommand {
        RedisCommand::new("GET", [self.config.key(key)])
    }

    /// 字符串写入。
    /// String set.
    pub fn set(&self, key: impl AsRef<str>, value: impl Into<String>) -> RedisCommand {
        RedisCommand::new("SET", [self.config.key(key), value.into()])
    }

    /// list push。
    /// List push.
    pub fn list_push(&self, key: impl AsRef<str>, value: impl Into<String>) -> RedisCommand {
        RedisCommand::new("LPUSH", [self.config.key(key), value.into()])
    }

    /// set add。
    /// Set add.
    pub fn set_add(&self, key: impl AsRef<str>, value: impl Into<String>) -> RedisCommand {
        RedisCommand::new("SADD", [self.config.key(key), value.into()])
    }

    /// hash set。
    /// Hash set.
    pub fn hash_set(
        &self,
        key: impl AsRef<str>,
        field: impl Into<String>,
        value: impl Into<String>,
    ) -> RedisCommand {
        RedisCommand::new("HSET", [self.config.key(key), field.into(), value.into()])
    }

    /// JSON/object set。
    /// JSON/object set.
    pub fn object_set(
        &self,
        key: impl AsRef<str>,
        value: &Value,
    ) -> Result<RedisCommand, serde_json::Error> {
        Ok(RedisCommand::new(
            "SET",
            [self.config.key(key), serde_json::to_string(value)?],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redis_helper_builds_common_commands() {
        let helper =
            RedisKeyValueHelper::new(RedisConfig::new("redis://localhost").with_key_prefix("ospf"));

        assert_eq!(
            helper.hash_set("user:1", "status", "active"),
            RedisCommand::new("HSET", ["ospf:user:1", "status", "active"])
        );
        assert_eq!(
            helper.object_set("user:1:json", &json!({"id": 1})).unwrap(),
            RedisCommand::new("SET", ["ospf:user:1:json", r#"{"id":1}"#])
        );
    }
}
