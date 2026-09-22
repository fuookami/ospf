//! 布尔表达式规范化与结构键
//! Boolean expression normalization and structural keys

use super::boolean::{BooleanExpression, ParsedBooleanExpression};
use super::operators::*;
use super::property_path::PropertyPath;
use super::scalar::{ParsedScalarExpression, ScalarExpression};
use super::value::ExpressionValue;
use crate::Trivalent;
use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub struct NormalizeConfig {
    /// 是否扁平化 And/Or。
    /// Whether to flatten And/Or.
    pub flatten: bool,
    /// 是否进行常量折叠。
    /// Whether to perform constant folding.
    pub constant_folding: bool,
    /// 是否去重。
    /// Whether to deduplicate.
    pub deduplicate: bool,
    /// 是否消除双重否定。
    /// Whether to eliminate double negation.
    pub eliminate_double_negation: bool,
    /// 是否应用德摩根定律。
    /// Whether to apply De Morgan's laws.
    pub apply_de_morgan: bool,
    /// 是否按结构键排序操作数。
    /// Whether to sort operands by structural key.
    pub sort_operands: bool,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            flatten: true,
            constant_folding: true,
            deduplicate: true,
            eliminate_double_negation: true,
            apply_de_morgan: false,
            sort_operands: false,
        }
    }
}

/// 规范化布尔表达式。
/// Normalize boolean expression.
pub fn normalize_boolean_expression<T>(
    expression: &BooleanExpression<T>,
    config: NormalizeConfig,
) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    let mut result = expression.clone();
    if config.flatten {
        result = flatten_boolean_expression(&result);
    }
    if config.eliminate_double_negation {
        result = eliminate_double_negation(&result);
    }
    if config.apply_de_morgan {
        result = apply_de_morgan(&result);
    }
    if config.constant_folding {
        result = constant_fold_boolean_expression(&result);
    }
    if config.deduplicate {
        result = deduplicate_boolean_expression(&result);
    }
    if config.sort_operands {
        result = sort_boolean_operands(&result);
    }
    result = normalize_boolean_children(&result, config);
    simplify_single_boolean_operand(&result)
}

/// 扁平化布尔表达式中的 And/Or。
/// Flatten And/Or in a boolean expression.
pub fn flatten_boolean_expression<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => {
            let mut flattened = Vec::new();
            for operand in operands {
                match flatten_boolean_expression(operand) {
                    BooleanExpression::And(items) => flattened.extend(items),
                    item => flattened.push(item),
                }
            }
            BooleanExpression::And(flattened)
        }
        BooleanExpression::Or(operands) => {
            let mut flattened = Vec::new();
            for operand in operands {
                match flatten_boolean_expression(operand) {
                    BooleanExpression::Or(items) => flattened.extend(items),
                    item => flattened.push(item),
                }
            }
            BooleanExpression::Or(flattened)
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(flatten_boolean_expression(operand)))
        }
        _ => expression.clone(),
    }
}

/// 常量折叠布尔表达式。
/// Constant-fold boolean expression.
pub fn constant_fold_boolean_expression<T>(
    expression: &BooleanExpression<T>,
) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Constant(_) => expression.clone(),
        BooleanExpression::And(operands) => {
            let operands = operands
                .iter()
                .map(constant_fold_boolean_expression)
                .collect::<Vec<_>>();
            if operands
                .iter()
                .any(|operand| matches!(operand, BooleanExpression::Constant(Trivalent::False)))
            {
                return BooleanExpression::Constant(Trivalent::False);
            }
            let filtered = operands
                .into_iter()
                .filter(|operand| !matches!(operand, BooleanExpression::Constant(Trivalent::True)))
                .collect::<Vec<_>>();
            match filtered.len() {
                0 => BooleanExpression::Constant(Trivalent::True),
                1 => filtered
                    .into_iter()
                    .next()
                    .expect("filtered has exactly one element / filtered 恰好有一个元素"),
                _ => BooleanExpression::And(filtered),
            }
        }
        BooleanExpression::Or(operands) => {
            let operands = operands
                .iter()
                .map(constant_fold_boolean_expression)
                .collect::<Vec<_>>();
            if operands
                .iter()
                .any(|operand| matches!(operand, BooleanExpression::Constant(Trivalent::True)))
            {
                return BooleanExpression::Constant(Trivalent::True);
            }
            let filtered = operands
                .into_iter()
                .filter(|operand| !matches!(operand, BooleanExpression::Constant(Trivalent::False)))
                .collect::<Vec<_>>();
            match filtered.len() {
                0 => BooleanExpression::Constant(Trivalent::False),
                1 => filtered
                    .into_iter()
                    .next()
                    .expect("filtered has exactly one element / filtered 恰好有一个元素"),
                _ => BooleanExpression::Or(filtered),
            }
        }
        BooleanExpression::Not(operand) => match constant_fold_boolean_expression(operand) {
            BooleanExpression::Constant(value) => BooleanExpression::Constant(match value {
                Trivalent::True => Trivalent::False,
                Trivalent::False => Trivalent::True,
                Trivalent::Unknown => Trivalent::Unknown,
            }),
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        _ => expression.clone(),
    }
}

