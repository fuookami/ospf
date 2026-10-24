use std::time::Duration;
use ospf_rust_core::error::{ModelError, Result as CoreResult};
use ospf_rust_core::model::{MetaModel, intermediate::LinearTriadModel};
use ospf_rust_core::solver::{LinearSolver, SolveHandle, SolveOptions, SolveReport};
use ospf_rust_framework_bpp::infrastructure::SameUnitPackingValueAdapter;
use ospf_rust_framework_bpp1d::{application::*, domain::*, infrastructure::*};
use ospf_rust_quantities::{Quantity, unit::{Meter, Kilogram}};

const ITEMS_CSV: &str = "id,length,weight,amount\na,6,1,2\nb,4,1,2\n";
const BINS_CSV: &str = "id,length,max_weight,cost,inventory\nsmall,10,100,3,2\nlarge,20,100,10,1\n";

fn parse_number(text: &str) -> CoreResult<f64> {
    text.parse::<f64>().map_err(|e| ModelError::InvalidConstraint(format!("Invalid number {text}: {e}")).into())
}

pub fn load_scenario() -> CoreResult<PackingContext<f64, Meter, Kilogram>> {
    // #region csv
    let options = PackingCsvOptions { length_unit: Meter, weight_unit: Kilogram,
        spacing: Quantity::new_ct(0.0), objective: PackingObjective::Cost };
    let context = load_packing_csv(ITEMS_CSV, BINS_CSV, options, parse_number,
        &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter)?;
    // #endregion csv
    Ok(context)
}

fn backend(solver: &dyn LinearSolver) -> impl FnMut(&LinearTriadModel, Duration, &SolveHandle) -> CoreResult<SolveReport<f64>> + '_ {
    move |model, remaining, handle| solver.solve_linear_report_with_options(model, &SolveOptions {
        time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default()
    })
}

// #region policy
pub struct NoMixedItems;
impl PatternPolicy for NoMixedItems {
    fn accept(&self, p: &PackingPattern) -> CoreResult<bool> {
        Ok(!(p.amounts.contains_key("a") && p.amounts.contains_key("b")))
    }
}
// #endregion policy

pub fn solve_no_mixed(solver: &dyn LinearSolver, context: PackingContext<f64, Meter, Kilogram>) -> CoreResult<Bpp1dSolution<Meter>> {
    // #region policy-injection
    let result = Bpp1dMilp::solve_with_extension(context, NoPatternExtension, &NoMixedItems,
        Bpp1dSolveConfig::default(), &SolveHandle::new(), backend(solver))?;
    // #endregion policy-injection
    Ok(result)
}

pub fn solve_column_generation(solver: &dyn LinearSolver, context: PackingContext<f64, Meter, Kilogram>) -> CoreResult<Bpp1dSolution<Meter>> {
    // #region cg
    let config = Bpp1dSolveConfig {
        generation: PatternGenerationConfig { max_patterns: 10_000, max_nodes: 100_000 },
        iteration_limit: 32, columns_per_iteration: 32, ..Default::default()
    };
    let result = Bpp1dColumnGeneration::solve(context, config, &SolveHandle::new(), backend(solver))?;
    // #endregion cg
    Ok(result)
}

// #region extension
#[derive(Clone)]
pub struct BinLimit(pub f64);
impl PatternModelingExtension<f64, Meter, Kilogram> for BinLimit {
    fn register(&self, model: &mut MetaModel<f64>, _: &PackingContext<f64, Meter, Kilogram>,
        a: &PatternAggregation<f64>, _: PackingModelStage) -> CoreResult<()> {
        let terms = a.inventories.values().flat_map(|expr| expr.monomials())
            .map(|term| (term.var_index(), *term.coefficient())).collect::<Vec<_>>();
        model.add_le_constraint(&terms, self.0, "business_bin_limit")?;
        Ok(())
    }
}
// #endregion extension

pub fn solve_with_bin_limit(solver: &dyn LinearSolver, context: PackingContext<f64, Meter, Kilogram>) -> CoreResult<Bpp1dSolution<Meter>> {
    // #region extension-injection
    let result = Bpp1dMilp::solve_with_extension(context, BinLimit(2.0), &AcceptAllPatterns,
        Bpp1dSolveConfig::default(), &SolveHandle::new(), backend(solver))?;
    // #endregion extension-injection
    Ok(result)
}

pub fn run_business_example(solver: &dyn LinearSolver) -> Result<(), String> {
    let run = || -> CoreResult<()> {
        let context = load_scenario()?;
        for objective in [PackingObjective::BinCount, PackingObjective::ConsumedLength, PackingObjective::Cost] {
            let mut problem = context.problem().clone();
            problem.objective = objective;
            let input = PackingContext::new(problem, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter)?;
            let result = Bpp1dMilp::solve(input, Default::default(), &SolveHandle::new(), backend(solver))?;
            println!("{objective:?}: complete={}, report={:?}, packing={:?}", result.candidate_complete, result.report, result.packing);
        }
        println!("no_mix={:?}", solve_no_mixed(solver, context.clone())?.packing);
        println!("cg={:?}", solve_column_generation(solver, context.clone())?.report);
        println!("bin_limit={:?}", solve_with_bin_limit(solver, context)?.packing);
        Ok(())
    };
    run().map_err(|e| format!("{e:?}"))
}
