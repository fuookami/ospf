//! 并行组合模式定义
//! Parallel Combinatorial Mode Definitions

/// 并行组合模式 / Parallel Combinatorial Mode
///
/// 定义多个求解器并行执行时的结果选择策略。
/// Defines result selection strategy when multiple solvers execute in parallel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParallelCombinatorialMode {
    /// 返回第一个成功结果 / Return first successful result
    ///
    /// 当任何一个求解器成功找到解时，立即返回该结果并取消其他求解器。
    /// When any solver successfully finds a solution, immediately return that result and cancel other solvers.
    First,

    /// 返回最优结果（默认）/ Return best result (default)
    ///
    /// 等待所有求解器完成，然后返回最优解。
    /// Wait for all solvers to complete, then return the best solution.
    #[default]
    Best,
}

impl ParallelCombinatorialMode {
    /// 是否为 First 模式 / Whether it's First mode
    pub fn is_first(&self) -> bool {
        matches!(self, Self::First)
    }

    /// 是否为 Best 模式 / Whether it's Best mode
    pub fn is_best(&self) -> bool {
        matches!(self, Self::Best)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        let mode = ParallelCombinatorialMode::default();
        assert!(mode.is_best());
        assert!(!mode.is_first());
    }

    #[test]
    fn test_first_mode() {
        let mode = ParallelCombinatorialMode::First;
        assert!(mode.is_first());
        assert!(!mode.is_best());
    }
}
