//! 表达式 DSL 与便捷构造函数
//! Expression DSL and convenience constructors

use super::boolean::{BooleanExpression, ParsedBooleanExpression};
use super::operators::*;
use super::property_path::PropertyPath;
use super::scalar::ScalarExpression;
use super::value::ExpressionValue;
use crate::Trivalent;
use std::marker::PhantomData;

/// 标量表达式构建扩展。
/// Scalar expression builder extension.
pub trait ScalarExpressionDsl<T>: Sized {
    /// 创建标量比较表达式。
    /// Create scalar comparison expression.
    fn compare_expr(
        self,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T>;

    /// 创建相等比较表达式。
    /// Create equal comparison expression.
    fn eq_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建不等比较表达式。
    /// Create not-equal comparison expression.
    fn ne_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建小于比较表达式。
    /// Create less-than comparison expression.
    fn lt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建小于等于比较表达式。
    /// Create less-than-or-equal comparison expression.
    fn le_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建大于比较表达式。
    /// Create greater-than comparison expression.
    fn gt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建大于等于比较表达式。
    /// Create greater-than-or-equal comparison expression.
    fn ge_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;
}

impl<T> ScalarExpressionDsl<T> for ScalarExpression<T> {
    fn compare_expr(
        self,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        BooleanExpression::comparison(operator, self, value.into())
    }

    fn eq_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Eq, value)
    }

    fn ne_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Ne, value)
    }

    fn lt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Lt, value)
    }

    fn le_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Le, value)
    }

    fn gt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Gt, value)
    }

    fn ge_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Ge, value)
    }
}

/// 路径表达式构建器。
/// Path expression builder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathBuilder<T = ExpressionValue> {
    path: PropertyPath,
    _marker: PhantomData<fn() -> T>,
}

impl<T> PathBuilder<T> {
    /// 创建路径表达式构建器。
    /// Create a path expression builder.
    pub fn new(path: impl Into<PropertyPath>) -> Self {
        Self {
            path: path.into(),
            _marker: PhantomData,
        }
    }

    /// 解析路径表达式构建器。
    /// Parse a path expression builder.
    pub fn parse(path: impl AsRef<str>) -> Self {
        Self::new(PropertyPath::parse(path))
    }

    /// 切换为另一个标量值类型。
    /// Switch to another scalar value type.
    pub fn typed<U>(&self) -> PathBuilder<U> {
        PathBuilder::new(self.path.clone())
    }

    /// 获取属性路径。
    /// Get property path.
    pub fn path(&self) -> &PropertyPath {
        &self.path
    }

    /// 转换为属性路径。
    /// Convert into property path.
    pub fn into_path(self) -> PropertyPath {
        self.path
    }

    /// 转换为标量引用表达式。
    /// Convert to scalar reference expression.
    pub fn as_scalar(&self) -> ScalarExpression<T> {
        ScalarExpression::reference(self.path.clone())
    }

    /// 创建比较表达式。
    /// Create comparison expression.
    pub fn compare(
        &self,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        BooleanExpression::comparison(operator, self.as_scalar(), value.into())
    }

