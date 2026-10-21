//! 可行性阶段与 LP 终态契约测试 / Feasibility phase and LP terminal-state contract tests
//!
//! 对应 Kotlin 27388bbd 的回归：LP 终态校验（不可行 / 无界 / 非最优 / 无对偶都返回显式错误）、
//! 可行性阶段的 objective 只含人工短缺、真实方案列系数为零、可行性 master 仍不可行时保留失败。
//! Mirrors the Kotlin 27388bbd regressions: LP terminal validation (infeasible, unbounded, non-optimal and
//! missing duals all return explicit errors), the feasibility phase's objective containing only artificial
//! deficits, real plan columns carrying a zero coefficient, and the failure being kept when the feasibility
//! master is still infeasible.

use std::sync::Arc;

use ospf_rust_base::error::Ret;
use ospf_rust_framework_csp1d::application::lp_solver::{
    Csp1dLpSolver, LpMasterSolution, LpTerminalStatus, require_pricing_ready,
};
use ospf_rust_framework_csp1d::domain::produce::{
    Csp1dModelContext, Csp1dModelingMode, Csp1dProduceContextBuilder, ProduceInput,
};
use ospf_rust_framework_csp1d::domain::material::{
    CuttingPlan, CuttingPlanDemandContribution, CuttingPlanProduction, CuttingPlanSlice, DemandMode,
    Machine, Material, Product, ProductDemand, WidthRange,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn quantity(value: f64) -> Quantity<f64, ospf_rust_quantities::unit::Unit> {
    Quantity::new(value, Meter::INSTANT.clone())
}

fn product() -> Product<f64> {
    Product {
        id: "p1".into(),
        name: "Product 1".into(),
        width: vec![quantity(30.0)],
        length: None,
        unit_weight: None,
        weight: None,
        max_over_produce_length: None,
        dynamic_length: false,
    }
}

fn material() -> Material<f64> {
    Material {
        id: "m1".into(),
        name: "Material 1".into(),
        width_range: WidthRange::new(quantity(0.0), quantity(100.0)),
        length: None,
        unit_weight: None,
        machine_id: Some("mc1".into()),
        available_batches: 3,
    }
}

fn machine() -> Machine<f64> {
    Machine {
        id: "mc1".into(),
        name: "Machine 1".into(),
        max_batch_count: Some(3),
        max_switch_count: Some(1),
        width_range: Some(WidthRange::new(quantity(0.0), quantity(100.0))),
        capacity: Some(quantity(8.0)),
    }
}

fn demand() -> ProductDemand<f64> {
    ProductDemand {
        product: product(),
        quantity: quantity(2.0),
        mode: Some(DemandMode::Roll),
    }
}

fn plan(id: &str) -> CuttingPlan<f64> {
    CuttingPlan {
        id: id.into(),
        material: material(),
        machine_id: Some("mc1".into()),
        slices: vec![CuttingPlanSlice {
            production: CuttingPlanProduction::Product(product()),
            width: quantity(30.0),
            amount: 1,
        }],
        demand_contributions: vec![CuttingPlanDemandContribution {
            product: product(),
            quantity: quantity(1.0),
        }],
        capacity_consumption: Some(quantity(1.0)),
    }
}

fn input() -> ProduceInput<f64> {
    ProduceInput {
        cutting_plans: vec![plan("p")],
        demands: vec![demand()],
        materials: vec![material()],
        machines: vec![machine()],
        warm_start_plan_usages: Vec::new(),
    }
}

// ============================================================================
// LP 终态校验 / LP terminal validation
// ============================================================================

/// 最优解且对偶非空才可用于定价 / Only an optimal solution with duals supports pricing
#[test]
fn optimal_with_duals_is_pricing_ready() {
    let solution = LpMasterSolution::optimal(1.0, vec![0.5], vec![1.0]);
    assert!(solution.status.is_optimal());
    assert!(require_pricing_ready(&solution).is_ok());
}

/// 不可行终态返回显式错误 / An infeasible terminal state returns an explicit error
#[test]
fn infeasible_is_rejected() {
    let solution = LpMasterSolution::infeasible();
    assert_eq!(solution.status, LpTerminalStatus::Infeasible);
    let error = require_pricing_ready(&solution).expect_err("must be rejected");
    assert!(
        error.to_string().contains("不可行") || error.to_string().contains("infeasible"),
        "{error}"
    );
}

/// 无界终态返回显式错误 / An unbounded terminal state returns an explicit error
#[test]
fn unbounded_is_rejected() {
    let mut solution = LpMasterSolution::optimal(0.0, vec![1.0], vec![]);
    solution.status = LpTerminalStatus::Unbounded;
    let error = require_pricing_ready(&solution).expect_err("must be rejected");
    assert!(
        error.to_string().contains("无界") || error.to_string().contains("unbounded"),
        "{error}"
    );
}

/// 其它非最优终态返回显式错误 / Any other non-optimal terminal state returns an explicit error
#[test]
fn other_terminal_state_is_rejected() {
    let mut solution = LpMasterSolution::optimal(0.0, vec![1.0], vec![]);
    solution.status = LpTerminalStatus::Other;
    let error = require_pricing_ready(&solution).expect_err("must be rejected");
    assert!(
        error.to_string().contains("非最优") || error.to_string().contains("non-optimal"),
        "{error}"
    );
}

/// 缺对偶返回显式错误，不用零值伪造 / Missing duals return an explicit error, never fabricated zeros
#[test]
fn missing_duals_is_rejected() {
    let solution = LpMasterSolution::optimal(1.0, Vec::new(), vec![1.0]);
    let error = require_pricing_ready(&solution).expect_err("must be rejected");
    assert!(
        error.to_string().contains("对偶") || error.to_string().contains("dual"),
        "{error}"
    );
}

/// 诊断信息可追加 / Diagnostics can be appended
#[test]
fn diagnostics_can_be_appended() {
    let solution = LpMasterSolution::optimal(1.0, vec![0.5], vec![1.0])
        .with_info("backend", "fake");
    assert_eq!(solution.info.get("backend").map(String::as_str), Some("fake"));
}

// ============================================================================
// 可行性阶段 / Feasibility phase
// ============================================================================

/// 可行性阶段注册人工短缺变量并只把它们放进目标 / The feasibility phase registers artificial deficits and
/// only they enter the objective
#[test]
fn feasibility_phase_objective_contains_only_deficits() {
    let mut builder = Csp1dProduceContextBuilder::new(input());
    builder.mode(Csp1dModelingMode::LP);
    builder.feasibility_phase();
    // 人工短缺权重 1、超产 0。
    let mut under = std::collections::BTreeMap::new();
    let mut over = std::collections::BTreeMap::new();
    let key = ospf_rust_framework_csp1d::domain::material::ProductDemandShadowPriceKey {
        product_id: "p1".into(),
        unit_symbol: ospf_rust_framework_csp1d::domain::material::shadow_price_unit_symbol(
            &quantity(1.0).unit,
        ),
    };
    under.insert(key.clone(), 1.0);
    over.insert(key, 0.0);
    builder.yield_config(ospf_rust_framework_csp1d::domain::r#yield::YieldModelingConfig {
        under_production_penalty: under,
        over_production_penalty: over,
        over_production_upper_bound: std::collections::BTreeMap::new(),
    });

    let mut context = builder.build().expect("context");
    assert!(context.feasibility_phase, "flag must propagate to the context");

    let mut model = ospf_rust_core::model::MetaModel::<f64>::new("phase-one");
    context.register(&mut model).expect("register");

    // 目标名应体现 phase-one，且真实方案列不应出现在目标里。
    // The objective name reflects phase-one and no real plan column may appear in it.
    let objective = model.objective();
    assert!(!objective.sub_objectives.is_empty(), "phase one must have terms");
    let plan_tokens: Vec<usize> = context
        .produce
        .cutting_plans
        .iter()
        .enumerate()
        .filter_map(|(index, _)| context.produce.plan_variable_index(index))
        .collect();
    for sub in &objective.sub_objectives {
        for monomial in sub.polynomial.monomials() {
            assert!(
                !plan_tokens.contains(&monomial.var_index()),
                "phase one must not carry plan columns in the objective"
            );
        }
    }
}

