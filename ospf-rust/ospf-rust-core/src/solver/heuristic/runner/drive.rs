//! 统一生命周期循环 / Unified lifecycle loop
//!
//! 所有骨架共用同一个循环：先取初始状态并刷新档案，然后「推进 → 更新档案 → 广播 → 刷新模型 → 推进迭代」。
//! 循环本身不理解任何算法，算法只在 `advance` 回调里出现。
//! Every skeleton shares one loop: take the initial state and refresh the archive, then "advance → update the archive →
//! broadcast → flush the model → advance the iteration". The loop knows no algorithm; algorithms only appear in the
//! `advance` callback.

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 驱动一次完整搜索 / Drive one complete search
///
/// `advance` 负责推进一代，`members` 把状态摊平为个体列表用于档案更新与观测广播。
/// `advance` performs one generation, and `members` flattens the state into individuals for archive updates and observer
/// broadcasts.
pub fn drive<S, O, V, M>(
    core: &mut RunnerCore<O, V>,
    model: &mut M,
    iteration: &mut Iteration,
    state: S,
    mut advance: impl FnMut(&mut M, &mut Iteration, S, &mut dyn Entropy) -> Result<S>,
    members: impl Fn(&S) -> Vec<Individual<O, V>>,
    entropy: &mut dyn Entropy,
) -> Result<Vec<Individual<O, V>>>
where
    M: ObjectiveModel<O, V>,
    O: Clone,
    V: HeuristicValue,
{
    core.archive.update(model, members(&state));
    iteration.next(true);

    let mut state = state;
    while !core.stopping.should_stop(iteration) {
        state = advance(model, iteration, state, entropy)?;
        let improved = core.archive.update(model, members(&state));
        for observer in core.observers.iter_mut() {
            observer.observe(iteration, &members(&state), improved);
        }
        model.flush();
        iteration.next(improved);
    }

    Ok(core.archive.individuals().to_vec())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::model::callback::AbstractCallBackModel;
    use crate::solver::heuristic::entropy::{Entropy, SeededEntropy};
    use crate::solver::heuristic::individual::Individual;
    use crate::solver::heuristic::iteration::Iteration;
    use crate::solver::heuristic::operator::archive::BestArchive;
    use crate::solver::heuristic::operator::stopping::IterationLimit;
    use crate::solver::heuristic::runner::iteration_observer::IterationObserver;
    use crate::variable::new_standalone_id;

    use super::{RunnerCore, drive};

    struct RecordingObserver {
        improved: Arc<Mutex<Vec<bool>>>,
    }

    impl IterationObserver<f64, f64> for RecordingObserver {
        fn observe(
            &mut self,
            _iteration: &Iteration,
            _members: &[Individual<f64, f64>],
            improved: bool,
        ) {
            self.improved.lock().unwrap().push(improved);
        }
    }

    #[test]
    fn drive_updates_archive_and_broadcasts_improvement() {
        let x = new_standalone_id();
        let mut model = AbstractCallBackModel::<f64>::new("drive");
        model.register_variable_name(x, "x".to_string());
        model.set_objective_evaluator(Arc::new(|solution| {
            Some(solution.values().copied().sum())
        }));

        let recorded = Arc::new(Mutex::new(Vec::new()));
        let mut core = RunnerCore::new(
            Box::new(BestArchive::new(1)),
            Box::new(IterationLimit::new(3)),
        );
        core.push_observer(Box::new(RecordingObserver {
            improved: recorded.clone(),
        }));

        let initial = vec![Individual::new(
            std::collections::HashMap::from([(x, 10.0)]),
            10.0,
        )];
        let mut iteration = Iteration::new();
        let mut entropy = SeededEntropy::new(7);
        let result = drive(
            &mut core,
            &mut model,
            &mut iteration,
            initial,
            |_model, _iteration, state, _entropy: &mut dyn Entropy| {
                let next_value = state[0].fitness - 1.0;
                Ok(vec![Individual::new(
                    std::collections::HashMap::from([(x, next_value)]),
                    next_value,
                )])
            },
            |state| state.clone(),
            &mut entropy,
        )
        .expect("drive should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fitness, 8.0);
        // 初始更新 + 两次推进 = 3 次迭代；观测者在两次推进后各收到一次信号。
        // Initial update plus two advances = 3 iterations; the observer receives one signal per advance.
        assert_eq!(iteration.iteration(), 3);
        assert_eq!(*recorded.lock().unwrap(), vec![true, true]);
    }
}
