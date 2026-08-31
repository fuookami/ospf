//! 管线入口 / Pipeline entry point
use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

/// Demo2 流水线模式类型别名 / Demo2 pipeline mode type alias
#[allow(dead_code)]
pub type Demo2PipelineMode =
    crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

/// 应用领域流水线约束到元模型 / Apply domain pipeline constraints to the meta model
#[allow(dead_code)]
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
    crate::framework::demo2::domain::service::domain_pipeline::apply_domain_pipeline(
        mode, model, request, x_idx, z,
        estimate_load_weight_idx, estimate_loaded_idx, loaded_idx,
    )
}
