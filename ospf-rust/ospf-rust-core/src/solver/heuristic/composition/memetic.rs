//! Memetic 组合 / Memetic composition
//!
//! Memetic 把局部搜索当作个体的一次「学习」：先跑全局求解器得到一个好解，再以该解为初始点、用轨迹骨架在邻域内精修。
//! 两阶段共用同一套算子，因此它本身不引入新的搜索机制，只是**调度**。
//! Memetic treats local search as an individual's learning step: run a global solver, then refine its best solution with a
//! trajectory skeleton. Both phases share the same operators, so no new search mechanism is introduced — only scheduling.

use std::cell::RefCell;
use std::rc::Rc;

use crate::solver::heuristic::contract::{HeuristicSolver, RunnerSolver};
use crate::solver::heuristic::entropy::{Entropy, SharedEntropy};
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel, to_f64};
use crate::solver::heuristic::operator::acceptance::{Acceptance, NotWorseAcceptance};
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::compare_strictly;
use crate::solver::heuristic::operator::initializer::FixedInitializer;
use crate::solver::heuristic::operator::neighborhood::{Neighborhood, RandomPerturbation};
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::result::{HeuristicResult, Result};
use crate::solver::heuristic::runner::runner_core::RunnerCore;
use crate::solver::heuristic::runner::trajectory_runner::TrajectoryRunner;

/// 局部精修阶段配置 / Local refinement phase configuration
///
/// 由于算子被局部阶段独占，这里保存的是构造器而不是实例，保证同一个 memetic 求解器可以重复使用。
/// Because operators are owned by the local phase, constructors are stored instead of instances so one memetic solver stays
/// reusable.
pub struct MemeticConfig<V>
where
    V: HeuristicValue,
{
    /// 局部阶段终止判据构造器 / Local-phase stopping constructor
    pub polish_stopping: Box<dyn Fn() -> Box<dyn StoppingCriterion>>,
    /// 邻域算子构造器 / Neighbourhood constructor
    pub neighborhood: Box<dyn Fn() -> Box<dyn Neighborhood<V>>>,
    /// 接受判据构造器 / Acceptance constructor
    pub acceptance: Box<dyn Fn() -> Box<dyn Acceptance<V, V>>>,
}

impl<V> MemeticConfig<V>
where
    V: HeuristicValue,
{
    /// 以局部终止判据创建配置，默认邻域均匀扰动、接受判据「不劣即接受」 / Create a configuration with the local stopping criterion, defaulting to uniform perturbation and not-worse acceptance
    pub fn new(
        polish_stopping: Box<dyn Fn() -> Box<dyn StoppingCriterion>>,
    ) -> Self {
        Self {
            polish_stopping,
            neighborhood: Box::new(|| Box::new(RandomPerturbation::default())),
            acceptance: Box::new(|| Box::new(NotWorseAcceptance)),
        }
    }
}

/// Memetic 求解器：全局搜索 + 局部精修 / Memetic solver: global search plus local refinement
pub struct MemeticSolver<V, G>
where
    V: HeuristicValue,
    G: HeuristicSolver<V, V>,
{
    name: String,
    global: G,
    config: MemeticConfig<V>,
    entropy: Rc<RefCell<Box<dyn Entropy>>>,
}

impl<V, G> MemeticSolver<V, G>
where
    V: HeuristicValue,
    G: HeuristicSolver<V, V>,
{
    /// 创建 memetic 求解器 / Create a memetic solver
    pub fn new(
        name: impl Into<String>,
        global: G,
        config: MemeticConfig<V>,
        entropy: Box<dyn Entropy>,
    ) -> Self {
        Self {
            name: name.into(),
            global,
            config,
            entropy: Rc::new(RefCell::new(entropy)),
        }
    }

    /// 全局阶段求解器引用 / Global-phase solver reference
    pub fn global(&self) -> &G {
        &self.global
    }
}

impl<V, G> HeuristicSolver<V, V> for MemeticSolver<V, G>
where
    V: HeuristicValue,
    G: HeuristicSolver<V, V>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn solve<M>(
        &mut self,
        model: &mut M,
        iteration: &mut Iteration,
    ) -> Result<HeuristicResult<V, V>>
    where
        M: ObjectiveModel<V, V>,
    {
        let global_result = self.global.solve(model, iteration)?;
        let best = match &global_result.best_solution {
            Some(best) => best,
            None => return Ok(global_result),
        };

        let order = crate::solver::heuristic::model::VariableModel::variable_order(model);
        let values: Vec<f64> = order
            .iter()
            .map(|id| best.get(id).map(to_f64).unwrap_or(f64::NAN))
            .collect();

        let core = RunnerCore::new(
            Box::new(BestArchive::new(1)),
            (self.config.polish_stopping)(),
        );
        let runner = TrajectoryRunner::new(
            (self.config.neighborhood)(),
            (self.config.acceptance)(),
            Box::new(SharedEntropy::from_shared(Rc::clone(&self.entropy))),
        )
        .with_initializer(Box::new(FixedInitializer::new(values)))
        .with_core(core);
        let mut polish = RunnerSolver::new("memetic-polish", runner);

        match polish.solve(model, iteration) {
            Ok(polish_result) => {
                let better = match (&polish_result.best_objective, &global_result.best_objective) {
                    (None, _) => false,
                    (Some(_), None) => true,
                    (Some(candidate), Some(incumbent)) => {
                        compare_strictly(model, candidate, incumbent)
                            == std::cmp::Ordering::Less
                    }
                };
                if better {
                    Ok(polish_result)
                } else {
                    Ok(global_result)
                }
            }
            // 局部阶段失败不应掩盖全局阶段的有效结果。
            // A local-phase failure must not hide the valid global result.
            Err(_) => Ok(global_result),
        }
    }
}

/// 组装「全局种群搜索 + 局部轨迹精修」 / Compose "global population search plus local trajectory refinement"
pub fn memetic<V, G>(
    global: G,
    config: MemeticConfig<V>,
    entropy: Box<dyn Entropy>,
) -> MemeticSolver<V, G>
where
    V: HeuristicValue,
    G: HeuristicSolver<V, V>,
{
    let name = format!("memetic-{}", global.name());
    MemeticSolver::new(name, global, config, entropy)
}
