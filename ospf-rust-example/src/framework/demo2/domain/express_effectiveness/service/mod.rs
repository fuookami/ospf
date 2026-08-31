use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework_demo::demo2::domain::express_effectiveness::aggregation::ExpressEffectivenessAggregation;
use crate::framework_demo::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework_demo::demo2::infrastructure::dto::Demo2Request;

mod limits;
pub(crate) mod pipeline_list_generator;
mod policy;

pub fn apply_express_effectiveness_pipeline(
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    mode: Demo2PipelineMode,
) -> Result<(), Box<dyn Error>> {
    let context = ExpressEffectivenessContext {
        request,
        x_idx,
        mode,
    };
    let aggregation = ExpressEffectivenessAggregation::from_context(&context);
    for step in pipeline_list_generator::pipeline_steps(context.mode) {
        step(model, &context, &aggregation)?;
    }
    Ok(())
}
