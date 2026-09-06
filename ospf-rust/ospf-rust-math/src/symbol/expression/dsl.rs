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
