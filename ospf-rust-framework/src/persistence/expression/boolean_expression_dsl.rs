//! 布尔表达式顶层 DSL
//! Top-level boolean expression DSL

use ospf_rust_math::Trivalent;
use ospf_rust_math::symbol::{
    BooleanExpression, ComparisonOperator, ExpressionValue, NullCheckType, ParsedBooleanExpression,
    PropertyPath, ScalarExpression,
};

/// 布尔表达式收集作用域。
/// Boolean expression collection scope.
#[derive(Debug, Clone, PartialEq)]
pub struct BooleanExpressionScope<T = ExpressionValue> {
    expressions: Vec<BooleanExpression<T>>,
}

impl<T> BooleanExpressionScope<T> {
    /// 创建空作用域。
    /// Create an empty scope.
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
        }
    }

    /// 以 AND 语义构造当前作用域表达式。
    /// Build the current scope expression with AND semantics.
    pub fn build_and(self) -> BooleanExpression<T> {
        combine_typed_and(self.expressions)
    }

    /// 以 OR 语义构造当前作用域表达式。
    /// Build the current scope expression with OR semantics.
    pub fn build_or(self) -> BooleanExpression<T> {
        combine_typed_or(self.expressions)
    }
}

impl<T> Default for BooleanExpressionScope<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BooleanExpressionScope<T>
where
    T: Clone,
{
    /// 构造并收集比较表达式。
    /// Build and collect a comparison expression.
    pub fn compare(
        &mut self,
        path: impl Into<PropertyPath>,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.add(typed_compare(path, operator, value))
    }

    /// 构造并收集等值比较。
    /// Build and collect an equality comparison.
    pub fn eq(
        &mut self,
        path: impl Into<PropertyPath>,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.compare(path, ComparisonOperator::Eq, value)
    }

    /// 构造并收集不等值比较。
    /// Build and collect a not-equal comparison.
    pub fn ne(
        &mut self,
        path: impl Into<PropertyPath>,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.compare(path, ComparisonOperator::Ne, value)
    }

    /// 构造并收集大于比较。
    /// Build and collect a greater-than comparison.
    pub fn gt(
        &mut self,
        path: impl Into<PropertyPath>,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.compare(path, ComparisonOperator::Gt, value)
    }

    /// 构造并收集大于等于比较。
    /// Build and collect a greater-than-or-equal comparison.
    pub fn ge(
        &mut self,
        path: impl Into<PropertyPath>,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.compare(path, ComparisonOperator::Ge, value)
    }

    /// 构造并收集小于比较。
    /// Build and collect a less-than comparison.
    pub fn lt(
        &mut self,
        path: impl Into<PropertyPath>,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.compare(path, ComparisonOperator::Lt, value)
    }

    /// 构造并收集小于等于比较。
    /// Build and collect a less-than-or-equal comparison.
    pub fn le(
        &mut self,
        path: impl Into<PropertyPath>,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.compare(path, ComparisonOperator::Le, value)
    }

    /// 构造并收集集合成员判断。
    /// Build and collect an in-list expression.
    pub fn in_values<I, V>(
        &mut self,
        path: impl Into<PropertyPath>,
        values: I,
    ) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.add(typed_in_values(path, values))
    }

    /// 构造并收集非集合成员判断。
    /// Build and collect a not-in-list expression.
    pub fn not_in_values<I, V>(
        &mut self,
        path: impl Into<PropertyPath>,
        values: I,
    ) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.add(typed_not_in_values(path, values))
    }

    /// 构造并收集字段为空判断。
    /// Build and collect an is-null check.
    pub fn is_null(&mut self, path: impl Into<PropertyPath>) -> BooleanExpression<T> {
        self.add(typed_is_null(path))
    }

    /// 构造并收集字段非空判断。
    /// Build and collect an is-not-null check.
    pub fn is_not_null(&mut self, path: impl Into<PropertyPath>) -> BooleanExpression<T> {
        self.add(typed_is_not_null(path))
    }

    /// 记录表达式并返回原表达式。
    /// Record an expression and return it unchanged.
    pub fn push(&mut self, expression: BooleanExpression<T>) -> BooleanExpression<T> {
        self.add(expression)
    }

    fn add(&mut self, expression: BooleanExpression<T>) -> BooleanExpression<T> {
        self.expressions.push(expression.clone());
        expression
    }
}

