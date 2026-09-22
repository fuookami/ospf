use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::XorFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::IntermediateSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn Xor_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<XorFunction<f64>>();
}

#[test]
fn xor_is_one_only_when_exactly_one_of_three_inputs_is_non_zero() {
    let function = XorFunction::new(
        1,
        "xor_three",
        (0..3)
            .map(|index| Linear::new(vec![LinearMonomial::new(1.0, index)], 0.0))
            .collect(),
    );

    for (values, expected) in [
        ([0.0, 0.0, 0.0], 0.0),
        ([1.0, 0.0, 0.0], 1.0),
        ([1.0, 1.0, 0.0], 0.0),
        ([1.0, 1.0, 1.0], 0.0),
    ] {
        let mut tokens = VecTokenList::new();
        for (index, value) in values.into_iter().enumerate() {
            let variable =
                ContinuousVariableItem::create(VariableId::standalone(index), &format!("x{index}"));
            let token = Token::from_generic(variable, index);
            token.set_result(value);
            tokens.add_token(token);
        }
        assert_eq!(function.calculate_value(&tokens, false), Some(expected));
    }
}

#[test]
fn xor_accepts_a_single_input() {
    let function = XorFunction::new(
        2,
        "xor_single",
        vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
    );
    assert_eq!(function.polynomials().len(), 1);
}

#[test]
fn xor_registers_indicator_rows_and_exact_one_rows() {
    let function = XorFunction::new(
        3,
        "xor_rows",
        (0..3)
            .map(|index| Linear::new(vec![LinearMonomial::new(1.0, index)], 0.0))
            .collect(),
    )
    .with_big_m(42.0);

    let mut auxiliary_tokens = Vec::new();
    function.register_tokens(&mut auxiliary_tokens).unwrap();
    let symbol_to_index = auxiliary_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect::<HashMap<_, _>>();

    let constraints = function.mechanism_constraints(&symbol_to_index).unwrap();
    // 4 indicator rows per input, one upper-sum row, three single rows,
    // and three pair rows for exactly-one semantics.
    assert_eq!(constraints.len(), 19);
    assert!(constraints
        .iter()
        .any(|row| row.name == "xor_rows_xor_nz_0_band_ub"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "xor_rows_xor_nz_2_out_ub"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "xor_rows_xor_sum_ub"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "xor_rows_xor_single_1"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "xor_rows_xor_pair_1_2"));

    let band = constraints
        .iter()
        .find(|row| row.name == "xor_rows_xor_nz_0_band_ub")
        .unwrap();
    assert!((band.inequality.rhs - 1e-10_f64).abs() <= 1e-15_f64);
    let indicator_index = *symbol_to_index
        .get(&(function.indicator_variables()[0].id().unique_id() as usize))
        .unwrap();
    let indicator_term = band
        .inequality
        .polynomial
        .monomials()
        .iter()
        .find(|monomial| monomial.var_index() == indicator_index)
        .unwrap();
    assert!((*indicator_term.coefficient() + 42.0_f64).abs() <= 1e-9_f64);
}