/// 对布尔表达式操作数去重。
/// Deduplicate boolean expression operands.
pub fn deduplicate_boolean_expression<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => {
            BooleanExpression::And(deduplicate_boolean_operands(operands))
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(deduplicate_boolean_operands(operands))
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(deduplicate_boolean_expression(operand)))
        }
        _ => expression.clone(),
    }
}

/// 消除布尔表达式中的双重否定。
/// Eliminate double negation in a boolean expression.
pub fn eliminate_double_negation<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Not(operand) => match eliminate_double_negation(operand) {
            BooleanExpression::Not(inner) => *inner,
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(eliminate_double_negation).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(eliminate_double_negation).collect())
        }
        _ => expression.clone(),
    }
}

/// 对布尔表达式应用德摩根定律。
/// Apply De Morgan's laws to a boolean expression.
pub fn apply_de_morgan<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Not(operand) => match apply_de_morgan(operand) {
            BooleanExpression::And(operands) => BooleanExpression::Or(
                operands
                    .into_iter()
                    .map(|operand| BooleanExpression::Not(Box::new(operand)))
                    .collect(),
            ),
            BooleanExpression::Or(operands) => BooleanExpression::And(
                operands
                    .into_iter()
                    .map(|operand| BooleanExpression::Not(Box::new(operand)))
                    .collect(),
            ),
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(apply_de_morgan).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(apply_de_morgan).collect())
        }
        _ => expression.clone(),
    }
}

/// 按结构键排序布尔表达式操作数。
/// Sort boolean expression operands by structural key.
pub fn sort_boolean_operands<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => {
            let mut operands = operands
                .iter()
                .map(sort_boolean_operands)
                .collect::<Vec<_>>();
            operands.sort_by_key(boolean_structural_key);
            BooleanExpression::And(operands)
        }
        BooleanExpression::Or(operands) => {
            let mut operands = operands
                .iter()
                .map(sort_boolean_operands)
                .collect::<Vec<_>>();
            operands.sort_by_key(boolean_structural_key);
            BooleanExpression::Or(operands)
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(sort_boolean_operands(operand)))
        }
        _ => expression.clone(),
    }
}

fn deduplicate_boolean_operands<T>(operands: &[BooleanExpression<T>]) -> Vec<BooleanExpression<T>>
where
    T: Clone + Display,
{
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for operand in operands {
        let operand = deduplicate_boolean_expression(operand);
        if seen.insert(boolean_structural_key(&operand)) {
            result.push(operand);
        }
    }
    result
}

fn normalize_boolean_children<T>(
    expression: &BooleanExpression<T>,
    config: NormalizeConfig,
) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => BooleanExpression::And(
            operands
                .iter()
                .map(|operand| normalize_boolean_expression(operand, config))
                .collect(),
        ),
        BooleanExpression::Or(operands) => BooleanExpression::Or(
            operands
                .iter()
                .map(|operand| normalize_boolean_expression(operand, config))
                .collect(),
        ),
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(normalize_boolean_expression(operand, config)))
        }
        _ => expression.clone(),
    }
}

fn simplify_single_boolean_operand<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => match operands.as_slice() {
            [] => BooleanExpression::Constant(Trivalent::True),
            [operand] => simplify_single_boolean_operand(operand),
            _ => BooleanExpression::And(
                operands
                    .iter()
                    .map(simplify_single_boolean_operand)
                    .collect(),
            ),
        },
        BooleanExpression::Or(operands) => match operands.as_slice() {
            [] => BooleanExpression::Constant(Trivalent::False),
            [operand] => simplify_single_boolean_operand(operand),
            _ => BooleanExpression::Or(
                operands
                    .iter()
                    .map(simplify_single_boolean_operand)
                    .collect(),
            ),
        },
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(simplify_single_boolean_operand(operand)))
        }
        _ => expression.clone(),
    }
}

