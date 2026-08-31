#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::Arc;

use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::intermediate::LinearTriadModel;
use ospf_rust_core::model::{
    ConstraintRelation, LinearConstraint, LinearInequality, MetaModel, ObjectiveCategory,
};
use ospf_rust_core::solver::{SolverOutput, SolverStatus, solvers::GurobiSolver};
use ospf_rust_core::symbol::functions::{
    AbsFunction, AndFunction, BalanceTernaryzationFunction, BinaryzationFunction,
    BivariateLinearPiecewiseFunction, IfElseFunction, IfThenFunction, InequalityFunction,
    InequalityKind, MaskingFunction, MaskingRangeFunction, MaxFunction, MaxMinFunction,
    MinFunction, MinMaxFunction, ModFunction, NotFunction, OneOfFunction, OrFunction, Point2,
    Point3, RoundingFunction, SigmoidFunction, SlackFunction, SlackRangeFunction,
    UnivariateLinearPiecewiseFunction, XorFunction,
};
use ospf_rust_core::variable::{
    BinaryVariableItem, ContinuousVariableItem, IntegerVariableItem, UIntegerVariableItem,
    VariableId, VariableRange,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-6,
        "expected {}, got {}",
        expected,
        actual
    );
}

fn var_poly(index: usize) -> Linear<f64> {
    Linear::new(vec![LinearMonomial::new(1.0, index)], 0.0)
}

fn linear_constraint(
    terms: &[(usize, f64)],
    relation: ConstraintRelation,
    rhs: f64,
    name: &str,
) -> LinearConstraint<f64> {
    let monomials = terms
        .iter()
        .map(|(index, coefficient)| LinearMonomial::new(*coefficient, *index))
        .collect::<Vec<_>>();
    LinearConstraint::new(
        LinearInequality::new(Linear::new(monomials, 0.0), relation, rhs),
        name,
    )
}

fn solve_linear_model(
    mut linear: LinearTriadModel,
    objective_terms: &[(usize, f64)],
    category: ObjectiveCategory,
) -> SolverOutput {
    let mut c = vec![0.0; linear.basic.num_variables()];
    for (index, coefficient) in objective_terms {
        c[*index] += *coefficient;
    }
    linear.set_objective(c, category);

    let solver = GurobiSolver::new();
    solver
        .solve_linear(&linear)
        .expect("gurobi solve should succeed")
}

fn expect_feasible(output: &SolverOutput) {
    assert!(
        output.status.is_feasible(),
        "expected feasible status, got {:?}",
        output.status
    );
}

