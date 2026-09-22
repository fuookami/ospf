use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticPositivePartFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::{Category, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};
use std::collections::HashMap;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn quadratic_positive_part_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<QuadraticPositivePartFunction<f64>>();
}

#[test]
fn quadratic_positive_part_clamps_negative_values_to_zero() {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
    let function = QuadraticPositivePartFunction::new(1, "positive_part", input);

    let negative = Token::from_generic(x.clone(), 0);
    negative.set_result(-2.0);
    let mut negative_tokens = VecTokenList::new();
    negative_tokens.add_token(negative);
    assert_eq!(function.calculate_value(&negative_tokens, false), Some(0.0));

    let positive = Token::from_generic(x, 0);
    positive.set_result(3.0);
    let mut positive_tokens = VecTokenList::new();
    positive_tokens.add_token(positive);
    assert_eq!(function.calculate_value(&positive_tokens, false), Some(3.0));

    let zero = Token::from_generic(
        ContinuousVariableItem::create(VariableId::standalone(0), "x_zero"),
        0,
    );
    zero.set_result(0.0);
    let mut zero_tokens = VecTokenList::new();
    zero_tokens.add_token(zero);
    assert_eq!(function.calculate_value(&zero_tokens, false), Some(0.0));
}

#[test]
fn quadratic_positive_part_has_expected_helpers_and_rows() {
    let linear = QuadraticPositivePartFunction::new(
        2,
        "linear_positive",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
    );
    let mut linear_tokens = Vec::new();
    linear.register_tokens(&mut linear_tokens).unwrap();
    assert_eq!(linear_tokens.len(), 3, "max result plus two selectors");
    let linear_indices: HashMap<usize, usize> = linear_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect();
    assert_eq!(
        linear.mechanism_constraints(&linear_indices).unwrap().len(),
        5
    );
    assert!(linear
        .quadratic_mechanism_constraints(&linear_indices)
        .unwrap()
        .is_empty());

    let quadratic = QuadraticPositivePartFunction::new(
        3,
        "quadratic_positive",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
    );
    let mut quadratic_tokens = Vec::new();
    quadratic.register_tokens(&mut quadratic_tokens).unwrap();
    assert_eq!(
        quadratic_tokens.len(),
        4,
        "quadratic bridge plus max helpers"
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
        5
    );
    assert_eq!(
        quadratic
            .quadratic_mechanism_constraints(&quadratic_indices)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        <QuadraticPositivePartFunction<f64> as IntermediateSymbol<f64>>::category(&quadratic),
        Category::Linear
    );
    assert_eq!(
        <QuadraticPositivePartFunction<f64> as IntermediateSymbol<f64>>::operation_category(
            &quadratic,
        ),
        Category::Quadratic
    );
}

#[test]
fn quadratic_positive_part_rejects_an_invalid_explicit_big_m() {
    let function = QuadraticPositivePartFunction::with_big_m(
        4,
        "bad_positive_m",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
        0.0,
    );
    let mut tokens = Vec::new();
    function.register_tokens(&mut tokens).unwrap();
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    assert!(function.mechanism_constraints(&symbol_to_index).is_err());
}

#[test]
fn quadratic_positive_part_prepare_ignores_a_stale_result_value() {
    let function = QuadraticPositivePartFunction::new(
        5,
        "source_positive_prepare",
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 2, 3)], 0.0),
    );
    let mut values = HashMap::new();
    values.insert(2, -2.0);
    values.insert(3, 3.0);
    values.insert(99, 999.0);
    assert_eq!(function.prepare(&values), Some(0.0));
}
