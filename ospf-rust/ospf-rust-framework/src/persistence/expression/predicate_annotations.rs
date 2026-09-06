//! 谓词注解占位
//! Predicate annotations placeholder

/// 谓词注解 / Predicate annotation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateAnnotation {
    /// 注解键 / Annotation key
    pub key: String,
    /// 注解值 / Annotation value
    pub value: String,
}
