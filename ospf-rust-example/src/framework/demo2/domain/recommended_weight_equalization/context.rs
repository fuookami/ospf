//! 推荐重量均衡上下文 / Recommended weight equalization context
use super::aggregation::Aggregation;
use crate::framework::demo2::domain::stowage::model::{LoadVariables, Position};

/// 推荐重量均衡上下文 / Recommended weight equalization context
/// 对齐 Kotlin RecommendedWeightEqualizationContext
#[derive(Debug)]
pub struct RecommendedWeightEqualizationContext {
    /// 推荐重量均衡聚合 / Recommended weight equalization aggregation
    pub aggregation: Option<Aggregation>,
}

impl RecommendedWeightEqualizationContext {
    /// 创建新的推荐重量均衡上下文 / Create a new recommended weight equalization context
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    /// 从飞机和装载上下文初始化 / Initialize from aircraft and stowage context
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

    /// 注册推荐重量均衡约束到模型 / Register recommended weight equalization constraints to model
    pub fn register(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
        x_idx: &[Vec<usize>],
        cargo_weights: &[f64],
        cargo_priorities: &[u8],
        position_count: usize,
        load_vars: &LoadVariables,
        positions: &[Position],
    ) -> Result<(), Box<dyn std::error::Error>> {
        super::service::generate_pipelines(model, x_idx, cargo_weights, cargo_priorities, position_count, load_vars, positions)
    }
}
