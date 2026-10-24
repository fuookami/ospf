use std::sync::Arc;
use ospf_rust_framework_csp1d::{Csp1dMilp, Csp1dColumnGeneration, Csp1dProblem};
use ospf_rust_framework_csp1d::application::model::{Csp1dSolveConfig, Csp1dConfiguration};
use ospf_rust_framework_csp1d::domain::material::{Material, Product, ProductDemand, WidthRange};
use ospf_rust_framework_csp1d::domain::produce::{Csp1dDomainPolicy, Csp1dDomainCalculationContext};
use ospf_rust_framework_csp1d::domain::cutting_plan_generation::{NSameGenerator, ReducedCostPricingGenerator};
use ospf_rust_quantities::{Quantity, unit::{CTUnit, Meter, Unit}};

pub fn make_problem() -> Csp1dProblem<f64> {
    let m = |value| Quantity::<f64, Unit>::new(value, Meter::INSTANT.clone());
    // #region input
    let product = Product { id: "A".into(), name: "strip".into(), width: vec![m(0.8)],
        length: None, unit_weight: None, weight: None, max_over_produce_length: None, dynamic_length: false };
    let stock = Material { id: "M1".into(), name: "stock".into(), width_range: WidthRange::new(m(0.8), m(1.6)),
        length: None, unit_weight: None, machine_id: None, available_batches: 3 };
    let quarantined = Material { id: "M2".into(), ..stock.clone() };
    Csp1dProblem::new(vec![product.clone()], vec![stock, quarantined], vec![], vec![ProductDemand::legacy_roll(product, 6.0)])
    // #endregion input
}

// #region rule
struct Quarantine;
impl Csp1dDomainPolicy<f64> for Quarantine {
    fn name(&self) -> &str { "quarantine-M2" }
    fn is_feasible(&self, context: &dyn Csp1dDomainCalculationContext<f64>) -> bool { context.material().id != "M2" }
}
// #endregion rule

pub fn run_business() {
    let problem = make_problem();
    // #region injection
    let mut config = Csp1dSolveConfig::default();
    config.extension_set.domain_policies.push(Arc::new(Quarantine));
    let initial = NSameGenerator::<f64>::default();
    let milp = Csp1dMilp::with_initial_generator(Box::new(initial.clone())).solve(problem.clone(), Some(config.clone()));
    println!("status={:?}, unmet={:?}", milp.status, milp.produce.unmet_demands);
    assert!(milp.produce.cutting_plans.iter().all(|usage| usage.plan.material.id != "M2"));
    assert!(milp.produce.unmet_demands.is_empty(), "Expected six rolls from three batches");
    // #endregion injection
    // #region entry
    config.column_generation = Csp1dConfiguration { max_initial_plans: 64, max_pricing_plans: 16, iteration_limit: 8 };
    let cg = Csp1dColumnGeneration::with_generators(Box::new(initial.clone()), Box::new(ReducedCostPricingGenerator::new(initial)))
        .solve_with_trace(problem, Some(config));
    println!("termination={:?}, final={:?}", cg.trace.termination_reason, cg.trace.final_milp_status);
    // #endregion entry
}
