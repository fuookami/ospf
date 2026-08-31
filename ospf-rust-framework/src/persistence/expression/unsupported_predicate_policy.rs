//! 不支持谓词策略
//! Unsupported predicate policy

/// 不支持谓词策略 / Unsupported predicate policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedPredicatePolicy {
    /// 无法下推时翻译为恒假条件。
    /// Translate unsupported predicates to an always-false condition.
    AlwaysFalse,
    /// 无法下推时立即返回错误。
    /// Fail immediately when a predicate cannot be pushed down.
    FailFast,
    /// 客户端过滤策略，当前仅作为显式保留策略。
    /// Client-side filter policy, currently reserved as an explicit strategy.
    ClientFilter,
}

impl UnsupportedPredicatePolicy {
    /// 兼容旧 `Ignore` 语义，等价于 `AlwaysFalse`。
    /// Compatibility alias for old `Ignore` semantics, equivalent to `AlwaysFalse`.
    pub const IGNORE: Self = Self::AlwaysFalse;

    /// 兼容旧 `Error` 语义，等价于 `FailFast`。
    /// Compatibility alias for old `Error` semantics, equivalent to `FailFast`.
    pub const ERROR: Self = Self::FailFast;

    /// 返回是否应在不支持谓词时失败。
    /// Return whether unsupported predicates should fail fast.
    pub const fn should_fail_fast(self) -> bool {
        matches!(self, Self::FailFast)
    }

    /// 返回是否应翻译为恒假条件。
    /// Return whether unsupported predicates should become always-false conditions.
    pub const fn should_use_always_false(self) -> bool {
        matches!(self, Self::AlwaysFalse)
    }
}

impl Default for UnsupportedPredicatePolicy {
    fn default() -> Self {
        Self::AlwaysFalse
    }
}
