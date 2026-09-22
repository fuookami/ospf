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

// ============================================================================
// 操作符测试 / Operator tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn unary_operator_symbols_match_kotlin_text() {
        assert_eq!(UnaryOperator::Negate.symbol(), "-");
        assert_eq!(UnaryOperator::Positive.symbol(), "+");
        assert_eq!(UnaryOperator::Abs.symbol(), "abs");
    }

    #[test]
    fn binary_operator_symbols_cover_all_variants() {
        assert_eq!(BinaryOperator::Add.symbol(), "+");
        assert_eq!(BinaryOperator::Subtract.symbol(), "-");
        assert_eq!(BinaryOperator::Multiply.symbol(), "*");
        assert_eq!(BinaryOperator::Divide.symbol(), "/");
        assert_eq!(BinaryOperator::Modulo.symbol(), "%");
        assert_eq!(BinaryOperator::Power.symbol(), "^");
    }

    #[test]
    fn comparison_operator_symbols_cover_all_variants() {
        assert_eq!(ComparisonOperator::Eq.symbol(), "=");
        assert_eq!(ComparisonOperator::Ne.symbol(), "<>");
        assert_eq!(ComparisonOperator::Lt.symbol(), "<");
        assert_eq!(ComparisonOperator::Le.symbol(), "<=");
        assert_eq!(ComparisonOperator::Gt.symbol(), ">");
        assert_eq!(ComparisonOperator::Ge.symbol(), ">=");
    }

    #[test]
    fn comparison_inverse_maps_to_opposite_operator() {
        assert_eq!(ComparisonOperator::Eq.inverse(), ComparisonOperator::Ne);
        assert_eq!(ComparisonOperator::Ne.inverse(), ComparisonOperator::Eq);
        assert_eq!(ComparisonOperator::Lt.inverse(), ComparisonOperator::Gt);
        assert_eq!(ComparisonOperator::Le.inverse(), ComparisonOperator::Ge);
        assert_eq!(ComparisonOperator::Gt.inverse(), ComparisonOperator::Lt);
        assert_eq!(ComparisonOperator::Ge.inverse(), ComparisonOperator::Le);
    }

    #[test]
    fn comparison_inverse_is_involutive_and_never_identity() {
        let operators = [
            ComparisonOperator::Eq,
            ComparisonOperator::Ne,
            ComparisonOperator::Lt,
            ComparisonOperator::Le,
            ComparisonOperator::Gt,
            ComparisonOperator::Ge,
        ];
        for operator in operators {
            // 二次取反回到原操作符，且单次取反一定不是自身。
            // Double inversion returns the original operator, and one inversion is never identity.
            assert_eq!(operator.inverse().inverse(), operator);
            assert_ne!(operator.inverse(), operator);
        }
    }

    #[test]
    fn boolean_operator_symbols_are_lowercase_keywords() {
        assert_eq!(BooleanOperator::And.symbol(), "and");
        assert_eq!(BooleanOperator::Or.symbol(), "or");
        assert_eq!(BooleanOperator::Not.symbol(), "not");
    }

    #[test]
    fn null_check_type_symbols_read_as_sql_phrases() {
        assert_eq!(NullCheckType::IsNull.symbol(), "is null");
        assert_eq!(NullCheckType::IsNotNull.symbol(), "is not null");
    }

    #[test]
    fn scalar_function_names_are_stable_lowercase_identifiers() {
        let names = [
            ScalarFunctionNames::ABS,
            ScalarFunctionNames::LOWER,
            ScalarFunctionNames::UPPER,
            ScalarFunctionNames::TRIM,
            ScalarFunctionNames::LENGTH,
            ScalarFunctionNames::COALESCE,
        ];

        assert_eq!(ScalarFunctionNames::ABS, "abs");
        assert_eq!(ScalarFunctionNames::LOWER, "lower");
        assert_eq!(ScalarFunctionNames::UPPER, "upper");
        assert_eq!(ScalarFunctionNames::TRIM, "trim");
        assert_eq!(ScalarFunctionNames::LENGTH, "length");
        assert_eq!(ScalarFunctionNames::COALESCE, "coalesce");

        for name in names {
            assert_eq!(name, name.to_ascii_lowercase());
            assert!(!name.is_empty());
        }
        assert_eq!(names.iter().collect::<HashSet<_>>().len(), names.len());
    }

    #[test]
    fn operators_are_usable_as_hash_set_members() {
        // 操作符需要可哈希，便于在集合与结构键中做去重。
        // Operators must be hashable so they can be deduplicated in sets and structural keys.
        let mut binary = HashSet::new();
        assert!(binary.insert(BinaryOperator::Add));
        assert!(!binary.insert(BinaryOperator::Add));
        assert!(binary.insert(BinaryOperator::Power));
        assert_eq!(binary.len(), 2);

        let mut modes = HashSet::new();
        assert!(modes.insert(PatternMatchMode::Exact));
        assert!(modes.insert(PatternMatchMode::Prefix));
        assert!(modes.insert(PatternMatchMode::Suffix));
        assert!(modes.insert(PatternMatchMode::Contains));
        assert!(modes.insert(PatternMatchMode::Like));
        assert!(modes.insert(PatternMatchMode::Regex));
        assert_eq!(modes.len(), 6);
    }

    #[test]
    fn operators_are_copyable_without_clone_noise() {
        let operator = ComparisonOperator::Ge;
        let copied = operator;
        assert_eq!(copied, operator);
        assert_eq!(copied.inverse().symbol(), "<=");

        let unary = UnaryOperator::Abs;
        assert_eq!(unary.symbol(), "abs");
        assert_eq!(PatternMatchMode::Like, PatternMatchMode::Like);
        assert_ne!(PatternMatchMode::Like, PatternMatchMode::Regex);
        assert_ne!(NullCheckType::IsNull, NullCheckType::IsNotNull);
    }
}
