use ospf_rust_framework::solver::meta_model_backend::MetaModelSolverBackend;
use ospf_rust_framework_bpp3d::application::service::*;
use ospf_rust_framework_bpp3d::application::CsvDatasetLoader;

// Requires the `serde` feature. Backends are real, separately configured dependencies.
pub fn run_example<R: MetaModelSolverBackend, F: MetaModelSolverBackend>(
    rmp_backend: R, final_backend: F,
) -> Result<(), String> {
    let csv = "# table:items
item_id,name,shape_type,width,height,depth,weight,amount
i1,carton,cuboid,2,3,4,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BOX,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,4
";
    let request = CsvDatasetLoader::load_str(csv).map_err(|e| format!("{e:?}"))?
        .materialize().map_err(|e| format!("{e:?}"))?;
    let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
    let rmp = SolverBackedMetaModelRmpExecutor::new(MetaModelRmpExecutorConfig::default(), rmp_backend);
    let final_executor = SolverBackedMetaModelFinalExecutor::new(MetaModelFinalExecutorConfig::default(), final_backend);
    let result = service.run_csv_materialized_with_bins(request, &rmp, &final_executor)
        .map_err(|e| format!("{e:?}"))?;
    println!("rmp objective={:?}", result.rmp.objective);
    // #region result
    println!("final={}, objective={:?}", result.final_execution.final_solved, result.final_execution.objective);
    for bin in &result.final_execution.packed_bins {
        println!("bin={}, type={:?}, weight={:?}, volume={:?}", bin.name, bin.bin_type.type_code, bin.total_weight(), bin.total_actual_volume());
        for item in &bin.items {
            println!("item={:?}, xyz={:?}, orientation={:?}, order={}", item.item.id, item.position, item.orientation, item.loading_order);
        }
    }
    let count: usize = result.final_execution.packed_bins.iter().map(|b| b.items.len()).sum();
    if result.final_execution.final_solved && count != 1 { return Err(format!("Expected one carton, got {count}")); }
    // #endregion result
    println!("diagnostics={:?}", result.result.info);
    Ok(())
}
