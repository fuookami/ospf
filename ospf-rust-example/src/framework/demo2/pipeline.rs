use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework_demo::demo2::infrastructure::dto::Demo2Request;

#[allow(dead_code)]
pub type Demo2PipelineMode =
    crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

#[allow(dead_code)]
pub fn apply_domain_pipeline(
    mode: Demo2PipelineMode,
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    z: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    crate::framework_demo::demo2::domain::service::domain_pipeline::apply_domain_pipeline(
        mode, model, request, x_idx, z,
    )
}
