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
    println!("covered: {}", solution.fully_covered);
    println!("material usage: {}", solution.total_material_usage());
    println!("waste area: {}", solution.total_waste_area());
    Ok(())
}
