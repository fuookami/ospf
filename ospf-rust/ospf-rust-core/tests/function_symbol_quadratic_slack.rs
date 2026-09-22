use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSlackFunction;
use ospf_rust_core::symbol::{Category, FunctionSymbol, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};
use std::collections::HashMap;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn QuadraticSlack_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<QuadraticSlackFunction<f64>>();
}

#[test]
fn quadratic_slack_registers_only_operation_helpers_and_rows() {
    let linear = QuadraticSlackFunction::new(
        2,
        "linear_slack",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 1.0),
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 1)], 0.0),
    );
    let mut linear_tokens = Vec::new();
    linear.register_tokens(&mut linear_tokens).unwrap();
    assert_eq!(linear_tokens.len(), 2, "slack result and side only");
    let linear_indices: HashMap<usize, usize> = linear_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect();
    assert_eq!(
        linear.mechanism_constraints(&linear_indices).unwrap().len(),
        4
    );
    assert!(linear
        .quadratic_mechanism_constraints(&linear_indices)
        .unwrap()
        .is_empty());

    let quadratic = QuadraticSlackFunction::new(
        3,
        "quadratic_slack",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
        Quadratic::new(vec![], 1.0),
    );
    let mut quadratic_tokens = Vec::new();
    quadratic.register_tokens(&mut quadratic_tokens).unwrap();
    assert_eq!(
        quadratic_tokens.len(),
        3,
        "one quadratic bridge plus slack helpers"
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
        4
    );
    assert_eq!(
        quadratic
            .quadratic_mechanism_constraints(&quadratic_indices)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        <QuadraticSlackFunction<f64> as IntermediateSymbol<f64>>::category(&quadratic),
        Category::Linear
    );
    assert_eq!(
        <QuadraticSlackFunction<f64> as IntermediateSymbol<f64>>::operation_category(&quadratic),
        Category::Quadratic
    );
}

#[test]
fn quadratic_slack_evaluates_negative_zero_and_positive_differences() {
    let x = ContinuousVariableItem::create(VariableId::standalone(4), "x");
    let y = ContinuousVariableItem::create(VariableId::standalone(5), "y");
    let function = QuadraticSlackFunction::new(
        6,
        "slack_values",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 1)], 0.0),
    );
    let evaluate = |left: f64, right: f64| {
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x.clone(), 0);
        tx.set_result(left);
        tokens.add_token(tx);
        let ty = Token::from_generic(y.clone(), 1);
        ty.set_result(right);
        tokens.add_token(ty);
        function.calculate_value(&tokens, false)
    };
    assert_eq!(evaluate(-2.0, 3.0), Some(5.0));
    assert_eq!(evaluate(2.0, 2.0), Some(0.0));
    assert_eq!(evaluate(5.0, -1.0), Some(6.0));
}

#[test]
fn quadratic_slack_rejects_an_invalid_explicit_big_m_and_recomputes_prepare() {
    let invalid = QuadraticSlackFunction::with_big_m(
        7,
        "bad_slack_m",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
        Quadratic::new(vec![], 0.0),
        0.0,
    );
    let mut invalid_tokens = Vec::new();
    invalid.register_tokens(&mut invalid_tokens).unwrap();
    let invalid_indices: HashMap<usize, usize> = invalid_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    assert!(invalid.mechanism_constraints(&invalid_indices).is_err());

    let function = QuadraticSlackFunction::new(
        8,
        "slack_prepare",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 2, 3)], 0.0),
        Quadratic::new(vec![], 0.0),
    );
    let mut values = HashMap::new();
    values.insert(2, 2.0);
    values.insert(3, -3.0);
    values.insert(99, 999.0);
    assert_eq!(function.prepare(&values), Some(6.0));
}