    /// 创建相等比较表达式。
    /// Create equal comparison expression.
    pub fn eq(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Eq, value)
    }

    /// 创建不等比较表达式。
    /// Create not-equal comparison expression.
    pub fn ne(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Ne, value)
    }

    /// 创建小于比较表达式。
    /// Create less-than comparison expression.
    pub fn lt(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Lt, value)
    }

    /// 创建小于等于比较表达式。
    /// Create less-than-or-equal comparison expression.
    pub fn le(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Le, value)
    }

    /// 创建大于比较表达式。
    /// Create greater-than comparison expression.
    pub fn gt(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Gt, value)
    }

    /// 创建大于等于比较表达式。
    /// Create greater-than-or-equal comparison expression.
    pub fn ge(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Ge, value)
    }

    /// 创建集合成员判断表达式。
    /// Create set membership expression.
    pub fn in_values<I, V>(&self, values: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.set_membership(values, false)
    }

    /// 创建非集合成员判断表达式。
    /// Create negated set membership expression.
    pub fn not_in_values<I, V>(&self, values: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.set_membership(values, true)
    }

    /// 创建空值检查表达式。
    /// Create is-null expression.
    pub fn is_null(&self) -> BooleanExpression<T> {
        BooleanExpression::is_null(self.path.clone())
    }

    /// 创建非空检查表达式。
    /// Create is-not-null expression.
    pub fn is_not_null(&self) -> BooleanExpression<T> {
        BooleanExpression::is_not_null(self.path.clone())
    }

    /// 创建模式匹配表达式。
    /// Create pattern match expression.
    pub fn pattern_match(
        &self,
        pattern: impl Into<ScalarExpression<T>>,
        mode: PatternMatchMode,
        negated: bool,
    ) -> BooleanExpression<T> {
        BooleanExpression::pattern_match(self.as_scalar(), pattern.into(), mode, negated)
    }

    /// 创建 LIKE 模式匹配表达式。
    /// Create LIKE pattern match expression.
    pub fn like(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Like, false)
    }

    /// 创建精确模式匹配表达式。
    /// Create exact pattern match expression.
    pub fn like_exact(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Exact, false)
    }

    /// 创建前缀模式匹配表达式。
    /// Create prefix pattern match expression.
    pub fn like_prefix(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Prefix, false)
    }

    /// 创建后缀模式匹配表达式。
    /// Create suffix pattern match expression.
    pub fn like_suffix(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Suffix, false)
    }

    /// 创建包含模式匹配表达式。
    /// Create contains pattern match expression.
    pub fn like_contains(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Contains, false)
    }

    /// 创建正则模式匹配表达式。
    /// Create regex pattern match expression.
    pub fn regex(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Regex, false)
    }

    /// 创建否定 LIKE 模式匹配表达式。
    /// Create negated LIKE pattern match expression.
    pub fn not_like(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Like, true)
    }

    fn set_membership<I, V>(&self, values: I, negated: bool) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        BooleanExpression::in_expr(
            self.as_scalar(),
            values.into_iter().map(Into::into).collect(),
            negated,
        )
    }
}

impl<T> From<PathBuilder<T>> for ScalarExpression<T> {
    fn from(value: PathBuilder<T>) -> Self {
        ScalarExpression::reference(value.path)
    }
}

impl<T> From<&PathBuilder<T>> for ScalarExpression<T> {
    fn from(value: &PathBuilder<T>) -> Self {
        value.as_scalar()
    }
}

/// 布尔表达式组合扩展。
/// Boolean expression composition extension.
pub trait BooleanExpressionDsl<T>: Sized {
    /// 逻辑与组合。
    /// Compose with logical AND.
    fn and_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T>;

    /// 逻辑或组合。
    /// Compose with logical OR.
    fn or_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T>;

    /// 逻辑非组合。
    /// Compose with logical NOT.
    fn not_expr(self) -> BooleanExpression<T>;
}

impl<T> BooleanExpressionDsl<T> for BooleanExpression<T> {
    fn and_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T> {
        and_pair(self, other)
    }

    fn or_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T> {
        or_pair(self, other)
    }

    fn not_expr(self) -> BooleanExpression<T> {
        BooleanExpression::not_expr(self)
    }
}

/// 创建默认运行时路径构建器。
/// Create a default runtime path builder.
pub fn path(path: impl AsRef<str>) -> PathBuilder<ExpressionValue> {
    PathBuilder::parse(path)
}

/// 创建类型化路径构建器。
/// Create a typed path builder.
pub fn typed_path<T>(path: impl AsRef<str>) -> PathBuilder<T> {
    PathBuilder::parse(path)
}

/// 创建类型化标量路径引用。
/// Create a typed scalar path reference.
pub fn scalar_path<T>(path: impl Into<PropertyPath>) -> ScalarExpression<T> {
    ScalarExpression::reference(path)
}

/// 创建布尔常量表达式。
/// Create boolean constant expression.
pub fn bool_expr<T>(value: bool) -> BooleanExpression<T> {
    BooleanExpression::constant(Trivalent::from(value))
}

/// 创建三值布尔常量表达式。
/// Create trivalent boolean constant expression.
pub fn trivalent_expr<T>(value: impl Into<Trivalent>) -> BooleanExpression<T> {
    BooleanExpression::constant(value.into())
}

