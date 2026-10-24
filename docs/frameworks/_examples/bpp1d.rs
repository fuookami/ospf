use std::collections::BTreeMap;
use std::time::Duration;
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{LinearSolver, SolveHandle, SolveOptions};
use ospf_rust_framework_bpp::infrastructure::SameUnitPackingValueAdapter;
use ospf_rust_framework_bpp1d::application::Bpp1dMilp;
use ospf_rust_framework_bpp1d::domain::*;
use ospf_rust_quantities::{Quantity, unit::{Meter, Kilogram}};

pub fn run_example(solver: &dyn LinearSolver) -> Result<(), String> {
    let problem = PackingProblem {
        items: vec![Item {
            id: "a".into(), length: Quantity::<f64, Meter>::new_ct(0.4),
            weight: Quantity::<f64, Kilogram>::new_ct(2.0), amount: 3, materials: BTreeMap::new(),
        }],
        bin_types: vec![
            BinType { id: "small".into(), length: Quantity::new_ct(1.0), max_weight: Quantity::new_ct(5.0), cost: 1.0, inventory: Some(1) },
            BinType { id: "large".into(), length: Quantity::new_ct(2.0), max_weight: Quantity::new_ct(10.0), cost: 3.0, inventory: Some(1) },
        ],
        spacing: Quantity::new_ct(0.2), length_unit: Meter, weight_unit: Kilogram,
        objective: PackingObjective::BinCount,
    };
    let context = PackingContext::new(problem, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter)
        .map_err(|e| format!("{e:?}"))?;
    let solve = |model: &LinearTriadModel, remaining: Duration, handle: &SolveHandle| {
        solver.solve_linear_report_with_options(model, &SolveOptions {
            time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default()
        })
    };
    let result = Bpp1dMilp::solve(context, Default::default(), &SolveHandle::new(), solve)
        .map_err(|e| format!("{e:?}"))?;
    println!("globally_optimal={}", result.globally_optimal);
    match result.packing {
        Some(packing) => println!("plans={:?}", packing.plans),
        None => println!("No packing; inspect the solve report"),
    }
    Ok(())
}
