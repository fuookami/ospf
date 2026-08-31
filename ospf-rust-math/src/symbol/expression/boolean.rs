//! 布尔表达式 AST
//! Boolean expression AST

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::ops::{BitAnd, BitOr, Not as StdNot};
use crate::Trivalent;
use super::property_path::PropertyPath;
use super::operators::*;
use super::value::ExpressionValue;
use super::scalar::ScalarExpression;
use super::dsl::{and_pair, or_pair};
use super::normalize::{boolean_structural_key, normalize_boolean_expression, NormalizeConfig};

/// 解析后的布尔表达式。
/// Parsed boolean expression.
pub type ParsedBooleanExpression = BooleanExpression<ExpressionValue>;

/// 布尔表达式。
/// Boolean expression.
#[derive(Debug, Clone, PartialEq)]
pub enum BooleanExpression<T> {
    /// 布尔常量 / Boolean constant
    Constant(Trivalent),
    /// 比较表达式 / Comparison expression
    Comparison {
        /// 操作符 / Operator
        operator: ComparisonOperator,
        /// 左操作数 / Left operand
        left: ScalarExpression<T>,
        /// 右操作数 / Right operand
        right: ScalarExpression<T>,
    },
    /// 集合成员判断 / Set membership expression
    In {
        /// 被检查的值 / Checked value
        value: ScalarExpression<T>,
        /// 候选值列表 / Candidate values
        candidates: Vec<ScalarExpression<T>>,
        /// 是否取反 / Whether negated
        negated: bool,
    },
    /// 模式匹配 / Pattern match
    PatternMatch {
        /// 被匹配的值 / Matched value
        value: ScalarExpression<T>,
        /// 模式表达式 / Pattern expression
        pattern: ScalarExpression<T>,
        /// 匹配模式 / Match mode
        mode: PatternMatchMode,
        /// 是否取反 / Whether negated
        negated: bool,
    },
    /// 空值检查 / Null check
    NullCheck {
        /// 属性路径 / Property path
        path: PropertyPath,
        /// 空值检查类型 / Null check type
        null_check_type: NullCheckType,
    },
    /// 逻辑与 / Logical AND
    And(Vec<BooleanExpression<T>>),
    /// 逻辑或 / Logical OR
    Or(Vec<BooleanExpression<T>>),
    /// 逻辑非 / Logical NOT
    Not(Box<BooleanExpression<T>>),
    /// 自定义表达式 / Custom expression
    Custom {
        /// 自定义载荷 / Custom payload
        payload: String,
        /// 可选描述 / Optional description
        description: Option<String>,
    },
}

impl<T> BooleanExpression<T> {
    /// 创建布尔常量表达式。
    /// Create boolean constant expression.
    pub const fn constant(value: Trivalent) -> Self {
        Self::Constant(value)
    }

    /// 创建 true 常量表达式。
    /// Create true constant expression.
    pub const fn true_constant() -> Self {
        Self::Constant(Trivalent::True)
    }

    /// 创建 false 常量表达式。
    /// Create false constant expression.
    pub const fn false_constant() -> Self {
        Self::Constant(Trivalent::False)
    }

    /// 创建 unknown 常量表达式。
    /// Create unknown constant expression.
    pub const fn unknown_constant() -> Self {
        Self::Constant(Trivalent::Unknown)
    }

    /// 创建比较表达式。
    /// Create comparison expression.
    pub fn comparison(
        operator: ComparisonOperator,
        left: ScalarExpression<T>,
        right: ScalarExpression<T>,
    ) -> Self {
        Self::Comparison {
            operator,
            left,
            right,
        }
    }