/// 使用闭包创建布尔表达式。
/// Create a boolean expression with a closure.
pub fn boolean_expression<T>(block: impl FnOnce() -> BooleanExpression<T>) -> BooleanExpression<T> {
    block()
}

/// 创建指定名称的标量函数表达式。
/// Create a scalar function expression with the given name.
pub fn scalar_function<T>(
    name: impl Into<String>,
    arguments: impl IntoIterator<Item = ScalarExpression<T>>,
) -> ScalarExpression<T> {
    ScalarExpression::function(name, arguments.into_iter().collect())
}

/// 创建绝对值函数表达式。
/// Create absolute-value function expression.
pub fn abs<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::ABS, [expression.into()])
}

/// 创建小写函数表达式。
/// Create lowercase function expression.
pub fn lower<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::LOWER, [expression.into()])
}

/// 创建大写函数表达式。
/// Create uppercase function expression.
pub fn upper<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::UPPER, [expression.into()])
}

/// 创建裁剪函数表达式。
/// Create trim function expression.
pub fn trim<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::TRIM, [expression.into()])
}

/// 创建长度函数表达式。
/// Create length function expression.
pub fn length<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::LENGTH, [expression.into()])
}

/// 创建合并空值函数表达式。
/// Create coalesce function expression.
pub fn coalesce<T>(
    expressions: impl IntoIterator<Item = ScalarExpression<T>>,
) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::COALESCE, expressions)
}

/// 快速创建比较表达式。
/// Quickly create comparison expression.
pub fn compare<T>(
    path: impl Into<PropertyPath>,
    operator: ComparisonOperator,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    BooleanExpression::comparison(operator, ScalarExpression::reference(path), value.into())
}

/// 快速创建相等比较表达式。
/// Quickly create equal comparison expression.
pub fn eq<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Eq, value)
}

/// 快速创建不等比较表达式。
/// Quickly create not-equal comparison expression.
pub fn ne<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Ne, value)
}

/// 快速创建小于比较表达式。
/// Quickly create less-than comparison expression.
pub fn lt<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Lt, value)
}

/// 快速创建小于等于比较表达式。
/// Quickly create less-than-or-equal comparison expression.
pub fn le<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Le, value)
}

/// 快速创建大于比较表达式。
/// Quickly create greater-than comparison expression.
pub fn gt<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Gt, value)
}

/// 快速创建大于等于比较表达式。
/// Quickly create greater-than-or-equal comparison expression.
pub fn ge<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Ge, value)
}

