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
    println!("status={:?}, failure={:?}", solution.status, solution.failure_message);
    println!("unmet={:?}", solution.produce.unmet_demands);
    for usage in solution.produce.cutting_plans {
        println!("plan={}, usages={}", usage.plan.id, usage.amount);
    }
}
