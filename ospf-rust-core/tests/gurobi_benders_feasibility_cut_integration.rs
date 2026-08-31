#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::collections::HashMap;

use ospf_rust_core::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};
use ospf_rust_core::model::{
    BasicMechanismModel, ConstraintRelation, Linear, LinearConstraint, LinearInequality,
    LinearMonomial, MechanismModel, ObjectiveCategory, QuadraticConstraint, QuadraticInequality,
};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn sparse_row(entries: &[(usize, f64)]) -> SparseVector<f64> {
    let mut row = SparseVector::new();
    for (index, value) in entries {
        row.add(*index, *value);
    }
    row
}

fn lhs_value(inequality: &LinearInequality<f64>, values: &HashMap<usize, f64>) -> f64 {
    let mut value = *inequality.polynomial.constant_term();
    for monomial in inequality.polynomial.monomials() {
        value +=
            *monomial.coefficient() * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
    }
    value
}

fn satisfies(inequality: &LinearInequality<f64>, values: &HashMap<usize, f64>) -> bool {
    let lhs = lhs_value(inequality, values);
    match inequality.relation {
        ConstraintRelation::LessEqual => lhs <= inequality.rhs + 1e-9,
        ConstraintRelation::GreaterEqual => lhs + 1e-9 >= inequality.rhs,
        ConstraintRelation::Equal => (lhs - inequality.rhs).abs() <= 1e-9,
    }
}

#[test]
fn gurobi_farkas_solution_can_generate_mechanism_feasibility_cut() {
    // 机制层模板：x 为主问题固定变量，y 为子问题变量。
    // Mechanism template: x is fixed by master, y is recourse variable.
    let x = ContinuousVariableItem::create(VariableId::standalone(9301), "x");
    let x_id = x.id();
    let y = ContinuousVariableItem::create(VariableId::standalone(9302), "y");

    let mut mechanism_basic = BasicMechanismModel::new("benders_template");
    mechanism_basic.add_token(Token::from_generic(x, 0));
    mechanism_basic.add_token(Token::from_generic(y, 1));

    // y >= x  =>  x - y <= 0
    mechanism_basic.add_constraint(LinearConstraint::new(
        LinearInequality::less_equal(
            Linear::new(
                vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                0.0,
            ),
            0.0,
        ),
        "link_xy",
    ));
    // y <= 1
    mechanism_basic.add_constraint(LinearConstraint::new(
        LinearInequality::less_equal(Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0), 1.0),
        "cap_y",
    ));

    let mechanism = MechanismModel::from_basic(mechanism_basic);

    // 固定 x = 2 后的子问题（仅保留 y）：
    // -y <= -2  (y >= 2)
    //  y <=  1
    let mut fixed_subproblem_basic = BasicLinearTriadModel::new("fixed_subproblem");
    let y_sub = ContinuousVariableItem::create(VariableId::standalone(9303), "y_sub");
    fixed_subproblem_basic.add_variable_with_bounds(
        Token::from_generic(y_sub, 0),
        f64::NEG_INFINITY,
        f64::INFINITY,
        VariableType::Continuous,
    );
    fixed_subproblem_basic.add_constraint(sparse_row(&[(0, -1.0)]), -2.0);
    fixed_subproblem_basic.add_constraint(sparse_row(&[(0, 1.0)]), 1.0);

    let mut fixed_subproblem = LinearTriadModel::from_basic(fixed_subproblem_basic);
    fixed_subproblem.set_objective(vec![0.0], ObjectiveCategory::Minimum);

    let solver = GurobiSolver::new();
    let primal_output = solver
        .solve_linear(&fixed_subproblem)
        .expect("fixed subproblem should be solvable by Gurobi");
    assert!(
        primal_output.status.is_infeasible(),
        "fixed subproblem should be infeasible, got {:?}",
        primal_output.status
    );

    let farkas = fixed_subproblem.to_farkas_dual();
    let farkas_output = solver
        .solve_linear(&farkas)
        .expect("farkas dual should be solvable by Gurobi");
    assert!(
        farkas_output.status.is_feasible(),
        "farkas dual should be feasible, got {:?}",
        farkas_output.status
    );

    let cut = mechanism
        .generate_feasibility_cuts_from_farkas_output(&HashMap::from([(x_id, 2.0)]), &farkas_output)
        .expect("mechanism model should generate feasibility cut from farkas solution")
        .into_iter()
        .next()
        .expect("feasibility cut list should contain one cut");
    assert_eq!(cut.relation, ConstraintRelation::LessEqual);

    // 当前固定点 x=2 应被切除，而 x=1 仍保留。
    // Current fixed point x=2 should be cut off, while x=1 should remain.
    let lhs_at_infeasible = lhs_value(&cut, &HashMap::from([(0usize, 2.0)]));
    assert!(
        lhs_at_infeasible > 1e-6,
        "cut should violate infeasible fixed point, lhs={}",
        lhs_at_infeasible
    );
    let lhs_at_feasible = lhs_value(&cut, &HashMap::from([(0usize, 1.0)]));
    assert!(
        lhs_at_feasible <= 1e-6,
        "cut should keep feasible point, lhs={}",
        lhs_at_feasible
    );
}