/// 快速创建集合成员判断表达式。
/// Quickly create set membership expression.
pub fn in_expr<T, I, V>(path: impl Into<PropertyPath>, values: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = V>,
    V: Into<ScalarExpression<T>>,
{
    BooleanExpression::in_expr(
        ScalarExpression::reference(path),
        values.into_iter().map(Into::into).collect(),
        false,
    )
}

/// 快速创建非集合成员判断表达式。
/// Quickly create negated set membership expression.
pub fn not_in_expr<T, I, V>(path: impl Into<PropertyPath>, values: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = V>,
    V: Into<ScalarExpression<T>>,
{
    BooleanExpression::in_expr(
        ScalarExpression::reference(path),
        values.into_iter().map(Into::into).collect(),
        true,
    )
}

/// 快速创建默认运行时空值检查表达式。
/// Quickly create default runtime is-null expression.
pub fn is_null(path: impl Into<PropertyPath>) -> ParsedBooleanExpression {
    BooleanExpression::is_null(path)
}

/// 快速创建默认运行时非空检查表达式。
/// Quickly create default runtime is-not-null expression.
pub fn is_not_null(path: impl Into<PropertyPath>) -> ParsedBooleanExpression {
    BooleanExpression::is_not_null(path)
}

/// 快速创建逻辑与表达式。
/// Quickly create logical AND expression.
pub fn and<T, I>(expressions: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = BooleanExpression<T>>,
{
    let mut operands = Vec::new();
    for expression in expressions {
        if let BooleanExpression::And(items) = expression {
            operands.extend(items);
        } else {
            operands.push(expression);
        }
    }
    BooleanExpression::and(operands)
}

/// 快速创建逻辑或表达式。
/// Quickly create logical OR expression.
pub fn or<T, I>(expressions: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = BooleanExpression<T>>,
{
    let mut operands = Vec::new();
    for expression in expressions {
        if let BooleanExpression::Or(items) = expression {
            operands.extend(items);
        } else {
            operands.push(expression);
        }
    }
    BooleanExpression::or(operands)
}

/// 快速创建逻辑非表达式。
/// Quickly create logical NOT expression.
pub fn not_expr<T>(expression: BooleanExpression<T>) -> BooleanExpression<T> {
    BooleanExpression::not_expr(expression)
}

pub(super) fn and_pair<T>(
    left: BooleanExpression<T>,
    right: BooleanExpression<T>,
) -> BooleanExpression<T> {
    let mut operands = Vec::new();
    if let BooleanExpression::And(items) = left {
        operands.extend(items);
    } else {
        operands.push(left);
    }
    if let BooleanExpression::And(items) = right {
        operands.extend(items);
    } else {
        operands.push(right);
    }
    BooleanExpression::And(operands)
}

pub(super) fn or_pair<T>(
    left: BooleanExpression<T>,
    right: BooleanExpression<T>,
) -> BooleanExpression<T> {
    let mut operands = Vec::new();
    if let BooleanExpression::Or(items) = left {
        operands.extend(items);
    } else {
        operands.push(left);
    }
    if let BooleanExpression::Or(items) = right {
        operands.extend(items);
    } else {
        operands.push(right);
    }
    BooleanExpression::Or(operands)
}

// ============================================================================
// 表达式 DSL 测试 / Expression DSL tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::HashSet;

    use super::*;

    /// 断言布尔表达式是比较表达式并返回其操作符。
    /// Assert the boolean expression is a comparison and return its operator.
    fn comparison_operator(expression: &BooleanExpression<ExpressionValue>) -> ComparisonOperator {
        let BooleanExpression::Comparison { operator, .. } = expression else {
            panic!("expected comparison expression");
        };
        *operator
    }

    /// 构造运行时路径引用标量，避免泛型参数推断歧义。
    /// Build a runtime path reference scalar to avoid ambiguous generic inference.
    fn scalar(path: &str) -> ScalarExpression<ExpressionValue> {
        ScalarExpression::reference(path)
    }

    // ========================================================================
    // 标量表达式 DSL / Scalar expression DSL
    // ========================================================================

    #[test]
    fn scalar_expression_dsl_builds_each_comparison() {
        let scalar = || ScalarExpression::<ExpressionValue>::reference("age");

        assert_eq!(
            comparison_operator(&scalar().eq_expr(18)),
            ComparisonOperator::Eq
        );
        assert_eq!(
            comparison_operator(&scalar().ne_expr(18)),
            ComparisonOperator::Ne
        );
        assert_eq!(
            comparison_operator(&scalar().lt_expr(18)),
            ComparisonOperator::Lt
        );
        assert_eq!(
            comparison_operator(&scalar().le_expr(18)),
            ComparisonOperator::Le
        );
        assert_eq!(
            comparison_operator(&scalar().gt_expr(18)),
            ComparisonOperator::Gt
        );
        assert_eq!(
            comparison_operator(&scalar().ge_expr(18)),
            ComparisonOperator::Ge
        );
    }

    #[test]
    fn scalar_expression_dsl_compare_expr_uses_explicit_operator() {
        let expression = ScalarExpression::<ExpressionValue>::reference("status")
            .compare_expr(ComparisonOperator::Ne, "active");

        assert_eq!(
            expression,
            BooleanExpression::ne(
                ScalarExpression::reference("status"),
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
            )
        );
    }

    #[test]
    fn scalar_expression_dsl_accepts_path_references_as_operands() {
        let limit = ScalarExpression::<ExpressionValue>::reference("limit");

        assert_eq!(
            limit.gt_expr(ScalarExpression::<ExpressionValue>::reference("score")),
            BooleanExpression::gt(
                ScalarExpression::reference("limit"),
                ScalarExpression::reference("score"),
            )
        );
    }

    // ========================================================================
    // 路径构建器 / Path builder
    // ========================================================================

    #[test]
    fn path_builder_constructors_keep_the_same_path() {
        let from_path = PathBuilder::<ExpressionValue>::new(PropertyPath::parse("user.age"));
        let from_str = PathBuilder::<ExpressionValue>::parse("user.age");
        let from_helper = path("user.age");

        assert_eq!(from_path, from_str);
        assert_eq!(from_path, from_helper);
        assert_eq!(from_path.path(), &PropertyPath::parse("user.age"));
        assert_eq!(from_path.clone().into_path(), PropertyPath::parse("user.age"));
    }

    #[test]
    fn path_builder_typed_switches_value_type_and_keeps_path() {
        let runtime = path("user.age");
        let typed = runtime.typed::<i32>();

        assert_eq!(typed.path(), runtime.path());
        assert_eq!(typed.as_scalar(), ScalarExpression::<i32>::reference("user.age"));
    }

    #[test]
    fn path_builder_as_scalar_builds_reference_expression() {
        let builder = path("user.age");

        assert_eq!(
            builder.as_scalar(),
            ScalarExpression::<ExpressionValue>::reference("user.age")
        );
        assert!(builder.as_scalar().contains_reference());
    }

    #[test]
    fn path_builder_comparison_methods_match_operators() {
        let builder = path("age");

        assert_eq!(
            comparison_operator(&builder.compare(ComparisonOperator::Le, 18)),
            ComparisonOperator::Le
        );
        assert_eq!(comparison_operator(&builder.eq(18)), ComparisonOperator::Eq);
        assert_eq!(comparison_operator(&builder.ne(18)), ComparisonOperator::Ne);
        assert_eq!(comparison_operator(&builder.lt(18)), ComparisonOperator::Lt);
        assert_eq!(comparison_operator(&builder.le(18)), ComparisonOperator::Le);
        assert_eq!(comparison_operator(&builder.gt(18)), ComparisonOperator::Gt);
        assert_eq!(comparison_operator(&builder.ge(18)), ComparisonOperator::Ge);

        assert_eq!(
            builder.gt(18),
            BooleanExpression::gt(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
        );
    }

    #[test]
    fn path_builder_comparison_accepts_path_references_on_both_sides() {
        let expression = typed_path::<i32>("score").lt(typed_path::<i32>("limit"));

        assert_eq!(
            expression,
            BooleanExpression::lt(
                ScalarExpression::reference("score"),
                ScalarExpression::reference("limit"),
            )
        );
    }

    #[test]
    fn path_builder_in_values_and_not_in_values_store_negation() {
        let builder = path("status");

        let in_expression = builder.in_values(["active", "pending"]);
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

        let not_in_expression = builder.not_in_values(["archived"]);
        let BooleanExpression::In {
            value,
            candidates,
            negated,
        } = not_in_expression
        else {
            panic!("expected in expression");
        };
        assert_eq!(value, ScalarExpression::reference("status"));
        assert_eq!(candidates.len(), 1);
        assert!(negated);
    }

    #[test]
    fn path_builder_null_checks_use_the_builder_path() {
        assert_eq!(
            path("deleted_at").is_null(),
            BooleanExpression::is_null("deleted_at")
        );
        assert_eq!(
            path("profile.email").is_not_null(),
            BooleanExpression::is_not_null("profile.email")
        );
        assert_eq!(path("deleted_at").is_null().type_name(), "NullCheck");
    }

    #[test]
    fn path_builder_pattern_helpers_map_to_expected_modes() {
        let builder = path("name");
        let expected = [
            (builder.like("A%"), PatternMatchMode::Like),
            (builder.like_exact("Alice"), PatternMatchMode::Exact),
            (builder.like_prefix("Al"), PatternMatchMode::Prefix),
            (builder.like_suffix("ce"), PatternMatchMode::Suffix),
            (builder.like_contains("lic"), PatternMatchMode::Contains),
            (builder.regex("^A"), PatternMatchMode::Regex),
        ];

        for (expression, mode) in expected {
            let BooleanExpression::PatternMatch {
                value,
                pattern: _,
                mode: actual_mode,
                negated,
            } = expression
            else {
                panic!("expected pattern match expression");
            };
            assert_eq!(value, ScalarExpression::reference("name"));
            assert_eq!(actual_mode, mode);
            assert!(!negated);
        }
    }

    #[test]
    fn path_builder_pattern_match_sets_negation_flags() {
        let builder = path("name");

        let negated = builder.pattern_match("A%", PatternMatchMode::Like, true);
        let BooleanExpression::PatternMatch { negated: flag, .. } = negated else {
            panic!("expected pattern match expression");
        };
        assert!(flag);

        let not_like = builder.not_like("A%");
        let BooleanExpression::PatternMatch {
            mode,
            negated: flag,
            ..
        } = not_like
        else {
            panic!("expected pattern match expression");
        };
        assert_eq!(mode, PatternMatchMode::Like);
        assert!(flag);
    }

    #[test]
    fn path_builder_converts_into_scalar_expression() {
        let builder = path("delta");

        assert_eq!(
            ScalarExpression::<ExpressionValue>::from(builder.clone()),
            ScalarExpression::reference("delta")
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::from(&builder),
            ScalarExpression::reference("delta")
        );
        // 转换后原构建器仍可使用 / The builder stays usable after conversion
        assert_eq!(builder.path().value(), "delta");
    }

    #[test]
    fn path_builder_equality_and_hashing_follow_the_path() {
        let mut builders = HashSet::new();
        builders.insert(typed_path::<i32>("a.b"));
        builders.insert(typed_path::<i32>("a.b"));
        builders.insert(typed_path::<i32>("a.c"));

        assert_eq!(builders.len(), 2);
        assert_eq!(typed_path::<i32>("a.b"), typed_path::<i32>("a.b"));
        assert_ne!(typed_path::<i32>("a.b"), typed_path::<i32>("a.c"));
    }

    // ========================================================================
    // 布尔表达式 DSL / Boolean expression DSL
    // ========================================================================

    #[test]
    fn boolean_expression_dsl_composes_logically() {
        let age = || path("age").ge(18);
        let status = || path("status").eq("active");

        let and_expression = age().and_expr(status());
        let BooleanExpression::And(operands) = and_expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);

        let or_expression = age().or_expr(status());
        let BooleanExpression::Or(operands) = or_expression else {
            panic!("expected OR expression");
        };
        assert_eq!(operands.len(), 2);

        let not_expression = age().not_expr();
        assert_eq!(not_expression, BooleanExpression::not_expr(age()));
        assert_eq!(not_expression.type_name(), "Not");
    }

    #[test]
    fn boolean_expression_dsl_flattens_same_operator_pairs() {
        let a = || path("a").is_null();
        let b = || path("b").is_null();
        let c = || path("c").is_null();

        let and_chain = a().and_expr(b()).and_expr(c());
        let BooleanExpression::And(operands) = and_chain else {
            panic!("expected flattened AND");
        };
        assert_eq!(operands.len(), 3);
        assert_eq!(operands[2], c());

        let or_chain = a().or_expr(b()).or_expr(c());
        let BooleanExpression::Or(operands) = or_chain else {
            panic!("expected flattened OR");
        };
        assert_eq!(operands.len(), 3);
    }

    // ========================================================================
    // 便捷构造函数 / Convenience constructors
    // ========================================================================

    #[test]
    fn path_and_typed_path_helpers_build_matching_builders() {
        assert_eq!(path("user.age"), PathBuilder::<ExpressionValue>::parse("  user.age  "));
        assert_eq!(typed_path::<i32>("user.age").path(), path("user.age").path());
        assert_eq!(
            scalar_path::<ExpressionValue>("user.age"),
            ScalarExpression::reference("user.age")
        );
    }

    #[test]
    fn bool_expr_and_trivalent_expr_build_constants() {
        assert_eq!(
            bool_expr::<ExpressionValue>(true),
            BooleanExpression::Constant(Trivalent::True)
        );
        assert_eq!(
            bool_expr::<ExpressionValue>(false),
            BooleanExpression::Constant(Trivalent::False)
        );
        assert_eq!(
            trivalent_expr::<ExpressionValue>(Trivalent::Unknown),
            BooleanExpression::Constant(Trivalent::Unknown)
        );
        assert_eq!(
            trivalent_expr::<ExpressionValue>(true),
            BooleanExpression::Constant(Trivalent::True)
        );
    }

    #[test]
    fn boolean_expression_helper_invokes_the_closure_exactly_once() {
        let calls = Cell::new(0);

        let expression = boolean_expression(|| {
            calls.set(calls.get() + 1);
            BooleanExpression::<ExpressionValue>::true_constant()
        });

        assert_eq!(calls.get(), 1);
        assert_eq!(expression, BooleanExpression::<ExpressionValue>::true_constant());
    }

    #[test]
    fn scalar_function_helper_builds_named_function_expression() {
        let expression = scalar_function("max", [scalar("a"), scalar("b")]);

        let ScalarExpression::Function { name, arguments } = expression else {
            panic!("expected function expression");
        };
        assert_eq!(name, "max");
        assert_eq!(arguments.len(), 2);
    }

    #[test]
    fn string_and_numeric_helpers_build_expected_function_names() {
        let helpers = [
            (abs::<ExpressionValue>(scalar("a")), ScalarFunctionNames::ABS),
            (lower::<ExpressionValue>(scalar("a")), ScalarFunctionNames::LOWER),
            (upper::<ExpressionValue>(scalar("a")), ScalarFunctionNames::UPPER),
            (trim::<ExpressionValue>(scalar("a")), ScalarFunctionNames::TRIM),
            (length::<ExpressionValue>(scalar("a")), ScalarFunctionNames::LENGTH),
        ];

        for (expression, expected) in helpers {
            let ScalarExpression::Function { name, arguments } = expression else {
                panic!("expected function expression");
            };
            assert_eq!(name, expected);
            assert_eq!(arguments.len(), 1);
        }
    }

    #[test]
    fn helpers_accept_path_builders_directly() {
        let expression = abs::<ExpressionValue>(path("delta")).gt_expr(0);
        let BooleanExpression::Comparison { left, .. } = expression else {
            panic!("expected comparison expression");
        };
        assert_eq!(
            left,
            ScalarExpression::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::reference("delta")],
            )
        );

        let lower_expression = lower::<ExpressionValue>(path("status")).eq_expr("active");
        let BooleanExpression::Comparison { left, .. } = lower_expression else {
            panic!("expected comparison expression");
        };
        assert_eq!(
            left,
            ScalarExpression::function(
                ScalarFunctionNames::LOWER,
                vec![ScalarExpression::reference("status")],
            )
        );
    }

    #[test]
    fn coalesce_helper_keeps_argument_order() {
        let expression = coalesce([
            scalar("nickname"),
            scalar("name"),
            ScalarExpression::constant(ExpressionValue::String("fallback".to_string())),
        ]);

        let ScalarExpression::Function { name, arguments } = expression else {
            panic!("expected function expression");
        };
        assert_eq!(name, ScalarFunctionNames::COALESCE);
        assert_eq!(
            arguments,
            vec![
                ScalarExpression::reference("nickname"),
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("fallback".to_string())),
            ]
        );
    }

    #[test]
    fn quick_comparison_helpers_use_the_given_path() {
        let helpers = [
            (
                compare::<ExpressionValue>("age", ComparisonOperator::Gt, 18),
                ComparisonOperator::Gt,
            ),
            (eq::<ExpressionValue>("age", 18), ComparisonOperator::Eq),
            (ne::<ExpressionValue>("age", 18), ComparisonOperator::Ne),
            (lt::<ExpressionValue>("age", 18), ComparisonOperator::Lt),
            (le::<ExpressionValue>("age", 18), ComparisonOperator::Le),
            (gt::<ExpressionValue>("age", 18), ComparisonOperator::Gt),
            (ge::<ExpressionValue>("age", 18), ComparisonOperator::Ge),
        ];

        for (expression, expected) in helpers {
            let BooleanExpression::Comparison { operator, left, .. } = expression else {
                panic!("expected comparison expression");
            };
            assert_eq!(operator, expected);
            assert_eq!(left, ScalarExpression::reference("age"));
        }
    }

    #[test]
    fn quick_in_helpers_set_negation() {
        let in_expression: ParsedBooleanExpression = in_expr("status", ["active", "pending"]);
        let not_in_expression: ParsedBooleanExpression = not_in_expr("status", ["archived"]);

        let BooleanExpression::In {
            candidates,
            negated,
            ..
        } = in_expression
        else {
            panic!("expected in expression");
        };
        assert_eq!(candidates.len(), 2);
        assert!(!negated);

        let BooleanExpression::In { negated, .. } = not_in_expression else {
            panic!("expected in expression");
        };
        assert!(negated);
    }

    #[test]
    fn quick_null_check_helpers_build_runtime_expressions() {
        assert_eq!(is_null("deleted_at"), BooleanExpression::is_null("deleted_at"));
        assert_eq!(
            is_not_null("profile.email"),
            BooleanExpression::is_not_null("profile.email")
        );
    }

    #[test]
    fn quick_and_or_flatten_nested_same_operator_expressions() {
        let a = || is_null("a");
        let b = || is_null("b");
        let c = || is_null("c");

        let and_expression = and([and([a(), b()]), c()]);
        let BooleanExpression::And(operands) = and_expression else {
            panic!("expected flattened AND");
        };
        assert_eq!(operands.len(), 3);

        let or_expression = or([or([a(), b()]), c()]);
        let BooleanExpression::Or(operands) = or_expression else {
            panic!("expected flattened OR");
        };
        assert_eq!(operands.len(), 3);
        assert_eq!(operands[0], a());
    }

    #[test]
    #[should_panic(expected = "And expression requires at least one operand")]
    fn quick_and_panics_on_empty_input() {
        let _ = and(Vec::<ParsedBooleanExpression>::new());
    }

    #[test]
    #[should_panic(expected = "Or expression requires at least one operand")]
    fn quick_or_panics_on_empty_input() {
        let _ = or(Vec::<ParsedBooleanExpression>::new());
    }

    #[test]
    fn quick_not_expr_wraps_the_expression() {
        let expression = not_expr(eq::<ExpressionValue>("status", "deleted"));

        let BooleanExpression::Not(operand) = expression else {
            panic!("expected NOT expression");
        };
        assert_eq!(*operand, eq::<ExpressionValue>("status", "deleted"));
    }

    // ========================================================================
    // DSL 与解析器等价性 / DSL and parser equivalence
    //
    // 这些用例依赖 `parser` feature，未启用时整体跳过。
    // These cases depend on the `parser` feature and are skipped when it is disabled.
    // ========================================================================

    #[cfg(feature = "parser")]
    #[test]
    fn dsl_and_parser_agree_on_simple_comparison() {
        let dsl_expression = path("age").gt(18);
        let parser_expression =
            crate::symbol::expression::parse_boolean_expression("age > 18").unwrap();

        assert_eq!(dsl_expression, parser_expression);
    }

    #[cfg(feature = "parser")]
    #[test]
    fn dsl_and_parser_agree_on_and_expression() {
        let dsl_expression = path("age")
            .gt(18)
            .and_expr(path("status").eq("active"));
        let parser_expression = crate::symbol::expression::parse_boolean_expression(
            "age > 18 and status = 'active'",
        )
        .unwrap();

        assert_eq!(dsl_expression, parser_expression);
    }

    #[cfg(feature = "parser")]
    #[test]
    fn dsl_and_parser_agree_on_in_expression() {
        let dsl_expression = path("status").in_values(["active", "pending"]);
        let parser_expression = crate::symbol::expression::parse_boolean_expression(
            "status in ('active', 'pending')",
        )
        .unwrap();

        assert_eq!(dsl_expression, parser_expression);
    }

    #[cfg(feature = "parser")]
    #[test]
    fn dsl_and_parser_agree_on_null_check() {
        let dsl_expression = path("profile.email").is_not_null();
        let parser_expression =
            crate::symbol::expression::parse_boolean_expression("profile.email is not null")
                .unwrap();

        assert_eq!(dsl_expression, parser_expression);
    }

    #[cfg(feature = "parser")]
    #[test]
    fn dsl_and_parser_agree_on_pattern_match() {
        let dsl_expression = path("name").like("A%");
        let parser_expression =
            crate::symbol::expression::parse_boolean_expression("name like 'A%'").unwrap();

        assert_eq!(dsl_expression, parser_expression);
    }

    #[cfg(feature = "parser")]
    #[test]
    fn dsl_and_parser_agree_on_nested_path_reference() {
        let dsl_expression = path("user.address.city").eq("Beijing");
        let parser_expression = crate::symbol::expression::parse_boolean_expression(
            "user.address.city = 'Beijing'",
        )
        .unwrap();

        assert_eq!(dsl_expression, parser_expression);
    }
}
