//! 适航安全领域服务 / Airworthiness security domain service
//!
//! 提供适航性约束的应用管道，包括各种限制条件的注册和执行。
//! Provides the airworthiness constraint application pipeline,
//! including registration and execution of various limit conditions.
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;
use ospf_rust_core::model::MetaModel;
use std::error::Error;

mod limits;
pub(crate) mod pipeline_list_generator;
mod policy;

/// 应用适航性安全管道 / Apply airworthiness security pipeline
///
/// 根据管道模式依次执行所有适航性约束步骤，将约束注册到模型中。
/// Executes all airworthiness constraint steps in order based on the pipeline mode,
/// registering constraints into the model.
pub fn apply_airworthiness_security_pipeline(
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    mode: Demo2PipelineMode,
    estimate_load_weight_idx: &[usize],
    estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    let context = AirworthinessContext::new(request, x_idx, mode);
    let aggregation = AirworthinessAggregation::from_context(&context);
    for step in pipeline_list_generator::pipeline_steps(context.mode) {
        step(
            model,
            &context,
            &aggregation,
            estimate_load_weight_idx,
            estimate_loaded_idx,
        )?;
    }
    Ok(())
}
