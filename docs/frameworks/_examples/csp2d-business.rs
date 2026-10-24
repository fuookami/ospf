use ospf_rust_base::error::Ret;
use ospf_rust_quantities::{Quantity, unit::Meter};
use ospf_rust_framework_csp2d::application::{Csp2dProblem, Csp2dConfiguration, Csp2dSolvers};
use ospf_rust_framework_csp2d::domain::{material::*, pattern_generation::*};
use ospf_rust_framework_csp2d::infrastructure::cutting_process::CuttingProcess2;
use ospf_rust_framework_csp2d::application::master_solver::{Csp2dMasterSolver, Csp2dMasterSolveResult, require_deliverable};
use ospf_rust_framework_csp2d::domain::produce::{Csp2dMasterProblem, Csp2dPatternAssignment};
use ospf_rust_framework_csp2d::infrastructure::{region::{Region2, CutOrientation2}, cut_tree::{CutTree2, CutLine2}};
use ospf_rust_framework_geometry2d::{Placement2, QuantityPoint2, PackingShape2, rectangle_shape2};

pub fn make_problem(kerf: f64) -> Csp2dProblem {
    let m = |value| Quantity::<f64, Meter>::new_ct(value);
    // #region input
    let sheet = Material2::sheet(material_id2("S"), "sheet", m(10.0), m(6.0), 2);
    let a = Product2::new(product_id2("a"), "a", m(4.0), m(6.0)).with_allow_rotate(false);
    let b = Product2::new(product_id2("b"), "b", m(6.0), m(6.0)).with_allow_rotate(false);
    Csp2dProblem::new(vec![sheet], vec![a.clone(), b.clone()],
        vec![ProductDemand2::amount(a, 1), ProductDemand2::amount(b, 1)], m(kerf))
        .with_process(CuttingProcess2::GuillotineTwoStage)
    // #endregion input
}

// Specialized one-through-cut templates for this lesson's full-height products.
pub fn one_cut_plans(kerf: f64) -> Ret<Vec<CuttingPlan2<f64, Meter>>> {
    let problem = make_problem(kerf);
    let material = problem.materials[0].clone();
    let m = |value| Quantity::<f64, Meter>::new_ct(value);
    // #region cut
    let region = Region2::new(m(0.0), m(0.0), material.width.clone(), material.height.clone().unwrap());
    problem.products.iter().map(|product| {
        let (left, right) = region.cut(CutOrientation2::Vertical, &product.width, &problem.kerf)?;
        let tree = CutTree2::cut(CutLine2 { orientation: CutOrientation2::Vertical, position: product.width.clone(), stage: 1 },
            region.clone(), problem.kerf.clone(), CutTree2::leaf(left, Some(product.id.to_string())), CutTree2::leaf(right, None))?;
        let placement = Placement2::new(PackingShape2::Rectangle(rectangle_shape2(product.width.clone(), product.height.clone())?),
            QuantityPoint2 { x: m(0.0), y: m(0.0) }).with_item_id(product.id.to_string());
        let pattern = Pattern2::new(pattern_id2(format!("cut-{}", product.id)), material.clone(), region.clone(),
            vec![placement], problem.process).with_cut_tree(tree);
        pattern.validate(&problem.kerf)?;
        Ok(CuttingPlan2::from_pattern(pattern))
    }).collect()
    // #endregion cut
}

pub fn solve_restricted(solver: &dyn Csp2dMasterSolver, kerf: f64) -> Ret<Csp2dMasterSolveResult> {
    let problem = make_problem(kerf);
    // #region master
    let plans = if kerf == 0.0 { restricted_plans()? } else { one_cut_plans(kerf)? };
    let assignment = Csp2dPatternAssignment::new(plans, 2)?;
    let master = Csp2dMasterProblem::new(assignment, problem.master_demands())?;
    let solved = solver.solve_master(&master)?;
    require_deliverable(&solved)?;
    if solved.usage.len() != master.assignment.len() || !master.assignment.covers_all(&solved.usage, &master.demands)
        || solved.usage.iter().any(|u| !u.is_finite() || *u < 0.0 || (u.round() - u).abs() > 1e-7) {
        return Err(ospf_rust_framework_csp2d::infrastructure::error::validation_error("Invalid or incomplete integer usage"));
    }
    Ok(solved)
    // #endregion master
}

// #region rule
#[derive(Debug)]
struct NoMixedProducts;
impl PatternGenerator2<f64, Meter> for NoMixedProducts {
    fn name(&self) -> &str { "no-mixed-products" }
    fn generate(&self, request: &PatternGenerationRequest2<f64, Meter>) -> Vec<Pattern2<f64, Meter>> {
        request.products.iter().flat_map(|product| {
            let mut single = request.clone();
            single.products = vec![product.clone()];
            ShelfPatternGenerator2::new().generate(&single)
        }).filter(|p| p.product_amounts().len() <= 1).collect()
    }
}
// #endregion rule

// Domain generation seam; no high-level generator injection is available.
pub fn restricted_plans() -> Ret<Vec<CuttingPlan2<f64, Meter>>> {
    let problem = make_problem(0.0);
    // #region injection
    let generator: Box<dyn PatternGenerator2<f64, Meter>> = Box::new(NoMixedProducts);
    let request = PatternGenerationRequest2::new(problem.materials[0].clone(),
        problem.products.iter().map(|p| (p.clone(), 1)).collect(), problem.kerf).with_process(problem.process);
    request.validate()?;
    Ok(generator.generate(&request).into_iter().map(CuttingPlan2::from_pattern).collect())
    // #endregion injection
}

// #region entry
pub fn run_business() -> Ret<()> {
    let problem = make_problem(0.5).with_configuration(Csp2dConfiguration {
        max_candidates_per_iteration: 24, iteration_limit: 8, ..Default::default() });
    let milp = Csp2dSolvers::new().solve_milp(problem.clone())?;
    let cg = Csp2dSolvers::new().solve_column_generation(problem)?;
    println!("milp={:?}, cg={:?}", milp.termination, cg.termination);
    Ok(())
}
// #endregion entry
