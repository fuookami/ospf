//! 按编码隔离的小动作生态位组合 / Small niche-based action portfolios isolated by encoding

use crate::solver::heuristic::result::Result;
use super::config::{AlgorithmConfig, ComponentSpec};
use super::invalid;

/// 避免重复大动作集的正交生态位配置 / Orthogonal niches avoiding redundant large action sets
pub struct OrthogonalOperatorPortfolio;

impl OrthogonalOperatorPortfolio {
    /// 连续五生态位：分布、自适应差分、围捕、逃逸、局部精调 / Five continuous niches: distribution, adaptive DE, encircling, escape, intensification
    pub fn continuous(size: usize) -> Result<Vec<AlgorithmConfig>> {
        if size < 4 { return Err(invalid("orthogonal continuous portfolio requires at least four individuals")); }
        let mut distribution = AlgorithmConfig::new("distribution", "cma-es");
        distribution.size = size;
        distribution.algorithm = distribution.algorithm.parameter("restartOnImport", true);
        let mut difference = AlgorithmConfig::new("adaptive-difference", "l-shade"); difference.size = size;
        let mut encircling = AlgorithmConfig::new("encircling", "gwo"); encircling.size = size;
        encircling.updates = ["hho", "gwo"].map(ComponentSpec::new).to_vec();
        encircling.mutations = vec![ComponentSpec::new("none")];
        encircling.selection = ComponentSpec::new("frr-mab");
        let mut escape = AlgorithmConfig::new("escape", "genetic"); escape.size = size;
        escape.updates = ["opposition", "de"].map(ComponentSpec::new).to_vec();
        escape.mutations = vec![ComponentSpec::new("levy")];
        escape.selection = ComponentSpec::new("frr-mab");
        let mut intensification = AlgorithmConfig::new("intensification", "genetic"); intensification.size = 1;
        intensification.updates = vec![ComponentSpec::new("pattern-search")];
        intensification.mutations = vec![ComponentSpec::new("none")];
        Ok(vec![distribution, difference, encircling, escape, intensification])
    }

    /// 排列专用保序交叉及反转/交换；初始合法性仍由业务模型负责 / Permutation-only order crossover and inversion/swap; the domain model remains responsible for valid initialization
    pub fn permutation(size: usize) -> Result<Vec<AlgorithmConfig>> {
        if size == 0 { return Err(invalid("invalid permutation portfolio size")); }
        let mut config = AlgorithmConfig::new("permutation", "genetic"); config.size = size;
        config.updates = vec![ComponentSpec::new("order")];
        config.mutations = ["inversion", "swap"].map(ComponentSpec::new).to_vec();
        config.selection = ComponentSpec::new("frr-mab");
        Ok(vec![config])
    }
}
