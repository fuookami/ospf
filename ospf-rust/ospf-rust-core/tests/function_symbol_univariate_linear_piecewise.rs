use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{Point2, UnivariateLinearPiecewiseFunction};
use ospf_rust_core::symbol::{FunctionSymbol, IntermediateSymbol};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn UnivariateLinearPiecewise_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<UnivariateLinearPiecewiseFunction<f64>>();
}

#[test]
fn univariate_piecewise_has_one_selector_per_adjacent_segment() {
    let function = UnivariateLinearPiecewiseFunction::new(
        1,
        "ulp",
        Linear::new(vec![], 1.0),
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 2.0),
            Point2::new(2.0, 0.0),
        ],
    );
    assert_eq!(function.lambda_variables().len(), 3);
    assert_eq!(function.selector_variables().len(), 2);
}

#[test]
fn univariate_piecewise_registers_adjacent_selector_and_graph_rows() {
    let function = UnivariateLinearPiecewiseFunction::new(
        1_001,
        "ulp_rows",
        Linear::constant(0.0),
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 2.0),
            Point2::new(2.0, 0.0),
        ],
    );
    let mut tokens = Vec::new();
    function
        .register_auxiliary_tokens(&mut tokens)
        .expect("ulp helpers should register");
    assert_eq!(tokens.len(), 6, "result, three weights, and two selectors");
    let symbol_to_index: HashMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index))
        .collect();
    let constraints = function
        .mechanism_constraints(&symbol_to_index)
        .expect("ulp graph rows should be constructible");
    assert_eq!(constraints.len(), 7);
    assert!(constraints
        .iter()
        .any(|row| row.name == "ulp_rows_ulp_selector_sum"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "ulp_rows_ulp_adjacent_1"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "ulp_rows_ulp_x_relation"));
    assert!(constraints
        .iter()
        .any(|row| row.name == "ulp_rows_ulp_y_relation"));
}

#[test]
fn univariate_piecewise_segment_factory_preserves_point_contract() {
    let function = UnivariateLinearPiecewiseFunction::from_segments(
        1_002,
        "ulp_segments",
        Linear::constant(1.5),
        vec![0.0, 1.0, 2.0],
        vec![2.0, -2.0],
        vec![0.0, 4.0],
    );
    assert_eq!(function.points().len(), 3);
    assert_eq!(function.selector_variables().len(), 2);
}

#[test]
#[should_panic(expected = "strictly increasing")]
fn univariate_piecewise_rejects_duplicate_x_values() {
    let _ = UnivariateLinearPiecewiseFunction::new(
        2,
        "ulp_duplicate",
        Linear::new(vec![], 0.0),
        vec![Point2::new(0.0, 0.0), Point2::new(0.0, 1.0)],
    );
}
