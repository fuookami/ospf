use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

mod limits;
pub(crate) mod pipeline_list_generator;
mod policy;

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
