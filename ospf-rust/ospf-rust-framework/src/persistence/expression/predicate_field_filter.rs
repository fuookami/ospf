//! 谓词字段过滤提取
//! Predicate field filter extraction

use std::collections::HashMap;

use ospf_rust_math::symbol::{
    BooleanExpression, ComparisonOperator, ExpressionValue, NullCheckType, ParsedBooleanExpression,
    ScalarExpression,
};

/// 谓词过滤值。
/// Predicate filter value.
pub trait PredicateFilterValue: Clone {
    /// 判断该常量是否表示空值。
    /// Check whether this constant represents null.
    fn is_null_filter_value(&self) -> bool {
        false
    }
}

impl PredicateFilterValue for ExpressionValue {
    fn is_null_filter_value(&self) -> bool {
        matches!(self, Self::Null)
    }
}

macro_rules! impl_predicate_filter_value {
    ($($type:ty),* $(,)?) => {
        $(
            impl PredicateFilterValue for $type {}
        )*
    };
}

impl_predicate_filter_value!(
    bool, f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, String
);

impl PredicateFilterValue for &str {}

/// 字段级过滤条件。
/// Field-level filter.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldFilter<T> {
    /// 等值条件。
    /// Equality condition.
    pub eq: Option<T>,
    /// IN 集合条件。
    /// IN-list condition.
    pub in_values: Option<Vec<T>>,
    /// 大于条件。
    /// Greater-than condition.
    pub gt: Option<T>,
    /// 大于等于条件。
    /// Greater-than-or-equal condition.
    pub ge: Option<T>,
    /// 小于条件。
    /// Less-than condition.
    pub lt: Option<T>,
    /// 小于等于条件。
    /// Less-than-or-equal condition.
    pub le: Option<T>,
    /// 空值检查；true 表示 IS NULL，false 表示 IS NOT NULL。
    /// Null check; true means IS NULL, false means IS NOT NULL.
    pub is_null: Option<bool>,
}

impl<T> Default for FieldFilter<T> {
    fn default() -> Self {
        Self {
            eq: None,
            in_values: None,
            gt: None,
            ge: None,
            lt: None,
            le: None,
            is_null: None,
        }
    }
}

/// 等值或 IN 字段过滤条件。
/// Equality-or-IN field filter.
#[derive(Debug, Clone, PartialEq)]
pub enum EqOrInFilter<T> {
    /// 等值条件 / Equality condition
    Eq(T),
    /// IN 集合条件 / IN-list condition
    In(Vec<T>),
}

/// 谓词字段过滤提取扩展。
/// Predicate field filter extraction extension.
pub trait PredicateFieldFilterExt<T> {
    /// 尝试将谓词解析为字段名到等值条件的映射。
    /// Try to parse a predicate into field-to-equality filters.
    fn eq_filters(&self) -> Option<HashMap<String, T>>
    where
        T: Clone;

    /// 尝试将谓词解析为字段名到等值或 IN 条件的映射。
    /// Try to parse a predicate into field-to-equality-or-IN filters.
    fn eq_or_in_filters(&self) -> Option<HashMap<String, EqOrInFilter<T>>>
    where
        T: Clone;

    /// 尝试将谓词解析为字段级条件。
    /// Try to parse a predicate into field-level filters.
    fn field_filters(&self) -> Option<HashMap<String, FieldFilter<T>>>
    where
        T: PredicateFilterValue;
}

impl<T> PredicateFieldFilterExt<T> for BooleanExpression<T> {
    fn eq_filters(&self) -> Option<HashMap<String, T>>
    where
        T: Clone,
    {
        eq_filters(Some(self))
    }

    fn eq_or_in_filters(&self) -> Option<HashMap<String, EqOrInFilter<T>>>
    where
        T: Clone,
    {
        eq_or_in_filters(Some(self))
    }

    fn field_filters(&self) -> Option<HashMap<String, FieldFilter<T>>>
    where
        T: PredicateFilterValue,
    {
        field_filters(Some(self))
    }
}

impl<T> PredicateFieldFilterExt<T> for Option<BooleanExpression<T>> {
    fn eq_filters(&self) -> Option<HashMap<String, T>>
    where
        T: Clone,
    {
        eq_filters(self.as_ref())
    }