    /// 创建相等比较表达式。
    /// Create equal comparison expression.
    pub fn eq(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Eq, left, right)
    }

    /// 创建不等比较表达式。
    /// Create not-equal comparison expression.
    pub fn ne(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Ne, left, right)
    }

    /// 创建小于比较表达式。
    /// Create less-than comparison expression.
    pub fn lt(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Lt, left, right)
    }

    /// 创建小于等于比较表达式。
    /// Create less-than-or-equal comparison expression.
    pub fn le(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Le, left, right)
    }

    /// 创建大于比较表达式。
    /// Create greater-than comparison expression.
    pub fn gt(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Gt, left, right)
    }

    /// 创建大于等于比较表达式。
    /// Create greater-than-or-equal comparison expression.
    pub fn ge(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Ge, left, right)
    }

    /// 创建集合成员判断表达式。
    /// Create set membership expression.
    pub fn in_expr(
        value: ScalarExpression<T>,
        candidates: Vec<ScalarExpression<T>>,
        negated: bool,
    ) -> Self {
        Self::In {
            value,
            candidates,
            negated,
        }
    }

    /// 创建模式匹配表达式。
    /// Create pattern match expression.
    pub fn pattern_match(
        value: ScalarExpression<T>,
        pattern: ScalarExpression<T>,
        mode: PatternMatchMode,
        negated: bool,
    ) -> Self {
        Self::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        }
    }

    /// 创建空值检查表达式。
    /// Create null check expression.
    pub fn null_check(path: impl Into<PropertyPath>, null_check_type: NullCheckType) -> Self {
        Self::NullCheck {
            path: path.into(),
            null_check_type,
        }
    }

    /// 创建 is null 表达式。
    /// Create is-null expression.
    pub fn is_null(path: impl Into<PropertyPath>) -> Self {
        Self::null_check(path, NullCheckType::IsNull)
    }

    /// 创建 is not null 表达式。
    /// Create is-not-null expression.
    pub fn is_not_null(path: impl Into<PropertyPath>) -> Self {
        Self::null_check(path, NullCheckType::IsNotNull)
    }

    /// 创建逻辑与表达式。
    /// Create logical AND expression.
    pub fn and(operands: Vec<Self>) -> Self {
        assert!(
            !operands.is_empty(),
            "And expression requires at least one operand"
        );
        Self::And(operands)
    }

    /// 创建逻辑或表达式。
    /// Create logical OR expression.
    pub fn or(operands: Vec<Self>) -> Self {
        assert!(
            !operands.is_empty(),
            "Or expression requires at least one operand"
        );
        Self::Or(operands)
    }

    /// 创建逻辑非表达式。
    /// Create logical NOT expression.
    pub fn not_expr(operand: Self) -> Self {
        Self::Not(Box::new(operand))
    }

    /// 创建自定义表达式。
    /// Create custom expression.
    pub fn custom(payload: impl Into<String>, description: Option<String>) -> Self {
        Self::Custom {
            payload: payload.into(),
            description,
        }
    }

    /// 获取表达式类型名。
    /// Get expression type name.
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Constant(_) => "BooleanConstant",
            Self::Comparison { .. } => "Comparison",
            Self::In { negated, .. } => {
                if *negated {
                    "NotIn"
                } else {
                    "In"
                }
            }
            Self::PatternMatch { negated, .. } => {
                if *negated {
                    "NotPatternMatch"
                } else {
                    "PatternMatch"
                }
            }
            Self::NullCheck { .. } => "NullCheck",
            Self::And(_) => "And",
            Self::Or(_) => "Or",
            Self::Not(_) => "Not",
            Self::Custom { .. } => "Custom",
        }
    }

    /// 判断表达式是否是常量表达式。
    /// Check whether the expression is constant.
    pub fn is_constant(&self) -> bool {
        match self {
            Self::Constant(_) => true,
            Self::Comparison { left, right, .. } => left.is_constant() && right.is_constant(),
            Self::In { value, .. } | Self::PatternMatch { value, .. } => value.is_constant(),
            Self::NullCheck { .. } | Self::Custom { .. } => false,
            Self::And(operands) | Self::Or(operands) => operands.iter().all(Self::is_constant),
            Self::Not(operand) => operand.is_constant(),
        }
    }

    /// 判断表达式是否是纯逻辑表达式。
    /// Check whether the expression is pure logical.
    pub fn is_pure_logical(&self) -> bool {
        match self {
            Self::Constant(_) => true,
            Self::Comparison { .. }
            | Self::In { .. }
            | Self::PatternMatch { .. }
            | Self::NullCheck { .. }
            | Self::Custom { .. } => false,
            Self::And(operands) | Self::Or(operands) => operands.iter().all(Self::is_pure_logical),
            Self::Not(operand) => operand.is_pure_logical(),
        }
    }

    /// 收集表达式中的属性路径引用。
    /// Collect property path references in the expression.
    pub fn collect_references(&self) -> HashSet<PropertyPath> {
        let mut references = HashSet::new();
        self.collect_references_into(&mut references);
        references
    }

    /// 将属性路径引用收集到给定集合。
    /// Collect property path references into the given set.
    pub fn collect_references_into(&self, references: &mut HashSet<PropertyPath>) {
        match self {
            Self::Comparison { left, right, .. } => {
                left.collect_references_into(references);
                right.collect_references_into(references);
            }
            Self::In {
                value, candidates, ..
            } => {
                value.collect_references_into(references);
                for candidate in candidates {
                    candidate.collect_references_into(references);
                }
            }
            Self::PatternMatch { value, pattern, .. } => {
                value.collect_references_into(references);
                pattern.collect_references_into(references);
            }
            Self::NullCheck { path, .. } => {
                references.insert(path.clone());
            }
            Self::And(operands) | Self::Or(operands) => {
                for operand in operands {
                    operand.collect_references_into(references);
                }
            }
            Self::Not(operand) => operand.collect_references_into(references),
            Self::Constant(_) | Self::Custom { .. } => {}
        }
    }

    /// 获取逻辑操作符数量。
    /// Get logical operator count.
    pub fn logical_operator_count(&self) -> usize {
        match self {
            Self::Constant(_)
            | Self::Comparison { .. }
            | Self::In { .. }
            | Self::PatternMatch { .. }
            | Self::NullCheck { .. }
            | Self::Custom { .. } => 0,
            Self::And(operands) | Self::Or(operands) => {
                operands.len()
                    + operands
                        .iter()
                        .map(Self::logical_operator_count)
                        .sum::<usize>()
            }
            Self::Not(operand) => 1 + operand.logical_operator_count(),
        }
    }

    /// 获取表达式深度。
    /// Get expression depth.
    pub fn depth(&self) -> usize {
        match self {
            Self::Constant(_)
            | Self::Comparison { .. }
            | Self::In { .. }
            | Self::PatternMatch { .. }
            | Self::NullCheck { .. }
            | Self::Custom { .. } => 1,
            Self::And(operands) | Self::Or(operands) => {
                1 + operands.iter().map(Self::depth).max().unwrap_or(0)
            }
            Self::Not(operand) => 1 + operand.depth(),
        }
    }

    /// 获取结构键，用于表达式去重和排序。
    /// Get structural key for expression deduplication and sorting.
    pub fn structural_key(&self) -> String
    where
        T: Display,
    {
        boolean_structural_key(self)
    }

    /// 使用默认配置规范化布尔表达式。
    /// Normalize boolean expression with default configuration.
    pub fn normalize(&self) -> Self
    where
        T: Clone + Display,
    {
        normalize_boolean_expression(self, NormalizeConfig::default())
    }

    /// 使用指定配置规范化布尔表达式。
    /// Normalize boolean expression with the specified configuration.
    pub fn normalize_with_config(&self, config: NormalizeConfig) -> Self
    where
        T: Clone + Display,
    {
        normalize_boolean_expression(self, config)
    }
}

