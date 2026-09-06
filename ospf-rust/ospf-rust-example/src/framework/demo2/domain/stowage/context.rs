//! 装载上下文 / Stowage context
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

/// 装载上下文 / Stowage context
pub struct StowageContext<'a> {
    /// 请求引用 / Request reference
    pub request: &'a Demo2Request,
    /// 决策变量索引矩阵 / Decision variable index matrix
    pub x_idx: &'a [Vec<usize>],
    /// 管线模式 / Pipeline mode
    pub mode: Demo2PipelineMode,
}