    fn eq_or_in_filters(&self) -> Option<HashMap<String, EqOrInFilter<T>>>
    where
        T: Clone,
    {
        eq_or_in_filters(self.as_ref())
    }

    fn field_filters(&self) -> Option<HashMap<String, FieldFilter<T>>>
    where
        T: PredicateFilterValue,
    {
        field_filters(self.as_ref())
    }
}

impl<T> PredicateFieldFilterExt<T> for Option<&BooleanExpression<T>> {
    fn eq_filters(&self) -> Option<HashMap<String, T>>
    where
        T: Clone,
    {
        eq_filters(*self)
    }

    fn eq_or_in_filters(&self) -> Option<HashMap<String, EqOrInFilter<T>>>
    where
        T: Clone,
    {
        eq_or_in_filters(*self)
    }

    fn field_filters(&self) -> Option<HashMap<String, FieldFilter<T>>>
    where
        T: PredicateFilterValue,
    {
        field_filters(*self)
    }
}

/// 尝试将谓词解析为字段名到等值条件的映射。
/// Try to parse a predicate into field-to-equality filters.
pub fn eq_filters<T>(expression: Option<&BooleanExpression<T>>) -> Option<HashMap<String, T>>
where
    T: Clone,
{
    let Some(expression) = expression else {
        return Some(HashMap::new());
    };

    let mut filters = HashMap::new();
    if collect_eq_filters(expression, &mut filters) {
        Some(filters)
    } else {
        None
    }
}

/// 尝试将谓词解析为字段名到等值或 IN 条件的映射。
/// Try to parse a predicate into field-to-equality-or-IN filters.
pub fn eq_or_in_filters<T>(
    expression: Option<&BooleanExpression<T>>,
) -> Option<HashMap<String, EqOrInFilter<T>>>
where
    T: Clone,
{
    let Some(expression) = expression else {
        return Some(HashMap::new());
    };

    let mut filters = HashMap::new();
    if collect_eq_or_in_filters(expression, &mut filters) {
        Some(filters)
    } else {
        None
    }
}

/// 尝试将谓词解析为字段级条件。
/// Try to parse a predicate into field-level filters.
pub fn field_filters<T>(
    expression: Option<&BooleanExpression<T>>,
) -> Option<HashMap<String, FieldFilter<T>>>
where
    T: PredicateFilterValue,
{
    let Some(expression) = expression else {
        return Some(HashMap::new());
    };

    let mut filters = HashMap::new();
    if collect_field_filters(expression, &mut filters) {
        Some(filters)
    } else {
        None
    }
}

/// 尝试将默认运行时谓词解析为字段级条件。
/// Try to parse a default runtime predicate into field-level filters.
pub fn expression_value_field_filters(
    expression: Option<&ParsedBooleanExpression>,
) -> Option<HashMap<String, FieldFilter<ExpressionValue>>> {
    field_filters(expression)
}

fn collect_eq_filters<T>(
    expression: &BooleanExpression<T>,
    filters: &mut HashMap<String, T>,
) -> bool
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => operands
            .iter()
            .all(|operand| collect_eq_filters(operand, filters)),
        BooleanExpression::Comparison { .. } => {
            let Some(field) = field_comparison(expression) else {
                return false;
            };
            if field.operator != ComparisonOperator::Eq {
                return false;
            }
            filters.insert(field.path, field.value);
            true
        }
        _ => false,
    }
}

fn collect_eq_or_in_filters<T>(
    expression: &BooleanExpression<T>,
    filters: &mut HashMap<String, EqOrInFilter<T>>,
) -> bool
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => operands
            .iter()
            .all(|operand| collect_eq_or_in_filters(operand, filters)),
        BooleanExpression::Comparison { .. } => {
            let Some(field) = field_comparison(expression) else {
                return false;
            };
            if field.operator != ComparisonOperator::Eq {
                return false;
            }
            filters.insert(field.path, EqOrInFilter::Eq(field.value));
            true
        }
        BooleanExpression::In { .. } => {
            let Some(field) = field_in_values(expression) else {
                return false;
            };
            filters.insert(field.path, EqOrInFilter::In(field.values));
            true
        }
        _ => false,
    }
}