#[test]
fn abs_test_parity() {
    let mut model = MetaModel::<f64>::new("kotlin_abs_parity");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(1000),
        "x",
        VariableRange::bounded(-3.0, 2.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let abs_fn = AbsFunction::new(1001, "abs", var_poly(x_index));
    let abs_id = abs_fn.result_variable().id();
    model.add_symbol(Arc::new(abs_fn)).unwrap();

    let mechanism = model.try_into_mechanism_model().unwrap();
    let abs_index = mechanism.find_token(abs_id).unwrap().solver_index;

    let output_min_abs = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(abs_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&output_min_abs);
    assert_close(output_min_abs.objective_value.unwrap(), 0.0);

    let output_max_abs = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(abs_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&output_max_abs);
    assert_close(output_max_abs.objective_value.unwrap(), 3.0);

    let mut mechanism_abs_eq_1 = mechanism.clone();
    mechanism_abs_eq_1.add_constraint(linear_constraint(
        &[(abs_index, 1.0)],
        ConstraintRelation::Equal,
        1.0,
        "abs_eq_1",
    ));
    let output_max_x = solve_linear_model(
        mechanism_abs_eq_1.clone().into_linear_triad_model(),
        &[(x_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&output_max_x);
    assert_close(output_max_x.objective_value.unwrap(), 1.0);

    let output_min_x = solve_linear_model(
        mechanism_abs_eq_1.into_linear_triad_model(),
        &[(x_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&output_min_x);
    assert_close(output_min_x.objective_value.unwrap(), -1.0);
}

#[test]
fn and_or_not_xor_parity() {
    let mut and_model = MetaModel::<f64>::new("kotlin_and_parity");
    let x = BinaryVariableItem::create(VariableId::standalone(1010), "x");
    let y = BinaryVariableItem::create(VariableId::standalone(1011), "y");
    let x_index = and_model.register_variable(x).unwrap();
    let y_index = and_model.register_variable(y).unwrap();
    let and_fn = AndFunction::new(1012, "and", vec![var_poly(x_index), var_poly(y_index)]);
    let and_id = and_fn.result_variable().id();
    and_model.add_symbol(Arc::new(and_fn)).unwrap();

    let mut and_mechanism = and_model.try_into_mechanism_model().unwrap();
    let and_index = and_mechanism.find_token(and_id).unwrap().solver_index;
    and_mechanism.add_constraint(linear_constraint(
        &[(and_index, 1.0)],
        ConstraintRelation::Equal,
        0.0,
        "and_is_false",
    ));
    let and_output = solve_linear_model(
        and_mechanism.into_linear_triad_model(),
        &[(x_index, 1.0), (y_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&and_output);
    assert_close(and_output.objective_value.unwrap(), 1.0);

    let mut or_model = MetaModel::<f64>::new("kotlin_or_parity");
    let ox = BinaryVariableItem::create(VariableId::standalone(1020), "x");
    let oy = BinaryVariableItem::create(VariableId::standalone(1021), "y");
    let ox_index = or_model.register_variable(ox).unwrap();
    let oy_index = or_model.register_variable(oy).unwrap();
    let or_fn = OrFunction::new(1022, "or", vec![var_poly(ox_index), var_poly(oy_index)]);
    let or_id = or_fn.result_variable().id();
    or_model.add_symbol(Arc::new(or_fn)).unwrap();
    let mut or_mechanism = or_model.try_into_mechanism_model().unwrap();
    let or_index = or_mechanism.find_token(or_id).unwrap().solver_index;
    or_mechanism.add_constraint(linear_constraint(
        &[(or_index, 1.0)],
        ConstraintRelation::Equal,
        1.0,
        "or_is_true",
    ));
    or_mechanism.add_constraint(linear_constraint(
        &[(ox_index, 1.0), (oy_index, 1.0)],
        ConstraintRelation::GreaterEqual,
        1.0,
        "or_support_lb",
    ));
    let or_output = solve_linear_model(
        or_mechanism.into_linear_triad_model(),
        &[(ox_index, 1.0), (oy_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&or_output);
    assert_close(or_output.objective_value.unwrap(), 1.0);

    let mut not_model = MetaModel::<f64>::new("kotlin_not_parity");
    let nx = ContinuousVariableItem::with_range(
        VariableId::standalone(1030),
        "x",
        VariableRange::bounded(0.0, 1.0),
    );
    let nx_index = not_model.register_variable(nx).unwrap();
    let not_fn = NotFunction::new(1031, "not", var_poly(nx_index));
    let not_id = not_fn.result_variable().id();
    not_model.add_symbol(Arc::new(not_fn)).unwrap();
    let mut not_mechanism = not_model.try_into_mechanism_model().unwrap();
    let not_index = not_mechanism.find_token(not_id).unwrap().solver_index;
    not_mechanism.add_constraint(linear_constraint(
        &[(not_index, 1.0)],
        ConstraintRelation::Equal,
        1.0,
        "not_true",
    ));
    let not_output = solve_linear_model(
        not_mechanism.into_linear_triad_model(),
        &[(nx_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&not_output);
    assert_close(not_output.objective_value.unwrap(), 0.0);

    let mut xor_model = MetaModel::<f64>::new("kotlin_xor_parity");
    let xx = BinaryVariableItem::create(VariableId::standalone(1040), "x");
    let xy = BinaryVariableItem::create(VariableId::standalone(1041), "y");
    let xz = BinaryVariableItem::create(VariableId::standalone(1042), "z");
    let xx_index = xor_model.register_variable(xx).unwrap();
    let xy_index = xor_model.register_variable(xy).unwrap();
    let xz_index = xor_model.register_variable(xz).unwrap();
    let xor_fn = XorFunction::new(
        1043,
        "xor",
        vec![var_poly(xx_index), var_poly(xy_index), var_poly(xz_index)],
    );
    let xor_id = xor_fn.result_variable().id();
    xor_model.add_symbol(Arc::new(xor_fn)).unwrap();

    let mut xor_mechanism = xor_model.try_into_mechanism_model().unwrap();
    let xor_index = xor_mechanism.find_token(xor_id).unwrap().solver_index;
    xor_mechanism.add_constraint(linear_constraint(
        &[(xor_index, 1.0)],
        ConstraintRelation::Equal,
        1.0,
        "xor_true",
    ));
    let xor_output = solve_linear_model(
        xor_mechanism.into_linear_triad_model(),
        &[(xx_index, 1.0), (xy_index, 1.0), (xz_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&xor_output);
    assert_close(xor_output.objective_value.unwrap(), 2.0);
}

#[test]
fn bin_and_bter_parity() {
    let mut bin_model = MetaModel::<f64>::new("kotlin_bin_parity");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(1100),
        "x",
        VariableRange::bounded(0.0, 2.0),
    );
    let x_index = bin_model.register_variable(x).unwrap();
    let bin_fn = BinaryzationFunction::with_big_m(1101, "bin", var_poly(x_index), 10.0);
    let bin_id = bin_fn.result_variable().id();
    bin_model.add_symbol(Arc::new(bin_fn)).unwrap();

    let mechanism = bin_model.try_into_mechanism_model().unwrap();
    let bin_index = mechanism.find_token(bin_id).unwrap().solver_index;

    let output_min = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(bin_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&output_min);
    assert_close(output_min.objective_value.unwrap(), 0.0);

    let output_max = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(bin_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&output_max);
    assert_close(output_max.objective_value.unwrap(), 1.0);

    let mut fixed_zero = mechanism.clone();
    fixed_zero.add_constraint(linear_constraint(
        &[(x_index, 1.0)],
        ConstraintRelation::Equal,
        0.0,
        "x_eq_0",
    ));
    fixed_zero.add_constraint(linear_constraint(
        &[(bin_index, 1.0), (x_index, -1000.0)],
        ConstraintRelation::LessEqual,
        0.0,
        "bin_le_scaled_x",
    ));
    let output_zero = solve_linear_model(
        fixed_zero.into_linear_triad_model(),
        &[(bin_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&output_zero);
    assert_close(output_zero.objective_value.unwrap(), 0.0);

    let mut bter_model = MetaModel::<f64>::new("kotlin_bter_parity");
    let sx = ContinuousVariableItem::with_range(
        VariableId::standalone(1110),
        "x",
        VariableRange::bounded(-2.0, 2.0),
    );
    let sx_index = bter_model.register_variable(sx).unwrap();
    let pos_flag = BinaryzationFunction::with_big_m(1111, "pos_flag", var_poly(sx_index), 10.0);
    let neg_flag = BinaryzationFunction::with_big_m(
        1112,
        "neg_flag",
        Linear::new(vec![LinearMonomial::new(-1.0, sx_index)], 0.0),
        10.0,
    );
    let pos_id = pos_flag.result_variable().id();
    let neg_id = neg_flag.result_variable().id();
    bter_model.add_symbol(Arc::new(pos_flag)).unwrap();
    bter_model.add_symbol(Arc::new(neg_flag)).unwrap();

    let bter = BalanceTernaryzationFunction::new(1113, "bter");
    let bter_id = bter.result_variable().id();
    let bter_pos_id = bter.positive_variable().id();
    let bter_neg_id = bter.negative_variable().id();
    bter_model.add_symbol(Arc::new(bter)).unwrap();

    let mut bter_mechanism = bter_model.try_into_mechanism_model().unwrap();
    let pos_index = bter_mechanism.find_token(pos_id).unwrap().solver_index;
    let neg_index = bter_mechanism.find_token(neg_id).unwrap().solver_index;
    let bter_pos_index = bter_mechanism.find_token(bter_pos_id).unwrap().solver_index;
    let bter_neg_index = bter_mechanism.find_token(bter_neg_id).unwrap().solver_index;
    let bter_index = bter_mechanism.find_token(bter_id).unwrap().solver_index;

    bter_mechanism.add_constraint(linear_constraint(
        &[(bter_pos_index, 1.0), (pos_index, -1.0)],
        ConstraintRelation::Equal,
        0.0,
        "bter_link_pos",
    ));
    bter_mechanism.add_constraint(linear_constraint(
        &[(bter_neg_index, 1.0), (neg_index, -1.0)],
        ConstraintRelation::Equal,
        0.0,
        "bter_link_neg",
    ));

    let bter_min = solve_linear_model(
        bter_mechanism.clone().into_linear_triad_model(),
        &[(bter_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&bter_min);
    assert_close(bter_min.objective_value.unwrap(), -1.0);

    let bter_max = solve_linear_model(
        bter_mechanism.into_linear_triad_model(),
        &[(bter_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&bter_max);
    assert_close(bter_max.objective_value.unwrap(), 1.0);
}

#[test]
fn floor_ceiling_round_mod_parity() {
    let mut floor_model = MetaModel::<f64>::new("kotlin_floor_parity");
    let floor_x = ContinuousVariableItem::with_range(
        VariableId::standalone(1200),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let floor_x_index = floor_model.register_variable(floor_x).unwrap();
    let floor_scaled_input = Linear::new(vec![LinearMonomial::new(1.0 / 0.7, floor_x_index)], 0.0);
    let floor_fn = RoundingFunction::floor(1201, "floor", floor_scaled_input);
    let floor_id = floor_fn.result_variable().id();
    floor_model.add_symbol(Arc::new(floor_fn)).unwrap();
    let floor_mechanism = floor_model.try_into_mechanism_model().unwrap();
    let floor_index = floor_mechanism.find_token(floor_id).unwrap().solver_index;
    let floor_min = solve_linear_model(
        floor_mechanism.into_linear_triad_model(),
        &[(floor_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&floor_min);
    assert_close(floor_min.objective_value.unwrap(), 2.0);

    let mut ceil_model = MetaModel::<f64>::new("kotlin_ceil_parity");
    let ceil_x = ContinuousVariableItem::with_range(
        VariableId::standalone(1210),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let ceil_x_index = ceil_model.register_variable(ceil_x).unwrap();
    let ceil_scaled_input = Linear::new(vec![LinearMonomial::new(1.0 / 0.7, ceil_x_index)], 0.0);
    let ceil_fn = RoundingFunction::ceil(1211, "ceil", ceil_scaled_input);
    let ceil_id = ceil_fn.result_variable().id();
    ceil_model.add_symbol(Arc::new(ceil_fn)).unwrap();
    let ceil_mechanism = ceil_model.try_into_mechanism_model().unwrap();
    let ceil_index = ceil_mechanism.find_token(ceil_id).unwrap().solver_index;
    let ceil_max = solve_linear_model(
        ceil_mechanism.into_linear_triad_model(),
        &[(ceil_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&ceil_max);
    assert_close(ceil_max.objective_value.unwrap(), 8.0);

    let mut round_model = MetaModel::<f64>::new("kotlin_round_parity");
    let round_x = ContinuousVariableItem::with_range(
        VariableId::standalone(1220),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let round_x_index = round_model.register_variable(round_x).unwrap();
    let round_scaled_input = Linear::new(vec![LinearMonomial::new(1.0 / 0.7, round_x_index)], 0.5);
    let round_fn = RoundingFunction::floor(1221, "round", round_scaled_input);
    let round_id = round_fn.result_variable().id();
    round_model.add_symbol(Arc::new(round_fn)).unwrap();
    let round_mechanism = round_model.try_into_mechanism_model().unwrap();
    let round_index = round_mechanism.find_token(round_id).unwrap().solver_index;
    let round_min = solve_linear_model(
        round_mechanism.into_linear_triad_model(),
        &[(round_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&round_min);
    assert_close(round_min.objective_value.unwrap(), 3.0);

    let mut mod_model = MetaModel::<f64>::new("kotlin_mod_parity");
    let mod_x = ContinuousVariableItem::with_range(
        VariableId::standalone(1230),
        "x",
        VariableRange::fixed(3.0),
    );
    let mod_x_index = mod_model.register_variable(mod_x).unwrap();
    let mod_fn = ModFunction::new(1231, "mod", var_poly(mod_x_index), 0.7);
    let mod_id = mod_fn.result_variable().id();
    mod_model.add_symbol(Arc::new(mod_fn)).unwrap();
    let mut mod_mechanism = mod_model.try_into_mechanism_model().unwrap();
    let mod_index = mod_mechanism.find_token(mod_id).unwrap().solver_index;
    mod_mechanism.add_constraint(linear_constraint(
        &[(mod_x_index, 1.0)],
        ConstraintRelation::Equal,
        3.0,
        "x_eq_3",
    ));
    let mod_value = solve_linear_model(
        mod_mechanism.into_linear_triad_model(),
        &[(mod_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&mod_value);
    assert_close(mod_value.objective_value.unwrap(), 0.2);
}

#[test]
fn if_one_of_if_else_parity() {
    let mut if_model = MetaModel::<f64>::new("kotlin_if_parity");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(1300),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let x_index = if_model.register_variable(x).unwrap();

    let c1 = InequalityFunction::greater_equal(1301, "c1", var_poly(x_index), 3.0, 10.0);
    let c2 = InequalityFunction::new(
        1302,
        "c2",
        var_poly(x_index),
        3.0,
        InequalityKind::Greater,
        10.0,
    );
    let c3 = InequalityFunction::less_equal(1303, "c3", var_poly(x_index), 1.0, 10.0);
    let c1_id = c1.result_variable().id();
    let c2_id = c2.result_variable().id();
    let c3_id = c3.result_variable().id();
    if_model.add_symbol(Arc::new(c1)).unwrap();
    if_model.add_symbol(Arc::new(c2)).unwrap();
    if_model.add_symbol(Arc::new(c3)).unwrap();

    let if_mechanism = if_model.try_into_mechanism_model().unwrap();
    let c1_index = if_mechanism.find_token(c1_id).unwrap().solver_index;
    let c2_index = if_mechanism.find_token(c2_id).unwrap().solver_index;
    let c3_index = if_mechanism.find_token(c3_id).unwrap().solver_index;

    let mut c1_true = if_mechanism.clone();
    c1_true.add_constraint(linear_constraint(
        &[(c1_index, 1.0)],
        ConstraintRelation::Equal,
        1.0,
        "c1_true",
    ));
    c1_true.add_constraint(linear_constraint(
        &[(x_index, 1.0)],
        ConstraintRelation::GreaterEqual,
        3.0,
        "x_ge_3_when_c1_true",
    ));
    let if_case1 = solve_linear_model(
        c1_true.into_linear_triad_model(),
        &[(x_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&if_case1);
    assert_close(if_case1.objective_value.unwrap(), 3.0);

    let mut c1_false = if_mechanism.clone();
    c1_false.add_constraint(linear_constraint(
        &[(c1_index, 1.0)],
        ConstraintRelation::Equal,
        0.0,
        "c1_false",
    ));
    let if_case2 = solve_linear_model(
        c1_false.into_linear_triad_model(),
        &[(x_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&if_case2);
    assert!(if_case2.objective_value.unwrap() < 3.0);

    let mut c2_false = if_mechanism.clone();
    c2_false.add_constraint(linear_constraint(
        &[(c2_index, 1.0)],
        ConstraintRelation::Equal,
        0.0,
        "c2_false",
    ));
    let if_case3 = solve_linear_model(
        c2_false.into_linear_triad_model(),
        &[(x_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&if_case3);
    assert_close(if_case3.objective_value.unwrap(), 3.0);

    let if_case4 = solve_linear_model(
        if_mechanism.clone().into_linear_triad_model(),
        &[(c3_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&if_case4);
    assert_close(if_case4.objective_value.unwrap(), 0.0);

    let mut one_of_model = MetaModel::<f64>::new("kotlin_one_of_parity");
    let ox = ContinuousVariableItem::with_range(
        VariableId::standalone(1310),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let ox_index = one_of_model.register_variable(ox).unwrap();
    let cond1 = InequalityFunction::greater_equal(1311, "cond1", var_poly(ox_index), 3.0, 10.0);
    let cond2 = InequalityFunction::less_equal(1312, "cond2", var_poly(ox_index), 1.0, 10.0);
    let cond1_id = cond1.result_variable().id();
    let cond2_id = cond2.result_variable().id();
    let one_of = OneOfFunction::new(
        1313,
        "one_of",
        vec![Linear::new(vec![], 0.0), Linear::new(vec![], 1.0)],
    );
    let one_of_id = one_of.result_variable().id();
    let sel0_id = one_of.selection_variables()[0].id();
    let sel1_id = one_of.selection_variables()[1].id();
    one_of_model.add_symbol(Arc::new(cond1)).unwrap();
    one_of_model.add_symbol(Arc::new(cond2)).unwrap();
    one_of_model.add_symbol(Arc::new(one_of)).unwrap();

    let mut one_of_mechanism = one_of_model.try_into_mechanism_model().unwrap();
    let cond1_index = one_of_mechanism.find_token(cond1_id).unwrap().solver_index;
    let cond2_index = one_of_mechanism.find_token(cond2_id).unwrap().solver_index;
    let one_of_index = one_of_mechanism.find_token(one_of_id).unwrap().solver_index;
    let sel0_index = one_of_mechanism.find_token(sel0_id).unwrap().solver_index;
    let sel1_index = one_of_mechanism.find_token(sel1_id).unwrap().solver_index;

    one_of_mechanism.add_constraint(linear_constraint(
        &[(sel0_index, 1.0), (cond1_index, -1.0)],
        ConstraintRelation::LessEqual,
        0.0,
        "sel0_le_cond1",
    ));
    one_of_mechanism.add_constraint(linear_constraint(
        &[(sel1_index, 1.0), (cond2_index, -1.0)],
        ConstraintRelation::LessEqual,
        0.0,
        "sel1_le_cond2",
    ));
    one_of_mechanism.add_constraint(linear_constraint(
        &[(ox_index, 1.0)],
        ConstraintRelation::GreaterEqual,
        3.0,
        "x_ge_3_for_one_of",
    ));

    let one_of_case1 = solve_linear_model(
        one_of_mechanism.clone().into_linear_triad_model(),
        &[(ox_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&one_of_case1);
    assert_close(one_of_case1.objective_value.unwrap(), 3.0);

    let one_of_case2 = solve_linear_model(
        one_of_mechanism.into_linear_triad_model(),
        &[(one_of_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&one_of_case2);
    assert_close(one_of_case2.objective_value.unwrap(), 0.0);

    let mut if_else_model = MetaModel::<f64>::new("kotlin_if_else_parity");
    let ix = ContinuousVariableItem::with_range(
        VariableId::standalone(1320),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let ix_index = if_else_model.register_variable(ix).unwrap();
    let condition =
        InequalityFunction::greater_equal(1321, "if_cond", var_poly(ix_index), 3.0, 10.0);
    let condition_var = condition.result_variable().clone();
    if_else_model.add_symbol(Arc::new(condition)).unwrap();
    let if_else = IfElseFunction::new(
        1322,
        "if_else",
        condition_var,
        var_poly(ix_index),
        Linear::new(vec![], 0.0),
    );
    let if_else_id = if_else.result_variable().id();
    if_else_model.add_symbol(Arc::new(if_else)).unwrap();

    let if_else_mechanism = if_else_model.try_into_mechanism_model().unwrap();
    let if_else_index = if_else_mechanism
        .find_token(if_else_id)
        .unwrap()
        .solver_index;

    let mut if_branch = if_else_mechanism.clone();
    if_branch.add_constraint(linear_constraint(
        &[(ix_index, 1.0)],
        ConstraintRelation::GreaterEqual,
        3.0,
        "x_ge_3",
    ));
    let if_branch_output = solve_linear_model(
        if_branch.into_linear_triad_model(),
        &[(if_else_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&if_branch_output);
    assert_close(if_branch_output.objective_value.unwrap(), 5.0);

    let else_output = solve_linear_model(
        if_else_mechanism.clone().into_linear_triad_model(),
        &[(if_else_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&else_output);
    assert_close(else_output.objective_value.unwrap(), 0.0);

    let mut fixed_two = if_else_mechanism;
    fixed_two.add_constraint(linear_constraint(
        &[(ix_index, 1.0)],
        ConstraintRelation::Equal,
        2.0,
        "x_eq_2",
    ));
    let fixed_two_output = solve_linear_model(
        fixed_two.into_linear_triad_model(),
        &[(if_else_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&fixed_two_output);
    assert_close(fixed_two_output.objective_value.unwrap(), 0.0);
}

#[test]
fn if_then_parity() {
    let mut indicator_model = MetaModel::<f64>::new("kotlin_if_then_indicator_parity");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(1330),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let x_index = indicator_model.register_variable(x).unwrap();

    let premise = LinearInequality::new(var_poly(x_index), ConstraintRelation::GreaterEqual, 3.0);
    let consequence =
        LinearInequality::new(var_poly(x_index), ConstraintRelation::GreaterEqual, 4.0);
    let if_then = IfThenFunction::indicator(1331, "if_then", premise, consequence, 10.0);
    let if_then_id = if_then.result_variable().id();
    indicator_model.add_symbol(Arc::new(if_then)).unwrap();

    let indicator_mechanism = indicator_model.try_into_mechanism_model().unwrap();
    let if_then_index = indicator_mechanism
        .find_token(if_then_id)
        .unwrap()
        .solver_index;

    let indicator_max = solve_linear_model(
        indicator_mechanism.clone().into_linear_triad_model(),
        &[(if_then_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&indicator_max);
    assert_close(indicator_max.objective_value.unwrap(), 1.0);

    let indicator_min = solve_linear_model(
        indicator_mechanism.into_linear_triad_model(),
        &[(if_then_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&indicator_min);
    assert_close(indicator_min.objective_value.unwrap(), 0.0);

    let mut constraint_model = MetaModel::<f64>::new("kotlin_if_then_constraint_parity");
    let cx = ContinuousVariableItem::with_range(
        VariableId::standalone(1340),
        "x",
        VariableRange::bounded(2.0, 5.0),
    );
    let cx_index = constraint_model.register_variable(cx).unwrap();
    let cp = LinearInequality::new(var_poly(cx_index), ConstraintRelation::GreaterEqual, 3.0);
    let cq = LinearInequality::new(var_poly(cx_index), ConstraintRelation::GreaterEqual, 4.0);
    let if_then_constraint = IfThenFunction::new(1341, "if_then_constraint", cp, cq, 10.0);
    constraint_model
        .add_symbol(Arc::new(if_then_constraint))
        .unwrap();

    let mut constraint_mechanism = constraint_model.try_into_mechanism_model().unwrap();
    constraint_mechanism.add_constraint(linear_constraint(
        &[(cx_index, 1.0)],
        ConstraintRelation::Equal,
        3.5,
        "x_eq_3_5",
    ));
    let infeasible_output = solve_linear_model(
        constraint_mechanism.into_linear_triad_model(),
        &[(cx_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    assert_eq!(infeasible_output.status, SolverStatus::Infeasible);
}

#[test]
fn masking_slack_piecewise_parity() {
    let mut masking_model = MetaModel::<f64>::new("kotlin_masking_parity");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(1400),
        "x",
        VariableRange::bounded(-3.0, 2.0),
    );
    let mask = BinaryVariableItem::create(VariableId::standalone(1401), "mask");
    let x_index = masking_model.register_variable(x).unwrap();
    masking_model.register_variable(mask.clone()).unwrap();
    let masking = MaskingFunction::new(1402, "masking", var_poly(x_index), mask);
    let masking_id = masking.result_variable().id();
    masking_model.add_symbol(Arc::new(masking)).unwrap();

    let mechanism = masking_model.try_into_mechanism_model().unwrap();
    let masking_index = mechanism.find_token(masking_id).unwrap().solver_index;

    let min_masking = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(masking_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&min_masking);
    assert_close(min_masking.objective_value.unwrap(), -3.0);

    let mut mask_off = mechanism;
    let mask_solver_index = mask_off
        .find_token(VariableId::standalone(1401))
        .unwrap()
        .solver_index;
    mask_off.add_constraint(linear_constraint(
        &[(x_index, 1.0)],
        ConstraintRelation::Equal,
        -1.0,
        "x_eq_neg1",
    ));
    mask_off.add_constraint(linear_constraint(
        &[(mask_solver_index, 1.0)],
        ConstraintRelation::Equal,
        0.0,
        "mask_eq_0",
    ));
    let mask_off_output = solve_linear_model(
        mask_off.into_linear_triad_model(),
        &[(masking_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&mask_off_output);
    assert_close(mask_off_output.objective_value.unwrap(), 0.0);

    let mut slack_model = MetaModel::<f64>::new("kotlin_slack_parity");
    let sx = ContinuousVariableItem::with_range(
        VariableId::standalone(1410),
        "x",
        VariableRange::bounded(-3.0, 2.0),
    );
    let sx_index = slack_model.register_variable(sx).unwrap();
    let slack = SlackFunction::with_target(1411, "slack", var_poly(sx_index), 5.0);
    let slack_id = slack.result_variable().id();
    slack_model.add_symbol(Arc::new(slack)).unwrap();
    let mut slack_mechanism = slack_model.try_into_mechanism_model().unwrap();
    let slack_index = slack_mechanism.find_token(slack_id).unwrap().solver_index;
    slack_mechanism.add_constraint(linear_constraint(
        &[(sx_index, 1.0)],
        ConstraintRelation::GreaterEqual,
        -3.0,
        "sx_lb",
    ));
    slack_mechanism.add_constraint(linear_constraint(
        &[(sx_index, 1.0)],
        ConstraintRelation::LessEqual,
        2.0,
        "sx_ub",
    ));
    let slack_output = solve_linear_model(
        slack_mechanism.into_linear_triad_model(),
        &[(slack_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&slack_output);
    assert_close(slack_output.objective_value.unwrap(), 3.0);

    let mut slack_range_model = MetaModel::<f64>::new("kotlin_slack_range_parity");
    let rx = ContinuousVariableItem::with_range(
        VariableId::standalone(1420),
        "x",
        VariableRange::bounded(-3.0, 2.0),
    );
    let rx_index = slack_range_model.register_variable(rx).unwrap();
    let slack_range = SlackRangeFunction::new(1421, "slack_range", var_poly(rx_index), 5.0, 6.0);
    let slack_range_id = slack_range.result_variable().id();
    slack_range_model.add_symbol(Arc::new(slack_range)).unwrap();
    let mut slack_range_mechanism = slack_range_model.try_into_mechanism_model().unwrap();
    let slack_range_index = slack_range_mechanism
        .find_token(slack_range_id)
        .unwrap()
        .solver_index;
    slack_range_mechanism.add_constraint(linear_constraint(
        &[(rx_index, 1.0)],
        ConstraintRelation::GreaterEqual,
        -3.0,
        "rx_lb",
    ));
    slack_range_mechanism.add_constraint(linear_constraint(
        &[(rx_index, 1.0)],
        ConstraintRelation::LessEqual,
        2.0,
        "rx_ub",
    ));
    let slack_range_output = solve_linear_model(
        slack_range_mechanism.into_linear_triad_model(),
        &[(slack_range_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&slack_range_output);
    assert_close(slack_range_output.objective_value.unwrap(), 3.0);

    let mut ulp_model = MetaModel::<f64>::new("kotlin_ulp_parity");
    let ux = ContinuousVariableItem::with_range(
        VariableId::standalone(1430),
        "x",
        VariableRange::bounded(0.0, 2.0),
    );
    let ux_index = ulp_model.register_variable(ux).unwrap();
    let ulp = UnivariateLinearPiecewiseFunction::new(
        1431,
        "ulp",
        var_poly(ux_index),
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 2.0),
            Point2::new(2.0, 1.0),
        ],
    );
    let ulp_id = ulp.result_variable().id();
    ulp_model.add_symbol(Arc::new(ulp)).unwrap();
    let ulp_mechanism = ulp_model.try_into_mechanism_model().unwrap();
    let ulp_index = ulp_mechanism.find_token(ulp_id).unwrap().solver_index;
    let ulp_output = solve_linear_model(
        ulp_mechanism.into_linear_triad_model(),
        &[(ulp_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&ulp_output);
    assert_close(ulp_output.objective_value.unwrap(), 2.0);
    let ulp_solution = ulp_output.solution.unwrap();
    assert_close(ulp_solution[ux_index], 1.0);

    let mut blp_model = MetaModel::<f64>::new("kotlin_blp_parity");
    let bx = ContinuousVariableItem::with_range(
        VariableId::standalone(1440),
        "x",
        VariableRange::bounded(0.0, 2.0),
    );
    let by = ContinuousVariableItem::with_range(
        VariableId::standalone(1441),
        "y",
        VariableRange::bounded(0.0, 2.0),
    );
    let bx_index = blp_model.register_variable(bx).unwrap();
    let by_index = blp_model.register_variable(by).unwrap();
    let blp = BivariateLinearPiecewiseFunction::new(
        1442,
        "blp",
        var_poly(bx_index),
        var_poly(by_index),
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(1.0, 1.0, 1.0),
        ],
    );
    let blp_id = blp.result_variable().id();
    blp_model.add_symbol(Arc::new(blp)).unwrap();
    let blp_mechanism = blp_model.try_into_mechanism_model().unwrap();
    let blp_index = blp_mechanism.find_token(blp_id).unwrap().solver_index;
    let blp_output = solve_linear_model(
        blp_mechanism.into_linear_triad_model(),
        &[(blp_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&blp_output);
    let blp_solution = blp_output.solution.unwrap();
    assert_close(blp_solution[bx_index], 1.0);
    assert_close(blp_solution[by_index], 1.0);
}

#[test]
fn max_min_and_semi_like_parity() {
    let mut max_min_model = MetaModel::<f64>::new("kotlin_max_min_parity");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(1500),
        "x",
        VariableRange::bounded(3.0, 5.0),
    );
    let y = ContinuousVariableItem::with_range(
        VariableId::standalone(1501),
        "y",
        VariableRange::bounded(2.0, 10.0),
    );
    let x_index = max_min_model.register_variable(x).unwrap();
    let y_index = max_min_model.register_variable(y).unwrap();

    let minmax = MinMaxFunction::new(1502, "minmax", vec![var_poly(x_index), var_poly(y_index)]);
    let minmax_id = minmax.result_variable().id();
    max_min_model.add_symbol(Arc::new(minmax)).unwrap();
    let max = MaxFunction::new(
        1510,
        "max",
        vec![var_poly(x_index), var_poly(y_index)],
        false,
    );
    let max_id = max.result_variable().id();
    max_min_model.add_symbol(Arc::new(max)).unwrap();

    let maxmin = MaxMinFunction::new(1520, "maxmin", vec![var_poly(x_index), var_poly(y_index)]);
    let maxmin_id = maxmin.result_variable().id();
    max_min_model.add_symbol(Arc::new(maxmin)).unwrap();
    let min = MinFunction::new(
        1530,
        "min",
        vec![var_poly(x_index), var_poly(y_index)],
        false,
    );
    let min_id = min.result_variable().id();
    max_min_model.add_symbol(Arc::new(min)).unwrap();

    let mechanism = max_min_model.try_into_mechanism_model().unwrap();
    let minmax_index = mechanism.find_token(minmax_id).unwrap().solver_index;
    let max_index = mechanism.find_token(max_id).unwrap().solver_index;
    let maxmin_index = mechanism.find_token(maxmin_id).unwrap().solver_index;
    let min_index = mechanism.find_token(min_id).unwrap().solver_index;

    let minmax_min = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(minmax_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&minmax_min);
    assert_close(minmax_min.objective_value.unwrap(), 3.0);

    let minmax_max = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(minmax_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&minmax_max);
    assert_close(minmax_max.objective_value.unwrap(), 10.0);

    let max_minimize = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(max_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&max_minimize);
    assert_close(max_minimize.objective_value.unwrap(), 3.0);

    let max_maximize = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(max_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    assert_eq!(max_maximize.status, SolverStatus::Unbounded);

    let maxmin_minimize = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(maxmin_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&maxmin_minimize);
    assert_close(maxmin_minimize.objective_value.unwrap(), 2.0);

    let maxmin_maximize = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(maxmin_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&maxmin_maximize);
    assert_close(maxmin_maximize.objective_value.unwrap(), 5.0);

    let min_minimize = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(min_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    assert_eq!(min_minimize.status, SolverStatus::Unbounded);

    let min_maximize = solve_linear_model(
        mechanism.into_linear_triad_model(),
        &[(min_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&min_maximize);
    assert_close(min_maximize.objective_value.unwrap(), 5.0);

    let mut semi_model = MetaModel::<f64>::new("kotlin_semi_like_parity");
    let sx = ContinuousVariableItem::with_range(
        VariableId::standalone(1510),
        "x",
        VariableRange::bounded(0.0, 3.0),
    );
    let sy = ContinuousVariableItem::with_range(
        VariableId::standalone(1511),
        "y",
        VariableRange::bounded(2.0, 5.0),
    );
    let sx_index = semi_model.register_variable(sx).unwrap();
    let sy_index = semi_model.register_variable(sy).unwrap();
    let semi_like = BinaryzationFunction::with_big_m(
        1512,
        "semi_like",
        Linear::new(
            vec![
                LinearMonomial::new(1.0, sx_index),
                LinearMonomial::new(-1.0, sy_index),
            ],
            0.0,
        ),
        10.0,
    );
    let semi_id = semi_like.result_variable().id();
    semi_model.add_symbol(Arc::new(semi_like)).unwrap();
    let semi_mechanism = semi_model.try_into_mechanism_model().unwrap();
    let semi_index = semi_mechanism.find_token(semi_id).unwrap().solver_index;

    let semi_min = solve_linear_model(
        semi_mechanism.clone().into_linear_triad_model(),
        &[(semi_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&semi_min);
    assert_close(semi_min.objective_value.unwrap(), 0.0);
    let semi_min_solution = semi_min.solution.unwrap();
    assert!(semi_min_solution[sy_index] >= semi_min_solution[sx_index] - 1e-6);

    let semi_max = solve_linear_model(
        semi_mechanism.into_linear_triad_model(),
        &[(semi_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&semi_max);
    assert_close(semi_max.objective_value.unwrap(), 1.0);
}

#[test]
fn sigmoid_and_masking_range_parity() {
    let mut masking_range_model = MetaModel::<f64>::new("kotlin_masking_range_parity");
    let mask = BinaryVariableItem::create(VariableId::standalone(1550), "mask");
    let mask_index = masking_range_model.register_variable(mask).unwrap();
    let masking_range =
        MaskingRangeFunction::new(1551, "masking_range", var_poly(mask_index), -2.0, 3.0);
    let masking_range_id = masking_range.result_variable().id();
    masking_range_model
        .add_symbol(Arc::new(masking_range))
        .unwrap();

    let mechanism = masking_range_model.try_into_mechanism_model().unwrap();
    let y_index = mechanism.find_token(masking_range_id).unwrap().solver_index;

    let mut mask_on = mechanism.clone();
    mask_on.add_constraint(linear_constraint(
        &[(mask_index, 1.0)],
        ConstraintRelation::Equal,
        1.0,
        "mask_eq_1",
    ));
    let mask_on_output = solve_linear_model(
        mask_on.into_linear_triad_model(),
        &[(y_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&mask_on_output);
    assert_close(mask_on_output.objective_value.unwrap(), 3.0);

    let mut mask_off = mechanism;
    mask_off.add_constraint(linear_constraint(
        &[(mask_index, 1.0)],
        ConstraintRelation::Equal,
        0.0,
        "mask_eq_0",
    ));
    let mask_off_output = solve_linear_model(
        mask_off.into_linear_triad_model(),
        &[(y_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&mask_off_output);
    assert_close(mask_off_output.objective_value.unwrap(), 0.0);

    let mut sigmoid_model = MetaModel::<f64>::new("kotlin_sigmoid_parity");
    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(1560),
        "x",
        VariableRange::fixed(0.0),
    );
    let x_index = sigmoid_model.register_variable(x).unwrap();
    let sigmoid = SigmoidFunction::new(1561, "sigmoid", var_poly(x_index));
    let sigmoid_id = sigmoid.result_variable().id();
    sigmoid_model.add_symbol(Arc::new(sigmoid)).unwrap();

    let mut sigmoid_mechanism = sigmoid_model.try_into_mechanism_model().unwrap();
    let sigmoid_index = sigmoid_mechanism
        .find_token(sigmoid_id)
        .unwrap()
        .solver_index;
    sigmoid_mechanism.add_constraint(linear_constraint(
        &[(sigmoid_index, 1.0)],
        ConstraintRelation::Equal,
        0.5,
        "sigmoid_eq_half",
    ));
    let sigmoid_output = solve_linear_model(
        sigmoid_mechanism.into_linear_triad_model(),
        &[(sigmoid_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&sigmoid_output);
    assert_close(sigmoid_output.objective_value.unwrap(), 0.5);
}

#[test]
fn and_with_nonbinary_integer_inputs_parity() {
    let mut model = MetaModel::<f64>::new("kotlin_and_integer_inputs_parity");
    let x = UIntegerVariableItem::with_range(
        VariableId::standalone(1600),
        "x",
        VariableRange::bounded(0.0, 1.0),
    );
    let y = UIntegerVariableItem::with_range(
        VariableId::standalone(1601),
        "y",
        VariableRange::bounded(0.0, 2.0),
    );
    let x_index = model.register_variable(x).unwrap();
    let y_index = model.register_variable(y).unwrap();
    let and_fn = AndFunction::new(1602, "and", vec![var_poly(x_index), var_poly(y_index)]);
    let and_id = and_fn.result_variable().id();
    model.add_symbol(Arc::new(and_fn)).unwrap();

    let mut mechanism = model.try_into_mechanism_model().unwrap();
    let and_index = mechanism.find_token(and_id).unwrap().solver_index;
    mechanism.add_constraint(linear_constraint(
        &[(and_index, 1.0)],
        ConstraintRelation::Equal,
        1.0,
        "and_true",
    ));
    mechanism.add_constraint(linear_constraint(
        &[(x_index, 1.0), (and_index, -1.0)],
        ConstraintRelation::GreaterEqual,
        0.0,
        "x_ge_and",
    ));
    mechanism.add_constraint(linear_constraint(
        &[(y_index, 1.0), (and_index, -1.0)],
        ConstraintRelation::GreaterEqual,
        0.0,
        "y_ge_and",
    ));

    let max_output = solve_linear_model(
        mechanism.clone().into_linear_triad_model(),
        &[(x_index, 1.0), (y_index, 1.0)],
        ObjectiveCategory::Maximum,
    );
    expect_feasible(&max_output);
    assert_close(max_output.objective_value.unwrap(), 3.0);

    let min_output = solve_linear_model(
        mechanism.into_linear_triad_model(),
        &[(x_index, 1.0), (y_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&min_output);
    assert_close(min_output.objective_value.unwrap(), 2.0);
}

#[test]
fn integer_bter_smoke() {
    let mut model = MetaModel::<f64>::new("kotlin_bter_integer_smoke");
    let x = IntegerVariableItem::with_range(
        VariableId::standalone(1700),
        "x",
        VariableRange::bounded(-2.0, 2.0),
    );
    let x_index = model.register_variable(x).unwrap();

    let pos_flag = BinaryzationFunction::with_big_m(1701, "pos", var_poly(x_index), 10.0);
    let neg_flag = BinaryzationFunction::with_big_m(
        1702,
        "neg",
        Linear::new(vec![LinearMonomial::new(-1.0, x_index)], 0.0),
        10.0,
    );
    let pos_id = pos_flag.result_variable().id();
    let neg_id = neg_flag.result_variable().id();
    model.add_symbol(Arc::new(pos_flag)).unwrap();
    model.add_symbol(Arc::new(neg_flag)).unwrap();

    let bter = BalanceTernaryzationFunction::new(1703, "bter");
    let bter_id = bter.result_variable().id();
    let bter_pos_id = bter.positive_variable().id();
    let bter_neg_id = bter.negative_variable().id();
    model.add_symbol(Arc::new(bter)).unwrap();

    let mut mechanism = model.try_into_mechanism_model().unwrap();
    let pos_index = mechanism.find_token(pos_id).unwrap().solver_index;
    let neg_index = mechanism.find_token(neg_id).unwrap().solver_index;
    let bter_pos_index = mechanism.find_token(bter_pos_id).unwrap().solver_index;
    let bter_neg_index = mechanism.find_token(bter_neg_id).unwrap().solver_index;
    let bter_index = mechanism.find_token(bter_id).unwrap().solver_index;

    mechanism.add_constraint(linear_constraint(
        &[(bter_pos_index, 1.0), (pos_index, -1.0)],
        ConstraintRelation::Equal,
        0.0,
        "link_pos",
    ));
    mechanism.add_constraint(linear_constraint(
        &[(bter_neg_index, 1.0), (neg_index, -1.0)],
        ConstraintRelation::Equal,
        0.0,
        "link_neg",
    ));
    mechanism.add_constraint(linear_constraint(
        &[(x_index, 1.0)],
        ConstraintRelation::Equal,
        -2.0,
        "x_eq_neg2",
    ));

    let output = solve_linear_model(
        mechanism.into_linear_triad_model(),
        &[(bter_index, 1.0)],
        ObjectiveCategory::Minimum,
    );
    expect_feasible(&output);
    assert_close(output.objective_value.unwrap(), -1.0);
}