pub(super) fn boolean_structural_key<T>(expression: &BooleanExpression<T>) -> String
where
    T: Display,
{
    match expression {
        BooleanExpression::Constant(value) => format!("Const:{value:?}"),
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => format!(
            "Cmp:{operator:?}:{}:{}",
            scalar_structural_key(left),
            scalar_structural_key(right)
        ),
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => format!(
            "In:{negated}:{}:{}",
            scalar_structural_key(value),
            candidates
                .iter()
                .map(scalar_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => format!(
            "Match:{mode:?}:{negated}:{}:{}",
            scalar_structural_key(value),
            scalar_structural_key(pattern)
        ),
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => format!("Null:{null_check_type:?}:{path}"),
        BooleanExpression::And(operands) => format!(
            "And:{}",
            operands
                .iter()
                .map(boolean_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::Or(operands) => format!(
            "Or:{}",
            operands
                .iter()
                .map(boolean_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::Not(operand) => format!("Not:{}", boolean_structural_key(operand)),
        BooleanExpression::Custom {
            payload,
            description,
        } => format!("Custom:{}", description.as_deref().unwrap_or(payload)),
    }
}

pub(super) fn scalar_structural_key<T>(expression: &ScalarExpression<T>) -> String
where
    T: Display,
{
    match expression {
        ScalarExpression::Constant(value) => format!("Const:{value}"),
        ScalarExpression::Reference(path) => format!("Ref:{path}"),
        ScalarExpression::SymbolReference(symbol) => format!("SymRef:{}", symbol.name()),
        ScalarExpression::Unary { operator, operand } => {
            format!("Unary:{operator:?}:{}", scalar_structural_key(operand))
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => format!(
            "Bin:{operator:?}:{}:{}",
            scalar_structural_key(left),
            scalar_structural_key(right)
        ),
        ScalarExpression::Function { name, arguments } => format!(
            "Func:{name}:{}",
            arguments
                .iter()
                .map(scalar_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        ScalarExpression::Custom {
            payload,
            description,
        } => format!("Custom:{}", description.as_deref().unwrap_or(payload)),
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "Cond:{}:{}:{}",
            boolean_structural_key(condition),
            scalar_structural_key(then_branch),
            scalar_structural_key(else_branch)
        ),
        ScalarExpression::Boolean(expr) => format!("Bool:{}", boolean_structural_key(expr)),
    }
}

// ============================================================================
// 规范化测试 / Normalization tests
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::symbol::expression::path_owned_symbol;

    use super::*;

    /// 构造 `path = value` 运行时常量比较。
    /// Build the `path = value` runtime constant comparison.
    fn comparison(path: &str, value: f64) -> ParsedBooleanExpression {
        BooleanExpression::eq(
            ScalarExpression::reference(path),
            ScalarExpression::constant(ExpressionValue::Number(value)),
        )
    }

    /// 构造 `path is null` 空值检查。
    /// Build the `path is null` null check.
    fn null_check(path: &str) -> ParsedBooleanExpression {
        BooleanExpression::is_null(path)
    }

    fn true_constant() -> ParsedBooleanExpression {
        BooleanExpression::true_constant()
    }

    fn false_constant() -> ParsedBooleanExpression {
        BooleanExpression::false_constant()
    }

    fn unknown_constant() -> ParsedBooleanExpression {
        BooleanExpression::unknown_constant()
    }

    // ========================================================================
    // 配置 / Configuration
    // ========================================================================

    #[test]
    fn default_config_enables_flatten_fold_dedup_and_double_negation() {
        let config = NormalizeConfig::default();

        assert!(config.flatten);
        assert!(config.constant_folding);
        assert!(config.deduplicate);
        assert!(config.eliminate_double_negation);
        assert!(!config.apply_de_morgan);
        assert!(!config.sort_operands);
    }

    #[test]
    fn config_allows_field_overrides_with_struct_update_syntax() {
        let config = NormalizeConfig {
            apply_de_morgan: true,
            sort_operands: true,
            ..NormalizeConfig::default()
        };

        assert!(config.apply_de_morgan);
        assert!(config.sort_operands);
        assert!(config.flatten);
        // 配置是 Copy，可按值传递而不丢失所有权。
        // The config is Copy, so it can be passed by value without losing ownership.
        let copied = config;
        assert!(copied.apply_de_morgan);
    }

    // ========================================================================
    // 扁平化 / Flattening
    // ========================================================================

    #[test]
    fn flatten_merges_nested_and_expressions() {
        let expression = BooleanExpression::And(vec![
            comparison("a", 1.0),
            BooleanExpression::And(vec![comparison("b", 2.0), comparison("c", 3.0)]),
        ]);

        let flattened = flatten_boolean_expression(&expression);

        let BooleanExpression::And(operands) = flattened else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 3);
        assert_eq!(operands[0], comparison("a", 1.0));
        assert_eq!(operands[2], comparison("c", 3.0));
    }

    #[test]
    fn flatten_merges_nested_or_expressions() {
        let expression = BooleanExpression::Or(vec![
            comparison("a", 1.0),
            BooleanExpression::Or(vec![
                comparison("b", 2.0),
                BooleanExpression::Or(vec![comparison("c", 3.0)]),
            ]),
        ]);

        let flattened = flatten_boolean_expression(&expression);

        let BooleanExpression::Or(operands) = flattened else {
            panic!("expected OR expression");
        };
        assert_eq!(operands.len(), 3);
    }

    #[test]
    fn flatten_keeps_mixed_operator_boundaries() {
        let expression = BooleanExpression::And(vec![
            comparison("a", 1.0),
            BooleanExpression::Or(vec![comparison("b", 2.0), comparison("c", 3.0)]),
        ]);

        let flattened = flatten_boolean_expression(&expression);

        let BooleanExpression::And(operands) = flattened else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
        assert!(matches!(operands[1], BooleanExpression::Or(_)));
    }

    #[test]
    fn flatten_descends_into_not_and_leaves_leaves_untouched() {
        let expression = BooleanExpression::not_expr(BooleanExpression::And(vec![
            comparison("a", 1.0),
            BooleanExpression::And(vec![comparison("b", 2.0), comparison("c", 3.0)]),
        ]));

        let flattened = flatten_boolean_expression(&expression);

        let BooleanExpression::Not(operand) = flattened else {
            panic!("expected NOT expression");
        };
        let BooleanExpression::And(operands) = *operand else {
            panic!("expected flattened AND under NOT");
        };
        assert_eq!(operands.len(), 3);

        assert_eq!(
            flatten_boolean_expression(&comparison("a", 1.0)),
            comparison("a", 1.0)
        );
        assert_eq!(flatten_boolean_expression(&true_constant()), true_constant());
        assert_eq!(
            flatten_boolean_expression(&null_check("a")),
            null_check("a")
        );
    }

    // ========================================================================
    // 常量折叠 / Constant folding
    // ========================================================================

    #[test]
    fn constant_fold_and_collapses_false_and_drops_true() {
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::And(vec![
                comparison("a", 1.0),
                false_constant(),
            ])),
            false_constant()
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::And(vec![
                true_constant(),
                comparison("a", 1.0),
                BooleanExpression::And(vec![comparison("b", 2.0), false_constant()]),
            ])),
            false_constant(),
            "嵌套的 false 也应折叠为 false / A nested false folds to false"
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::And(vec![
                true_constant(),
                comparison("a", 1.0),
                true_constant(),
            ])),
            comparison("a", 1.0)
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::And(vec![true_constant()])),
            true_constant()
        );
    }

    #[test]
    fn constant_fold_or_collapses_true_and_drops_false() {
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::Or(vec![
                comparison("a", 1.0),
                true_constant(),
            ])),
            true_constant()
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::Or(vec![
                false_constant(),
                comparison("a", 1.0),
                false_constant(),
            ])),
            comparison("a", 1.0)
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::Or(vec![false_constant()])),
            false_constant()
        );
    }

    #[test]
    fn constant_fold_not_negates_known_constants_only() {
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::not_expr(true_constant())),
            false_constant()
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::not_expr(false_constant())),
            true_constant()
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::not_expr(unknown_constant())),
            unknown_constant(),
            "unknown 取反仍是 unknown / Negating unknown yields unknown"
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::not_expr(comparison("a", 1.0))),
            BooleanExpression::not_expr(comparison("a", 1.0))
        );
    }

    #[test]
    fn constant_fold_keeps_unknown_operands_without_guessing() {
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::And(vec![
                unknown_constant(),
                true_constant(),
            ])),
            unknown_constant()
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::Or(vec![
                unknown_constant(),
                false_constant(),
            ])),
            unknown_constant()
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::And(vec![
                unknown_constant(),
                false_constant(),
            ])),
            false_constant(),
            "false 支配 And / false dominates AND"
        );
    }

    #[test]
    fn constant_fold_preserves_non_logical_nodes() {
        let expression = comparison("a", 1.0);

        assert_eq!(constant_fold_boolean_expression(&expression), expression);
        assert_eq!(
            constant_fold_boolean_expression(&null_check("a")),
            null_check("a")
        );
        assert_eq!(
            constant_fold_boolean_expression(&BooleanExpression::<ExpressionValue>::custom(
                "raw",
                None
            )),
            BooleanExpression::<ExpressionValue>::custom("raw", None)
        );
    }

    // ========================================================================
    // 去重 / Deduplication
    // ========================================================================

    #[test]
    fn deduplicate_removes_repeated_and_and_or_operands() {
        let expression = BooleanExpression::And(vec![
            comparison("a", 1.0),
            comparison("a", 1.0),
            comparison("b", 2.0),
        ]);
        let BooleanExpression::And(operands) = deduplicate_boolean_expression(&expression) else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);

        let expression = BooleanExpression::Or(vec![
            comparison("a", 1.0),
            comparison("a", 1.0),
            comparison("a", 1.0),
        ]);
        let BooleanExpression::Or(operands) = deduplicate_boolean_expression(&expression) else {
            panic!("expected OR expression");
        };
        assert_eq!(operands.len(), 1);
    }

    #[test]
    fn deduplicate_descends_into_not_and_nested_groups() {
        let expression = BooleanExpression::And(vec![
            BooleanExpression::not_expr(comparison("a", 1.0)),
            BooleanExpression::not_expr(comparison("a", 1.0)),
            BooleanExpression::And(vec![comparison("b", 2.0), comparison("b", 2.0)]),
        ]);

        let BooleanExpression::And(operands) = deduplicate_boolean_expression(&expression) else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
        let BooleanExpression::And(nested) = &operands[1] else {
            panic!("expected nested AND expression");
        };
        assert_eq!(nested.len(), 1);
    }

    #[test]
    fn deduplicate_keeps_structurally_different_in_candidates() {
        let first = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
                ScalarExpression::constant(ExpressionValue::String("pending".to_string())),
            ],
            false,
        );
        let second = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
                ScalarExpression::constant(ExpressionValue::String("archived".to_string())),
            ],
            false,
        );

        let expression = BooleanExpression::And(vec![first, second]);
        let BooleanExpression::And(operands) = deduplicate_boolean_expression(&expression) else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
    }

    #[test]
    fn deduplicate_distinguishes_negation_and_pattern_text() {
        let negated = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::String(
                "active".to_string(),
            ))],
            true,
        );
        let plain = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::String(
                "active".to_string(),
            ))],
            false,
        );
        let expression = BooleanExpression::Or(vec![plain, negated]);
        let BooleanExpression::Or(operands) = deduplicate_boolean_expression(&expression) else {
            panic!("expected OR expression");
        };
        assert_eq!(operands.len(), 2);

        let like = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
            PatternMatchMode::Like,
            false,
        );
        let other_like = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::String("B%".to_string())),
            PatternMatchMode::Like,
            false,
        );
        let expression = BooleanExpression::Or(vec![like, other_like]);
        let BooleanExpression::Or(operands) = deduplicate_boolean_expression(&expression) else {
            panic!("expected OR expression");
        };
        assert_eq!(operands.len(), 2);
    }

    // ========================================================================
    // 双重否定 / Double negation
    // ========================================================================

    #[test]
    fn eliminate_double_negation_unwraps_matching_pairs() {
        let expression = BooleanExpression::not_expr(BooleanExpression::not_expr(comparison(
            "a", 1.0,
        )));

        assert_eq!(eliminate_double_negation(&expression), comparison("a", 1.0));

        let nested = BooleanExpression::not_expr(BooleanExpression::not_expr(
            BooleanExpression::not_expr(comparison("a", 1.0)),
        ));
        assert_eq!(
            eliminate_double_negation(&nested),
            BooleanExpression::not_expr(comparison("a", 1.0)),
            "三重否定只消去一对 / Triple negation eliminates exactly one pair"
        );
    }

    #[test]
    fn eliminate_double_negation_recurses_into_groups() {
        let expression = BooleanExpression::And(vec![
            BooleanExpression::not_expr(BooleanExpression::not_expr(comparison("a", 1.0))),
            BooleanExpression::Or(vec![
                BooleanExpression::not_expr(BooleanExpression::not_expr(comparison("b", 2.0))),
                comparison("c", 3.0),
            ]),
        ]);

        let eliminated = eliminate_double_negation(&expression);

        let BooleanExpression::And(operands) = eliminated else {
            panic!("expected AND expression");
        };
        assert_eq!(operands[0], comparison("a", 1.0));
        let BooleanExpression::Or(nested) = &operands[1] else {
            panic!("expected nested OR expression");
        };
        assert_eq!(nested[0], comparison("b", 2.0));
    }

    // ========================================================================
    // 德摩根定律 / De Morgan's laws
    // ========================================================================

    #[test]
    fn apply_de_morgan_rewrites_negated_and_into_or() {
        let expression = BooleanExpression::not_expr(BooleanExpression::And(vec![
            comparison("a", 1.0),
            comparison("b", 2.0),
        ]));

        let transformed = apply_de_morgan(&expression);

        let BooleanExpression::Or(operands) = transformed else {
            panic!("expected OR expression");
        };
        assert_eq!(operands.len(), 2);
        assert!(matches!(operands[0], BooleanExpression::Not(_)));
        assert!(matches!(operands[1], BooleanExpression::Not(_)));
    }

    #[test]
    fn apply_de_morgan_rewrites_negated_or_into_and() {
        let expression = BooleanExpression::not_expr(BooleanExpression::Or(vec![
            comparison("a", 1.0),
            comparison("b", 2.0),
        ]));

        let transformed = apply_de_morgan(&expression);

        let BooleanExpression::And(operands) = transformed else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
        assert!(matches!(operands[0], BooleanExpression::Not(_)));
    }

    #[test]
    fn apply_de_morgan_keeps_negation_over_leaf_nodes() {
        let expression = BooleanExpression::not_expr(comparison("a", 1.0));

        assert_eq!(
            apply_de_morgan(&expression),
            BooleanExpression::not_expr(comparison("a", 1.0))
        );
        assert_eq!(
            apply_de_morgan(&BooleanExpression::And(vec![
                comparison("a", 1.0),
                comparison("b", 2.0),
            ])),
            BooleanExpression::And(vec![comparison("a", 1.0), comparison("b", 2.0)])
        );
    }

    // ========================================================================
    // 排序 / Operand sorting
    // ========================================================================

    #[test]
    fn sort_boolean_operands_orders_by_structural_key() {
        let expression = BooleanExpression::And(vec![
            null_check("z"),
            comparison("b", 2.0),
            comparison("a", 1.0),
        ]);

        let sorted = sort_boolean_operands(&expression);

        let BooleanExpression::And(operands) = sorted else {
            panic!("expected AND expression");
        };
        // "Cmp:..." 键先于 "Null:..." 键 / "Cmp:..." keys sort before "Null:..." keys
        assert_eq!(operands[0], comparison("a", 1.0));
        assert_eq!(operands[1], comparison("b", 2.0));
        assert_eq!(operands[2], null_check("z"));
    }

    #[test]
    fn sort_boolean_operands_recurses_and_is_idempotent() {
        let expression = BooleanExpression::Or(vec![
            comparison("c", 3.0),
            BooleanExpression::Or(vec![comparison("b", 2.0), comparison("a", 1.0)]),
        ]);

        let sorted = sort_boolean_operands(&expression);

        assert_eq!(sort_boolean_operands(&sorted), sorted);
        assert_eq!(
            sort_boolean_operands(&comparison("a", 1.0)),
            comparison("a", 1.0)
        );
    }

    // ========================================================================
    // 完整规范化 / Full normalization
    // ========================================================================

    #[test]
    fn normalize_collapses_single_operand_groups() {
        let expression = BooleanExpression::And(vec![
            true_constant(),
            comparison("a", 1.0),
            BooleanExpression::And(vec![comparison("a", 1.0), true_constant()]),
        ]);

        assert_eq!(
            normalize_boolean_expression(&expression, NormalizeConfig::default()),
            comparison("a", 1.0)
        );
        assert_eq!(
            normalize_boolean_expression(
                &BooleanExpression::Or(vec![BooleanExpression::Or(vec![comparison("a", 1.0)])]),
                NormalizeConfig::default()
            ),
            comparison("a", 1.0)
        );
    }

    #[test]
    fn normalize_folds_empty_operand_groups_to_identity_constants() {
        // 枚举变体是公开的，可以构造规范化过程中才会出现的空分组。
        // Enum variants are public, so empty groups that only appear during normalization can be built.
        assert_eq!(
            normalize_boolean_expression(
                &BooleanExpression::<ExpressionValue>::And(Vec::new()),
                NormalizeConfig::default()
            ),
            true_constant()
        );
        assert_eq!(
            normalize_boolean_expression(
                &BooleanExpression::<ExpressionValue>::Or(Vec::new()),
                NormalizeConfig::default()
            ),
            false_constant()
        );
    }

    #[test]
    fn normalize_is_idempotent() {
        let expression = BooleanExpression::And(vec![
            comparison("a", 1.0),
            true_constant(),
            BooleanExpression::Or(vec![comparison("b", 2.0), comparison("b", 2.0)]),
            BooleanExpression::not_expr(BooleanExpression::not_expr(comparison("a", 1.0))),
        ]);

        let once = normalize_boolean_expression(&expression, NormalizeConfig::default());
        let twice = normalize_boolean_expression(&once, NormalizeConfig::default());

        assert_eq!(once, twice);

        let de_morgan_config = NormalizeConfig {
            apply_de_morgan: true,
            sort_operands: true,
            ..NormalizeConfig::default()
        };
        let once = normalize_boolean_expression(&expression, de_morgan_config);
        assert_eq!(
            normalize_boolean_expression(&once, de_morgan_config),
            once,
            "启用德摩根与排序后仍应幂等 / Idempotent even with De Morgan and sorting enabled"
        );
    }

    #[test]
    fn normalize_with_disabled_rules_preserves_operand_groups() {
        let config = NormalizeConfig {
            flatten: false,
            constant_folding: false,
            deduplicate: false,
            eliminate_double_negation: false,
            apply_de_morgan: false,
            sort_operands: false,
        };
        let expression = BooleanExpression::And(vec![true_constant(), comparison("a", 1.0)]);

        let normalized = normalize_boolean_expression(&expression, config);

        let BooleanExpression::And(operands) = normalized else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
        assert_eq!(operands[0], true_constant());
        assert_eq!(operands[1], comparison("a", 1.0));
    }

    #[test]
    fn normalize_applies_de_morgan_only_when_enabled() {
        let expression = BooleanExpression::not_expr(BooleanExpression::And(vec![
            comparison("a", 1.0),
            comparison("b", 2.0),
        ]));

        let default = normalize_boolean_expression(
            &expression,
            NormalizeConfig {
                apply_de_morgan: false,
                ..NormalizeConfig::default()
            },
        );
        assert!(
            matches!(default, BooleanExpression::Not(_)),
            "默认配置不应用德摩根 / The default config skips De Morgan"
        );

        let transformed = normalize_boolean_expression(
            &expression,
            NormalizeConfig {
                apply_de_morgan: true,
                ..NormalizeConfig::default()
            },
        );
        assert!(matches!(transformed, BooleanExpression::Or(_)));
    }

    #[test]
    fn normalize_sorts_operands_only_when_enabled() {
        let expression =
            BooleanExpression::And(vec![comparison("b", 2.0), comparison("a", 1.0)]);

        let unsorted = normalize_boolean_expression(&expression, NormalizeConfig::default());
        let BooleanExpression::And(operands) = &unsorted else {
            panic!("expected AND expression");
        };
        assert_eq!(operands[0], comparison("b", 2.0));

        let sorted = normalize_boolean_expression(
            &expression,
            NormalizeConfig {
                sort_operands: true,
                ..NormalizeConfig::default()
            },
        );
        let BooleanExpression::And(operands) = sorted else {
            panic!("expected AND expression");
        };
        assert_eq!(operands[0], comparison("a", 1.0));
        assert_eq!(operands[1], comparison("b", 2.0));
    }

    #[test]
    fn normalize_keeps_distinct_operands_of_different_kinds() {
        let expression = BooleanExpression::And(vec![
            comparison("a", 1.0),
            null_check("a"),
            BooleanExpression::<ExpressionValue>::custom("raw", None),
        ]);

        let BooleanExpression::And(operands) =
            normalize_boolean_expression(&expression, NormalizeConfig::default())
        else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 3);
    }

    // ========================================================================
    // 结构键 / Structural keys
    // ========================================================================

    #[test]
    fn boolean_structural_key_covers_every_variant() {
        assert_eq!(boolean_structural_key(&true_constant()), "Const:True");
        assert_eq!(boolean_structural_key(&unknown_constant()), "Const:Unknown");
        assert_eq!(
            boolean_structural_key(&comparison("a", 1.0)),
            "Cmp:Eq:Ref:a:Const:1"
        );
        assert_eq!(
            boolean_structural_key(&BooleanExpression::in_expr(
                ScalarExpression::reference("status"),
                vec![ScalarExpression::constant(ExpressionValue::String(
                    "active".to_string(),
                ))],
                true,
            )),
            "In:true:Ref:status:Const:active"
        );
        assert_eq!(
            boolean_structural_key(&BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
                PatternMatchMode::Like,
                false,
            )),
            "Match:Like:false:Ref:name:Const:A%"
        );
        assert_eq!(
            boolean_structural_key(&null_check("a")),
            "Null:IsNull:a"
        );
        assert_eq!(
            boolean_structural_key(&BooleanExpression::And(vec![
                comparison("a", 1.0),
                null_check("b"),
            ])),
            "And:Cmp:Eq:Ref:a:Const:1,Null:IsNull:b"
        );
        assert_eq!(
            boolean_structural_key(&BooleanExpression::Or(vec![comparison("a", 1.0)])),
            "Or:Cmp:Eq:Ref:a:Const:1"
        );
        assert_eq!(
            boolean_structural_key(&BooleanExpression::not_expr(comparison("a", 1.0))),
            "Not:Cmp:Eq:Ref:a:Const:1"
        );
        assert_eq!(
            boolean_structural_key(&BooleanExpression::<ExpressionValue>::custom("raw", None)),
            "Custom:raw"
        );
        assert_eq!(
            boolean_structural_key(&BooleanExpression::<ExpressionValue>::custom(
                "raw",
                Some("shown".to_string())
            )),
            "Custom:shown"
        );
    }

    #[test]
    fn scalar_structural_key_covers_every_variant() {
        assert_eq!(
            scalar_structural_key(&ScalarExpression::<ExpressionValue>::constant(
                ExpressionValue::Number(1.0)
            )),
            "Const:1"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::<ExpressionValue>::reference("a")),
            "Ref:a"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::<ExpressionValue>::symbol_reference(
                path_owned_symbol("user.name"),
            )),
            "SymRef:user.name"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::unary(
                UnaryOperator::Negate,
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )),
            "Unary:Negate:Const:1"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::binary(
                BinaryOperator::Add,
                ScalarExpression::reference("a"),
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )),
            "Bin:Add:Ref:a:Const:1"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::<ExpressionValue>::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::reference("x")],
            )),
            "Func:abs:Ref:x"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::<ExpressionValue>::custom("raw", None)),
            "Custom:raw"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::conditional(
                comparison("a", 1.0),
                ScalarExpression::constant(ExpressionValue::Number(2.0)),
                ScalarExpression::constant(ExpressionValue::Number(3.0)),
            )),
            "Cond:Cmp:Eq:Ref:a:Const:1:Const:2:Const:3"
        );
        assert_eq!(
            scalar_structural_key(&ScalarExpression::boolean_expr(comparison("a", 1.0))),
            "Bool:Cmp:Eq:Ref:a:Const:1"
        );
    }

    #[test]
    fn structural_keys_are_stable_across_independent_builds() {
        assert_eq!(
            boolean_structural_key(&comparison("a", 1.0)),
            boolean_structural_key(&comparison("a", 1.0))
        );
        assert_ne!(
            boolean_structural_key(&comparison("a", 1.0)),
            boolean_structural_key(&comparison("a", 2.0))
        );
        assert_ne!(
            boolean_structural_key(&comparison("a", 1.0)),
            boolean_structural_key(&BooleanExpression::ge(
                ScalarExpression::reference("a"),
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            ))
        );
    }
}
