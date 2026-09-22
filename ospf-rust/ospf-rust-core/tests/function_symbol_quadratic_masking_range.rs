use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMaskingRangeFunction;
use ospf_rust_core::symbol::{Category, FunctionSymbol, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId};
use std::collections::HashMap;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn QuadraticMaskingRange_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<QuadraticMaskingRangeFunction<f64>>();
}

#[test]
fn quadratic_masking_range_returns_signed_quadratic_value_when_enabled() {
    let mask = BinaryVariableItem::create(VariableId::standalone(1), "mask");
    let x = ContinuousVariableItem::create(VariableId::standalone(2), "x");
    let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 1, 1)], 0.0);
    let function =
        QuadraticMaskingRangeFunction::with_big_m(3, "masked_square", input, mask.clone(), 100.0);

    let mut enabled = VecTokenList::<f64>::new();
    let mask_token = Token::from_generic(mask.clone(), 0);
    mask_token.set_result(1.0);
    enabled.add_token(mask_token);
    let x_token = Token::from_generic(x, 1);
    x_token.set_result(-2.0);
    enabled.add_token(x_token);
    assert_eq!(function.calculate_value(&enabled, false), Some(4.0));

    let mut disabled = VecTokenList::<f64>::new();
    let mask_token = Token::from_generic(mask, 0);
    mask_token.set_result(0.0);
    disabled.add_token(mask_token);
    assert_eq!(function.calculate_value(&disabled, true), Some(0.0));
}

#[test]
fn quadratic_masking_range_missing_mask_is_the_disabled_branch() {
    let mask = BinaryVariableItem::create(VariableId::standalone(3), "missing_mask");
    let x = ContinuousVariableItem::create(VariableId::standalone(4), "x");
    let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(-1.0, 1, 1)], 0.0);
    let function =
        QuadraticMaskingRangeFunction::with_big_m(5, "missing_mask_value", input, mask, 100.0);
    let mut tokens = VecTokenList::<f64>::new();
    let x_token = Token::from_generic(x, 1);
    x_token.set_result(2.0);
    tokens.add_token(x_token);
    assert_eq!(function.calculate_value(&tokens, false), Some(0.0));
    assert_eq!(function.calculate_value(&tokens, true), Some(0.0));
}

#[test]
fn quadratic_masking_range_registers_one_result_helper_and_four_rows() {
    let mask = BinaryVariableItem::create(VariableId::standalone(6), "row_mask");
    let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 2, 2)], 0.0);
    let function =
        QuadraticMaskingRangeFunction::with_big_m(7, "mask_rows", input, mask.clone(), 100.0);

    let mut tokens = Vec::new();
    function
        .register_tokens(&mut tokens)
        .expect("mask result helper should register");
    assert_eq!(tokens.len(), 1);
    let mut symbol_to_index = HashMap::new();
    symbol_to_index.insert(function.result_variable().id().unique_id() as usize, 10);
    symbol_to_index.insert(mask.id().unique_id() as usize, 11);
    let rows = function
        .quadratic_mechanism_constraints(&symbol_to_index)
        .expect("quadratic masking rows should generate");
    assert_eq!(rows.len(), 4);
    assert_eq!(
        <QuadraticMaskingRangeFunction<f64> as IntermediateSymbol<f64>>::category(&function),
        Category::Linear
    );
    assert_eq!(
        <QuadraticMaskingRangeFunction<f64> as IntermediateSymbol<f64>>::operation_category(
            &function,
        ),
        Category::Quadratic
    );
}

#[test]
fn quadratic_masking_range_rejects_nonpositive_big_m() {
    let mask = BinaryVariableItem::create(VariableId::standalone(8), "bad_mask");
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 3)], 0.0);
    let function =
        QuadraticMaskingRangeFunction::with_big_m(9, "bad_mask_m", input, mask.clone(), 0.0);
    let mut tokens = Vec::new();
    function.register_tokens(&mut tokens).unwrap();
    let mut symbol_to_index = HashMap::new();
    symbol_to_index.insert(function.result_variable().id().unique_id() as usize, 10);
    symbol_to_index.insert(mask.id().unique_id() as usize, 11);
    assert!(function.mechanism_constraints(&symbol_to_index).is_err());
}

#[test]
fn quadratic_masking_range_prepare_uses_source_not_result_token() {
    let mask = BinaryVariableItem::create(VariableId::new(12, 1), "prepare_mask");
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 2)], 0.0);
    let function = QuadraticMaskingRangeFunction::with_big_m(
        13,
        "prepare_mask_source",
        input,
        mask.clone(),
        100.0,
    );
    let mut values = HashMap::new();
    values.insert(1, 1.0);
    values.insert(2, 3.0);
    values.insert(3, 999.0);
    assert_eq!(function.prepare(&values), Some(3.0));
}
