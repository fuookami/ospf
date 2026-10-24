//! 轨迹类算法组合 / Trajectory algorithm compositions
//!
//! 轨迹类算法只有一个状态个体，全部差异落在「邻域算子 + 接受判据」这两个算子上。
//! Trajectory algorithms have a single state individual; all the difference lives in the two operators "neighbourhood" and
//! "acceptance".

use crate::solver::heuristic::contract::RunnerSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::acceptance::{
    Acceptance, AlwaysAcceptance, BetterOnlyAcceptance, MetropolisAcceptance, NotWorseAcceptance,
};
use crate::solver::heuristic::operator::neighborhood::Neighborhood;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::runner::runner_core::RunnerCore;
use crate::solver::heuristic::runner::trajectory_runner::TrajectoryRunner;

/// 轨迹类求解器类型别名 / Trajectory solver type alias
pub type TrajectorySolver<V> = RunnerSolver<TrajectoryRunner<V>>;

fn trajectory_solver<V>(
    name: &str,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    neighborhood: Box<dyn Neighborhood<V>>,
    acceptance: Box<dyn Acceptance<V, V>>,
) -> TrajectorySolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(
        Box::new(crate::solver::heuristic::operator::archive::BestArchive::new(1)),
        stopping,
    );
    RunnerSolver::new(
        name,
        TrajectoryRunner::new(neighborhood, acceptance, entropy).with_core(core),
    )
}

/// 爬山算法：轨迹骨架 + `BetterOnlyAcceptance` / Hill climbing: trajectory skeleton + `BetterOnlyAcceptance`
pub fn hill_climbing<V>(
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    neighborhood: Box<dyn Neighborhood<V>>,
) -> TrajectorySolver<V>
where
    V: HeuristicValue,
{
    trajectory_solver(
        "hill-climbing",
        stopping,
        entropy,
        neighborhood,
        Box::new(BetterOnlyAcceptance),
    )
}

/// 邻域搜索：轨迹骨架 + `NotWorseAcceptance` / Neighbourhood search: trajectory skeleton + `NotWorseAcceptance`
pub fn neighborhood_search<V>(
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    neighborhood: Box<dyn Neighborhood<V>>,
) -> TrajectorySolver<V>
where
    V: HeuristicValue,
{
    trajectory_solver(
        "ns",
        stopping,
        entropy,
        neighborhood,
        Box::new(NotWorseAcceptance),
    )
}

/// 随机游走：轨迹骨架 + `AlwaysAcceptance` / Random walk: trajectory skeleton + `AlwaysAcceptance`
pub fn random_walk<V>(
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    neighborhood: Box<dyn Neighborhood<V>>,
) -> TrajectorySolver<V>
where
    V: HeuristicValue,
{
    trajectory_solver(
        "random-walk",
        stopping,
        entropy,
        neighborhood,
        Box::new(AlwaysAcceptance),
    )
}

/// 模拟退火：轨迹骨架 + `MetropolisAcceptance` / Simulated annealing: trajectory skeleton + `MetropolisAcceptance`
pub fn simulated_annealing<V>(
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    neighborhood: Box<dyn Neighborhood<V>>,
) -> TrajectorySolver<V>
where
    V: HeuristicValue,
{
    trajectory_solver(
        "simulated-annealing",
        stopping,
        entropy,
        neighborhood,
        Box::new(MetropolisAcceptance::default()),
    )
}
