use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework_demo::demo2::infrastructure::dto::Demo2Request;

pub struct StowageContext<'a> {
    pub request: &'a Demo2Request,
    pub x_idx: &'a [Vec<usize>],
    pub mode: Demo2PipelineMode,
}
