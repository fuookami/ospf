use std::collections::HashMap;

use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{BivariateLinearPiecewiseFunction, Point3, Triangle3};
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::IntermediateSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn BivariateLinearPiecewise_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<BivariateLinearPiecewiseFunction<f64>>();
}

#[test]
fn bivariate_piecewise_uses_one_selector_per_triangle_and_barycentric_evaluation() {
    let function = BivariateLinearPiecewiseFunction::new(
        1,
        "blp",
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
        vec![Triangle3::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 10.0),
            Point3::new(0.0, 1.0, 20.0),
        )],
    );
    assert_eq!(function.lambda_variables().len(), 3);
    assert_eq!(function.selector_variables().len(), 1);

    let mut tokens = VecTokenList::new();
    for (index, value) in [0.5, 0.25].into_iter().enumerate() {
        let variable =
            ContinuousVariableItem::create(VariableId::standalone(index), &format!("blp_x{index}"));
        let token = Token::from_generic(variable, index);
        token.set_result(value);
        tokens.add_token(token);
    }
    assert_eq!(function.calculate_value(&tokens, false), Some(10.0));
}

#[test]
#[should_panic(expected = "non-degenerate")]
fn bivariate_piecewise_rejects_degenerate_triangles() {
    let _ = BivariateLinearPiecewiseFunction::new(
        2,
        "blp_degenerate",
        Linear::new(vec![], 0.0),
        Linear::new(vec![], 0.0),
        vec![Triangle3::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(2.0, 2.0, 2.0),
        )],
    );
}

#[test]
fn bivariate_piecewise_accepts_matching_boundary_tolerance_and_registers_rows() {
    let function = BivariateLinearPiecewiseFunction::new(
        3,
        "blp_boundary",
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
        vec![Triangle3::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 10.0),
            Point3::new(0.0, 1.0, 20.0),
        )],
    );

    let mut tokens = VecTokenList::new();
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "blp_boundary_x");
    let x_token = Token::from_generic(x, 0);
    x_token.set_result(-0.5e-12);
    tokens.add_token(x_token);
    let y = ContinuousVariableItem::create(VariableId::standalone(1), "blp_boundary_y");
    let y_token = Token::from_generic(y, 1);
    y_token.set_result(0.0);
    tokens.add_token(y_token);
    assert!(function.calculate_value(&tokens, false).is_some());

    let mut outside_tokens = VecTokenList::new();
    let x_out = ContinuousVariableItem::create(VariableId::standalone(0), "blp_boundary_x");
    let x_out_token = Token::from_generic(x_out, 0);
    x_out_token.set_result(-2.0e-12);
    outside_tokens.add_token(x_out_token);
    let y_out = ContinuousVariableItem::create(VariableId::standalone(1), "blp_boundary_y");
    let y_out_token = Token::from_generic(y_out, 1);
    y_out_token.set_result(0.0);
    outside_tokens.add_token(y_out_token);
    assert_eq!(function.calculate_value(&outside_tokens, false), None);

    let mut auxiliary_tokens = Vec::new();
    function.register_tokens(&mut auxiliary_tokens).unwrap();
    let symbol_to_index = auxiliary_tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.id().unique_id() as usize, index + 10))
        .collect::<HashMap<_, _>>();
    let constraints = function.mechanism_constraints(&symbol_to_index).unwrap();
    assert_eq!(constraints.len(), 5);
    for name in [
        "blp_boundary_blp_selector_sum",
        "blp_boundary_blp_triangle_sum_0",
        "blp_boundary_blp_x_relation",
        "blp_boundary_blp_y_relation",
        "blp_boundary_blp_z_relation",
    ] {
        assert!(
            constraints.iter().any(|row| row.name == name),
            "missing {name}"
        );
    }
}