#[test]
fn gurobi_dual_solution_can_generate_mechanism_optimal_cut() {
    // 机制层模板：x 为主问题固定变量，y 为子问题变量，theta 为主问题目标近似变量。
    // Mechanism template: x fixed by master, y recourse variable, theta master objective estimator.
    let x = ContinuousVariableItem::create(VariableId::standalone(9401), "x");
    let x_id = x.id();
    let y = ContinuousVariableItem::create(VariableId::standalone(9402), "y");
    let theta = ContinuousVariableItem::create(VariableId::standalone(9403), "theta");
    let theta_id = theta.id();

    let mut mechanism_basic = BasicMechanismModel::new("benders_optimal_template");
    mechanism_basic.add_token(Token::from_generic(x, 0));
    mechanism_basic.add_token(Token::from_generic(y, 1));
    mechanism_basic.add_token(Token::from_generic(theta, 2));
    mechanism_basic.add_constraint(LinearConstraint::new(
        // y >= x  =>  x - y <= 0
        LinearInequality::less_equal(
            Linear::new(
                vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                0.0,
            ),
            0.0,
        ),
        "link_xy",
    ));
    let mechanism = MechanismModel::from_basic(mechanism_basic);

    // 固定 x = 2 后的子问题（仅保留 y）：
    // min y
    // s.t. y >= 2  => -y <= -2
    let mut fixed_subproblem_basic = BasicLinearTriadModel::new("fixed_subproblem_optimal");
    let y_sub = ContinuousVariableItem::create(VariableId::standalone(9404), "y_sub");
    fixed_subproblem_basic.add_variable_with_bounds(
        Token::from_generic(y_sub, 0),
        f64::NEG_INFINITY,
        f64::INFINITY,
        VariableType::Continuous,
    );
    fixed_subproblem_basic.add_constraint(sparse_row(&[(0, -1.0)]), -2.0);

    let mut fixed_subproblem = LinearTriadModel::from_basic(fixed_subproblem_basic);
    fixed_subproblem.set_objective(vec![1.0], ObjectiveCategory::Minimum);

    let solver = GurobiSolver::new();
    let primal_output = solver
        .solve_linear(&fixed_subproblem)
        .expect("fixed subproblem should be solvable by Gurobi");
    assert!(
        primal_output.status.is_feasible(),
        "fixed subproblem should be feasible, got {:?}",
        primal_output.status
    );
    let primal_obj = primal_output
        .objective_value
        .expect("fixed subproblem objective should be available");
    assert!(
        (primal_obj - 2.0).abs() <= 1e-6,
        "expected fixed subproblem objective 2.0, got {}",
        primal_obj
    );

    let dual_model = fixed_subproblem.to_dual();
    let dual_output = solver
        .solve_linear(&dual_model)
        .expect("dual model should be solvable by Gurobi");
    assert!(
        dual_output.status.is_feasible(),
        "dual model should be feasible, got {:?}",
        dual_output.status
    );
    let cut = mechanism
        .generate_optimal_cuts_from_dual_output(
            theta_id,
            &HashMap::from([(x_id, 2.0)]),
            &dual_output,
        )
        .expect("mechanism model should generate optimal cut from dual solution")
        .into_iter()
        .next()
        .expect("optimal cut list should contain one cut");
    assert_eq!(cut.relation, ConstraintRelation::GreaterEqual);

    // x=2 时，theta=1 应违反 cut，theta=2 应满足 cut。
    // At x=2, theta=1 should violate the cut while theta=2 should satisfy it.
    assert!(
        !satisfies(&cut, &HashMap::from([(0usize, 2.0), (2usize, 1.0)])),
        "theta=1 should violate optimal cut at x=2"
    );
    assert!(
        satisfies(&cut, &HashMap::from([(0usize, 2.0), (2usize, 2.0)])),
        "theta=2 should satisfy optimal cut at x=2"
    );
}

