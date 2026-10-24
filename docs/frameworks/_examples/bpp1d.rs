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
    let result = Bpp1dMilp::solve(context.clone(), Default::default(), &SolveHandle::new(), solve)
        .map_err(|e| format!("{e:?}"))?;
    println!("report={:?}, globally_optimal={}", result.report, result.globally_optimal);
    if let Some(packing) = &result.packing {
        // #region result
        println!("candidate_complete={}", result.candidate_complete);
        for plan in &packing.plans {
            let bin_type = context.problem().bin_types.iter().find(|b| b.id == plan.bin_type_id).ok_or("Unknown bin type")?;
            println!("bin={}, type={}", plan.bin_id, bin_type.id);
            for p in &plan.placements { println!("{}: [{}, {}) m", p.item_id, p.start.value, p.start.value + p.length.value); }
        }
        for f in &packing.free_intervals { println!("free {}: [{}, {}) m", f.bin_id, f.start.value, f.start.value + f.length.value); }
        let kpi = context.kpi(packing).map_err(|e| format!("{e:?}"))?;
        println!("bins={}, items={}, cost={}, utilization={}", kpi.bin_count, kpi.item_count, kpi.cost, kpi.utilization);
        println!("consumed={} m, tail={} m", kpi.consumed_length.value, kpi.remnant_length.value);
        // #endregion result
        let mut counts = BTreeMap::<&str, u64>::new();
        for p in packing.plans.iter().flat_map(|plan| &plan.placements) {
            *counts.entry(p.item_id.as_str()).or_default() += 1;
        }
        if !context.problem().items.iter().all(|item| counts.get(item.id.as_str()).copied().unwrap_or(0) == item.amount) {
            return Err("Packed item counts differ from demand".into());
        }
    } else {
        println!("No packing; inspect the solve report");
    }
    Ok(())
}
