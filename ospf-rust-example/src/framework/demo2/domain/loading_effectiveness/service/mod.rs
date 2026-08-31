//! 装载效能领域服务 / Loading effectiveness domain service.
use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

mod limits;
pub(crate) mod pipeline_list_generator;
mod policy;

/// 应用装载效能流水线 / Apply loading effectiveness pipeline
///
/// 根据流水线模式构建上下文和聚合数据，依次执行各约束和目标注册步骤。
pub fn apply_loading_effectiveness_pipeline(
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    mode: Demo2PipelineMode,
) -> Result<(), Box<dyn Error>> {
    let context = LoadingEffectivenessContext {
        request,
        x_idx,
        mode,
    };
    let aggregation = LoadingEffectivenessAggregation::from_context(&context);
    for step in pipeline_list_generator::pipeline_steps(context.mode) {
        step(model, &context, &aggregation)?;
    }
    Ok(())
}
