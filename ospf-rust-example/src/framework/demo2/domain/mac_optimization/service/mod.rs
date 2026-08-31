//! MAC 优化领域服务 / MAC optimization domain service.
use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

mod limits;
pub(crate) mod pipeline_list_generator;
mod policy;

/// 应用 MAC 优化流水线 / Apply MAC optimization pipeline
///
/// 根据流水线模式构建上下文和聚合，依次执行约束限制步骤。
/// Builds context and aggregation based on pipeline mode, then executes constraint limit steps in order.
pub fn apply_mac_optimization_pipeline(
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    z: Option<usize>,
    mode: Demo2PipelineMode,
) -> Result<(), Box<dyn Error>> {
    let context = MacOptimizationContext {
        request,
        x_idx,
        z,
        mode,
    };
    let aggregation = MacOptimizationAggregation::from_context(&context);
    for step in pipeline_list_generator::pipeline_steps(context.mode) {
        step(model, &context, &aggregation)?;
    }
    Ok(())
}
