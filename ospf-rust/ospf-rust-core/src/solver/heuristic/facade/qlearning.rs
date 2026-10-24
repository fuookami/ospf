//! Q 学习入口 / Q-learning entry

use crate::solver::heuristic::composition::population::PopulationSolver;
use crate::solver::heuristic::composition::q_learning as compose;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::operator::update::{GreyWolfUpdate, SineCosineUpdate};

/// Q 学习驱动的种群搜索，默认在灰狼更新与正弦余弦更新之间选择 / Q-learning driven population search choosing between grey-wolf and sine-cosine updates by default
pub fn q_learning<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    compose(
        size,
        stopping,
        entropy,
        vec![
            Box::new(GreyWolfUpdate::default()),
            Box::new(SineCosineUpdate::default()),
        ],
    )
}
