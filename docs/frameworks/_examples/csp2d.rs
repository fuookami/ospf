use ospf_rust_base::error::Ret;
use ospf_rust_framework_csp2d::application::{Csp2dProblem, Csp2dSolvers};
use ospf_rust_framework_csp2d::domain::material::{
    Material2, Product2, material_id2, product_id2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

pub fn run_example() -> Ret<()> {
    let material = Material2::sheet(
        material_id2("m1"), "sheet", meters(10.0), meters(8.0), 100,
    );
    let product = Product2::new(product_id2("a"), "a", meters(2.0), meters(2.0));
    let problem = Csp2dProblem::single(material, product, 20, meters(0.0));
    // The default service uses a heuristic master solver, not a native optimizer.
    let solution = Csp2dSolvers::new().solve_milp(problem)?;
    // #region result
    println!("covered={}, coverage={:?}, termination={:?}", solution.fully_covered, solution.coverage, solution.termination);
    for (plan, count) in &solution.usage {
        plan.validate(&meters(0.0))?;
        println!("plan={}, material={}, usages={}, per usage={:?}", plan.id, plan.pattern.material.id, count, plan.product_amounts());
        for p in &plan.pattern.placements { println!("{:?}: ({:?}, {:?}), size=({:?}, {:?})", p.item_id, p.x(), p.y(), p.width(), p.height()); }
        println!("cut tree={:?}", plan.pattern.cut_tree);
    }
    println!("material usage: {}", solution.total_material_usage());
    println!("waste area: {}", solution.total_waste_area());
    assert!(solution.fully_covered, "Demand is not fully covered");
    // #endregion result
    Ok(())
}
