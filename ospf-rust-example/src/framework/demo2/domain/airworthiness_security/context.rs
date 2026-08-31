use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

pub struct AirworthinessContext<'a> {
    pub request: &'a Demo2Request,
    pub x_idx: &'a [Vec<usize>],
    pub mode: Demo2PipelineMode,
    /// 压舱物重量变量索引 (可选)
    pub ballast_weight_idx: Option<usize>,
    /// 装载重量变量索引 (可选)
    pub load_weight_idx: Option<Vec<usize>>,
}

impl<'a> AirworthinessContext<'a> {
    pub fn new(
        request: &'a Demo2Request,
        x_idx: &'a [Vec<usize>],
        mode: Demo2PipelineMode,
    ) -> Self {
        Self {
            request,
            x_idx,
            mode,
            ballast_weight_idx: None,
            load_weight_idx: None,
        }
    }

    pub fn with_ballast_weight(mut self, idx: usize) -> Self {
        self.ballast_weight_idx = Some(idx);
        self
    }

    pub fn with_load_weight(mut self, idx: Vec<usize>) -> Self {
        self.load_weight_idx = Some(idx);
        self
    }
}