/// 默认（非可行性）阶段的目标仍含业务项 / The default phase keeps business objective terms
#[test]
fn default_phase_keeps_business_objective() {
    let mut builder = Csp1dProduceContextBuilder::new(input());
    builder.mode(Csp1dModelingMode::MILP);
    let mut context = builder.build().expect("context");
    assert!(!context.feasibility_phase);

    let mut model = ospf_rust_core::model::MetaModel::<f64>::new("business");
    context.register(&mut model).expect("register");
    assert!(!model.objective().sub_objectives.is_empty());
}

// ============================================================================
// 可注入 LP 求解器 / Injectable LP solver
// ============================================================================

/// 记录调用次数的假求解器 / A fake solver recording invocations
#[derive(Debug)]
struct FakeSolver {
    status: LpTerminalStatus,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl Csp1dLpSolver<f64> for FakeSolver {
    fn name(&self) -> &str {
        "fake"
    }

    fn solve_lp(
        &self,
        _name: &str,
        model: &ospf_rust_core::model::MetaModel<f64>,
    ) -> Ret<LpMasterSolution> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut solution = LpMasterSolution::optimal(
            0.0,
            vec![0.0; model.constraints().len()],
            vec![0.0; model.num_tokens()],
        );
        solution.status = self.status;
        Ok(solution)
    }
}

