//! 领域管线应用 / Domain pipeline application
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

/// 应用领域管线 / Apply domain pipeline
///
/// 按顺序执行所有领域约束管线：装载、适航、MAC优化、装载效能、快递效能、软安全、冗余。
/// Executes all domain constraint pipelines in order: stowage, airworthiness, MAC optimization, loading effectiveness, express effectiveness, soft security, redundancy.
pub fn apply_domain_pipeline(
    mode: Demo2PipelineMode,
    model: &mut MetaModel<f64>,
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    z: Option<usize>,
    estimate_load_weight_idx: &[usize],
    estimate_loaded_idx: &[usize],
    loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    apply_stowage_pipeline(model, request, x_idx, mode, loaded_idx, estimate_loaded_idx)?;
    apply_airworthiness_security_pipeline(
        model, request, x_idx, mode,
        estimate_load_weight_idx, estimate_loaded_idx,
    )?;
    apply_mac_optimization_pipeline(model, request, x_idx, z, mode)?;
    apply_loading_effectiveness_pipeline(model, request, x_idx, mode)?;
    apply_express_effectiveness_pipeline(model, request, x_idx, mode)?;
    apply_soft_security_pipeline(model, request, x_idx, mode)?;
    apply_redundancy_pipeline(model, request, x_idx, mode)?;
    Ok(())
}
