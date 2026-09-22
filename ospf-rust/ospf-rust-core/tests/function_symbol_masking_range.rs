use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::MaskingRangeFunction;
use ospf_rust_core::symbol::{FunctionSymbol, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn MaskingRange_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<MaskingRangeFunction<f64>>();
}

#[test]
fn masking_range_supports_signed_bounds_and_missing_values() {
    let mask = BinaryVariableItem::create(VariableId::new(3_001, 1), "range_mask");
    let mask_index = mask.index();
    let mask_poly = Linear::new(vec![LinearMonomial::new(1.0, mask_index)], 0.0);
    let function = MaskingRangeFunction::new(3_002, "signed_range", mask_poly, -2.0, 3.0);

    let mut on_tokens = VecTokenList::new();
    let mask_token = Token::from_generic(mask.clone(), mask_index);
    mask_token.set_result(1.0);
    on_tokens.add_token(mask_token);
    let result_token = Token::from_generic(
        function.result_variable().clone(),
        function.result_variable().index(),
    );
    result_token.set_result(-3.0);
    on_tokens.add_token(result_token);
    assert_eq!(function.calculate_value(&on_tokens, false), Some(-2.0));

    let mut missing_mask = VecTokenList::new();
    let missing_result = Token::from_generic(
        function.result_variable().clone(),
        function.result_variable().index(),
    );
    missing_result.set_result(1.0);
    missing_mask.add_token(missing_result);
    assert_eq!(function.calculate_value(&missing_mask, false), None);

    let mut missing_result_tokens = VecTokenList::new();
    let present_mask = Token::from_generic(mask, mask_index);
    present_mask.set_result(1.0);
    missing_result_tokens.add_token(present_mask);
    assert_eq!(
        function.calculate_value(&missing_result_tokens, false),
        None
    );
}

#[test]
fn masking_range_registers_signed_lower_and_upper_rows() {
    let mask = BinaryVariableItem::create(VariableId::new(3_011, 13), "range_mask_rows");
    let function = MaskingRangeFunction::new(
        3_012,
        "signed_range_rows",
        Linear::new(vec![LinearMonomial::new(1.0, mask.index())], 0.0),
        -2.0,
        3.0,
    );
    let symbol_to_index = HashMap::from([
        (
            function.result_variable().id().unique_id() as usize,
            12usize,
        ),
        (mask.id().unique_id() as usize, 13usize),
    ]);
    let constraints = function.mechanism_constraints(&symbol_to_index).unwrap();
    assert_eq!(constraints.len(), 2);
    assert!(constraints
        .iter()
        .any(|row| row.name == "signed_range_rows_masking_range_ub"));
    let lower = constraints
        .iter()
        .find(|row| row.name == "signed_range_rows_masking_range_lb")
        .unwrap();
    let mask_term = lower
        .inequality
        .polynomial
        .monomials()
        .iter()
        .find(|monomial| monomial.var_index() == 13)
        .unwrap();
    assert!((*mask_term.coefficient() - 2.0_f64).abs() <= 1e-9_f64);
}
