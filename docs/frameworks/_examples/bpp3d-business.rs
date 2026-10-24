use ospf_rust_framework::solver::meta_model_backend::MetaModelSolverBackend;
use ospf_rust_framework_bpp3d::application::{CsvDatasetLoader, csv::CsvMaterializedApplicationRequest, service::*};
use ospf_rust_framework_bpp3d::domain::packing::{PackedBin, service::{PackingGeometryContract, PackingGeometryGuard}};
use ospf_rust_quantities::unit::Meter;

pub fn make_request(small_only: bool) -> Result<CsvMaterializedApplicationRequest, String> {
    // The CSV adapter fixes geometry to metres; each record below is one physical carton.
    // #region input
    let mut csv = String::from("# table:items\nitem_id,name,shape_type,width,height,depth,weight,amount\n");
    for i in 0..10 { csv.push_str(&format!("i{i},carton,cuboid,2,2,2,1,1\n")); }
    csv.push_str("# table:bins\nbin_id,type_code,width,height,depth,capacity\ns1,S,4,4,4,100\ns2,S,4,4,4,100\nl1,L,4,4,6,100\n");
    csv.push_str("# table:layers\nlayer_id,bin_id,depth\ns1-layer,s1,4\ns2-layer,s2,4\nl-layer,l1,6\n");
    let mut request = CsvDatasetLoader::load_str(&csv).map_err(|e| format!("{e:?}"))?.materialize().map_err(|e| format!("{e:?}"))?;
    // #endregion input
    // Restrict both available bins and seed layers before optimization.
    if small_only {
        request.bins.retain(|b| b.type_code != "L");
        request.initial_layers.retain(|l| l.bin.as_ref().is_some_and(|b| b.type_code != "L"));
    }
    Ok(request)
}

// A delivery guard complements input restrictions; it is not a pricing constraint.
// #region rule
#[derive(Debug, Clone)]
struct SmallOnly;
impl PackingGeometryContract<f64, Meter> for SmallOnly {
    fn validate(&self, bin: &PackedBin<f64, Meter>) -> Result<(), Vec<String>> {
        PackingGeometryGuard::new().validate(bin)?;
        if bin.bin_type.type_code == "L" { Err(vec!["Only approved S bins may be delivered".into()]) } else { Ok(()) }
    }
}
// #endregion rule

pub fn run_business<R: MetaModelSolverBackend, F: MetaModelSolverBackend>(rmp_backend: R, final_backend: F) -> Result<(), String> {
    // #region objective
    let final_config = MetaModelFinalExecutorConfig { final_bin_volume_weight: 0.0,
        final_bin_amount_coefficient: 0.0, final_bin_type_cost_weight: 1.0,
        final_bin_type_costs: [("S".into(), 4.0), ("L".into(), 10.0)].into_iter().collect(), ..Default::default() };
    // #endregion objective
    // #region entry
    let config = ColumnGenerationConfig::new().with_max_iterations(12)
        .with_max_column_amount(256).with_max_candidates_per_iteration(24);
    // #endregion entry
    // #region injection
    let service = ColumnGenerationApplicationService::with_geometry_guard(config, SmallOnly);
    let request = make_request(true)?;
    let rmp = SolverBackedMetaModelRmpExecutor::new(MetaModelRmpExecutorConfig::default(), rmp_backend);
    let final_executor = SolverBackedMetaModelFinalExecutor::new(final_config, final_backend);
    let result = service.run_csv_materialized_with_bins(request, &rmp, &final_executor).map_err(|e| format!("{e:?}"))?;
    // #endregion injection
    println!("final={}, objective={:?}, diagnostics={:?}", result.final_execution.final_solved, result.final_execution.objective, result.result.info);
    Ok(())
}
