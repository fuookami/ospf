use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::InStepRangeIndicatorFunction;
use ospf_rust_core::symbol::{FunctionSymbol, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn in_step_range_indicator_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<InStepRangeIndicatorFunction<f64>>();
}

#[test]
fn in_step_range_indicator_tests_membership_in_the_discrete_set() {
    let x = ContinuousVariableItem::create(VariableId::standalone(1), "x");
    let function: InStepRangeIndicatorFunction<f64> = InStepRangeIndicatorFunction::new(
        20,
        "step_membership",
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        1.0,
        10.0,
        3.0,
    );

    let mut tokens = VecTokenList::new();
    let token = Token::from_generic(x.clone(), 0);
    token.set_result(7.0);
    tokens.add_token(token);
    assert_eq!(function.calculate_value(&tokens, false), Some(1.0));

    let mut tokens = VecTokenList::new();
    let token = Token::from_generic(x, 0);
    token.set_result(8.0);
    tokens.add_token(token);
    assert_eq!(function.calculate_value(&tokens, false), Some(0.0));
}

#[test]
fn in_step_range_indicator_registers_point_band_and_complement_rows() {
    let function: InStepRangeIndicatorFunction<f64> = InStepRangeIndicatorFunction::new(
        20_001,
        "step_indicator_rows",
        Linear::constant(0.0),
        1.0,
        7.0,
        3.0,
    );
    let mut tokens = Vec::new();
    function
        .register_auxiliary_tokens(&mut tokens)
        .expect("indicator helpers should register");
    assert_eq!(
        tokens.len(),
        7,
        "result plus three point and three side helpers"
    );
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    let constraints = function
        .mechanism_constraints(&symbol_to_index)
        .expect("indicator rows should be constructible");
    assert_eq!(constraints.len(), 16);
    assert!(constraints
        .iter()
        .any(|row| row.name == "step_indicator_rows_pt0_band_ub"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "step_indicator_rows_pt2_out_ub"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "step_indicator_rows_or_ub"));
    let band = constraints
        .iter()
        .find(|row| row.name == "step_indicator_rows_pt0_band_ub")
        .unwrap();
    assert!((band.inequality.rhs - (1_000_000.0 + 4.0e-10)).abs() < 1e-6);
}

#[test]
fn in_step_range_indicator_uses_explicit_big_m_in_registered_rows() {
    let function: InStepRangeIndicatorFunction<f64> = InStepRangeIndicatorFunction::with_big_m(
        20_003,
        "step_indicator_explicit_m",
        Linear::constant(4.0),
        1.0,
        7.0,
        3.0,
        17.0,
    );
    assert_eq!(function.big_m(), Some(&17.0));
    let mut tokens = Vec::new();
    function
        .register_auxiliary_tokens(&mut tokens)
        .expect("indicator helpers should register");
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    let constraints = function
        .mechanism_constraints(&symbol_to_index)
        .expect("indicator rows should be constructible");
    let band = constraints
        .iter()
        .find(|row| row.name == "step_indicator_explicit_m_pt0_band_ub")
        .expect("explicit-M band row should exist");
    assert!((band.inequality.rhs - (17.0 + 4.0e-10)).abs() < 1e-12);
    assert!(band
        .inequality
        .polynomial
        .monomials()
        .iter()
        .any(|monomial| (*monomial.coefficient() - 17.0).abs() < 1e-12));
}

#[test]
#[should_panic(expected = "positive finite step")]
fn in_step_range_indicator_rejects_non_positive_step() {
    let _ = InStepRangeIndicatorFunction::<f64>::new(
        20_002,
        "invalid_step_indicator",
        Linear::constant(0.0),
        0.0,
        1.0,
        0.0,
    );
}

#[test]
#[should_panic(expected = "positive finite Big-M")]
fn in_step_range_indicator_rejects_non_positive_big_m() {
    let _ = InStepRangeIndicatorFunction::<f64>::with_big_m(
        20_004,
        "invalid_big_m_indicator",
        Linear::constant(0.0),
        0.0,
        1.0,
        1.0,
        0.0,
    );
}
