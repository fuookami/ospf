//! Native multi-target analysis integration / 原生多 target 分析集成。

#![cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]

use ospf_rust_core::analysis::{
    AnalysisStatus, MultiTargetAnalysisOptions, MultiTargetAnalyzer, RelaxationCost,
};
use ospf_rust_core::model::constraint_programming::{
    ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
    ConstraintProgrammingSnapshot, IntegerDomain, IntegerExpression, IntegerObjective,
    IntegerRelation, IntegerVariable,
};
use ospf_rust_core::solver::constraint_programming::ConstraintProgrammingSolver;

fn snapshot() -> ConstraintProgrammingSnapshot {
    let x = IntegerVariable::new("native/x");
    let mut model = ConstraintProgrammingModel::new("native-multi-target-analysis");
    model
        .register_variable(x.clone(), IntegerDomain::boolean())
        .expect("variable");
    model
        .add_constraint(ConstraintDefinition::new(
            "native/upper",
            ConstraintProgrammingConstraint::integer(
                IntegerExpression::variable(x.clone()),
                IntegerRelation::LessOrEqual,
                0,
            ),
        ))
        .expect("constraint");
    model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
    model.freeze().expect("snapshot")
}

fn assert_batch<S: ConstraintProgrammingSolver + ?Sized>(solver: &S) {
    let report = MultiTargetAnalyzer::new()
        .analyze(
            solver,
            &snapshot(),
            &[
                ospf_rust_core::analysis::ObjectiveTarget::at_least("objective", 1.0)
                    .expect("unreachable target"),
                ospf_rust_core::analysis::ObjectiveTarget::at_least("objective", 0.0)
                    .expect("reachable target"),
            ],
            &MultiTargetAnalysisOptions::default().with_relaxation_cost(
                "constraint:native/upper",
                RelaxationCost {
                    weight: 2.0,
                    relaxation: 0.5,
                },
            ),
        )
        .expect("native multi-target report");

    assert_eq!(report.results.len(), 2);
    assert_eq!(report.results[0].feasibility.status, AnalysisStatus::Unreachable);
    assert_eq!(report.results[1].feasibility.status, AnalysisStatus::Reachable);
    assert_eq!(report.results[0].conflict.constraint_ids().len(), 1);
    assert_eq!(report.results[0].improvement_plans.len(), 1);
    assert_eq!(report.results[0].improvement_plans[0].correction_set.total_cost, 1.0);
    assert!(report.results[0].improvement_plans[0].requires_revalidation());
    assert!(report.results[0].conflict.minimality_verified());
    assert!(report.results[1].improvement_plans.is_empty());
    report
        .validate(&MultiTargetAnalysisOptions::default().relaxation_policy)
        .expect("native multi-target report should validate");
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
#[test]
fn gurobi_native_cp_adapter_runs_multi_target_analysis_and_recommendations() {
    use ospf_rust_core::solver::constraint_programming::MipBackedConstraintProgrammingSolver;
    use ospf_rust_core::solver::solvers::gurobi::{GurobiConfig, GurobiSolver};

    let solver = MipBackedConstraintProgrammingSolver::new(GurobiSolver::with_config(
        GurobiConfig::new().with_output(false),
    ));
    assert_batch(&solver);
}

#[cfg(feature = "scip")]
#[test]
fn scip_native_cp_adapter_runs_multi_target_analysis_and_recommendations() {
    use ospf_rust_core::solver::constraint_programming::ScipConstraintProgrammingSolver;
    use ospf_rust_core::solver::solvers::scip::SCIPConfig;

    let solver = ScipConstraintProgrammingSolver::with_config(
        SCIPConfig::recommended_lp_subproblem_defaults().with_output(false),
    );
    assert_batch(&solver);
}
