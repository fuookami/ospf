use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::airworthiness_security::service::apply_airworthiness_security_pipeline;
use crate::framework::demo2::domain::express_effectiveness::service::apply_express_effectiveness_pipeline;
use crate::framework::demo2::domain::loading_effectiveness::service::apply_loading_effectiveness_pipeline;
use crate::framework::demo2::domain::mac_optimization::service::apply_mac_optimization_pipeline;
use crate::framework::demo2::domain::redundancy::service::apply_redundancy_pipeline;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::soft_security::service::apply_soft_security_pipeline;
use crate::framework::demo2::domain::stowage::service::apply_stowage_pipeline;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

pub fn apply_domain_pipeline(
    mode: Demo2PipelineMode,
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    z: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    apply_stowage_pipeline(model, request, x_idx, mode)?;
    apply_airworthiness_security_pipeline(model, request, x_idx, mode)?;
    apply_mac_optimization_pipeline(model, request, x_idx, z, mode)?;
    apply_loading_effectiveness_pipeline(model, request, x_idx, mode)?;
    apply_express_effectiveness_pipeline(model, request, x_idx, mode)?;
    apply_soft_security_pipeline(model, request, x_idx, mode)?;
    apply_redundancy_pipeline(model, request, x_idx, mode)?;
    Ok(())
}
