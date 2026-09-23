#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::Arc;

use ospf_rust_core::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use ospf_rust_core::intermediate::{SparseMatrix, SparseVector};
use ospf_rust_core::model::{
    ConstraintRelation, LinearConstraint, LinearInequality, MetaModel, ObjectiveCategory,
    QuadraticConstraint, QuadraticInequality, SubObjective,
};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::symbol::function::{
    BinaryzationMethod, ConditionBounds, ConditionRelation, InequalityKind, Point2, Point3,
    Triangle3, QuadraticBinaryzationFunction, QuadraticBivariateLinearPiecewiseFunction,
    QuadraticCosFunction, QuadraticIfFunction, QuadraticIfInFunction, QuadraticIfThenFunction,
    QuadraticInStepRangeFunction, QuadraticInequalityFunction, QuadraticMaskingFunction,
    QuadraticMaskingRangeFunction, QuadraticMaxFunction, QuadraticMaxMinFunction,
    QuadraticMinFunction, QuadraticMinMaxFunction, QuadraticModFunction, QuadraticRoundingFunction,
    QuadraticPositivePartFunction, QuadraticLogisticFunction, QuadraticSinFunction,
    QuadraticSlackFunction,
    QuadraticSlackRangeFunction, QuadraticUnivariateLinearPiecewiseFunction,
};
use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-5,
        "expected {}, got {}",
        expected,
        actual
    );
}

#[test]
fn gurobi_solves_quadratic_constraint_model() {
    let mut model = MetaModel::<f64>::new("quadratic_constraint");

    let x = ContinuousVariableItem::create(VariableId::standalone(1900), "x");
    let y = ContinuousVariableItem::create(VariableId::standalone(1901), "y");
    let x_index = model.register_variable(x).unwrap();
    let y_index = model.register_variable(y).unwrap();

    model.maximize();
    model.add_sub_objective(SubObjective::new(
        ObjectiveCategory::Maximum,
        Linear::new(
            vec![
                LinearMonomial::new(1.0, x_index),
                LinearMonomial::new(1.0, y_index),
            ],
            0.0,
        ),
        "max_x_plus_y",
    ));

    let mut mechanism = model.try_into_mechanism_model().unwrap();
    let unit_ball = Quadratic::new(
        vec![
            QuadraticMonomial::new_quadratic(1.0, x_index, x_index),
            QuadraticMonomial::new_quadratic(1.0, y_index, y_index),
        ],
        0.0,
    );
    mechanism.add_quadratic_constraint(QuadraticConstraint::new(
        QuadraticInequality::new(unit_ball, ConstraintRelation::LessEqual, 1.0),
        "unit_ball",
    ));

    let quadratic = mechanism.into_quadratic_tetrad_model();
    assert_eq!(quadratic.num_quadratic_constraints(), 1);

    let solver = GurobiSolver::new();
    let output = solver.solve_quadratic(&quadratic).unwrap();
    assert!(output.status.is_feasible(), "status: {:?}", output.status);
    let objective = output.objective_value.unwrap();
    assert_close(objective, std::f64::consts::SQRT_2);
}

#[test]
fn gurobi_solves_quadratic_objective_model() {
    let mut model = MetaModel::<f64>::new("quadratic_objective");

    let x = ContinuousVariableItem::create(VariableId::standalone(1910), "x");
    let x_index = model.register_variable(x).unwrap();
    let mut mechanism = model.try_into_mechanism_model().unwrap();

    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            2.0,
        ),
        "x_lb",
    ));

    let mut quadratic = mechanism.into_quadratic_tetrad_model();
    let n = quadratic.num_variables();
    let mut c = vec![0.0; n];
    c[x_index] = 0.0;
    let mut q = SparseMatrix::new();
    for _ in 0..n {
        q.add_row(SparseVector::new());
    }
    q.rows[x_index].add(x_index, 1.0);
    quadratic.set_objective(c, q, ObjectiveCategory::Minimum);

    let solver = GurobiSolver::new();
    let output = solver.solve_quadratic(&quadratic).unwrap();
    assert!(output.status.is_feasible(), "status: {:?}", output.status);
    assert_close(output.objective_value.unwrap(), 4.0);
    let solution = output.solution.unwrap();
    assert_close(solution[x_index], 2.0);
}

