use std::error::Error;

use ospf_rust_core::model::MetaModel;

use crate::framework::demo2::domain::airworthiness::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

mod limits;
mod policy;
pub(crate) mod pipeline_list_generator;

pub fn apply_airworthiness_pipeline(
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    mode: Demo2PipelineMode,
) -> Result<(), Box<dyn Error>> {
    let context = AirworthinessContext {
        request,
        x_idx,
        mode,
    };
    let aggregation = AirworthinessAggregation::from_context(&context);
    for step in pipeline_list_generator::pipeline_steps(context.mode) {
        step(model, &context, &aggregation)?;
    }
    Ok(())
}
