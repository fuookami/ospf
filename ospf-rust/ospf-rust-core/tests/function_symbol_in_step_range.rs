use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::InStepRangeFunction;
use ospf_rust_core::symbol::{FunctionSymbol, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn in_step_range_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<InStepRangeFunction<f64>>();
}

#[test]
fn in_step_range_returns_the_largest_stepped_endpoint() {
    let upper = ContinuousVariableItem::create(VariableId::standalone(1), "upper");
    let function: InStepRangeFunction<f64> = InStepRangeFunction::new(
        10,
        "step_endpoint",
        Linear::new(vec![], 1.0),
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        3.0,
    );

    let mut tokens = VecTokenList::new();
    let token = Token::from_generic(upper.clone(), 0);
    token.set_result(10.0);
    tokens.add_token(token);
    assert_eq!(function.calculate_value(&tokens, false), Some(10.0));

    let mut tokens = VecTokenList::new();
    let token = Token::from_generic(upper, 0);
    token.set_result(9.0);
    tokens.add_token(token);
    assert_eq!(function.calculate_value(&tokens, false), Some(7.0));
}

#[test]
fn in_step_range_registers_floor_and_bound_rows() {
    let function: InStepRangeFunction<f64> = InStepRangeFunction::new(
        10_001,
        "step_rows",
        Linear::constant(1.0),
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        3.0,
    );
    let mut tokens = Vec::new();
    function
        .register_auxiliary_tokens(&mut tokens)
        .expect("in-step helpers should register");
    assert_eq!(tokens.len(), 2, "floor result and integer helpers");
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    let constraints = function
        .mechanism_constraints(&symbol_to_index)
        .expect("in-step rows should be constructible");
    assert_eq!(constraints.len(), 4);
    assert!(constraints
        .iter()
        .any(|row| row.name == "step_rows_quotient_floor_lb"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "step_rows_quotient_floor_ub"));
    assert!(constraints.iter().any(|row| row.name == "step_rows_bounds"));
    let floor_upper = constraints
        .iter()
        .find(|row| row.name == "step_rows_quotient_floor_ub")
        .unwrap();
    assert!((floor_upper.inequality.rhs - (1.0 - 1e-10)).abs() < 1e-12);
}

#[test]
#[should_panic(expected = "positive finite step")]
fn in_step_range_rejects_zero_step() {
    let _ = InStepRangeFunction::<f64>::new(
        11,
        "invalid_step_endpoint",
        Linear::new(vec![], 0.0),
        Linear::new(vec![], 1.0),
        0.0,
    );
}
