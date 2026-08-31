//! 谓词注解占位
//! Predicate annotations placeholder

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateAnnotation {
    pub key: String,
    pub value: String,
}