impl<T> BooleanExpressionScope<T>
where
    T: Clone + PartialEq,
{
    /// 构造并收集 AND 组合表达式。
    /// Build and collect an AND expression.
    pub fn and<I>(&mut self, expressions: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = BooleanExpression<T>>,
    {
        let expressions = expressions.into_iter().collect::<Vec<_>>();
        self.remove_collected_suffix(&expressions);
        self.add(combine_typed_and(expressions))
    }

    /// 构造并收集 OR 组合表达式。
    /// Build and collect an OR expression.
    pub fn or<I>(&mut self, expressions: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = BooleanExpression<T>>,
    {
        let expressions = expressions.into_iter().collect::<Vec<_>>();
        self.remove_collected_suffix(&expressions);
        self.add(combine_typed_or(expressions))
    }

    fn remove_collected_suffix(&mut self, suffix: &[BooleanExpression<T>]) {
        if suffix.is_empty() || suffix.len() > self.expressions.len() {
            return;
        }

        let offset = self.expressions.len() - suffix.len();
        if self.expressions[offset..] == *suffix {
            self.expressions.truncate(offset);
        }
    }
}

/// 构造默认运行时比较表达式。
/// Build a default runtime comparison expression.
pub fn compare(
    path: impl Into<PropertyPath>,
    operator: ComparisonOperator,
    value: impl Into<ScalarExpression<ExpressionValue>>,
) -> ParsedBooleanExpression {
    typed_compare(path, operator, value)
}

/// 构造类型化比较表达式。
/// Build a typed comparison expression.
pub fn typed_compare<T>(
    path: impl Into<PropertyPath>,
    operator: ComparisonOperator,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    BooleanExpression::comparison(operator, ScalarExpression::reference(path), value.into())
}

/// 构造默认运行时等值比较。
/// Build a default runtime equality comparison.
pub fn eq(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<ExpressionValue>>,
) -> ParsedBooleanExpression {
    compare(path, ComparisonOperator::Eq, value)
}

/// 构造类型化等值比较。
/// Build a typed equality comparison.
pub fn typed_eq<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    typed_compare(path, ComparisonOperator::Eq, value)
}

/// 构造默认运行时不等值比较。
/// Build a default runtime not-equal comparison.
pub fn ne(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<ExpressionValue>>,
) -> ParsedBooleanExpression {
    compare(path, ComparisonOperator::Ne, value)
}

/// 构造默认运行时大于比较。
/// Build a default runtime greater-than comparison.
pub fn gt(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<ExpressionValue>>,
) -> ParsedBooleanExpression {
    compare(path, ComparisonOperator::Gt, value)
}

/// 构造默认运行时大于等于比较。
/// Build a default runtime greater-than-or-equal comparison.
pub fn ge(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<ExpressionValue>>,
) -> ParsedBooleanExpression {
    compare(path, ComparisonOperator::Ge, value)
}

/// 构造默认运行时小于比较。
/// Build a default runtime less-than comparison.
pub fn lt(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<ExpressionValue>>,
) -> ParsedBooleanExpression {
    compare(path, ComparisonOperator::Lt, value)
}

/// 构造默认运行时小于等于比较。
/// Build a default runtime less-than-or-equal comparison.
pub fn le(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<ExpressionValue>>,
) -> ParsedBooleanExpression {
    compare(path, ComparisonOperator::Le, value)
}

/// 构造默认运行时集合成员判断。
/// Build a default runtime in-list expression.
pub fn in_values<I, V>(path: impl Into<PropertyPath>, values: I) -> ParsedBooleanExpression
where
    I: IntoIterator<Item = V>,
    V: Into<ScalarExpression<ExpressionValue>>,
{
    typed_in_values(path, values)
}

/// 构造类型化集合成员判断。
/// Build a typed in-list expression.
pub fn typed_in_values<T, I, V>(path: impl Into<PropertyPath>, values: I) -> BooleanExpression<T>
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

/// 构造默认运行时非集合成员判断。
/// Build a default runtime not-in-list expression.
pub fn not_in_values<I, V>(path: impl Into<PropertyPath>, values: I) -> ParsedBooleanExpression
where
    I: IntoIterator<Item = V>,
    V: Into<ScalarExpression<ExpressionValue>>,
{
    typed_not_in_values(path, values)
}

/// 构造类型化非集合成员判断。
/// Build a typed not-in-list expression.
pub fn typed_not_in_values<T, I, V>(
    path: impl Into<PropertyPath>,
    values: I,
) -> BooleanExpression<T>
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

/// 构造默认运行时字段为空判断。
/// Build a default runtime is-null check.
pub fn is_null(path: impl Into<PropertyPath>) -> ParsedBooleanExpression {
    typed_is_null(path)
}

/// 构造类型化字段为空判断。
/// Build a typed is-null check.
pub fn typed_is_null<T>(path: impl Into<PropertyPath>) -> BooleanExpression<T> {
    BooleanExpression::null_check(path, NullCheckType::IsNull)
}

/// 构造默认运行时字段非空判断。
/// Build a default runtime is-not-null check.
pub fn is_not_null(path: impl Into<PropertyPath>) -> ParsedBooleanExpression {
    typed_is_not_null(path)
}

/// 构造类型化字段非空判断。
/// Build a typed is-not-null check.
pub fn typed_is_not_null<T>(path: impl Into<PropertyPath>) -> BooleanExpression<T> {
    BooleanExpression::null_check(path, NullCheckType::IsNotNull)
}

/// 构造默认运行时 AND 组合表达式。
/// Build a default runtime AND expression.
pub fn and<I>(expressions: I) -> ParsedBooleanExpression
where
    I: IntoIterator<Item = ParsedBooleanExpression>,
{
    combine_typed_and(expressions.into_iter().collect())
}

/// 构造类型化 AND 组合表达式。
/// Build a typed AND expression.
pub fn typed_and<T, I>(expressions: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = BooleanExpression<T>>,
{
    combine_typed_and(expressions.into_iter().collect())
}

/// 构造默认运行时 lambda 风格的 AND 组合表达式。
/// Build a default runtime AND expression with a closure scope.
pub fn and_scope(
    init: impl FnOnce(&mut BooleanExpressionScope<ExpressionValue>),
) -> ParsedBooleanExpression {
    let mut scope = BooleanExpressionScope::new();
    init(&mut scope);
    scope.build_and()
}

/// 构造类型化 lambda 风格的 AND 组合表达式。
/// Build a typed AND expression with a closure scope.
pub fn typed_and_scope<T>(
    init: impl FnOnce(&mut BooleanExpressionScope<T>),
) -> BooleanExpression<T> {
    let mut scope = BooleanExpressionScope::new();
    init(&mut scope);
    scope.build_and()
}

/// 构造默认运行时 OR 组合表达式。
/// Build a default runtime OR expression.
pub fn or<I>(expressions: I) -> ParsedBooleanExpression
where
    I: IntoIterator<Item = ParsedBooleanExpression>,
{
    combine_typed_or(expressions.into_iter().collect())
}

/// 构造类型化 OR 组合表达式。
/// Build a typed OR expression.
pub fn typed_or<T, I>(expressions: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = BooleanExpression<T>>,
{
    combine_typed_or(expressions.into_iter().collect())
}

/// 构造默认运行时 lambda 风格的 OR 组合表达式。
/// Build a default runtime OR expression with a closure scope.
pub fn or_scope(
    init: impl FnOnce(&mut BooleanExpressionScope<ExpressionValue>),
) -> ParsedBooleanExpression {
    let mut scope = BooleanExpressionScope::new();
    init(&mut scope);
    scope.build_or()
}

/// 构造类型化 lambda 风格的 OR 组合表达式。
/// Build a typed OR expression with a closure scope.
pub fn typed_or_scope<T>(
    init: impl FnOnce(&mut BooleanExpressionScope<T>),
) -> BooleanExpression<T> {
    let mut scope = BooleanExpressionScope::new();
    init(&mut scope);
    scope.build_or()
}

/// 构造默认运行时字段等值范围组合。
/// Build a default runtime field equality scope combined with extra predicates.
pub fn scoped_and<I>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<ExpressionValue>>,
    additional: I,
) -> ParsedBooleanExpression
where
    I: IntoIterator<Item = ParsedBooleanExpression>,
{
    let mut expressions = vec![eq(path, value)];
    expressions.extend(additional);
    and(expressions)
}

/// 构造类型化字段等值范围组合。
/// Build a typed field equality scope combined with extra predicates.
pub fn typed_scoped_and<T, I>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
    additional: I,
) -> BooleanExpression<T>
where
    I: IntoIterator<Item = BooleanExpression<T>>,
{
    let mut expressions = vec![typed_eq(path, value)];
    expressions.extend(additional);
    typed_and(expressions)
}

fn combine_typed_and<T>(expressions: Vec<BooleanExpression<T>>) -> BooleanExpression<T> {
    match expressions.len() {
        0 => BooleanExpression::Constant(Trivalent::True),
        1 => {
            let mut expressions = expressions;
            expressions.remove(0)
        }
        _ => BooleanExpression::and(expressions),
    }
}

fn combine_typed_or<T>(expressions: Vec<BooleanExpression<T>>) -> BooleanExpression<T> {
    match expressions.len() {
        0 => BooleanExpression::Constant(Trivalent::False),
        1 => {
            let mut expressions = expressions;
            expressions.remove(0)
        }
        _ => BooleanExpression::or(expressions),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_dsl_builds_runtime_predicates() {
        let expression = and([eq("status", "active"), ge("age", 18)]);

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
    }

    #[test]
    fn boolean_dsl_uses_identity_for_empty_groups() {
        assert_eq!(and([]), BooleanExpression::Constant(Trivalent::True));
        assert_eq!(or([]), BooleanExpression::Constant(Trivalent::False));
    }

    #[test]
    fn boolean_scope_collects_predicates() {
        let expression = and_scope(|scope| {
            scope.ge("age", 18);
            scope.in_values("status", ["active", "pending"]);
            scope.is_null("deleted_at");
        });

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 3);
    }

    #[test]
    fn boolean_scope_removes_collected_suffix_for_grouping() {
        let expression = and_scope(|scope| {
            let lower = scope.ge("age", 18);
            let upper = scope.le("age", 65);
            scope.and([lower, upper]);
        });

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
    }
}
