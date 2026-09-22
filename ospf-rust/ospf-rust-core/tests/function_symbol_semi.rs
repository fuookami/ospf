use std::collections::HashMap;

use ospf_rust_core::symbol::function::SemiFunction;
use ospf_rust_core::symbol::{FunctionSymbol, IntermediateSymbol};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn Semi_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<SemiFunction<f64>>();
}

#[test]
fn semi_registers_two_bound_rows_with_result_and_indicator_helpers() {
    let function: SemiFunction<f64> = SemiFunction::new(32_001, "semi_rows", 2.0, 5.0);
    let mut tokens = Vec::new();
    function
        .register_auxiliary_tokens(&mut tokens)
        .expect("semi helpers should register");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].id(), function.result_variable().id());
    assert_eq!(tokens[1].id(), function.indicator_variable().id());

    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    let constraints = function
        .mechanism_constraints(&symbol_to_index)
        .expect("semi rows should be constructible");
    assert_eq!(constraints.len(), 2);
    assert!(constraints
        .iter()
        .any(|row| row.name == "semi_rows_semi_upper"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "semi_rows_semi_lower"));
    let upper = constraints
        .iter()
        .find(|row| row.name == "semi_rows_semi_upper")
        .unwrap();
    assert!(upper
        .inequality
        .polynomial
        .monomials()
        .iter()
        .any(|monomial| (*monomial.coefficient() + 5.0).abs() < 1e-12));
}

#[test]
fn semi_default_bounds_match_kotlin_defaults() {
    let function: SemiFunction<f64> =
        SemiFunction::<f64>::with_default_bounds(32_002, "semi_default");
    assert_eq!(*function.lower_bound(), 0.0);
    assert_eq!(*function.upper_bound(), 1_000_000.0);
}

#[test]
#[should_panic(expected = "lower <= upper")]
fn semi_rejects_reversed_bounds_before_registration() {
    let _ = SemiFunction::<f64>::new(32_003, "semi_invalid", 5.0, 2.0);
}
