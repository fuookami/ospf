use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticInStepRangeFunction;
use ospf_rust_core::symbol::{FunctionSymbol, IntermediateSymbol};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};
use std::collections::HashMap;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn QuadraticInStepRange_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<QuadraticInStepRangeFunction<f64>>();
}

#[test]
fn quadratic_in_step_range_is_a_closed_interval_gate() {
    let x = ContinuousVariableItem::create(VariableId::standalone(11), "x");
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
    let function =
        QuadraticInStepRangeFunction::with_big_m(12, "interval_gate", input, 0.0, 4.0, 100.0);

    let mut inside = VecTokenList::<f64>::new();
    let token = Token::from_generic(x.clone(), 0);
    token.set_result(4.0);
    inside.add_token(token);
    assert_eq!(function.calculate_value(&inside, false), Some(4.0));

    let mut outside = VecTokenList::<f64>::new();
    let token = Token::from_generic(x, 0);
    token.set_result(4.1);
    outside.add_token(token);
    assert_eq!(function.calculate_value(&outside, false), Some(0.0));
}

#[test]
fn quadratic_in_step_range_returns_none_in_the_exterior_tolerance_band() {
    let x = ContinuousVariableItem::create(VariableId::standalone(10), "x");
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
    let function = QuadraticInStepRangeFunction::with_parameters(
        11,
        "interval_tolerance",
        input,
        0.0,
        4.0,
        100.0,
        0.1,
    );

    let mut tokens = VecTokenList::<f64>::new();
    let token = Token::from_generic(x, 0);
    token.set_result(4.05);
    tokens.add_token(token);
    assert_eq!(function.calculate_value(&tokens, false), None);
}

#[test]
fn quadratic_in_step_range_default_tolerance_and_signs_are_stable() {
    let x = ContinuousVariableItem::create(VariableId::standalone(20), "x");
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
    let function = QuadraticInStepRangeFunction::new(21, "default_tolerance", input, 0.0, 4.0);

    let evaluate = |value: f64| {
        let mut tokens = VecTokenList::<f64>::new();
        let token = Token::from_generic(x.clone(), 0);
        token.set_result(value);
        tokens.add_token(token);
        function.calculate_value(&tokens, false)
    };

    assert_eq!(evaluate(-1.0), Some(0.0));
    assert_eq!(evaluate(0.0), Some(0.0));
    assert_eq!(evaluate(2.0), Some(2.0));
    assert_eq!(evaluate(4.0 + 0.5e-6), None);
    assert_eq!(evaluate(4.0 + 1.0e-6), Some(0.0));
}

#[test]
fn quadratic_in_step_range_registers_four_helpers_and_nine_rows() {
    let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
    let function =
        QuadraticInStepRangeFunction::with_big_m(22, "quadratic_rows", input, 0.0, 4.0, 100.0);

    let mut tokens = Vec::new();
    function
        .register_tokens(&mut tokens)
        .expect("four interval helpers should register");
    assert_eq!(tokens.len(), 4);
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect();

    let rows = function
        .quadratic_mechanism_constraints(&symbol_to_index)
        .expect("quadratic interval rows should generate");
    assert_eq!(rows.len(), 9);
    assert_eq!(
        <QuadraticInStepRangeFunction<f64> as IntermediateSymbol<f64>>::category(&function),
        ospf_rust_core::symbol::Category::Linear
    );
    assert_eq!(
        <QuadraticInStepRangeFunction<f64> as IntermediateSymbol<f64>>::operation_category(
            &function
        ),
        ospf_rust_core::symbol::Category::Quadratic
    );
}

#[test]
fn quadratic_in_step_range_rejects_invalid_bounds_and_big_m() {
    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
    assert!(std::panic::catch_unwind(|| {
        QuadraticInStepRangeFunction::new(23, "invalid_bounds", input.clone(), 4.0, 0.0);
    })
    .is_err());

    let function =
        QuadraticInStepRangeFunction::with_big_m(24, "invalid_big_m", input, 0.0, 4.0, 0.0);
    let mut tokens = Vec::new();
    function
        .register_tokens(&mut tokens)
        .expect("helper registration should not validate Big-M");
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    assert!(function.mechanism_constraints(&symbol_to_index).is_err());
}

#[test]
fn quadratic_in_step_range_prepare_ignores_a_stale_result_token() {
    let function = QuadraticInStepRangeFunction::with_big_m(
        25,
        "prepare_source",
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 2)], 0.0),
        0.0,
        4.0,
        100.0,
    );
    let mut values = HashMap::new();
    values.insert(2, 2.0);
    // Standalone helper variables use index 0; the source variable deliberately
    // uses index 2 so a stale helper value can coexist in the map.
    values.insert(function.result_variable().index(), 999.0);
    assert_eq!(function.prepare(&values), Some(2.0));
}