#[test]
fn gurobi_quadratic_duals_can_generate_mechanism_quadratic_optimal_cut() {
    let x = ContinuousVariableItem::create(VariableId::standalone(9501), "x");
    let x_id = x.id();
    let y = ContinuousVariableItem::create(VariableId::standalone(9502), "y");
    let theta = ContinuousVariableItem::create(VariableId::standalone(9503), "theta");
    let theta_id = theta.id();

    let mut mechanism_basic = BasicMechanismModel::new("benders_quadratic_optimal_template");
    mechanism_basic.add_token(Token::from_generic(x, 0));
    mechanism_basic.add_token(Token::from_generic(y, 1));
    mechanism_basic.add_token(Token::from_generic(theta, 2));
    mechanism_basic.add_constraint(LinearConstraint::new(
        LinearInequality::less_equal(
            Linear::new(
                vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                0.0,
            ),
            0.0,
        ),
        "link_xy",
    ));
    mechanism_basic.add_quadratic_constraint(QuadraticConstraint::new(
        QuadraticInequality::new(
            Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
            ConstraintRelation::LessEqual,
            4.0,
        ),
        "x_square_cap",
    ));
    let mechanism = MechanismModel::from_basic(mechanism_basic);

    // 固定 x=2 后子问题：
    // min -y
    // s.t. y >= 2    (from x - y <= 0)
    //      y^2 <= 4  (quadratic bound)
    let mut sub_basic = BasicQuadraticTetradModel::new("fixed_quadratic_subproblem");
    let y_sub = ContinuousVariableItem::create(VariableId::standalone(9504), "y_sub");
    sub_basic.linear.add_variable_with_bounds(
        Token::from_generic(y_sub, 0),
        f64::NEG_INFINITY,
        f64::INFINITY,
        VariableType::Continuous,
    );
    sub_basic
        .linear
        .add_constraint(sparse_row(&[(0, -1.0)]), -2.0);

    let mut subproblem = QuadraticTetradModel::from_basic(sub_basic);
    let mut q = SparseMatrix::new();
    q.add_row(SparseVector::new());
    subproblem.set_objective(vec![-1.0], q, ObjectiveCategory::Minimum);
    subproblem.add_quadratic_constraint_with_metadata(
        QuadraticInequality::new(
            Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
            ConstraintRelation::LessEqual,
            4.0,
        ),
        "y_square_cap".to_string(),
        None,
        false,
        0,
        None,
        None,
    );

    let solver = GurobiSolver::new();
    let output = solver
        .solve_quadratic(&subproblem)
        .expect("fixed quadratic subproblem should be solvable by Gurobi");
    assert!(
        output.status.is_feasible(),
        "fixed quadratic subproblem should be feasible, got {:?}",
        output.status
    );
    assert!(
        output
            .dual_solution
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false),
        "linear dual solution should be available"
    );
    assert!(
        output
            .quadratic_dual_solution
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false),
        "quadratic dual solution should be available"
    );

    let cut = mechanism
        .generate_optimal_quadratic_cut_from_output(
            theta_id,
            &HashMap::from([(x_id, 2.0)]),
            &output,
        )
        .expect("mechanism model should generate quadratic optimal cut from solver output");
    assert_eq!(cut.relation, ConstraintRelation::GreaterEqual);
    assert!(
        cut.polynomial
            .monomials()
            .iter()
            .any(|mono| mono.var_index1() == 0 && mono.var_index2() == Some(0)),
        "quadratic optimal cut should contain x^2 term"
    );
    assert!(
        cut.polynomial
            .monomials()
            .iter()
            .any(|mono| mono.var_index1() == 2 && mono.var_index2().is_none()),
        "quadratic optimal cut should contain theta linear term"
    );
}