impl<T> From<Trivalent> for BooleanExpression<T> {
    fn from(value: Trivalent) -> Self {
        Self::constant(value)
    }
}

impl<T> From<bool> for BooleanExpression<T> {
    fn from(value: bool) -> Self {
        Self::constant(Trivalent::from(value))
    }
}

impl<T> From<Option<bool>> for BooleanExpression<T> {
    fn from(value: Option<bool>) -> Self {
        Self::constant(Trivalent::from(value))
    }
}

impl<T> BitAnd for BooleanExpression<T> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        and_pair(self, rhs)
    }
}

impl<T> BitOr for BooleanExpression<T> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        or_pair(self, rhs)
    }
}

impl<T> StdNot for BooleanExpression<T> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::not_expr(self)
    }
}

impl<T: Display> Display for BooleanExpression<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(value) => match value {
                Trivalent::True => write!(f, "true"),
                Trivalent::False => write!(f, "false"),
                Trivalent::Unknown => write!(f, "unknown"),
            },
            Self::Comparison {
                operator,
                left,
                right,
            } => write!(f, "{} {} {}", left, operator.symbol(), right),
            Self::In {
                value,
                candidates,
                negated,
            } => {
                write!(f, "{} {}in (", value, if *negated { "not " } else { "" })?;
                for (index, candidate) in candidates.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", candidate)?;
                }
                write!(f, ")")
            }
            Self::PatternMatch {
                value,
                pattern,
                mode,
                negated,
            } => write!(
                f,
                "{} {}{:?} {}",
                value,
                if *negated { "not " } else { "" },
                mode,
                pattern
            ),
            Self::NullCheck {
                path,
                null_check_type,
            } => write!(f, "{} {}", path, null_check_type.symbol()),
            Self::And(operands) => write_boolean_operands(f, operands, BooleanOperator::And),
            Self::Or(operands) => write_boolean_operands(f, operands, BooleanOperator::Or),
            Self::Not(operand) => write!(f, "not ({})", operand),
            Self::Custom {
                payload,
                description,
            } => write!(f, "{}", description.as_deref().unwrap_or(payload)),
        }
    }
}

fn write_boolean_operands<T: Display>(
    f: &mut Formatter<'_>,
    operands: &[BooleanExpression<T>],
    operator: BooleanOperator,
) -> std::fmt::Result {
    for (index, operand) in operands.iter().enumerate() {
        if index > 0 {
            write!(f, " {} ", operator.symbol())?;
        }
        write!(f, "({})", operand)?;
    }
    Ok(())
}
