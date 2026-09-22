use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSlackRangeFunction;
use ospf_rust_core::symbol::{Category, FunctionSymbol, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};
use std::collections::HashMap;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn QuadraticSlackRange_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<QuadraticSlackRangeFunction<f64>>();
}

#[test]
fn quadratic_slack_range_registers_expected_helpers_and_rows() {
    let linear = QuadraticSlackRangeFunction::new(
        2,
        "linear_range_slack",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
        1.0,
        3.0,
    );
    let mut linear_tokens = Vec::new();
    linear.register_tokens(&mut linear_tokens).unwrap();
    assert_eq!(
        linear_tokens.len(),
        4,
        "range max result and three selectors"
    );
    let linear_indices: HashMap<usize, usize> = linear_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect();
    assert_eq!(
        linear.mechanism_constraints(&linear_indices).unwrap().len(),
        7
    );
    assert!(linear
        .quadratic_mechanism_constraints(&linear_indices)
        .unwrap()
        .is_empty());

    let quadratic = QuadraticSlackRangeFunction::with_big_m(
        3,
        "quadratic_range_slack",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
        1.0,
        3.0,
        100.0,
    );
    let mut quadratic_tokens = Vec::new();
    quadratic.register_tokens(&mut quadratic_tokens).unwrap();
    assert_eq!(
        quadratic_tokens.len(),
        5,
        "quadratic bridge plus range helpers"
    );
    let quadratic_indices: HashMap<usize, usize> = quadratic_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect();
    assert_eq!(
        quadratic
            .mechanism_constraints(&quadratic_indices)
            .unwrap()
            .len(),
        7
    );
    assert_eq!(
        quadratic
            .quadratic_mechanism_constraints(&quadratic_indices)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        <QuadraticSlackRangeFunction<f64> as IntermediateSymbol<f64>>::category(&quadratic),
        Category::Linear
    );
    assert_eq!(
        <QuadraticSlackRangeFunction<f64> as IntermediateSymbol<f64>>::operation_category(
            &quadratic,
        ),
        Category::Quadratic
    );
}

#[test]
fn quadratic_slack_range_evaluates_below_inside_and_above() {
    let x = ContinuousVariableItem::create(VariableId::standalone(4), "x");
    let function = QuadraticSlackRangeFunction::new(
        5,
        "range_values",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
        1.0,
        3.0,
    );
    let evaluate = |value: f64| {
        let token = Token::from_generic(x.clone(), 0);
        token.set_result(value);
        let mut tokens = VecTokenList::<f64>::new();
        tokens.add_token(token);
        function.calculate_value(&tokens, false)
    };
    assert_eq!(evaluate(-1.0), Some(2.0));
    assert_eq!(evaluate(2.0), Some(0.0));
    assert_eq!(evaluate(5.0), Some(2.0));
}

#[test]
fn quadratic_slack_range_validates_bounds_and_prepare_source() {
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
    assert!(std::panic::catch_unwind(|| {
        QuadraticSlackRangeFunction::new(6, "invalid_range", input.clone(), 3.0, 1.0);
    })
    .is_err());
    assert!(std::panic::catch_unwind(|| {
        QuadraticSlackRangeFunction::with_big_m(7, "invalid_m", input.clone(), 1.0, 3.0, 0.0);
    })
    .is_err());

    let function = QuadraticSlackRangeFunction::new(
        8,
        "range_prepare",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 2, 3)], 0.0),
        1.0,
        3.0,
    );
    let mut values = HashMap::new();
    values.insert(2, 2.0);
    values.insert(3, 2.0);
    values.insert(99, 999.0);
    assert_eq!(function.prepare(&values), Some(1.0));
}
