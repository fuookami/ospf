//! 邻域搜索入口 / Neighbourhood search entry

use crate::solver::heuristic::composition::neighborhood_search as compose;
use crate::solver::heuristic::composition::TrajectorySolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::neighborhood::RandomPerturbation;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;

/// 邻域搜索，即轨迹骨架 + `NotWorseAcceptance` / Neighbourhood search: trajectory skeleton + `NotWorseAcceptance`
pub fn neighborhood_search<V>(
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> TrajectorySolver<V>
where
    V: HeuristicValue,
{
    compose(
        stopping,
        entropy,
        Box::new(RandomPerturbation::default()),
    )
}
