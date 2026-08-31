//! 软安全领域服务 / Soft security domain service.
use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

pub mod limits;
pub(crate) mod pipeline_list_generator;
mod policy;

/// 应用软安全管线 / Apply soft security pipeline
///
/// 根据管线模式依次执行软安全约束步骤。
/// Executes soft security constraint steps sequentially based on pipeline mode.
pub fn apply_soft_security_pipeline(
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    mode: Demo2PipelineMode,
) -> Result<(), Box<dyn Error>> {
    let context = SoftSecurityContext {
        request,
        x_idx,
        mode,
    };
    let mut aggregation = SoftSecurityAggregation::from_context(&context);
    for step in pipeline_list_generator::pipeline_steps(context.mode) {
        step(model, &context, &mut aggregation)?;
    }
    Ok(())
}
