//! 装载领域服务 / Stowage domain service.
use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

mod limits;
pub(crate) mod pipeline_list_generator;
mod policy;

/// 应用装载管线约束 / Apply stowage pipeline constraints
pub fn apply_stowage_pipeline(
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    mode: Demo2PipelineMode,
    loaded_idx: &[usize],
    estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    let context = StowageContext {
        request,
        x_idx,
        mode,
    };
    let aggregation = StowageAggregation::from_context(&context);
    for step in pipeline_list_generator::pipeline_steps(context.mode) {
        step(model, &context, &aggregation, loaded_idx, estimate_loaded_idx)?;
    }
    Ok(())
}