fn collect_field_filters<T>(
    expression: &BooleanExpression<T>,
    filters: &mut HashMap<String, FieldFilter<T>>,
) -> bool
where
    T: PredicateFilterValue,
{
    match expression {
        BooleanExpression::And(operands) => operands
            .iter()
            .all(|operand| collect_field_filters(operand, filters)),
        BooleanExpression::Comparison { .. } => collect_comparison_filter(expression, filters),
        BooleanExpression::In { .. } => {
            let Some(field) = field_in_values(expression) else {
                return false;
            };
            mutable_filter(filters, field.path).in_values = Some(field.values);
            true
        }
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => {
            mutable_filter(filters, path.value().to_string()).is_null =
                Some(*null_check_type == NullCheckType::IsNull);
            true
        }
        _ => false,
    }
}

fn collect_comparison_filter<T>(
    expression: &BooleanExpression<T>,
    filters: &mut HashMap<String, FieldFilter<T>>,
) -> bool
where
    T: PredicateFilterValue,
{
    let Some(field) = field_comparison(expression) else {
        return false;
    };
    let filter = mutable_filter(filters, field.path);
    match field.operator {
        ComparisonOperator::Eq => {
            if field.value.is_null_filter_value() {
                filter.is_null = Some(true);
            } else {
                filter.eq = Some(field.value);
            }
            true
        }
        ComparisonOperator::Gt => {
            filter.gt = Some(field.value);
            true
        }
        ComparisonOperator::Ge => {
            filter.ge = Some(field.value);
            true
        }
        ComparisonOperator::Lt => {
            filter.lt = Some(field.value);
            true
        }
        ComparisonOperator::Le => {
            filter.le = Some(field.value);
            true
        }
        ComparisonOperator::Ne => false,
    }
}

fn mutable_filter<T>(
    filters: &mut HashMap<String, FieldFilter<T>>,
    path: String,
) -> &mut FieldFilter<T> {
    filters.entry(path).or_default()
}

struct FieldComparison<T> {
    path: String,
    operator: ComparisonOperator,
    value: T,
}

struct FieldInValues<T> {
    path: String,
    values: Vec<T>,
}

fn field_comparison<T>(expression: &BooleanExpression<T>) -> Option<FieldComparison<T>>
where
    T: Clone,
{
    let BooleanExpression::Comparison {
        operator,
        left,
        right,
    } = expression
    else {
        return None;
    };

    match (left, right) {
        (ScalarExpression::Reference(path), ScalarExpression::Constant(value)) => {
            Some(FieldComparison {
                path: path.value().to_string(),
                operator: *operator,
                value: value.clone(),
            })
        }
        (ScalarExpression::Constant(value), ScalarExpression::Reference(path)) => {
            Some(FieldComparison {
                path: path.value().to_string(),
                operator: flip_comparison_side(*operator),
                value: value.clone(),
            })
        }
        _ => None,
    }
}

fn field_in_values<T>(expression: &BooleanExpression<T>) -> Option<FieldInValues<T>>
where
    T: Clone,
{
    let BooleanExpression::In {
        value,
        candidates,
        negated,
    } = expression
    else {
        return None;
    };

    if *negated {
        return None;
    }
    let ScalarExpression::Reference(path) = value else {
        return None;
    };
    if candidates.is_empty() {
        return None;
    }

    let mut values = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let ScalarExpression::Constant(value) = candidate else {
            return None;
        };
        values.push(value.clone());
    }
    Some(FieldInValues {
        path: path.value().to_string(),
        values,
    })
}

