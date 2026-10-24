use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{rectangle_shape2, PackingShape2};
use ospf_rust_quantities::{Quantity, unit::Meter};
use ospf_rust_framework_bpp2d::application::{algorithm::*, config::ColumnGenerationConfig,
    state::{ColumnGenerationInput, ColumnGenerationResult}, service::ColumnGenerationPackingAnalyzer};
use ospf_rust_framework_bpp2d::domain::{item::model::*, layer_generation::*, layer_assignment::Bpp2dAssignmentDemandEntry};

// Pass this configuration to the application's standard executor adapters.
// #region objective
pub fn cost_config() -> ospf_rust_framework_bpp2d::application::service::ColumnGenerationStandardExecutorConfig {
    ospf_rust_framework_bpp2d::application::service::ColumnGenerationStandardExecutorConfig {
        sheet_area_weight: 0.0, sheet_amount_weight: 0.0, sheet_type_cost_weight: 1.0,
        sheet_type_costs: [(SheetTypeId::new("S"), 5.0), (SheetTypeId::new("L"), 3.0)].into_iter().collect(),
        ..Default::default()
    }
}
// #endregion objective

// Geometric input only: the current application seam shares U for weight and length.
pub fn make_input(spacing: f64) -> Ret<ColumnGenerationInput<f64, Meter>> {
    let m = |value| Quantity::<f64, Meter>::new_ct(value);
    // #region input
    let part = ActualItem::new(ItemId::new("R"), "part", PackingShape2::Rectangle(rectangle_shape2(m(2.0), m(2.0))?), false, m(0.0));
    let small = SheetType::new(m(4.0), m(4.0), m(0.0), SheetTypeId::new("S"));
    let large = SheetType::new(m(5.0), m(5.0), m(0.0), SheetTypeId::new("L"));
    Ok(ColumnGenerationInput { sheet: small.clone(), sheets: vec![small.clone(), large], items: vec![(part, 4)],
        demands: vec![Bpp2dAssignmentDemandEntry::item_amount("R", 4)], kerf: Some(m(spacing)),
        sheet_shape: small.geometry(), item_catalog: [(ItemId::new("R"), false)].into_iter().collect() })
    // #endregion input
}

// #region rule
struct ApprovedSheets;
impl Bpp2dLayerGenerator<f64, Meter> for ApprovedSheets {
    fn name(&self) -> &str { "approved-sheets" }
    fn generate(&self, request: &Bpp2dLayerGenerationRequest<f64, Meter>) -> Vec<Bpp2dLayerGenerationResult<f64, Meter>> {
        LayerGenerationContext::new().generate(request, &LayerGenerationContext::default_generators())
            .into_iter().filter(|c| c.layer.sheet.as_ref().is_some_and(|s| s.type_code == SheetTypeId::new("S"))).collect()
    }
}
// #endregion rule

pub fn run_business(rmp: Box<dyn ColumnGenerationRmpSolver>, final_solver: Box<dyn ColumnGenerationFinalSolver>)
    -> Ret<ColumnGenerationResult<f64, Meter>> {
    let input = make_input(0.0)?;
    // #region entry
    let config = ColumnGenerationConfig { iteration_limit: 12, max_columns_per_iteration: 24,
        time_limit_ms: 30_000, final_milp_enabled: true, ..Default::default() };
    // #endregion entry
    // #region injection
    let algorithm = ColumnGenerationAlgorithm::new(Box::new(ApprovedSheets), input.demands.clone())
        .with_rmp_solver(rmp).with_final_solver(final_solver);
    let request = Bpp2dLayerGenerationRequest::new(0, input.items)
        .with_sheets(input.sheets).with_kerf(input.kerf.clone().unwrap()).with_max_candidates(24);
    let mut result = algorithm.solve(request, &config)?;
    let analyzer = ColumnGenerationPackingAnalyzer::new();
    let packing = analyzer.analyze(&result, input.kerf.as_ref())?;
    result.render = Some(analyzer.render(&packing));
    result.packing = Some(packing);
    // #endregion injection
    Ok(result)
}
