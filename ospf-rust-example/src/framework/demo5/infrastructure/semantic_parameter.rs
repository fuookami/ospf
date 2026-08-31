//! Demo5 语义参数 / Demo5 semantic parameters.

use std::time::Duration;

/// 统一封装 demo5 的求解参数 / Encapsulate demo5 solving parameters.
#[derive(Debug, Clone)]
pub struct SemanticParameter {
    /// 时间上限 / Time limit.
    pub time_limit: Option<Duration>,
    /// 节点上限 / Node limit.
    pub node_limit: usize,
    /// 相对 gap 容差 / Relative gap tolerance.
    pub relative_gap_tolerance: f64,
    /// 每节点最大列生成迭代次数 / Maximum column-generation iterations per node.
    pub max_cg_iterations_per_node: usize,
}

impl Default for SemanticParameter {
    fn default() -> Self {
        Self {
            time_limit: Some(Duration::from_secs(30)),
            node_limit: 10_000,
            relative_gap_tolerance: 1e-4,
            max_cg_iterations_per_node: 1_000,
        }
    }
}
