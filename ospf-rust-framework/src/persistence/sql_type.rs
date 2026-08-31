//! SQL 类型占位
//! SQL type placeholder

/// SQL 数据类型 / SQL data type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlType {
    /// 整数类型 / Integer type
    Integer,
    /// 浮点类型 / Float type
    Float,
    /// 文本类型 / Text type
    Text,
    /// 布尔类型 / Boolean type
    Boolean,
}
