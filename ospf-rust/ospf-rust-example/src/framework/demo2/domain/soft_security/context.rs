//! 软性安全上下文 / Soft security context
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

/// 软安全上下文 / Soft security context
///
/// 持有请求引用、决策变量索引和管线模式。
/// Holds request reference, decision variable indices, and pipeline mode.
pub struct SoftSecurityContext<'a> {
    /// 请求引用 / Request reference
    pub request: &'a Demo2Request,
    /// 决策变量索引 x[c][p] / Decision variable indices x[c][p]
    pub x_idx: &'a [Vec<usize>],
    /// 管线模式 / Pipeline mode
    pub mode: Demo2PipelineMode,
}