/// 假求解器可用于注入并记录调用 / The fake solver is injectable and records calls
#[test]
fn injectable_lp_solver_records_calls() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let solver: Arc<dyn Csp1dLpSolver<f64>> = Arc::new(FakeSolver {
        status: LpTerminalStatus::Optimal,
        calls: calls.clone(),
    });

    let service = ospf_rust_framework_csp1d::Csp1dColumnGeneration::<f64> {
        lp_solver: Some(solver),
        ..Default::default()
    };

    let problem = ospf_rust_framework_csp1d::csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });
    let _ = service.solve_with_trace(problem, None);

    assert!(calls.load(std::sync::atomic::Ordering::SeqCst) > 0);
}

/// 不可行的假求解器不会让流程崩溃 / An infeasible fake solver does not crash the flow
#[test]
fn infeasible_lp_is_reported_not_panicking() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let solver: Arc<dyn Csp1dLpSolver<f64>> = Arc::new(FakeSolver {
        status: LpTerminalStatus::Infeasible,
        calls: calls.clone(),
    });
    let service = ospf_rust_framework_csp1d::Csp1dColumnGeneration::<f64> {
        lp_solver: Some(solver),
        ..Default::default()
    };
    let problem = ospf_rust_framework_csp1d::csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });

    let result = service.solve_with_trace(problem, None);

    // 不可行会被记录为 LP 失败原因，人工短缺不会成为业务产出。
    // The infeasibility is recorded as the LP failure reason and deficits never become business output.
    assert!(
        result.trace.lp_failure_message.is_some()
            || result.solution.failure_message.is_some(),
        "an infeasible LP must surface a failure message"
    );
}

/// 先不可行、可行性恢复后可行的假求解器 / A fake solver that is infeasible first and feasible after phase one
///
/// 调用顺序：第 1 次原始 master 不可行 → 第 2 次可行性 master 最优且目标为 0（短缺已消除）
/// → 第 3 次重建的原始 master 最优。
/// Call order: the first business master is infeasible, the second feasibility master is optimal with a zero
/// objective (deficits removed), and the third rebuilt business master is optimal.
#[derive(Debug)]
struct RecoveringSolver {
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl Csp1dLpSolver<f64> for RecoveringSolver {
    fn name(&self) -> &str {
        "recovering"
    }

    fn solve_lp(
        &self,
        _name: &str,
        model: &ospf_rust_core::model::MetaModel<f64>,
    ) -> Ret<LpMasterSolution> {
        let index = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let duals = vec![0.5; model.constraints().len()];
        let primal = vec![1.0; model.num_tokens()];
        let mut solution = match index {
            0 => LpMasterSolution::infeasible(),
            // 可行性 master：目标 0 表示人工短缺已消除。
            // Feasibility master: a zero objective means the artificial deficits are gone.
            1 => LpMasterSolution::optimal(0.0, duals, primal),
            _ => LpMasterSolution::optimal(1.0, duals, primal),
        };
        solution.info.insert("call".into(), index.to_string());
        Ok(solution)
    }
}

/// 可行性恢复后会重建原始 master 并继续 / Feasibility restoration rebuilds the business master and continues
#[test]
fn restored_feasibility_rebuilds_business_master() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let solver: Arc<dyn Csp1dLpSolver<f64>> = Arc::new(RecoveringSolver {
        calls: calls.clone(),
    });
    let service = ospf_rust_framework_csp1d::Csp1dColumnGeneration::<f64> {
        lp_solver: Some(solver),
        ..Default::default()
    };
    let problem = ospf_rust_framework_csp1d::csp1d_problem::<f64, _>(|builder| {
        builder
            .product(product())
            .material(material())
            .machine(machine())
            .demand(demand());
    });

    let result = service.solve_with_trace(problem, None);

    // 至少调用三次：原始 → 可行性 → 重建的原始。
    // At least three calls: business, feasibility, rebuilt business.
    assert!(
        calls.load(std::sync::atomic::Ordering::SeqCst) >= 3,
        "恢复后必须重建原始 master 并再次求解"
    );
    // 恢复成功时不应留下 LP 失败信息。
    // A successful restoration must leave no LP failure message.
    assert!(
        result.trace.lp_failure_message.is_none(),
        "恢复后不应残留失败信息：{:?}",
        result.trace.lp_failure_message
    );
}