fn flip_comparison_side(operator: ComparisonOperator) -> ComparisonOperator {
    match operator {
        ComparisonOperator::Eq => ComparisonOperator::Eq,
        ComparisonOperator::Ne => ComparisonOperator::Ne,
        ComparisonOperator::Lt => ComparisonOperator::Gt,
        ComparisonOperator::Le => ComparisonOperator::Ge,
        ComparisonOperator::Gt => ComparisonOperator::Lt,
        ComparisonOperator::Ge => ComparisonOperator::Le,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::expression::{and, and_scope, eq, ge, in_values, not_in_values};

    #[test]
    fn eq_filters_parse_and_equality_predicates() {
        let expression = and([eq("status", "active"), eq("age", 18)]);

        let filters = expression.eq_filters().expect("eq filters should parse");
        assert_eq!(
            filters.get("status"),
            Some(&ExpressionValue::String("active".to_string()))
        );
        assert_eq!(filters.get("age"), Some(&ExpressionValue::Number(18.0)));
    }

    #[test]
    fn eq_filters_reject_range_predicates() {
        let expression = and([eq("status", "active"), ge("age", 18)]);

        assert_eq!(expression.eq_filters(), None);
    }

    #[test]
    fn eq_or_in_filters_parse_equality_and_in_predicates() {
        let expression = and([in_values("status", ["active", "pending"]), eq("age", 18)]);

        let filters = expression
            .eq_or_in_filters()
            .expect("eq-or-in filters should parse");
        assert_eq!(
            filters.get("status"),
            Some(&EqOrInFilter::In(vec![
                ExpressionValue::String("active".to_string()),
                ExpressionValue::String("pending".to_string())
            ]))
        );
        assert_eq!(
            filters.get("age"),
            Some(&EqOrInFilter::Eq(ExpressionValue::Number(18.0)))
        );
    }

    #[test]
    fn eq_or_in_filters_reject_not_in_predicates() {
        let expression = not_in_values("status", ["deleted"]);

        assert_eq!(expression.eq_or_in_filters(), None);
    }

    #[test]
    fn field_filters_parse_range_in_and_null_checks() {
        let expression = and_scope(|scope| {
            scope.ge("age", 18);
            scope.le("age", 65);
            scope.in_values("status", ["active", "pending"]);
            scope.is_null("deleted_at");
        });

        let filters = expression
            .field_filters()
            .expect("field filters should parse");
        assert_eq!(
            filters.get("age").and_then(|filter| filter.ge.as_ref()),
            Some(&ExpressionValue::Number(18.0))
        );
        assert_eq!(
            filters.get("age").and_then(|filter| filter.le.as_ref()),
            Some(&ExpressionValue::Number(65.0))
        );
        assert_eq!(
            filters
                .get("status")
                .and_then(|filter| filter.in_values.as_ref()),
            Some(&vec![
                ExpressionValue::String("active".to_string()),
                ExpressionValue::String("pending".to_string())
            ])
        );
        assert_eq!(
            filters.get("deleted_at").and_then(|filter| filter.is_null),
            Some(true)
        );
    }

    #[test]
    fn field_filters_flip_comparison_when_constant_is_on_left() {
        let expression = BooleanExpression::comparison(
            ComparisonOperator::Le,
            ScalarExpression::constant(ExpressionValue::from(18)),
            ScalarExpression::reference("age"),
        );

        let filters = expression
            .field_filters()
            .expect("field filters should parse");
        assert_eq!(
            filters.get("age").and_then(|filter| filter.ge.as_ref()),
            Some(&ExpressionValue::Number(18.0))
        );
    }

    #[test]
    fn field_filters_treat_expression_value_null_equality_as_null_check() {
        let expression = eq("deleted_at", ExpressionValue::Null);

        let filters = expression
            .field_filters()
            .expect("field filters should parse");
        assert_eq!(
            filters.get("deleted_at").and_then(|filter| filter.is_null),
            Some(true)
        );
        assert_eq!(
            filters
                .get("deleted_at")
                .and_then(|filter| filter.eq.as_ref()),
            None
        );
    }

    #[test]
    fn field_filters_reject_non_constant_in_candidates() {
        let expression = BooleanExpression::in_expr(
            ScalarExpression::<ExpressionValue>::reference("status"),
            vec![ScalarExpression::<ExpressionValue>::reference(
                "other_status",
            )],
            false,
        );

        assert_eq!(expression.field_filters(), None);
    }

    #[test]
    fn empty_optional_predicate_returns_empty_filters() {
        let expression: Option<ParsedBooleanExpression> = None;

        assert_eq!(expression.eq_filters(), Some(HashMap::new()));
        assert_eq!(expression.field_filters(), Some(HashMap::new()));
    }
}
