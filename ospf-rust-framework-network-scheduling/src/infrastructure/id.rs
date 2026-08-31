//! 稳定网络标识 / Stable network identifiers.

use std::fmt::{Display, Formatter};
use std::hash::Hash;

/// 网络节点稳定 ID / Stable network-node ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkNodeId(String);

impl NetworkNodeId {
    /// 创建节点 ID；空字符串由上层图校验拒绝 / Create a node ID; empty values are rejected by graph validation.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 获取 ID 字符串 / Get the ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for NetworkNodeId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for NetworkNodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for NetworkNodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// 网络弧稳定 ID / Stable network-arc ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkArcId(String);

impl NetworkArcId {
    /// 创建弧 ID / Create an arc ID.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 获取 ID 字符串 / Get the ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for NetworkArcId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for NetworkArcId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for NetworkArcId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
