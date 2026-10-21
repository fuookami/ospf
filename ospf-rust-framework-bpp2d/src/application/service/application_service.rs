//! 端到端应用服务 / End-to-end application service
//!
//! 装配列生成算法、执行器、装箱与渲染，对外只暴露一个入口。
//! Assembles the column-generation algorithm, executors, packing and rendering behind a single entry point.

use ospf_rust_base::error::Ret;

use crate::application::algorithm::ColumnGenerationAlgorithm;
use crate::application::config::ColumnGenerationConfig;
use crate::application::service::analyzer::ColumnGenerationPackingAnalyzer;
use crate::application::state::{ColumnGenerationInput, ColumnGenerationResult};
use crate::domain::layer_generation::LayerGenerationContext;

/// 端到端应用服务 / End-to-end application service
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationApplicationService;

impl ColumnGenerationApplicationService {
    /// 创建服务 / Create the service
    pub fn new() -> Self {
        Self
    }

    /// 执行端到端求解 / Run the end-to-end solve
    ///
    /// 流程：列生成 → 最终 MILP → 装箱 → 渲染。装箱失败不会抹掉已有的列生成结果，
    /// 而是以 `Err` 上报，让调用方自行决定是否接受部分结果。
    /// Flow: column generation → final MILP → packing → rendering. A packing failure never discards the
    /// column-generation result; it surfaces as `Err` so the caller decides whether to accept a partial result.
    pub fn solve(
        &self,
        input: ColumnGenerationInput<f64, ospf_rust_quantities::unit::derived::Meter>,
        config: &ColumnGenerationConfig,
        rmp_solver: Option<Box<dyn crate::application::algorithm::ColumnGenerationRmpSolver>>,
        final_solver: Option<Box<dyn crate::application::algorithm::ColumnGenerationFinalSolver>>,
    ) -> Ret<ColumnGenerationResult<f64, ospf_rust_quantities::unit::derived::Meter>> {
        type V = f64;
        type U = ospf_rust_quantities::unit::derived::Meter;
        let generators = LayerGenerationContext::default_generators::<f64, U>();
        let generator = LayerGenerationContext::new();
        let mut algorithm: ColumnGenerationAlgorithm =
            ColumnGenerationAlgorithm::new(Box::new(CompositeGenerator { generator, generators }), input.demands.clone());
        if let Some(solver) = rmp_solver {
            algorithm = algorithm.with_rmp_solver(solver);
        }
        if let Some(solver) = final_solver {
            algorithm = algorithm.with_final_solver(solver);
        }

        let request = crate::domain::layer_generation::Bpp2dLayerGenerationRequest {
            iteration: 0,
            sheet: Some(input.sheet.clone()),
            sheets: input.sheets.clone(),
            items: input.items.clone(),
            existing_layers: Vec::new(),
            demand_entries: Vec::new(),
            shadow_prices: std::collections::BTreeMap::new(),
            kerf: input.kerf.clone(),
            max_candidates: config.max_columns_per_iteration,
        };

        let mut result = algorithm.solve(request, config)?;
        let analyzer = ColumnGenerationPackingAnalyzer::new();
        let packing = analyzer.analyze(&result, input.kerf.as_ref())?;
        result.render = Some(analyzer.render(&packing));
        result.packing = Some(packing);
        Ok(result)
    }
}

/// 组合生成器：把 `LayerGenerationContext` 适配成单一 `Bpp2dLayerGenerator`
/// / Composite generator adapting `LayerGenerationContext` into a single `Bpp2dLayerGenerator`
struct CompositeGenerator<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    generator: LayerGenerationContext,
    generators: Vec<Box<dyn crate::domain::layer_generation::Bpp2dLayerGenerator<V, U>>>,
}

impl<V, U> std::fmt::Debug for CompositeGenerator<V, U>
where
    U: ospf_rust_quantities::unit::concept::UnitTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeGenerator")
            .field("generators", &self.generators.len())
            .finish()
    }
}

impl crate::domain::layer_generation::Bpp2dLayerGenerator<f64, ospf_rust_quantities::unit::derived::Meter>
    for CompositeGenerator<f64, ospf_rust_quantities::unit::derived::Meter>
{
    fn name(&self) -> &str {
        "composite"
    }

    fn generate(
        &self,
        request: &crate::domain::layer_generation::Bpp2dLayerGenerationRequest<
            f64,
            ospf_rust_quantities::unit::derived::Meter,
        >,
    ) -> Vec<
        crate::domain::layer_generation::Bpp2dLayerGenerationResult<
            f64,
            ospf_rust_quantities::unit::derived::Meter,
        >,
    > {
        self.generator.generate(request, &self.generators)
    }
}
