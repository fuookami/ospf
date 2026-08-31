#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use ospf_rust_core::model::intermediate::{BasicLinearTriadModel, SparseVector};
use ospf_rust_core::solver::iis::{ConstraintSource, IISConfig, compute_iis};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn sparse_row(entries: &[(usize, f64)]) -> SparseVector<f64> {
    let mut row = SparseVector::new();
    for (index, value) in entries {
        row.add(*index, *value);
    }
    row
}

fn constraint_conflict_model() -> BasicLinearTriadModel {
    let mut model = BasicLinearTriadModel::new("iis_constraints_conflict");
    let x = ContinuousVariableItem::create(VariableId::standalone(5101), "x");
    model.add_variable_with_bounds(
        Token::from_generic(x, 0),
        f64::NEG_INFINITY,
        f64::INFINITY,
        VariableType::Continuous,
    );

    // x <= 0
    model.add_constraint(sparse_row(&[(0, 1.0)]), 0.0);
    // x >= 1 -> -x <= -1
    model.add_constraint(sparse_row(&[(0, -1.0)]), -1.0);
    model
}

fn bound_conflict_model() -> BasicLinearTriadModel {
    let mut model = BasicLinearTriadModel::new("iis_bounds_conflict");
    let x = ContinuousVariableItem::create(VariableId::standalone(5201), "x");
    model.add_variable_with_bounds(
        Token::from_generic(x, 0),
        1.0,
        0.0,
        VariableType::Continuous,
    );
    model
}

#[test]
fn gurobi_iis_deletion_finds_constraint_conflict() {
    let model = constraint_conflict_model();
    let config = IISConfig::new()
        .with_deletion_filtering()
        .with_bounds(false)
        .with_tolerance(1e-8);

    let iis = compute_iis(&model, &config).unwrap();
    assert_eq!(iis.num_constraints(), 2);
    assert_eq!(iis.num_bounds(), 0);
    assert!(iis.contains_constraint(0));
    assert!(iis.contains_constraint(1));
}

#[test]
fn gurobi_iis_elastic_finds_constraint_conflict() {
    let model = constraint_conflict_model();
    let config = IISConfig::new()
        .with_elastic_filtering()
        .with_bounds(false)
        .with_tolerance(1e-8);

    let iis = compute_iis(&model, &config).unwrap();
    assert_eq!(iis.num_constraints(), 2);
    assert_eq!(iis.num_bounds(), 0);
    assert!(iis.contains_constraint(0));
    assert!(iis.contains_constraint(1));
}

#[test]
fn gurobi_iis_deletion_finds_bound_conflict() {
    let model = bound_conflict_model();
    let config = IISConfig::new()
        .with_deletion_filtering()
        .with_bounds(true)
        .with_tolerance(1e-8);

    let iis = compute_iis(&model, &config).unwrap();
    assert_eq!(iis.num_constraints(), 0);
    assert_eq!(iis.num_bounds(), 2);
    assert!(iis.contains_bound(&ConstraintSource::LowerBound(0)));
    assert!(iis.contains_bound(&ConstraintSource::UpperBound(0)));
}

#[test]
fn gurobi_iis_elastic_finds_bound_conflict() {
    let model = bound_conflict_model();
    let config = IISConfig::new()
        .with_elastic_filtering()
        .with_bounds(true)
        .with_tolerance(1e-8);

    let iis = compute_iis(&model, &config).unwrap();
    assert_eq!(iis.num_constraints(), 0);
    assert_eq!(iis.num_bounds(), 2);
    assert!(iis.contains_bound(&ConstraintSource::LowerBound(0)));
    assert!(iis.contains_bound(&ConstraintSource::UpperBound(0)));
}
