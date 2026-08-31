//! SQL 类型占位
//! SQL type placeholder

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlType {
    Integer,
    Float,
    Text,
    Boolean,
}
