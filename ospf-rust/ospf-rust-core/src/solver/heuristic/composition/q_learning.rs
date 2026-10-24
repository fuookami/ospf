//! Q 学习驱动的种群搜索 / Q-learning driven population search
//!
//! 骨架拿到一个候选解更新算子池，Q 学习按「多样性 × 停滞度」的态势在池子里挑一个来执行本代更新：探索期偏向幅度大的
//! 更新，收敛期偏向精细更新。奖励来自档案的真实改进，而不是 Q 表自转。
//! The skeleton is given a pool of candidate solution updates and Q-learning picks one per generation according to the
//! "diversity × stagnation" situation: wide moves while exploring, fine moves while converging. The reward comes from the
//! archive's real improvement rather than the table feeding itself.

use std::cell::RefCell;
use std::rc::Rc;

use crate::solver::heuristic::composition::population::PopulationSolver;
use crate::solver::heuristic::contract::RunnerSolver;
use crate::solver::heuristic::entropy::{Entropy, SharedEntropy};
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::initializer::ModelInitializer;
use crate::solver::heuristic::operator::mutation::NoMutation;
use crate::solver::heuristic::operator::rl::{
    QlearningAgent, QlearningObserver, QlearningUpdate,
};
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::operator::survivor::ElitistSurvivor;
use crate::solver::heuristic::operator::topology::GlobalTopology;
use crate::solver::heuristic::operator::update::SolutionUpdate;
use crate::solver::heuristic::runner::population_runner::PopulationRunner;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 组装 Q 学习驱动的种群搜索 / Assemble a Q-learning driven population search
pub fn q_learning<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    candidates: Vec<Box<dyn SolutionUpdate<V>>>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    let shared = Rc::new(RefCell::new(entropy));
    let agent = Rc::new(RefCell::new(QlearningAgent::<V>::new(
        candidates.len(),
        Box::new(SharedEntropy::from_shared(Rc::clone(&shared))),
    )));
    let update = QlearningUpdate::new(candidates, Rc::clone(&agent));

    let mut core = RunnerCore::new(Box::new(BestArchive::new(1)), stopping);
    core.push_observer(Box::new(QlearningObserver::new(agent)));

    let runner = PopulationRunner::new(
        size,
        Box::new(update),
        Box::new(NoMutation),
        Box::new(ElitistSurvivor),
        Box::new(SharedEntropy::from_shared(shared)),
    )
    .with_mutation_rate(0.0)
    .with_initializer(Box::new(ModelInitializer))
    .with_topology(Box::new(GlobalTopology))
    .with_core(core);

    RunnerSolver::new("q-learning", runner)
}
