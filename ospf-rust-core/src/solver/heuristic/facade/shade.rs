//! SHADE 与 L-SHADE 入口 / SHADE and L-SHADE entries

use crate::solver::heuristic::contract::RunnerSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::evolution::{LinearSource, ParameterSource};
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::runner::runner_core::RunnerCore;
use crate::solver::heuristic::runner::shade_runner::{ShadeConfig, ShadeRunner};

/// SHADE 求解器类型别名 / SHADE solver type alias
pub type ShadeSolver<V> = RunnerSolver<ShadeRunner<V>>;

/// SHADE：成功历史自适应记忆驱动 `F`、`CR` 的采样 / SHADE: success-history adaptive memory drives the sampling of `F` and `CR`
pub fn shade<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> ShadeSolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(Box::new(BestArchive::new(1)), stopping);
    RunnerSolver::new(
        "shade",
        ShadeRunner::new(ShadeConfig::new(size), entropy).with_core(core),
    )
}

/// L-SHADE：种群规模随迭代线性缩减的 SHADE / L-SHADE: SHADE with a linearly shrinking population
pub fn l_shade<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> ShadeSolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(Box::new(BestArchive::new(1)), stopping);
    let config = ShadeConfig {
        size_ratio: Box::new(LinearSource::new(1.0, 0.05, 100)) as Box<dyn ParameterSource>,
        minimum_size: 4,
        ..ShadeConfig::new(size)
    };
    RunnerSolver::new(
        "l-shade",
        ShadeRunner::new(config, entropy).with_core(core),
    )
}
