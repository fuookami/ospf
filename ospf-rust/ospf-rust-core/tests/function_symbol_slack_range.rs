use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackRangeFunction;
use ospf_rust_core::symbol::{FunctionSymbol, IntermediateSymbol};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn SlackRange_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<SlackRangeFunction<f64>>();
}

#[test]
fn slack_range_registers_exact_max_rows_and_uses_explicit_big_m() {
    let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
    let function: SlackRangeFunction<f64> =
        SlackRangeFunction::with_big_m(31_001, "slack_rows", input, 1.0, 3.0, 13.0);

    let mut tokens = Vec::new();
    function
        .register_auxiliary_tokens(&mut tokens)
        .expect("slack-range helpers should register");
    assert_eq!(tokens.len(), 4, "result plus three max selectors");
    assert!(tokens
        .iter()
        .any(|token| token.name().contains("slack_rows_max")));

    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    let constraints = function
        .mechanism_constraints(&symbol_to_index)
        .expect("slack-range rows should be constructible");
    assert_eq!(
        constraints.len(),
        7,
        "exact max emits three lower, three upper, one sum row"
    );
    assert!(constraints
        .iter()
        .any(|row| row.name == "slack_rows_max_selector_sum"));

    let first_upper = constraints
        .iter()
        .find(|row| row.name == "slack_rows_max_ub_0")
        .expect("first max upper row should exist");
    assert!(first_upper
        .inequality
        .polynomial
        .monomials()
        .iter()
        .any(|monomial| (*monomial.coefficient() - 13.0).abs() < 1e-12));
}

#[test]
#[should_panic(expected = "lower <= upper")]
fn slack_range_rejects_reversed_bounds_before_registration() {
    let _ =
        SlackRangeFunction::<f64>::new(31_002, "slack_invalid", Linear::constant(0.0), 4.0, 2.0);
}
