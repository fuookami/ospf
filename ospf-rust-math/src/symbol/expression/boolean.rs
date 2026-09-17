//! 布尔表达式 AST
//! Boolean expression AST

use super::dsl::{and_pair, or_pair};
use super::normalize::{NormalizeConfig, boolean_structural_key, normalize_boolean_expression};
use super::operators::*;
use super::property_path::PropertyPath;
use super::scalar::ScalarExpression;
use super::value::ExpressionValue;
use crate::Trivalent;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::ops::{BitAnd, BitOr, Not as StdNot};

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

// ============================================================================
// 布尔表达式测试 / Boolean expression tests
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::symbol::expression::path_owned_symbol;

    use super::*;

    /// 构造 `age > 18` 运行时常量比较叶子。
    /// Build the `age > 18` runtime constant comparison leaf.
    fn age_over_eighteen() -> ParsedBooleanExpression {
        BooleanExpression::gt(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::Number(18.0)),
        )
    }

    /// 构造 `status = 'active'` 运行时常量比较叶子。
    /// Build the `status = 'active'` runtime constant comparison leaf.
    fn status_active() -> ParsedBooleanExpression {
        BooleanExpression::eq(
            ScalarExpression::reference("status"),
            ScalarExpression::constant(ExpressionValue::String("active".to_string())),
        )
    }

    // ========================================================================
    // 构造函数 / Constructors
    // ========================================================================

    #[test]
    fn constant_constructors_cover_trivalent_values() {
        assert_eq!(
            BooleanExpression::<ExpressionValue>::true_constant(),
            BooleanExpression::Constant(Trivalent::True)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::false_constant(),
            BooleanExpression::Constant(Trivalent::False)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::unknown_constant(),
            BooleanExpression::Constant(Trivalent::Unknown)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::constant(Trivalent::False),
            BooleanExpression::Constant(Trivalent::False)
        );
    }

    #[test]
    fn comparison_constructors_map_to_expected_operators() {
        let left = || ScalarExpression::<ExpressionValue>::reference("age");
        let right = || ScalarExpression::constant(ExpressionValue::Number(18.0));

        let cases = [
            (BooleanExpression::eq(left(), right()), ComparisonOperator::Eq),
            (BooleanExpression::ne(left(), right()), ComparisonOperator::Ne),
            (BooleanExpression::lt(left(), right()), ComparisonOperator::Lt),
            (BooleanExpression::le(left(), right()), ComparisonOperator::Le),
            (BooleanExpression::gt(left(), right()), ComparisonOperator::Gt),
            (BooleanExpression::ge(left(), right()), ComparisonOperator::Ge),
            (
                BooleanExpression::comparison(
                    ComparisonOperator::Ge,
                    left(),
                    right(),
                ),
                ComparisonOperator::Ge,
            ),
        ];

        for (expression, expected) in cases {
            let BooleanExpression::Comparison { operator, .. } = expression else {
                panic!("expected comparison expression");
            };
            assert_eq!(operator, expected);
        }
    }

    #[test]
    fn in_and_pattern_match_constructors_store_flags_and_operands() {
        let in_expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
                ScalarExpression::constant(ExpressionValue::String("pending".to_string())),
            ],
            false,
        );
        let not_in_expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::String(
                "archived".to_string(),
            ))],
            true,
        );
        let pattern = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
            PatternMatchMode::Like,
            false,
        );

        let BooleanExpression::In {
            value,
            candidates,
            negated,
        } = in_expression
        else {
            panic!("expected in expression");
        };
        assert_eq!(value, ScalarExpression::reference("status"));
        assert_eq!(candidates.len(), 2);
        assert!(!negated);

        let BooleanExpression::In { negated, .. } = not_in_expression else {
            panic!("expected negated in expression");
        };
        assert!(negated);

        let BooleanExpression::PatternMatch {
            value,
            pattern: pattern_operand,
            mode,
            negated,
        } = pattern
        else {
            panic!("expected pattern match expression");
        };
        assert_eq!(value, ScalarExpression::reference("name"));
        assert_eq!(
            pattern_operand,
            ScalarExpression::constant(ExpressionValue::String("A%".to_string()))
        );
        assert_eq!(mode, PatternMatchMode::Like);
        assert!(!negated);
    }

    #[test]
    fn null_check_and_custom_constructors_store_payloads() {
        assert_eq!(
            BooleanExpression::<ExpressionValue>::is_null("deleted_at"),
            BooleanExpression::null_check("deleted_at", NullCheckType::IsNull)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::is_not_null("profile.email"),
            BooleanExpression::null_check("profile.email", NullCheckType::IsNotNull)
        );

        let custom = BooleanExpression::<ExpressionValue>::custom(
            "raw",
            Some("described".to_string()),
        );
        assert_eq!(
            custom,
            BooleanExpression::Custom {
                payload: "raw".to_string(),
                description: Some("described".to_string()),
            }
        );
    }

    #[test]
    fn and_or_and_not_constructors_keep_operands() {
        let and_expression =
            BooleanExpression::and(vec![age_over_eighteen(), status_active()]);
        let or_expression = BooleanExpression::or(vec![age_over_eighteen(), status_active()]);
        let not_expression = BooleanExpression::not_expr(age_over_eighteen());

        assert_eq!(
            and_expression,
            BooleanExpression::And(vec![age_over_eighteen(), status_active()])
        );
        assert_eq!(
            or_expression,
            BooleanExpression::Or(vec![age_over_eighteen(), status_active()])
        );
        assert_eq!(
            not_expression,
            BooleanExpression::Not(Box::new(age_over_eighteen()))
        );
    }

    #[test]
    #[should_panic(expected = "And expression requires at least one operand")]
    fn and_with_no_operands_panics() {
        let _ = BooleanExpression::<ExpressionValue>::and(Vec::new());
    }

    #[test]
    #[should_panic(expected = "Or expression requires at least one operand")]
    fn or_with_no_operands_panics() {
        let _ = BooleanExpression::<ExpressionValue>::or(Vec::new());
    }

    // ========================================================================
    // 结构查询 / Structure queries
    // ========================================================================

    #[test]
    fn type_name_covers_every_variant() {
        assert_eq!(
            BooleanExpression::<ExpressionValue>::true_constant().type_name(),
            "BooleanConstant"
        );
        assert_eq!(age_over_eighteen().type_name(), "Comparison");
        assert_eq!(
            BooleanExpression::<ExpressionValue>::is_null("a").type_name(),
            "NullCheck"
        );
        assert_eq!(
            BooleanExpression::and(vec![age_over_eighteen()]).type_name(),
            "And"
        );
        assert_eq!(
            BooleanExpression::or(vec![age_over_eighteen()]).type_name(),
            "Or"
        );
        assert_eq!(
            BooleanExpression::not_expr(age_over_eighteen()).type_name(),
            "Not"
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::custom("raw", None).type_name(),
            "Custom"
        );
    }

    #[test]
    fn type_name_marks_negated_in_and_pattern_match() {
        let in_expression = || {
            BooleanExpression::in_expr(
                ScalarExpression::reference("status"),
                vec![ScalarExpression::constant(ExpressionValue::String(
                    "active".to_string(),
                ))],
                false,
            )
        };
        let pattern_match = |negated| {
            BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
                PatternMatchMode::Like,
                negated,
            )
        };

        assert_eq!(in_expression().type_name(), "In");
        let BooleanExpression::In {
            value,
            candidates,
            negated,
        } = in_expression()
        else {
            panic!("expected in expression");
        };
        let negated_in = BooleanExpression::in_expr(value, candidates, !negated);
        assert_eq!(negated_in.type_name(), "NotIn");

        assert_eq!(pattern_match(false).type_name(), "PatternMatch");
        assert_eq!(pattern_match(true).type_name(), "NotPatternMatch");
    }

    #[test]
    fn is_constant_requires_constant_leaves() {
        let constant_pair = || {
            BooleanExpression::eq(
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )
        };

        assert!(BooleanExpression::<ExpressionValue>::true_constant().is_constant());
        assert!(constant_pair().is_constant());
        assert!(
            BooleanExpression::and(vec![constant_pair(), constant_pair()]).is_constant(),
            "全常量操作数的 And 是常量 / An And over constant operands is constant"
        );
        assert!(!age_over_eighteen().is_constant());
        assert!(!BooleanExpression::<ExpressionValue>::is_null("a").is_constant());
        assert!(!BooleanExpression::<ExpressionValue>::custom("raw", None).is_constant());
        assert!(!BooleanExpression::not_expr(age_over_eighteen()).is_constant());
    }

    #[test]
    fn is_constant_for_in_and_pattern_match_only_checks_the_value() {
        let in_expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::String(
                "active".to_string(),
            ))],
            false,
        );
        let constant_in = BooleanExpression::in_expr(
            ScalarExpression::constant(ExpressionValue::String("status".to_string())),
            vec![ScalarExpression::constant(ExpressionValue::String(
                "active".to_string(),
            ))],
            false,
        );

        assert!(!in_expression.is_constant());
        assert!(constant_in.is_constant());
    }

    #[test]
    fn is_pure_logical_accepts_only_constants_and_logical_nodes() {
        assert!(BooleanExpression::<ExpressionValue>::true_constant().is_pure_logical());
        assert!(BooleanExpression::and(vec![
            BooleanExpression::<ExpressionValue>::true_constant(),
            BooleanExpression::not_expr(BooleanExpression::<ExpressionValue>::false_constant()),
        ])
        .is_pure_logical());

        assert!(!age_over_eighteen().is_pure_logical());
        assert!(!BooleanExpression::<ExpressionValue>::is_null("a").is_pure_logical());
        assert!(!BooleanExpression::<ExpressionValue>::custom("raw", None).is_pure_logical());
        assert!(!BooleanExpression::and(vec![
            BooleanExpression::<ExpressionValue>::true_constant(),
            age_over_eighteen(),
        ])
        .is_pure_logical());
    }

    #[test]
    fn collect_references_covers_every_leaf_kind() {
        let expression = BooleanExpression::and(vec![
            age_over_eighteen(),
            BooleanExpression::in_expr(
                ScalarExpression::reference("status"),
                vec![ScalarExpression::reference("fallback.status")],
                false,
            ),
            BooleanExpression::pattern_match(
                ScalarExpression::symbol_reference(path_owned_symbol("name")),
                ScalarExpression::reference("pattern.source"),
                PatternMatchMode::Exact,
                false,
            ),
            BooleanExpression::is_not_null("profile.email"),
            BooleanExpression::not_expr(BooleanExpression::is_null("deleted_at")),
            BooleanExpression::<ExpressionValue>::custom("raw", None),
        ]);

        let references = expression.collect_references();

        for path in [
            "age",
            "status",
            "fallback.status",
            "name",
            "pattern.source",
            "profile.email",
            "deleted_at",
        ] {
            assert!(
                references.contains(&PropertyPath::parse(path)),
                "missing reference: {path}"
            );
        }
        assert_eq!(references.len(), 7);
    }

    #[test]
    fn collect_references_into_merges_with_existing_paths() {
        let mut references = HashSet::new();
        references.insert(PropertyPath::parse("pre.existing"));

        age_over_eighteen().collect_references_into(&mut references);

        assert_eq!(references.len(), 2);
        assert!(references.contains(&PropertyPath::parse("pre.existing")));
        assert!(references.contains(&PropertyPath::parse("age")));
    }

    #[test]
    fn logical_operator_count_sums_nested_operators() {
        assert_eq!(age_over_eighteen().logical_operator_count(), 0);
        assert_eq!(
            BooleanExpression::and(vec![age_over_eighteen(), status_active()])
                .logical_operator_count(),
            2
        );
        assert_eq!(
            BooleanExpression::not_expr(age_over_eighteen()).logical_operator_count(),
            1
        );

        // and(2) + not(1) + or(2) = 5
        let nested = BooleanExpression::or(vec![
            BooleanExpression::and(vec![age_over_eighteen(), status_active()]),
            BooleanExpression::not_expr(BooleanExpression::is_null("deleted_at")),
        ]);
        assert_eq!(nested.logical_operator_count(), 5);
    }

    #[test]
    fn depth_follows_the_deepest_branch() {
        let leaf = age_over_eighteen();
        let level_two = BooleanExpression::not_expr(leaf.clone());
        let level_three = BooleanExpression::and(vec![level_two.clone(), leaf.clone()]);
        let level_four = BooleanExpression::or(vec![level_three.clone(), leaf.clone()]);

        assert_eq!(leaf.depth(), 1);
        assert_eq!(level_two.depth(), 2);
        assert_eq!(level_three.depth(), 3);
        assert_eq!(level_four.depth(), 4);
        assert_eq!(
            BooleanExpression::<ExpressionValue>::is_null("a").depth(),
            1
        );
    }

    // ========================================================================
    // 转换与运算符重载 / Conversions and operator overloading
    // ========================================================================

    #[test]
    fn from_conversions_build_constants() {
        assert_eq!(
            BooleanExpression::<ExpressionValue>::from(true),
            BooleanExpression::Constant(Trivalent::True)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::from(false),
            BooleanExpression::Constant(Trivalent::False)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::from(Some(true)),
            BooleanExpression::Constant(Trivalent::True)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::from(None),
            BooleanExpression::Constant(Trivalent::Unknown)
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::from(Trivalent::Unknown),
            BooleanExpression::Constant(Trivalent::Unknown)
        );
    }

    #[test]
    fn bitand_and_bitor_flatten_same_operator_operands() {
        let a = || BooleanExpression::<ExpressionValue>::is_null("a");
        let b = || BooleanExpression::<ExpressionValue>::is_null("b");
        let c = || BooleanExpression::<ExpressionValue>::is_null("c");

        let and_chain = (a() & b()) & c();
        let BooleanExpression::And(operands) = and_chain else {
            panic!("expected flattened AND");
        };
        assert_eq!(operands.len(), 3);
        assert_eq!(operands[0], a());

        let or_chain = (a() | b()) | c();
        let BooleanExpression::Or(operands) = or_chain else {
            panic!("expected flattened OR");
        };
        assert_eq!(operands.len(), 3);
        assert_eq!(operands[2], c());
    }

    #[test]
    fn bitand_and_bitor_keep_mixed_operator_boundaries() {
        let a = || BooleanExpression::<ExpressionValue>::is_null("a");
        let b = || BooleanExpression::<ExpressionValue>::is_null("b");
        let c = || BooleanExpression::<ExpressionValue>::is_null("c");

        let mixed = (a() & b()) | c();
        let BooleanExpression::Or(operands) = mixed else {
            panic!("expected OR at the top level");
        };
        assert_eq!(operands.len(), 2);
        assert!(matches!(operands[0], BooleanExpression::And(_)));
        assert_eq!(operands[1], c());
    }

    #[test]
    fn not_operator_wraps_the_operand() {
        let expression = !BooleanExpression::<ExpressionValue>::is_null("a");

        let BooleanExpression::Not(operand) = expression else {
            panic!("expected NOT expression");
        };
        assert_eq!(*operand, BooleanExpression::<ExpressionValue>::is_null("a"));
    }

    // ========================================================================
    // 显示 / Display
    // ========================================================================

    #[test]
    fn display_renders_constants_and_comparisons() {
        assert_eq!(
            BooleanExpression::<ExpressionValue>::true_constant().to_string(),
            "true"
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::false_constant().to_string(),
            "false"
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::unknown_constant().to_string(),
            "unknown"
        );
        assert_eq!(age_over_eighteen().to_string(), "age > 18");
        assert_eq!(
            BooleanExpression::<ExpressionValue>::is_not_null("profile.email").to_string(),
            "profile.email is not null"
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::custom("raw", Some("shown".to_string()))
                .to_string(),
            "shown"
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::custom("raw", None).to_string(),
            "raw"
        );
    }

    #[test]
    fn display_renders_logical_operators_with_parentheses() {
        assert_eq!(
            BooleanExpression::and(vec![age_over_eighteen(), status_active()]).to_string(),
            "(age > 18) and (status = active)"
        );
        assert_eq!(
            BooleanExpression::or(vec![age_over_eighteen(), status_active()]).to_string(),
            "(age > 18) or (status = active)"
        );
        assert_eq!(
            BooleanExpression::not_expr(age_over_eighteen()).to_string(),
            "not (age > 18)"
        );
    }

    #[test]
    fn display_renders_in_and_pattern_match() {
        let in_expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
                ScalarExpression::constant(ExpressionValue::String("pending".to_string())),
            ],
            false,
        );
        let not_in_expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::String(
                "archived".to_string(),
            ))],
            true,
        );
        let like = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
            PatternMatchMode::Like,
            false,
        );
        let not_like = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
            PatternMatchMode::Like,
            true,
        );

        assert_eq!(in_expression.to_string(), "status in (active, pending)");
        assert_eq!(not_in_expression.to_string(), "status not in (archived)");
        assert_eq!(like.to_string(), "name Like A%");
        assert_eq!(not_like.to_string(), "name not Like A%");
    }

    // ========================================================================
    // 规范化入口与结构键 / Normalization entry points and structural keys
    // ========================================================================

    #[test]
    fn normalize_entry_points_delegate_to_the_normalize_module() {
        let expression = BooleanExpression::and(vec![
            BooleanExpression::<ExpressionValue>::true_constant(),
            age_over_eighteen(),
            BooleanExpression::and(vec![
                age_over_eighteen(),
                BooleanExpression::<ExpressionValue>::true_constant(),
            ]),
        ]);

        assert_eq!(expression.normalize(), age_over_eighteen());
        assert_eq!(
            expression.normalize_with_config(NormalizeConfig {
                deduplicate: false,
                ..NormalizeConfig::default()
            }),
            BooleanExpression::And(vec![
                age_over_eighteen(),
                age_over_eighteen(),
            ])
        );
    }

    #[test]
    fn structural_key_is_stable_and_structure_sensitive() {
        assert_eq!(age_over_eighteen().structural_key(), "Cmp:Gt:Ref:age:Const:18");
        assert_eq!(
            BooleanExpression::<ExpressionValue>::true_constant().structural_key(),
            "Const:True"
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::is_null("email").structural_key(),
            "Null:IsNull:email"
        );
        assert_eq!(
            BooleanExpression::in_expr(
                ScalarExpression::reference("status"),
                vec![ScalarExpression::constant(ExpressionValue::String(
                    "active".to_string(),
                ))],
                false,
            )
            .structural_key(),
            "In:false:Ref:status:Const:active"
        );
        assert_eq!(
            BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
                PatternMatchMode::Like,
                false,
            )
            .structural_key(),
            "Match:Like:false:Ref:name:Const:A%"
        );
        assert_eq!(
            BooleanExpression::not_expr(age_over_eighteen()).structural_key(),
            "Not:Cmp:Gt:Ref:age:Const:18"
        );
        assert_eq!(
            BooleanExpression::<ExpressionValue>::custom("raw", None).structural_key(),
            "Custom:raw"
        );
        assert_eq!(
            BooleanExpression::and(vec![age_over_eighteen(), status_active()]).structural_key(),
            "And:Cmp:Gt:Ref:age:Const:18,Cmp:Eq:Ref:status:Const:active"
        );

        assert_eq!(age_over_eighteen().structural_key(), age_over_eighteen().structural_key());
        assert_ne!(
            age_over_eighteen().structural_key(),
            BooleanExpression::ge(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
            .structural_key()
        );
    }
}
