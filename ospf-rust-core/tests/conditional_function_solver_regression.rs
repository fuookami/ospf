#![cfg(any(
    feature = "scip",
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12"
))]

use std::sync::Arc;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::model::intermediate::LinearTriadModel;
#[cfg(any(
    feature = "scip",
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12"
))]
use ospf_rust_core::solver::LinearSolver;
use ospf_rust_core::solver::SolverOutput;
#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
use ospf_rust_core::solver::solvers::GurobiSolver;
#[cfg(feature = "scip")]
use ospf_rust_core::solver::solvers::SCIPSolver;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalIfFunction, ConditionalImplyFunction,
    ConditionalIndicatorFunction, ConditionalThenFunction, IfInFunction,
};
use ospf_rust_core::variable::{ContinuousVariableItem, IntegerVariableItem, VariableId, VariableRange};

const STRICT_BOUNDARY: f64 = 0.5;
const CASES: [(ConditionRelation, f64, f64); 8] = [
    (ConditionRelation::Greater, STRICT_BOUNDARY, 1.0),
    (ConditionRelation::Greater, 0.0, 0.0),
    (ConditionRelation::GreaterEqual, 0.0, 1.0),
    (ConditionRelation::GreaterEqual, -STRICT_BOUNDARY, 0.0),
    (ConditionRelation::Less, -STRICT_BOUNDARY, 1.0),
    (ConditionRelation::Less, 0.0, 0.0),
    (ConditionRelation::LessEqual, 0.0, 1.0),
    (ConditionRelation::LessEqual, STRICT_BOUNDARY, 0.0),
];

fn boundary_model(relation: ConditionRelation, value: f64) -> (LinearTriadModel, usize) {
    let mut model = MetaModel::<f64>::new("conditional_boundary_relation_regression");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(9_400),
        "fixed_condition_difference",
        VariableRange::fixed(value),
    );
    let x_index = model.register_variable(x).unwrap();
    let indicator = ConditionalIndicatorFunction::new(
        9_401,
        "boundary_relation",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        relation,
        STRICT_BOUNDARY,
        ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        },
    )
    .unwrap();
    let result_id = indicator.result_variable().id();
    model.add_symbol(Arc::new(indicator)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn condition(variable_index: usize, relation: ConditionRelation) -> ConditionalIfFunction<f64> {
    ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(1.0, variable_index)], 0.0),
        relation,
        STRICT_BOUNDARY,
        ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        },
    )
    .unwrap()
}

