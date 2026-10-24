use ospf_rust_base::error::Ret;
use ospf_rust_framework_bpp2d::application::{
    algorithm::{ColumnGenerationFinalSolver, ColumnGenerationRmpSolver},
    config::ColumnGenerationConfig,
    service::ColumnGenerationApplicationService,
    state::{ColumnGenerationInput, ColumnGenerationResult},
};
use ospf_rust_quantities::unit::Meter;

// The caller supplies normalized domain input and BOTH real solver adapters.
// This is an application boundary, not a standalone optimizer implementation.
pub fn solve_layout(
    input: ColumnGenerationInput<f64, Meter>,
    rmp: Box<dyn ColumnGenerationRmpSolver>,
    final_solver: Box<dyn ColumnGenerationFinalSolver>,
) -> Ret<ColumnGenerationResult<f64, Meter>> {
    let demands: Vec<_> = input.items.iter().map(|(item, count)| (item.id.clone(), *count as u64)).collect();
    let result = ColumnGenerationApplicationService::new().solve(
        input, &ColumnGenerationConfig::default(), Some(rmp), Some(final_solver),
    )?;
    println!("lp solves={}, final solved={}", result.lp_solved_times, result.final_solved);
    // #region result
    println!("limited={}, objective={:?}", result.terminated_by_iteration_limit, result.final_objective);
    if let Some(packing) = &result.packing {
        println!("counts={:?}, occupied={:?}, remnants={:?}", packing.item_amounts(), packing.occupied_area(), packing.remnant_area());
        for sheet in &packing.sheets {
            println!("sheet={}, type={:?}", sheet.name, sheet.sheet_type.type_code);
            for item in &sheet.items {
                let p = &item.placement;
                println!("{:?}: ({:?}, {:?}) -> ({:?}, {:?}), rotated={}", p.item_id, p.x(), p.y(), p.max_x()?, p.max_y()?, p.rotated);
            }
            for free in &sheet.remnants { println!("free={:?}, area={:?}", free.rectangle, free.area); }
        }
        let counts = packing.item_amounts();
        for (id, demand) in demands { assert_eq!(counts.get(&id).copied().unwrap_or(0), demand, "Demand mismatch: {id:?}"); }
    }
    // #endregion result
    Ok(result)
}
