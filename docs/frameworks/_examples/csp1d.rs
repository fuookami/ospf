use ospf_rust_framework_csp1d::{Csp1dMilp, Csp1dProblem};
use ospf_rust_framework_csp1d::domain::material::{Material, Product, ProductDemand, WidthRange};
use ospf_rust_quantities::{Quantity, unit::{CTUnit, Meter, Unit}};

fn meters(value: f64) -> Quantity<f64, Unit> {
    Quantity::new(value, Meter::INSTANT.clone())
}

pub fn run_example() {
    let product = Product {
        id: "P1".into(), name: "strip".into(), width: vec![meters(0.8)],
        length: None, unit_weight: None, weight: None,
        max_over_produce_length: None, dynamic_length: false,
    };
    let material = Material {
        id: "M1".into(), name: "stock".into(),
        width_range: WidthRange::new(meters(0.5), meters(1.5)),
        length: None, unit_weight: None, machine_id: None, available_batches: 3,
    };
    let demand = ProductDemand::legacy_roll(product.clone(), 3.0);
    let problem = Csp1dProblem::new(vec![product], vec![material], vec![], vec![demand]);
    // The default implementation is heuristic; this does not invoke native MILP.
    let solution = Csp1dMilp::<f64>::default().solve(problem, None);
    // #region result
    println!("status={:?}, failure={:?}, kpi={:?}", solution.status, solution.failure_message, solution.kpi);
    println!("unmet={:?}", solution.produce.unmet_demands);
    for usage in &solution.produce.cutting_plans {
        println!("plan={}, material={}, usages={}, rest={:?}", usage.plan.id, usage.plan.material.id, usage.amount, usage.plan.rest_width());
        for slice in &usage.plan.slices { println!("width={:?}, pieces per usage={}", slice.width, slice.amount); }
        for c in &usage.plan.demand_contributions { println!("{}: per usage={:?}, multiplier={}", c.product.id, c.quantity, usage.amount); }
    }
    assert!(solution.produce.unmet_demands.is_empty(), "Demand is not fully covered");
    // #endregion result
}
