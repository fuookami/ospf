//! 快递效能上下文 / Express effectiveness context
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

/// 快递效能上下文 / Express effectiveness context
pub struct ExpressEffectivenessContext<'a> {
    /// 请求数据 / Request data
    pub request: &'a Demo2Request,
    /// 决策变量索引 x[cargo][position] / Decision variable indices x[cargo][position]
    pub x_idx: &'a [Vec<usize>],
    /// 流水线模式 / Pipeline mode
    pub mode: Demo2PipelineMode,
}
