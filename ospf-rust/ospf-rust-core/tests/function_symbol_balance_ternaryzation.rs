use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::BalanceTernaryzationFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::IntermediateSymbol;
use ospf_rust_core::token::VecTokenList;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn balance_ternaryzation_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<BalanceTernaryzationFunction<f64>>();
}

#[test]
fn balance_ternaryzation_evaluates_the_input_zero_band() {
    let tokens = VecTokenList::<f64>::new();
    for (input, expected) in [
        (-2.0, -1.0),
        (-0.1, 0.0),
        (0.0, 0.0),
        (0.1, 0.0),
        (2.0, 1.0),
    ] {
        let function =
            BalanceTernaryzationFunction::new(10, "bter", Linear::new(vec![], input), 0.1, 100.0);
        assert_eq!(function.calculate_value(&tokens, false), Some(expected));

        let mut helpers = Vec::new();
        function.register_tokens(&mut helpers).unwrap();
        assert_eq!(helpers.len(), 3, "result and two state indicators");
    }
}

#[test]
#[should_panic(expected = "epsilon must be finite and non-negative")]
fn balance_ternaryzation_rejects_negative_epsilon() {
    let _ = BalanceTernaryzationFunction::new(
        20,
        "invalid_bter",
        Linear::new(vec![], 0.0),
        -0.1,
        100.0,
    );
}

#[test]
fn balance_ternaryzation_matches_strict_solver_boundary() {
    let tokens = VecTokenList::<f64>::new();
    let epsilon = 0.1;
    let delta = 1e-10;
    let at_positive_boundary = BalanceTernaryzationFunction::new(
        21,
        "bter_boundary",
        Linear::new(vec![], epsilon + delta),
        epsilon,
        100.0,
    );
    let in_positive_gap = BalanceTernaryzationFunction::new(
        22,
        "bter_positive_gap",
        Linear::new(vec![], epsilon + delta * 0.5),
        epsilon,
        100.0,
    );
    let at_negative_boundary = BalanceTernaryzationFunction::new(
        23,
        "bter_negative_boundary",
        Linear::new(vec![], -epsilon - delta),
        epsilon,
        100.0,
    );
    assert_eq!(
        at_positive_boundary.calculate_value(&tokens, false),
        Some(1.0)
    );
    assert_eq!(in_positive_gap.calculate_value(&tokens, false), None);
    assert_eq!(
        at_negative_boundary.calculate_value(&tokens, false),
        Some(-1.0)
    );

    let mut auxiliary_tokens = Vec::new();
    at_positive_boundary
        .register_tokens(&mut auxiliary_tokens)
        .unwrap();
    let symbol_to_index = auxiliary_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
        .collect::<HashMap<_, _>>();
    let constraints = at_positive_boundary
        .mechanism_constraints(&symbol_to_index)
        .unwrap();
    assert_eq!(constraints.len(), 6);
    assert!(constraints
        .iter()
        .any(|row| row.name == "bter_boundary_bter_positive_lb"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "bter_boundary_bter_negative_lb"));
    let positive_lower = constraints
        .iter()
        .find(|row| row.name == "bter_boundary_bter_positive_lb")
        .unwrap();
    assert!((positive_lower.inequality.rhs - (epsilon + delta - 100.0)).abs() <= 1e-9);
}
