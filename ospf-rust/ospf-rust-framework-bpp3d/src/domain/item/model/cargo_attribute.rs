// ============================================================================
// CargoAttributeKey - 货物属性键 / Cargo attribute key
// ============================================================================

/// 货物属性键 / Cargo attribute key
///
/// 轻量、可序列化、可比较的货物属性标识，对应 Kotlin `AbstractCargoAttribute`
/// marker interface 的 Rust 等价物。
/// Lightweight, serializable, comparable cargo attribute key, serving as the
/// Rust equivalent of Kotlin's `AbstractCargoAttribute` marker interface.
///
/// Kotlin 中 `AbstractCargoAttribute` 是空 marker interface，用于身份比较和下游
/// 扩展。Rust 以具体 key 类型替代闭包式扩展，优先保证序列化和测试友好。
/// In Kotlin, `AbstractCargoAttribute` is an empty marker interface used for
/// identity comparison and downstream extension. Rust replaces closure-based
/// extension with a concrete key type for serialization and testing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CargoAttributeKey {
    /// 标识字符串 / Identity string
    pub key: String,
    /// 可选标签 / Optional tags
    pub tags: Vec<String>,
}

impl CargoAttributeKey {
    /// 使用标识创建 / Create from identity string
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            tags: Vec::new(),
        }
    }

    /// 使用标识和标签创建 / Create from identity string with tags
    pub fn with_tags(key: impl Into<String>, tags: Vec<String>) -> Self {
        Self {
            key: key.into(),
            tags,
        }
    }
}

