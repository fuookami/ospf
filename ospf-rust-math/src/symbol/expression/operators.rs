//! 表达式操作符
//! Expression operators

/// 标量一元操作符。
/// Scalar unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryOperator {
    /// 负号 / Negation
    Negate,
    /// 正号 / Positive identity
    Positive,
    /// 绝对值 / Absolute value
    Abs,
}

impl UnaryOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Positive => "+",
            Self::Abs => "abs",
        }
    }
}

/// 标量二元操作符。
/// Scalar binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BinaryOperator {
    /// 加法 / Addition
    Add,
    /// 减法 / Subtraction
    Subtract,
    /// 乘法 / Multiplication
    Multiply,
    /// 除法 / Division
    Divide,
    /// 取模 / Modulo
    Modulo,
    /// 幂运算 / Power
    Power,
}

impl BinaryOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Power => "^",
        }
    }
}

/// 比较操作符。
/// Comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComparisonOperator {
    /// 等于 / Equal
    Eq,
    /// 不等于 / Not equal
    Ne,
    /// 小于 / Less than
    Lt,
    /// 小于等于 / Less than or equal
    Le,
    /// 大于 / Greater than
    Gt,
    /// 大于等于 / Greater than or equal
    Ge,
}

impl ComparisonOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }

    /// 获取反向比较操作符。
    /// Get inverse comparison operator.
    pub const fn inverse(self) -> Self {
        match self {
            Self::Eq => Self::Ne,
            Self::Ne => Self::Eq,
            Self::Lt => Self::Gt,
            Self::Le => Self::Ge,
            Self::Gt => Self::Lt,
            Self::Ge => Self::Le,
        }
    }
}

/// 模式匹配模式。
/// Pattern match mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PatternMatchMode {
    /// 精确匹配 / Exact match
    Exact,
    /// 前缀匹配 / Prefix match
    Prefix,
    /// 后缀匹配 / Suffix match
    Suffix,
    /// 包含匹配 / Contains match
    Contains,
    /// SQL LIKE 风格通配符匹配 / SQL LIKE-style wildcard match
    Like,
    /// 正则匹配 / Regex match
    Regex,
}

/// 布尔操作符。
/// Boolean operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BooleanOperator {
    /// 逻辑与 / Logical AND
    And,
    /// 逻辑或 / Logical OR
    Or,
    /// 逻辑非 / Logical NOT
    Not,
}

impl BooleanOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
        }
    }
}

/// 空值检查类型。
/// Null check type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NullCheckType {
    /// 是空值 / Is null
    IsNull,
    /// 非空值 / Is not null
    IsNotNull,
}

impl NullCheckType {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::IsNull => "is null",
            Self::IsNotNull => "is not null",
        }
    }
}

/// 标准标量函数名称。
/// Standard scalar function names.
pub struct ScalarFunctionNames;

impl ScalarFunctionNames {
    /// 绝对值函数 / Absolute value function
    pub const ABS: &'static str = "abs";
    /// 小写函数 / Lowercase function
    pub const LOWER: &'static str = "lower";
    /// 大写函数 / Uppercase function
    pub const UPPER: &'static str = "upper";
    /// 裁剪函数 / Trim function
    pub const TRIM: &'static str = "trim";
    /// 长度函数 / Length function
    pub const LENGTH: &'static str = "length";
    /// 合并空值函数 / Coalesce function
    pub const COALESCE: &'static str = "coalesce";
}