#[test]
fn gurobi_solves_quadratic_sigmoid_function_symbol_model() {
    let mut model = MetaModel::<f64>::new("quadratic_sigmoid_symbol");

    let x = ContinuousVariableItem::create(VariableId::standalone(1920), "x");
    let x_index = model.register_variable(x).unwrap();

    let input = Quadratic::new(
        vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
        0.0,
    );
    let qsigmoid = QuadraticLogisticFunction::new(1921, "qsigmoid", input);
    let y_id = qsigmoid.result_variable().id();
    model.add_symbol(Arc::new(qsigmoid)).unwrap();

    let mut mechanism = model.try_into_mechanism_model().unwrap();
    let y_index = mechanism.find_token(y_id).unwrap().solver_index;

    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "x_eq_0",
    ));
    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "sigmoid_eq_half",
    ));

    let quadratic = mechanism.into_quadratic_tetrad_model();
    let solver = GurobiSolver::new();
    let output = solver.solve_quadratic(&quadratic).unwrap();
    assert!(output.status.is_feasible(), "status: {:?}", output.status);

    let solution = output.solution.unwrap();
    assert_close(solution[x_index], 0.0);
    assert_close(solution[y_index], 0.5);
}

#[test]
fn gurobi_solves_quadratic_masking_range_with_binary_mask() {
    let mut model = MetaModel::<f64>::new("quadratic_masking_range_poly");

    let x = ContinuousVariableItem::create(VariableId::standalone(1930), "x");
    let mask = BinaryVariableItem::create(VariableId::standalone(1931), "mask");
    let x_index = model.register_variable(x).unwrap();
    let mask_index = model.register_variable(mask.clone()).unwrap();

    let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)], 0.0);
    let qmask_range = QuadraticMaskingRangeFunction::with_big_m(
        1932,
        "qmask_range_binary",
        input,
        mask.clone(),
        100.0,
    );
    let y_id = qmask_range.result_variable().id();
    model.add_symbol(Arc::new(qmask_range)).unwrap();

    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;
    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, mask_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "mask_eq_1",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            4.0,
        ),
        "y_eq_3",
    ));

    let solver = GurobiSolver::new();
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    let y_index2 = y_index;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, mask_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "mask_eq_1",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index2)], 0.0),
            ConstraintRelation::Equal,
            4.5,
        ),
        "y_eq_3_5",
    ));

    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_in_step_range_value_model() {
    let mut model = MetaModel::<f64>::new("quadratic_in_step_range_value");

    let x = ContinuousVariableItem::create(VariableId::standalone(1940), "x");
    let x_index = model.register_variable(x).unwrap();

    let upper_poly = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, x_index)], 0.0);
    let qstep = QuadraticInStepRangeFunction::new(1941, "qstep", upper_poly, 0.0, 4.0);
    let y_id = qstep.result_variable().id();
    model.add_symbol(Arc::new(qstep)).unwrap();

    let solver = GurobiSolver::new();

    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;
    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            3.7,
        ),
        "x_eq_3_7",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            3.7,
        ),
        "y_eq_3_7",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    let y_index2 = y_index;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            3.7,
        ),
        "x_eq_3_7",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index2)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_binaryzation_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_binaryzation_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(1944), "x");
    let x_index = model.register_variable(x).unwrap();

    // input = x^2; choose x=0.6 so x>0.5 but x^2<0.5
    let input = Quadratic::new(
        vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
        0.0,
    );
    let qbin = QuadraticBinaryzationFunction::new(
        1945,
        "qbin_non_linear",
        input,
        0.5,
        10.0,
        BinaryzationMethod::BigM,
    );
    let y_id = qbin.result_variable().id();
    model.add_symbol(Arc::new(qbin)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.6,
        ),
        "x_eq_0_6",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.6,
        ),
        "x_eq_0_6",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_inequality_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_inequality_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(1946), "x");
    let x_index = model.register_variable(x).unwrap();

    // input = x^2; choose x=0.6 so x>0.5 but x^2<0.5
    let input = Quadratic::new(
        vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
        0.0,
    );
    let qineq = QuadraticInequalityFunction::new(
        1947,
        "qineq_non_linear",
        input,
        0.5,
        InequalityKind::GreaterEqual,
        10.0,
    );
    let y_id = qineq.result_variable().id();
    model.add_symbol(Arc::new(qineq)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.6,
        ),
        "x_eq_0_6",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.6,
        ),
        "x_eq_0_6",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_rounding_floor_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_rounding_floor_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(1950), "x");
    let x_index = model.register_variable(x).unwrap();

    // input = x^2
    let input = Quadratic::new(
        vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
        0.0,
    );
    let qround = QuadraticRoundingFunction::floor(1951, "qround_floor", input);
    let y_id = qround.result_variable().id();
    model.add_symbol(Arc::new(qround)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    // x = 1.5 -> floor(x^2) = floor(2.25) = 2
    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "y_eq_2",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_mod_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_mod_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(1960), "x");
    let x_index = model.register_variable(x).unwrap();

    // input = x^2, divisor = 3
    let input = Quadratic::new(
        vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
        0.0,
    );
    let qmod = QuadraticModFunction::new(1961, "qmod_non_linear", input, 3.0);
    let y_id = qmod.result_variable().id();
    model.add_symbol(Arc::new(qmod)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    // x = 2 => x % 3 = 2, but x^2 % 3 = 1. We assert the quadratic-input semantics.
    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "y_eq_2",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_masking_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_masking_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(1970), "x");
    let mask = BinaryVariableItem::create(VariableId::standalone(1971), "mask");
    let x_index = model.register_variable(x).unwrap();
    let mask_index = model.register_variable(mask.clone()).unwrap();

    let input = Quadratic::new(
        vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
        0.0,
    );
    let qmask = QuadraticMaskingFunction::new(1972, "qmask_non_linear", input, mask);
    let y_id = qmask.result_variable().id();
    model.add_symbol(Arc::new(qmask)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, mask_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "mask_eq_1",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            4.0,
        ),
        "y_eq_4",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, mask_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "mask_eq_1",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "y_eq_2",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_min_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_min_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(1980), "x");
    let x_index = model.register_variable(x).unwrap();

    let qmin = QuadraticMinFunction::new(
        1981,
        "qmin_non_linear",
        vec![
            Quadratic::new(
                vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
                0.0,
            ),
            Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, x_index)], 0.0),
        ],
        true,
    );
    let y_id = qmin.result_variable().id();
    model.add_symbol(Arc::new(qmin)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.25,
        ),
        "y_eq_0_25",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "y_eq_0_5",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_max_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_max_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(1990), "x");
    let x_index = model.register_variable(x).unwrap();

    let qmax = QuadraticMaxFunction::new(
        1991,
        "qmax_non_linear",
        vec![
            Quadratic::new(
                vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
                0.0,
            ),
            Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, x_index)], 0.0),
        ],
        true,
    );
    let y_id = qmax.result_variable().id();
    model.add_symbol(Arc::new(qmax)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            4.0,
        ),
        "y_eq_4",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "y_eq_2",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_slack_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_slack_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2000), "x");
    let x_index = model.register_variable(x).unwrap();

    let qslack = QuadraticSlackFunction::new(
        2001,
        "qslack_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, x_index)], 0.0),
    );
    let y_id = qslack.result_variable().id();
    model.add_symbol(Arc::new(qslack)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.25,
        ),
        "y_eq_0_25",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_slack_range_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_slack_range_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2010), "x");
    let x_index = model.register_variable(x).unwrap();

    let qslack_range = QuadraticSlackRangeFunction::new(
        2011,
        "qslack_range_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
        1.0,
        2.0,
    );
    let y_id = qslack_range.result_variable().id();
    model.add_symbol(Arc::new(qslack_range)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.25,
        ),
        "y_eq_0_25",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_positive_part_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_positive_part_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2020), "x");
    let x_index = model.register_variable(x).unwrap();

    let positive_part = QuadraticPositivePartFunction::new(
        2021,
        "qpositive_part_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
    );
    let y_id = positive_part.result_variable().id();
    model.add_symbol(Arc::new(positive_part)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            -2.0,
        ),
        "x_eq_neg_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            4.0,
        ),
        "y_eq_4",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            -2.0,
        ),
        "x_eq_neg_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_sin_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_sin_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2030), "x");
    let x_index = model.register_variable(x).unwrap();

    let qsin = QuadraticSinFunction::new(
        2031,
        "qsin_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
    );
    let y_id = qsin.result_variable().id();
    model.add_symbol(Arc::new(qsin)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            0.2,
        ),
        "y_ge_0_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::LessEqual,
            0.3,
        ),
        "y_le_0_3",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            0.4,
        ),
        "y_ge_0_4",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::LessEqual,
            0.55,
        ),
        "y_le_0_55",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_cos_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_cos_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2040), "x");
    let x_index = model.register_variable(x).unwrap();

    let qcos = QuadraticCosFunction::new(
        2041,
        "qcos_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
    );
    let y_id = qcos.result_variable().id();
    model.add_symbol(Arc::new(qcos)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            0.94,
        ),
        "y_ge_0_94",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::LessEqual,
            1.0,
        ),
        "y_le_1_0",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            0.82,
        ),
        "y_ge_0_82",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::LessEqual,
            0.9,
        ),
        "y_le_0_9",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_univariate_piecewise_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_univariate_piecewise_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2050), "x");
    let x_index = model.register_variable(x).unwrap();

    let qulp = QuadraticUnivariateLinearPiecewiseFunction::new(
        2051,
        "qulp_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(4.0, 4.0),
        ],
    );
    let y_id = qulp.result_variable().id();
    model.add_symbol(Arc::new(qulp)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            2.25,
        ),
        "y_eq_2_25",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "y_eq_1_5",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_bivariate_piecewise_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_bivariate_piecewise_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2060), "x");
    let y = ContinuousVariableItem::create(VariableId::standalone(2061), "y");
    let x_index = model.register_variable(x).unwrap();
    let y_index = model.register_variable(y).unwrap();

    let qblp = QuadraticBivariateLinearPiecewiseFunction::new(
        2062,
        "qblp_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, y_index, y_index)],
            0.0,
        ),
        vec![
            Triangle3::new(
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(4.0, 0.0, 4.0),
                Point3::new(0.0, 4.0, 4.0),
            ),
            Triangle3::new(
                Point3::new(4.0, 0.0, 4.0),
                Point3::new(4.0, 4.0, 8.0),
                Point3::new(0.0, 4.0, 4.0),
            ),
        ],
    );
    let z_id = qblp.result_variable().id();
    model.add_symbol(Arc::new(qblp)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let z_index = base.find_token(z_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "y_eq_1_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, z_index)], 0.0),
            ConstraintRelation::Equal,
            6.25,
        ),
        "z_eq_6_25",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "y_eq_1_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, z_index)], 0.0),
            ConstraintRelation::Equal,
            3.5,
        ),
        "z_eq_3_5",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_sigmoid_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_sigmoid_non_linear_mapping");

    let x = ContinuousVariableItem::create(VariableId::standalone(2070), "x");
    let x_index = model.register_variable(x).unwrap();

    let qsigmoid = QuadraticLogisticFunction::new(
        2071,
        "qsigmoid_non_linear_mapping",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
    );
    let y_id = qsigmoid.result_variable().id();
    model.add_symbol(Arc::new(qsigmoid)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            0.96,
        ),
        "y_ge_0_96",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_binaryzation_big_m_with_tiny_interval_and_large_coefficient() {
    let mut model = MetaModel::<f64>::new("quadratic_binary_big_m_narrow");

    let x = ContinuousVariableItem::create(VariableId::standalone(2080), "x");
    let x_index = model.register_variable(x).unwrap();

    let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0e9, x_index)], 0.0);
    let qbin = QuadraticBinaryzationFunction::new(
        2081,
        "qbin_big_m_narrow",
        input,
        9.0e-4,
        1.0e12,
        BinaryzationMethod::BigM,
    );
    let y_id = qbin.result_variable().id();
    model.add_symbol(Arc::new(qbin)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0e-12,
        ),
        "x_ge_1e_12",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::LessEqual,
            1.000001e-12,
        ),
        "x_le_1_000001e_12",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_mod_on_exact_divisor_boundary() {
    let mut model = MetaModel::<f64>::new("quadratic_mod_boundary");

    let x = ContinuousVariableItem::create(VariableId::standalone(2090), "x");
    let x_index = model.register_variable(x).unwrap();

    let qmod = QuadraticModFunction::new(
        2091,
        "qmod_boundary",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
        3.0,
    );
    let y_id = qmod.result_variable().id();
    model.add_symbol(Arc::new(qmod)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            3.0,
        ),
        "x_eq_3",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            3.0,
        ),
        "x_eq_3",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_max_min_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_max_min_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2200), "x");
    let x_index = model.register_variable(x).unwrap();

    // 候选 x^2 与 x；x = 2 时 min(4, 2) = 2 / candidates x^2 and x; min(4, 2) = 2 at x = 2
    let qmaxmin = QuadraticMaxMinFunction::new(
        2201,
        "qmaxmin_non_linear",
        vec![
            Quadratic::new(
                vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
                0.0,
            ),
            Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, x_index)], 0.0),
        ],
    );
    let y_id = qmaxmin.result_variable().id();
    model.add_symbol(Arc::new(qmaxmin)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "y_eq_2",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            4.0,
        ),
        "y_eq_4",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_min_max_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_min_max_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2210), "x");
    let x_index = model.register_variable(x).unwrap();

    // 候选 x^2 与 x；x = 0.5 时 max(0.25, 0.5) = 0.5
    // candidates x^2 and x; max(0.25, 0.5) = 0.5 at x = 0.5
    let qminmax = QuadraticMinMaxFunction::new(
        2211,
        "qminmax_non_linear",
        vec![
            Quadratic::new(
                vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
                0.0,
            ),
            Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, x_index)], 0.0),
        ],
    );
    let y_id = qminmax.result_variable().id();
    model.add_symbol(Arc::new(qminmax)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "y_eq_0_5",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            0.5,
        ),
        "x_eq_0_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.25,
        ),
        "y_eq_0_25",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_if_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_if_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2220), "x");
    let x_index = model.register_variable(x).unwrap();

    // 条件 x^2 >= 0.5；x = 2 时结果必为 1 / condition x^2 >= 0.5; result forced to 1 at x = 2
    let qif = QuadraticIfFunction::new(
        2221,
        "qif_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
        ConditionRelation::Greater,
        0.5,
        ConditionBounds {
            lower: 0.0,
            upper: 4.0,
        },
    )
    .unwrap();
    let y_id = qif.result_variable().id();
    model.add_symbol(Arc::new(qif)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_if_in_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_if_in_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2230), "x");
    let x_index = model.register_variable(x).unwrap();

    // 输入 x^2，闭区间 [1, 4]；x = 1.5 时 x^2 = 2.25 在区间内，结果必为 1
    // input x^2, closed interval [1, 4]; x^2 = 2.25 inside at x = 1.5, result forced to 1
    let qifin = QuadraticIfInFunction::new(
        2231,
        "qifin_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
        1.0,
        4.0,
        0.5,
        ConditionBounds {
            lower: 0.0,
            upper: 4.0,
        },
    )
    .unwrap();
    let y_id = qifin.result_variable().id();
    model.add_symbol(Arc::new(qifin)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "y_eq_1",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            1.5,
        ),
        "x_eq_1_5",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_if_then_with_non_linear_input() {
    let mut model = MetaModel::<f64>::new("quadratic_if_then_non_linear");

    let x = ContinuousVariableItem::create(VariableId::standalone(2240), "x");
    let x_index = model.register_variable(x).unwrap();

    // 条件 x^2 - 1 >= 0.5；x = 2 时结果为 then = 2x^2 = 8
    // condition x^2 - 1 >= 0.5; result equals then = 2x^2 = 8 at x = 2
    let qifthen = QuadraticIfThenFunction::new(
        2241,
        "qifthen_non_linear",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            -1.0,
        ),
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(2.0, x_index, x_index)],
            0.0,
        ),
        ConditionRelation::Greater,
        0.5,
        ConditionBounds {
            lower: -1.0,
            upper: 3.0,
        },
        ConditionBounds {
            lower: 0.0,
            upper: 8.0,
        },
    )
    .unwrap();
    let y_id = qifthen.result_variable().id();
    model.add_symbol(Arc::new(qifthen)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            8.0,
        ),
        "y_eq_8",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            0.0,
        ),
        "y_eq_0",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}

