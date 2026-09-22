use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::SlackFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::IntermediateSymbol;
use ospf_rust_core::token::VecTokenList;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn Slack_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<SlackFunction<f64>>();
}

#[test]
fn slack_evaluates_absolute_difference_and_registers_exact_rows() {
    let function = SlackFunction::with_big_m(
        4_000,
        "slack_rows",
        Linear::new(vec![], 3.0),
        Linear::new(vec![], 1.0),
        42.0,
    );
    let tokens = VecTokenList::<f64>::new();
    assert_eq!(function.calculate_value(&tokens, false), Some(2.0));

    let mut auxiliary_tokens = Vec::new();
    function.register_tokens(&mut auxiliary_tokens).unwrap();
    let symbol_to_index = auxiliary_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect::<HashMap<_, _>>();
    let constraints = function.mechanism_constraints(&symbol_to_index).unwrap();
    assert_eq!(constraints.len(), 4);
    for name in [
        "slack_rows_slack_ge_diff",
        "slack_rows_slack_ge_neg_diff",
        "slack_rows_slack_branch_pos",
        "slack_rows_slack_branch_neg",
    ] {
        assert!(
            constraints.iter().any(|row| row.name == name),
            "missing {name}"
        );
    }
    let branch = constraints
        .iter()
        .find(|row| row.name == "slack_rows_slack_branch_pos")
        .unwrap();
    assert!((branch.inequality.rhs - 42.0_f64).abs() <= 1e-9_f64);
}
