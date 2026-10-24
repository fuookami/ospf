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
    let result = ColumnGenerationApplicationService::new().solve(
        input, &ColumnGenerationConfig::default(), Some(rmp), Some(final_solver),
    )?;
    println!("lp solves={}, final solved={}", result.lp_solved_times, result.final_solved);
    println!("sheets={}, objective={:?}", result.sheet_count(), result.final_objective);
    if result.packing.is_none() {
        println!("No deliverable geometry; do not present render presence as success");
    }
    Ok(result)
}