#[test]
fn gurobi_solves_quadratic_rounding_floor_on_integer_boundary() {
    let mut model = MetaModel::<f64>::new("quadratic_rounding_boundary");

    let x = ContinuousVariableItem::create(VariableId::standalone(2100), "x");
    let x_index = model.register_variable(x).unwrap();

    let qround = QuadraticRoundingFunction::floor(
        2101,
        "qround_floor_boundary",
        Quadratic::new(
            vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
            0.0,
        ),
    );
    let y_id = qround.result_variable().id();
    model.add_symbol(Arc::new(qround)).unwrap();

    let solver = GurobiSolver::new();
    let base = model.try_into_mechanism_model().unwrap();
    let y_index = base.find_token(y_id).unwrap().solver_index;

    let mut feasible = base.clone();
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    feasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            4.0,
        ),
        "y_eq_4",
    ));
    let feasible_output = solver
        .solve_quadratic(&feasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        feasible_output.status.is_feasible(),
        "status: {:?}",
        feasible_output.status
    );

    let mut infeasible = base;
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::Equal,
            2.0,
        ),
        "x_eq_2",
    ));
    infeasible.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::Equal,
            5.0,
        ),
        "y_eq_5",
    ));
    let infeasible_output = solver
        .solve_quadratic(&infeasible.into_quadratic_tetrad_model())
        .unwrap();
    assert!(
        infeasible_output.status.is_infeasible(),
        "expected infeasible-like status, got {:?}",
        infeasible_output.status
    );
}
