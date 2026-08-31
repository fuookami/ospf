use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

pub struct ExpressEffectivenessContext<'a> {
    pub request: &'a Demo2Request,
    pub x_idx: &'a [Vec<usize>],
    pub mode: Demo2PipelineMode,
}
