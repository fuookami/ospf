//! 适航性安全上下文 / Airworthiness security context
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

/// 适航性安全上下文 / Airworthiness security context
///
/// 持有适航性约束计算所需的请求引用、变量索引和管道模式。
/// Holds the request reference, variable indices, and pipeline mode
/// needed for airworthiness constraint computation.
pub struct AirworthinessContext<'a> {
    /// 装载请求 / Loading request
    pub request: &'a Demo2Request,
    /// 决策变量索引矩阵 x[c][p] / Decision variable index matrix x[c][p]
    pub x_idx: &'a [Vec<usize>],
    /// 管道模式 / Pipeline mode
    pub mode: Demo2PipelineMode,
    /// 压舱物重量变量索引（可选） / Ballast weight variable index (optional)
    pub ballast_weight_idx: Option<usize>,
    /// 装载重量变量索引（可选） / Load weight variable indices (optional)
    pub load_weight_idx: Option<Vec<usize>>,
}

impl<'a> AirworthinessContext<'a> {
    /// 创建新的适航性上下文 / Create a new airworthiness context
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

    /// 设置压舱物重量变量索引 / Set ballast weight variable index
    pub fn with_ballast_weight(mut self, idx: usize) -> Self {
        self.ballast_weight_idx = Some(idx);
        self
    }

    /// 设置装载重量变量索引 / Set load weight variable indices
    pub fn with_load_weight(mut self, idx: Vec<usize>) -> Self {
        self.load_weight_idx = Some(idx);
        self
    }
}
