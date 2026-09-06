//! 重心优化上下文 / MAC optimization context
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

/// MAC 优化上下文 / MAC optimization context
///
/// 包含 MAC 优化所需的请求数据、变量索引和流水线模式。
/// Contains request data, variable indices, and pipeline mode needed for MAC optimization.
pub struct MacOptimizationContext<'a> {
    /// 优化请求数据 / Optimization request data
    pub request: &'a Demo2Request,
    /// 货物-位置决策变量索引 / Cargo-position decision variable indices
    pub x_idx: &'a [Vec<usize>],
    /// 最大载荷偏差变量索引 / Maximum load deviation variable index
    pub z: Option<usize>,
    /// 流水线模式 / Pipeline mode
    pub mode: Demo2PipelineMode,
}
