use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticLinearFunction;
use ospf_rust_core::symbol::{
    Category, FunctionSymbol, IntermediateSymbol, LinearIntermediateSymbol,
};
use std::collections::HashMap;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn QuadraticLinear_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<QuadraticLinearFunction<f64>>();
}

#[test]
fn quadratic_linear_only_creates_a_helper_for_a_genuine_quadratic_term() {
    let linear = QuadraticLinearFunction::new(
        2,
        "linear_input",
        Quadratic::new(vec![QuadraticMonomial::new_linear(2.0, 0)], 1.0),
    );
    let mut linear_tokens = Vec::new();
    linear.register_tokens(&mut linear_tokens).unwrap();
    assert!(linear_tokens.is_empty());
    assert!(linear.input_linear_polynomial().is_some());
    assert!(linear
        .mechanism_constraints(&HashMap::new())
        .unwrap()
        .is_empty());
    assert!(linear
        .quadratic_mechanism_constraints(&HashMap::new())
        .unwrap()
        .is_empty());
    assert_eq!(
        <QuadraticLinearFunction<f64> as IntermediateSymbol<f64>>::category(&linear),
        Category::Linear
    );

    let quadratic = QuadraticLinearFunction::new(
        3,
        "quadratic_input",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(2.0, 0, 1)], 1.0),
    );
    let mut quadratic_tokens = Vec::new();
    quadratic.register_tokens(&mut quadratic_tokens).unwrap();
    assert_eq!(quadratic_tokens.len(), 1);
    let mut symbol_to_index = HashMap::new();
    symbol_to_index.insert(quadratic.result_variable().id().unique_id() as usize, 10);
    assert!(quadratic
        .mechanism_constraints(&symbol_to_index)
        .unwrap()
        .is_empty());
    assert_eq!(
        quadratic
            .quadratic_mechanism_constraints(&symbol_to_index)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        <QuadraticLinearFunction<f64> as IntermediateSymbol<f64>>::category(&quadratic),
        Category::Quadratic
    );
    assert_eq!(quadratic.to_linear_polynomial().monomials().len(), 1);
}

#[test]
fn quadratic_linear_prepare_recomputes_source_instead_of_trusting_helper() {
    let function = QuadraticLinearFunction::new(
        4,
        "source_prepare",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 2, 3)], 0.0),
    );
    let mut values = HashMap::new();
    values.insert(2, 2.0);
    values.insert(3, -3.0);
    values.insert(99, 999.0);
    assert_eq!(function.prepare(&values), Some(-6.0));
}
