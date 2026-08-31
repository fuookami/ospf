use super::aggregation::Aggregation;

/// 推荐重量均衡上下文 / Recommended weight equalization context
/// 对齐 Kotlin RecommendedWeightEqualizationContext
#[derive(Debug)]
pub struct RecommendedWeightEqualizationContext {
    pub aggregation: Option<Aggregation>,
}

impl RecommendedWeightEqualizationContext {
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    /// 从飞机和装载上下文初始化
    /// 对齐 Kotlin RecommendedWeightEqualizationContext.init
    pub fn init(
        &mut self,
        _aircraft_context: &super::super::aircraft::AircraftContext,
        _stowage_context: &super::super::stowage::context::StowageContext<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.aggregation = Some(Aggregation {
            appointments: Vec::new(),
        });
        Ok(())
    }

    pub fn register(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
        x_idx: &[Vec<usize>],
        cargo_weights: &[f64],
        cargo_priorities: &[u8],
        position_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        super::service::generate_pipelines(model, x_idx, cargo_weights, cargo_priorities, position_count)
    }
}