fn integer_threshold_model(
    relation: ConditionRelation,
    value: i32,
) -> (LinearTriadModel, usize) {
    let mut model = MetaModel::<f64>::new("integer_conditional_threshold_regression");
    let x = IntegerVariableItem::with_range(
        VariableId::standalone(9_500),
        "integer_threshold",
        VariableRange::fixed(f64::from(value)),
    );
    let x_index = model.register_variable(x).unwrap();
    let discrete_condition = ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], -10.0),
        relation,
        STRICT_BOUNDARY,
        ConditionBounds {
            lower: -11.0,
            upper: 11.0,
        },
    )
    .unwrap();
    let indicator = ConditionalIndicatorFunction::from_discrete_condition(
        9_501,
        "integer_threshold_indicator",
        discrete_condition,
        1.0,
        model.tokens(),
    )
    .unwrap();
    let result_id = indicator.result_variable().id();
    model.add_symbol(Arc::new(indicator)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn lattice_threshold_model(
    relation: ConditionRelation,
    value: i32,
) -> (LinearTriadModel, usize) {
    assert_eq!(value % 5, 0, "lattice test values must be multiples of 5");

    let mut model = MetaModel::<f64>::new("step_five_conditional_threshold_regression");
    let x = IntegerVariableItem::with_range(
        VariableId::standalone(9_540),
        "step_five_x",
        VariableRange::bounded(0.0, 100.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let x_step = IntegerVariableItem::with_range(
        VariableId::standalone(9_541),
        "step_five_x_index",
        VariableRange::bounded(0.0, 20.0),
    );
    let x_step_index = model.register_variable(x_step).unwrap();

    // 将实际 x 约束在步长为 5 的格点上 / Constrain actual x to a step-5 lattice.
    model
        .add_linear_constraint(
            &[(x_index, 1.0), (x_step_index, -5.0)],
            ConstraintRelation::Equal,
            0.0,
            "step_five_lattice",
        )
        .unwrap();
    model
        .add_linear_constraint(
            &[(x_index, 1.0)],
            ConstraintRelation::Equal,
            f64::from(value),
            "fix_step_five_x",
        )
        .unwrap();

    let discrete_condition = ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(5.0, x_index)], -250.0),
        relation,
        STRICT_BOUNDARY,
        ConditionBounds {
            lower: -250.0,
            upper: 250.0,
        },
    )
    .unwrap();
    let indicator = ConditionalIndicatorFunction::from_discrete_condition(
        9_542,
        "step_five_threshold_indicator",
        discrete_condition,
        5.0,
        model.tokens(),
    )
    .unwrap();
    let result_id = indicator.result_variable().id();
    model.add_symbol(Arc::new(indicator)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

const STEP_FIVE_DELTA_BOUNDARY: f64 = 2.0;

fn step_five_delta_boundary_base() -> (MetaModel<f64>, usize) {
    let mut model = MetaModel::<f64>::new("step_five_delta_boundary_regression");
    let x = IntegerVariableItem::with_range(
        VariableId::standalone(9_560),
        "step_five_delta_x",
        VariableRange::bounded(0.0, 100.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let x_step = IntegerVariableItem::with_range(
        VariableId::standalone(9_561),
        "step_five_delta_x_index",
        VariableRange::bounded(0.0, 20.0),
    );
    let x_step_index = model.register_variable(x_step).unwrap();

    // 将实际 x 约束在步长为 5 的格点，并固定到阈值点 / Constrain x to a step-5 lattice and fix the threshold point.
    model
        .add_linear_constraint(
            &[(x_index, 1.0), (x_step_index, -5.0)],
            ConstraintRelation::Equal,
            0.0,
            "step_five_delta_lattice",
        )
        .unwrap();
    model
        .add_linear_constraint(
            &[(x_index, 1.0)],
            ConstraintRelation::Equal,
            50.0,
            "fix_step_five_delta_x",
        )
        .unwrap();
    (model, x_index)
}

fn finish_step_five_delta_boundary_model(
    mut model: MetaModel<f64>,
    indicator: ConditionalIndicatorFunction<f64>,
) -> (LinearTriadModel, usize) {
    let result_id = indicator.result_variable().id();
    model.add_symbol(Arc::new(indicator)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn step_five_correct_delta_boundary_model() -> (LinearTriadModel, usize) {
    let (model, x_index) = step_five_delta_boundary_base();
    let discrete_condition = ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(5.0, x_index)], -250.0),
        ConditionRelation::GreaterEqual,
        STEP_FIVE_DELTA_BOUNDARY,
        ConditionBounds {
            lower: -250.0,
            upper: 250.0,
        },
    )
    .unwrap();
    let indicator = ConditionalIndicatorFunction::from_discrete_condition(
        9_562,
        "step_five_correct_delta_indicator",
        discrete_condition,
        5.0,
        model.tokens(),
    )
    .unwrap();
    finish_step_five_delta_boundary_model(model, indicator)
}

fn step_five_wrong_delta_boundary_model() -> (LinearTriadModel, usize) {
    let (model, x_index) = step_five_delta_boundary_base();

    // 错误 delta=1 将 d=0 变换为 1，落入 (0, 2) Undefined 区间 / The wrong delta=1 maps d=0 to 1, inside the (0, 2) Undefined interval.
    let indicator = ConditionalIndicatorFunction::new(
        9_563,
        "step_five_wrong_delta_indicator",
        Linear::new(vec![LinearMonomial::new(5.0, x_index)], -249.0),
        ConditionRelation::Greater,
        STEP_FIVE_DELTA_BOUNDARY,
        ConditionBounds {
            lower: -249.0,
            upper: 251.0,
        },
    )
    .unwrap();
    finish_step_five_delta_boundary_model(model, indicator)
}

fn undefined_boundary_model() -> LinearTriadModel {
    let (model, _) = boundary_model(ConditionRelation::Greater, 0.05);
    model
}

fn negative_then_model() -> (LinearTriadModel, usize) {
    let mut model = MetaModel::<f64>::new("negative_conditional_then_regression");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(9_510),
        "negative_then_condition",
        VariableRange::fixed(1.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let then = ConditionalThenFunction::new(
        condition(x_index, ConditionRelation::GreaterEqual),
        Linear::constant(-2.0),
    )
    .unwrap();
    let result_id = then.result_variable().id();
    model.add_symbol(Arc::new(then)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn imply_violation_model() -> (LinearTriadModel, usize) {
    let mut model = MetaModel::<f64>::new("conditional_imply_violation_regression");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(9_520),
        "imply_violation_x",
        VariableRange::fixed(2.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let premise = ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], -1.0),
        ConditionRelation::GreaterEqual,
        0.1,
        ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        },
    )
    .unwrap();
    let consequence = ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], -3.0),
        ConditionRelation::GreaterEqual,
        0.1,
        ConditionBounds {
            lower: -3.0,
            upper: 1.0,
        },
    )
    .unwrap();
    let imply =
        ConditionalImplyFunction::new(9_521, "imply_violation", premise, consequence).unwrap();
    let result_id = imply.result_variable().id();
    model.add_symbol(Arc::new(imply)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn if_in_nonmember_endpoint_model() -> (LinearTriadModel, usize) {
    let mut model = MetaModel::<f64>::new("if_in_nonmember_endpoint_regression");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(9_530),
        "if_in_endpoint_x",
        VariableRange::bounded(0.0, 1.0),
    );
    let x_index = model.register_variable(x).unwrap();
    model
        .add_linear_constraint(
            &[(x_index, 1.0)],
            ConstraintRelation::Equal,
            1.0,
            "fix_if_in_endpoint",
        )
        .unwrap();
    let if_in = IfInFunction::new(
        9_531,
        "if_in_endpoint",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        vec![0.0],
        1.0,
    );
    let result_id = if_in.result_variable().id();
    model.add_symbol(Arc::new(if_in)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn if_in_large_nonmember_endpoint_model() -> (LinearTriadModel, usize) {
    let mut model = MetaModel::<f64>::new("if_in_large_nonmember_endpoint_regression");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(9_532),
        "if_in_large_endpoint_x",
        VariableRange::bounded(0.0, 1e16),
    );
    let x_index = model.register_variable(x).unwrap();
    model
        .add_linear_constraint(
            &[(x_index, 1.0)],
            ConstraintRelation::Equal,
            1e16,
            "fix_if_in_large_endpoint",
        )
        .unwrap();
    let if_in = IfInFunction::new(
        9_533,
        "if_in_large_endpoint",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        vec![0.0],
        1.0,
    );
    let result_id = if_in.result_variable().id();
    model.add_symbol(Arc::new(if_in)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn if_in_large_lower_nonmember_endpoint_model() -> (LinearTriadModel, usize) {
    let mut model = MetaModel::<f64>::new("if_in_large_lower_nonmember_endpoint_regression");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(9_534),
        "if_in_large_lower_endpoint_x",
        VariableRange::bounded(0.0, 1e16),
    );
    let x_index = model.register_variable(x).unwrap();
    model
        .add_linear_constraint(
            &[(x_index, 1.0)],
            ConstraintRelation::Equal,
            0.0,
            "fix_if_in_large_lower_endpoint",
        )
        .unwrap();
    let if_in = IfInFunction::new(
        9_535,
        "if_in_large_lower_endpoint",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        vec![1e16],
        1.0,
    );
    let result_id = if_in.result_variable().id();
    model.add_symbol(Arc::new(if_in)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let result_index = mechanism.find_token(result_id).unwrap().solver_index;
    (mechanism.into_linear_triad_model(), result_index)
}

fn assert_boundary_output(output: SolverOutput, result_index: usize, expected: f64) {
    assert!(
        output.status.is_feasible(),
        "expected feasible solve status, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .expect("expected a feasible conditional relation solution");
    assert!(
        (solution[result_index] - expected).abs() <= 1e-6,
        "expected conditional result {expected}, got {}",
        solution[result_index]
    );
}

fn assert_infeasible(output: SolverOutput) {
    assert!(
        !output.status.is_feasible(),
        "expected undefined condition to be infeasible, got {:?}",
        output.status
    );
}

fn assert_extended_conditional_function_matrix<S>(solver: &S)
where
    S: LinearSolver,
{
    for (relation, cases) in [
        (
            ConditionRelation::Greater,
            [(9, 0.0), (10, 0.0), (11, 1.0)],
        ),
        (
            ConditionRelation::GreaterEqual,
            [(9, 0.0), (10, 1.0), (11, 1.0)],
        ),
        (
            ConditionRelation::Less,
            [(9, 1.0), (10, 0.0), (11, 0.0)],
        ),
        (
            ConditionRelation::LessEqual,
            [(9, 1.0), (10, 1.0), (11, 0.0)],
        ),
    ] {
        for (value, expected) in cases {
            let (model, result_index) = integer_threshold_model(relation, value);
            let output = solver
                .solve_linear(&model)
                .expect("solver should solve the integer threshold model");
            assert_boundary_output(output, result_index, expected);
        }
    }

    for (value, expected) in [(45, 0.0), (50, 1.0), (55, 1.0)] {
        let (model, result_index) =
            lattice_threshold_model(ConditionRelation::GreaterEqual, value);
        let output = solver
            .solve_linear(&model)
            .expect("solver should solve the step-five lattice threshold model");
        assert_boundary_output(output, result_index, expected);
    }
}

fn assert_step_five_delta_boundary<S>(solver: &S)
where
    S: LinearSolver,
{
    let (correct_model, correct_index) = step_five_correct_delta_boundary_model();
    let correct_output = solver
        .solve_linear(&correct_model)
        .expect("solver should solve the step-five delta boundary model");
    assert_boundary_output(correct_output, correct_index, 1.0);

    // 同一 d=0 点的错误 delta=1 变换必须不可行 / The wrong delta=1 transform at the same d=0 point must be infeasible.
    let (wrong_model, _) = step_five_wrong_delta_boundary_model();
    let wrong_output = solver
        .solve_linear(&wrong_model)
        .expect("solver should report the wrong step-five delta boundary as infeasible");
    assert_infeasible(wrong_output);
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
#[test]
fn gurobi_solves_conditional_relation_boundaries() {
    let solver = GurobiSolver::new();
    for (relation, value, expected) in CASES {
        let (model, result_index) = boundary_model(relation, value);
        let output = solver
            .solve_linear(&model)
            .expect("Gurobi should solve the conditional boundary model");
        assert_boundary_output(output, result_index, expected);
    }
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
#[test]
fn gurobi_solves_extended_conditional_function_matrix() {
    let solver = GurobiSolver::new();
    assert_extended_conditional_function_matrix(&solver);
    assert_step_five_delta_boundary(&solver);

    let undefined = solver
        .solve_linear(&undefined_boundary_model())
        .expect("Gurobi should return the undefined-boundary status");
    assert_infeasible(undefined);

    let (negative_model, negative_index) = negative_then_model();
    let output = solver
        .solve_linear(&negative_model)
        .expect("Gurobi should solve the negative conditional-then model");
    assert_boundary_output(output, negative_index, -2.0);

    let (imply_model, imply_index) = imply_violation_model();
    let output = solver
        .solve_linear(&imply_model)
        .expect("Gurobi should solve the violated implication model");
    assert_boundary_output(output, imply_index, 0.0);

    let (if_in_model, if_in_index) = if_in_nonmember_endpoint_model();
    let output = solver
        .solve_linear(&if_in_model)
        .expect("Gurobi should keep the IfIn non-member endpoint feasible");
    assert_boundary_output(output, if_in_index, 0.0);

    let (if_in_large_model, if_in_large_index) = if_in_large_nonmember_endpoint_model();
    let output = solver
        .solve_linear(&if_in_large_model)
        .expect("Gurobi should keep the large-scale IfIn endpoint feasible");
    assert_boundary_output(output, if_in_large_index, 0.0);

    let (if_in_large_lower_model, if_in_large_lower_index) =
        if_in_large_lower_nonmember_endpoint_model();
    let output = solver
        .solve_linear(&if_in_large_lower_model)
        .expect("Gurobi should keep the large-scale lower endpoint feasible");
    assert_boundary_output(output, if_in_large_lower_index, 0.0);
}

#[cfg(feature = "scip")]
#[test]
fn scip_solves_conditional_relation_boundaries() {
    let solver = SCIPSolver::new();
    for (relation, value, expected) in CASES {
        let (model, result_index) = boundary_model(relation, value);
        let output = solver
            .solve_linear(&model)
            .expect("SCIP should solve the conditional boundary model");
        assert_boundary_output(output, result_index, expected);
    }
}

#[cfg(feature = "scip")]
#[test]
fn scip_solves_extended_conditional_function_matrix() {
    let solver = SCIPSolver::new();
    assert_extended_conditional_function_matrix(&solver);
    assert_step_five_delta_boundary(&solver);

    let undefined = solver
        .solve_linear(&undefined_boundary_model())
        .expect("SCIP should return the undefined-boundary status");
    assert_infeasible(undefined);

    let (negative_model, negative_index) = negative_then_model();
    let output = solver
        .solve_linear(&negative_model)
        .expect("SCIP should solve the negative conditional-then model");
    assert_boundary_output(output, negative_index, -2.0);

    let (imply_model, imply_index) = imply_violation_model();
    let output = solver
        .solve_linear(&imply_model)
        .expect("SCIP should solve the violated implication model");
    assert_boundary_output(output, imply_index, 0.0);

    let (if_in_model, if_in_index) = if_in_nonmember_endpoint_model();
    let output = solver
        .solve_linear(&if_in_model)
        .expect("SCIP should keep the IfIn non-member endpoint feasible");
    assert_boundary_output(output, if_in_index, 0.0);

    let (if_in_large_model, if_in_large_index) = if_in_large_nonmember_endpoint_model();
    let output = solver
        .solve_linear(&if_in_large_model)
        .expect("SCIP should keep the large-scale IfIn endpoint feasible");
    assert_boundary_output(output, if_in_large_index, 0.0);

    let (if_in_large_lower_model, if_in_large_lower_index) =
        if_in_large_lower_nonmember_endpoint_model();
    let output = solver
        .solve_linear(&if_in_large_lower_model)
        .expect("SCIP should keep the large-scale lower endpoint feasible");
    assert_boundary_output(output, if_in_large_lower_index, 0.0);
}
