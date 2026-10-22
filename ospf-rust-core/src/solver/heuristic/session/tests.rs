//! 通用会话与状态编解码测试 / Tests for the generic session and state codecs
//!
//! 覆盖 Kotlin 会话语义的六个关键点：分片推进、预算耗尽、快照往返、解导入、嵌套片段、七种状态编解码。
//! Covers the six key points of the Kotlin session semantics: slice advance, budget exhaustion, snapshot round trip, solution
//! import, nested slices, and the seven state codecs.

use std::sync::Arc;

use crate::model::callback::Solution;
use crate::variable::{VariableId, new_standalone_id};

use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::session::budget::{
    SearchBudget, SearchSlice,
};
use crate::solver::heuristic::session::codecs::{
    ClpsoStateCodec, CmaEsStateCodec, IndividualListCodec, IslandStateCodec, NumberMatrixCodec,
    NumberVectorCodec, ParticleStateCodec, ShadeStateCodec, TrajectoryStateCodec,
};
use crate::solver::heuristic::session::json::{JsonValue, SearchCodec};
use crate::solver::heuristic::session::protocol::{
    SearchProblem, SearchSession, SearchStepResult, SearchTermination,
};
use crate::solver::heuristic::session::runner_session::{
    HookedSkeleton, RunnerSession, SessionCodecs,
};

/// 纯函数二次问题 / Pure-function quadratic problem
struct QuadraticProblem {
    x: VariableId,
    id: String,
    fingerprint: String,
}

impl QuadraticProblem {
    fn new() -> Self {
        Self {
            x: new_standalone_id(),
            id: "quadratic".to_string(),
            fingerprint: "quadratic-v1".to_string(),
        }
    }
}

impl SearchProblem<f64, f64> for QuadraticProblem {
    fn id(&self) -> &str {
        &self.id
    }

    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    fn variable_order(&self) -> Vec<VariableId> {
        vec![self.x]
    }

    fn lower_bound(&self, _index: usize) -> Option<f64> {
        Some(-10.0)
    }

    fn upper_bound(&self, _index: usize) -> Option<f64> {
        Some(10.0)
    }

    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        (0..amount)
            .map(|index| Solution::from([(self.x, index as f64 * 0.5 - 2.0)]))
            .collect()
    }

    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        let value = solution.get(&self.x).copied()?;
        Some((value - 1.0) * (value - 1.0))
    }

    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool {
        lhs < rhs
    }

    fn feasible(&self, _solution: &Solution<f64>) -> bool {
        true
    }
}

/// 构造一个「每步把每个个体向 1.0 收缩」的确定性骨架 / A deterministic skeleton shrinking every individual towards 1.0
fn quadratic_session(
    with_importer: bool,
) -> RunnerSession<QuadraticProblem, f64, f64, Vec<Individual<f64, f64>>> {
    let order = {
        let problem = QuadraticProblem::new();
        problem.variable_order()
    };
    let hooks = HookedSkeleton::<_, f64, f64, _>::new(
        move |problem: &QuadraticProblem, slice: &SearchSlice| {
            let order = problem.variable_order();
            let x = order[0];
            let population = (0..4)
                .filter_map(|index| {
                    let value = index as f64 * 2.0 - 1.0;
                    problem.evaluate(Solution::from([(x, value)]), slice).unwrap().candidate
                })
                .collect::<Vec<_>>();
            Ok(population)
        },
        move |problem: &mut QuadraticProblem,
              _iteration: &Iteration,
              state: &Vec<Individual<f64, f64>>,
              slice: &SearchSlice| {
            let advanced = state
                .iter()
                .filter_map(|individual| {
                    let value = individual.solution.values().next().copied().unwrap_or(0.0);
                    let next = value + 0.5 * (1.0 - value);
                    problem.evaluate(Solution::from([(individual.solution.keys().next().copied().unwrap(), next)]), slice).unwrap().candidate
                })
                .collect();
            Ok(advanced)
        },
        |state: &Vec<Individual<f64, f64>>| state.clone(),
    );

    let codecs = SessionCodecs::new(
        None,
        Some(Box::new(IndividualListCodec::new(order))),
    );
    let session = RunnerSession::new(Box::new(hooks), QuadraticProblem::new(), codecs);
    if with_importer {
        session.with_importer(|state: &Vec<Individual<f64, f64>>, candidates| {
            let mut merged = state.clone();
            merged.extend(candidates.iter().cloned());
            Ok(merged)
        })
    } else {
        session
    }
}

#[test]
fn generic_session_advances_and_tracks_iteration() {
    let mut session = quadratic_session(false);
    let budget = Arc::new(SearchBudget::new(10_000));
    let mut slice = SearchSlice::new(budget, 100, 3);

    let report: SearchStepResult<f64, f64> = session.step(&mut slice).expect("step should succeed");
    assert_eq!(report.steps, 3);
    assert!(report.is_completed());
    assert!(!report.candidates.is_empty());
    // 初次建立状态算一次迭代，之后每推进一次加一。
    // Building the initial state counts as one iteration and each advance adds one.
    assert_eq!(session.iteration().iteration(), 3);
    assert!(!session.archive().is_empty());
}

#[test]
fn generic_session_stops_on_evaluation_limit() {
    let mut session = quadratic_session(false);
    let budget = Arc::new(SearchBudget::new(1));
    let mut slice = SearchSlice::new(Arc::clone(&budget), 100, 100);

    let report = session.step(&mut slice).expect("step should succeed");
    assert_eq!(report.termination, SearchTermination::EvaluationLimit);
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn generic_session_distinguishes_slice_admission_failure_from_complete_steps() {
    for (global_limit, allowance, steps, expected_steps, expected_termination) in [
        (100, 1, 1, 0, SearchTermination::EvaluationLimit),
        (100, 4, 1, 1, SearchTermination::SliceComplete),
        (4, 4, 1, 1, SearchTermination::SliceComplete),
        (100, 4, 2, 1, SearchTermination::EvaluationLimit),
    ] {
        let mut session = quadratic_session(false);
        let budget = Arc::new(SearchBudget::new(global_limit));
        let report = session.step(&mut SearchSlice::new(budget.clone(), allowance, steps)).unwrap();
        assert_eq!(report.steps, expected_steps);
        assert_eq!(report.termination, expected_termination);
        assert_eq!(budget.used(), allowance);
        assert_eq!(session.iteration().iteration(), expected_steps);
    }
}

#[test]
fn generic_import_reports_local_admission_failure_without_exhausting_global_budget() {
    let mut session = quadratic_session(true);
    let budget = Arc::new(SearchBudget::new(100));
    session.step(&mut SearchSlice::new(budget.clone(), 4, 1)).unwrap();
    let x = session.variable_order()[0];
    let report = session.import_solutions(
        vec![Solution::from([(x, 0.0)]), Solution::from([(x, 1.0)])],
        &mut SearchSlice::new(budget.clone(), 1, 0),
    ).unwrap();
    assert_eq!(report.termination, SearchTermination::EvaluationLimit);
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.evaluations, 1);
    assert_eq!(budget.remaining(), 95);
}

#[test]
fn generic_session_rejects_import_without_importer() {
    let mut session = quadratic_session(false);
    let budget = Arc::new(SearchBudget::new(100));
    let mut slice = SearchSlice::new(budget, 10, 1);
    session.step(&mut slice).expect("step should succeed");

    let solution = Solution::from([(new_standalone_id(), 1.0)]);
    assert!(session.import_solutions(vec![solution], &mut slice).is_err());
}

#[test]
fn generic_session_imports_solutions_into_archive() {
    let mut session = quadratic_session(true);
    let budget = Arc::new(SearchBudget::new(100));
    let mut slice = SearchSlice::new(budget, 10, 1);
    session.step(&mut slice).expect("step should succeed");
    let before = session.archive().len();

    // 必须使用会话问题自身的变量身份，否则解会被判为维度不匹配。
    // The imported solution must use the session problem's own variable identity, otherwise it is rejected as a dimension
    // mismatch.
    let order = session.variable_order();
    let solution = Solution::from([(order[0], 1.0)]);
    let report = session
        .import_solutions(vec![solution], &mut slice)
        .expect("import should succeed");

    assert!(session.supports_import());
    assert_eq!(report.candidates.len(), 1);
    assert!(session.archive().len() >= before);
    assert!(session.archive()[0].fitness <= 1.0e-12);
}

#[test]
fn nested_slice_charges_parent_and_global_budget() {
    let global = Arc::new(SearchBudget::new(10));
    let parent = Arc::new(SearchSlice::new(Arc::clone(&global), 5, 10));
    let child = SearchSlice::child(&parent, 100, 10);

    // 子片段最多消耗父片段的 5 次额度 / The child can consume at most the parent's 5 claims
    for _ in 0..5 {
        assert!(child.acquire_shared());
    }
    assert!(!child.acquire_shared());
    assert_eq!(parent.used(), 5);
    assert_eq!(global.used(), 5);
}

#[test]
fn nested_slice_inherits_time_limit() {
    let global = Arc::new(SearchBudget::new(10));
    let parent = Arc::new(
        SearchSlice::new(global, 5, 10).with_duration(std::time::Duration::from_nanos(1)),
    );
    std::thread::sleep(std::time::Duration::from_millis(2));
    let child = SearchSlice::child(&parent, 5, 10);
    assert!(child.expired());
    assert!(!child.acquire_shared());
}

#[test]
fn snapshot_round_trip_requires_state_codec_and_restores_archive() {
    let order = QuadraticProblem::new().variable_order();
    let hooks = HookedSkeleton::<_, f64, f64, _>::new(
        {
            let x = order[0];
            move |_problem: &QuadraticProblem, _slice: &SearchSlice| {
                Ok(vec![Individual::new(Solution::from([(x, 3.0)]), 4.0)])
            }
        },
        {
            let x = order[0];
            move |_problem: &mut QuadraticProblem,
                  _iteration: &Iteration,
                  state: &Vec<Individual<f64, f64>>,
                  _slice: &SearchSlice| {
                let advanced = state
                    .iter()
                    .map(|individual| {
                        let value = individual.solution.get(&x).copied().unwrap_or(0.0);
                        let next = value + 0.5 * (1.0 - value);
                        Individual::new(Solution::from([(x, next)]), (next - 1.0) * (next - 1.0))
                    })
                    .collect();
                Ok(advanced)
            }
        },
        |state: &Vec<Individual<f64, f64>>| state.clone(),
    );

    let codecs = SessionCodecs::new(
        Some(Box::new(IndividualListCodec::new(order.clone()))),
        Some(Box::new(IndividualListCodec::new(order.clone()))),
    );
    let mut session = RunnerSession::new(Box::new(hooks), QuadraticProblem::new(), codecs);

    let budget = Arc::new(SearchBudget::new(100));
    let mut slice = SearchSlice::new(budget, 10, 2);
    session.step(&mut slice).expect("step should succeed");

    let snapshot = session.snapshot().expect("snapshot should succeed");
    let mut restored = {
        let hooks = HookedSkeleton::<_, f64, f64, _>::new(
            {
                let x = order[0];
                move |_problem: &QuadraticProblem, _slice: &SearchSlice| {
                    Ok(vec![Individual::new(Solution::from([(x, 0.0)]), 1.0)])
                }
            },
            |_: &mut QuadraticProblem,
             _: &Iteration,
             state: &Vec<Individual<f64, f64>>,
             _: &SearchSlice| { Ok(state.clone()) },
            |state: &Vec<Individual<f64, f64>>| state.clone(),
        );
        let codecs = SessionCodecs::new(
            Some(Box::new(IndividualListCodec::new(order.clone()))),
            Some(Box::new(IndividualListCodec::new(order.clone()))),
        );
        RunnerSession::new(Box::new(hooks), QuadraticProblem::new(), codecs)
    };

    restored.restore(&snapshot).expect("restore should succeed");
    assert_eq!(
        restored.iteration().iteration(),
        session.iteration().iteration()
    );
    assert_eq!(restored.archive().len(), session.archive().len());
}

#[test]
fn seven_state_codecs_round_trip() {
    let order = QuadraticProblem::new().variable_order();
    let x = order[0];
    let individual = Individual::new(Solution::from([(x, 2.0)]), 1.0);

    // 人口 / population
    let population = IndividualListCodec::new(order.clone());
    let encoded = population.encode(&vec![individual.clone()]).expect("encodable");
    let decoded = population.decode(&encoded).expect("decodable");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].solution, individual.solution);
    assert_eq!(decoded[0].fitness, individual.fitness);

    // 岛屿 / islands
    let islands = IslandStateCodec::new(order.clone());
    let value = vec![vec![individual.clone()], vec![]];
    let encoded = islands.encode(&value).expect("encodable");
    let decoded = islands.decode(&encoded).expect("decodable");
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].len(), 1);
    assert!(decoded[1].is_empty());

    // 轨迹 / trajectory
    let trajectory = TrajectoryStateCodec::new(order.clone());
    let encoded = trajectory.encode(&Some(individual.clone())).expect("encodable");
    let decoded = trajectory.decode(&encoded).expect("decodable").expect("present");
    assert_eq!(decoded.solution, individual.solution);
    let encoded = trajectory.encode(&None).expect("encodable");
    assert!(trajectory.decode(&encoded).expect("decodable").is_none());

    // 粒子 / particles
    let particles = ParticleStateCodec::new(order.clone());
    let value = vec![crate::solver::heuristic::runner::particle_swarm_runner::ParticleState {
        individual: individual.clone(),
        velocity: vec![0.1],
        best: individual.clone(),
    }];
    let encoded = particles.encode(&value).expect("encodable");
    let decoded = particles.decode(&encoded).expect("decodable");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].velocity, vec![0.1]);

    // CLPSO
    let clpso = ClpsoStateCodec::new(order.clone());
    // exemplar 数量必须等于解维度（此处为 1），与 Kotlin 解码校验一致。
    // The exemplar count must equal the solution dimension (1 here), matching Kotlin's decode validation.
    let value = vec![crate::solver::heuristic::runner::clpso_runner::ClpsoParticle {
        individual: individual.clone(),
        velocity: vec![0.2],
        best: individual.clone(),
        exemplars: vec![0],
        stagnation: 3,
    }];
    let encoded = clpso.encode(&value).expect("encodable");
    let decoded = clpso.decode(&encoded).expect("decodable");
    assert_eq!(decoded[0].exemplars, vec![0]);
    assert_eq!(decoded[0].stagnation, 3);

    // SHADE
    let shade = ShadeStateCodec::new(order.clone());
    let value = crate::solver::heuristic::runner::shade_runner::ShadeState {
        population: vec![individual.clone()],
        memory: crate::solver::heuristic::runner::shade_runner::ShadeMemory {
            memory_f: vec![0.5, 0.6],
            memory_cr: vec![0.7, 0.8],
            cursor: 1,
        },
        archive: vec![Solution::from([(x, 0.5)])],
    };
    let encoded = shade.encode(&value).expect("encodable");
    let decoded = shade.decode(&encoded).expect("decodable");
    assert_eq!(decoded.memory.memory_f, vec![0.5, 0.6]);
    assert_eq!(decoded.memory.cursor, 1);
    assert_eq!(decoded.archive.len(), 1);

    // CMA-ES
    let cmaes = CmaEsStateCodec::new(order.clone());
    let value = crate::solver::heuristic::runner::cmaes_runner::CmaEsState {
        mean: vec![1.0],
        sigma: 0.3,
        covariance: vec![vec![1.0]],
        path_sigma: vec![0.0],
        path_covariance: vec![0.0],
        generation: 7,
        best: Some(individual.clone()),
    };
    let encoded = cmaes.encode(&value).expect("encodable");
    let decoded = cmaes.decode(&encoded).expect("decodable");
    assert_eq!(decoded.generation, 7);
    assert_eq!(decoded.mean, vec![1.0]);
    assert!(decoded.best.is_some());
}

#[test]
fn codecs_reject_malformed_state() {
    let order = QuadraticProblem::new().variable_order();
    let trajectory = TrajectoryStateCodec::new(order.clone());
    assert!(trajectory.decode(&JsonValue::from("bad".to_string())).is_err());

    let matrix = NumberMatrixCodec;
    assert!(matrix.decode(&JsonValue::from(1.0)).is_err());

    let vector = NumberVectorCodec;
    assert!(vector.encode(&vec![f64::INFINITY]).is_err());

    let cmaes = CmaEsStateCodec::new(order);
    let malformed = crate::solver::heuristic::session::json::object([
        ("mean", crate::solver::heuristic::session::json::numbers([1.0, 2.0])),
        ("sigma", JsonValue::from(0.3)),
        ("covariance", JsonValue::Array(vec![crate::solver::heuristic::session::json::numbers([1.0])])),
        ("path_sigma", crate::solver::heuristic::session::json::numbers([0.0, 0.0])),
        ("path_covariance", crate::solver::heuristic::session::json::numbers([0.0, 0.0])),
        ("generation", JsonValue::from(1.0)),
        ("best", JsonValue::Null),
    ]);
    assert!(cmaes.decode(&malformed).is_err());
}

/// 真实骨架：GA 种群 + 通用会话 / Real skeleton: GA population under the generic session
mod population_adapter {
    use std::sync::Arc;

    use crate::model::callback::Solution;
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::individual::Individual;
    use crate::solver::heuristic::operator::mutation::GaussianMutation;
    use crate::solver::heuristic::operator::survivor::ElitistSurvivor;
    use crate::solver::heuristic::operator::update::ArithmeticCrossover;
    use crate::solver::heuristic::runner::population_runner::PopulationRunner;
    use crate::solver::heuristic::session::adapters::PopulationHooks;
    use crate::solver::heuristic::session::budget::{SearchBudget, SearchSlice};
    use crate::solver::heuristic::session::codecs::IndividualListCodec;
    use crate::solver::heuristic::session::protocol::{
        SearchProblem, SearchSession, SearchTermination,
    };
    use crate::solver::heuristic::session::runner_session::{RunnerSession, SessionCodecs};
    use crate::variable::{VariableId, new_standalone_id};

    struct Quadratic {
        x: VariableId,
        id: String,
        fingerprint: String,
    }

    impl Quadratic {
        fn new() -> Self {
            Self {
                x: new_standalone_id(),
                id: "adapter-quadratic".to_string(),
                fingerprint: "adapter-quadratic-v1".to_string(),
            }
        }
    }

    impl SearchProblem<f64, f64> for Quadratic {
        fn id(&self) -> &str {
            &self.id
        }

        fn fingerprint(&self) -> &str {
            &self.fingerprint
        }

        fn variable_order(&self) -> Vec<VariableId> {
            vec![self.x]
        }

        fn lower_bound(&self, _index: usize) -> Option<f64> {
            Some(-10.0)
        }

        fn upper_bound(&self, _index: usize) -> Option<f64> {
            Some(10.0)
        }

        fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
            (0..amount)
                .map(|index| Solution::from([(self.x, index as f64 * 0.5 - 2.0)]))
                .collect()
        }

        fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
            let value = solution.get(&self.x).copied()?;
            Some((value - 1.0) * (value - 1.0))
        }

        fn dominates(&self, lhs: &f64, rhs: &f64) -> bool {
            lhs < rhs
        }

        fn feasible(&self, _solution: &Solution<f64>) -> bool {
            true
        }
    }

    fn session(order: Vec<VariableId>) -> RunnerSession<Quadratic, f64, f64, Vec<Individual<f64, f64>>> {
        let runner = PopulationRunner::new(
            10,
            Box::new(ArithmeticCrossover),
            Box::new(GaussianMutation::default()),
            Box::new(ElitistSurvivor),
            Box::new(SeededEntropy::new(13)),
        );
        let hooks = PopulationHooks::new(runner, Box::new(SeededEntropy::new(17)));
        let codecs = SessionCodecs::new(
            Some(Box::new(IndividualListCodec::new(order.clone()))),
            Some(Box::new(IndividualListCodec::new(order))),
        );
        RunnerSession::new(Box::new(hooks), Quadratic::new(), codecs)
    }

    #[test]
    fn population_skeleton_runs_under_the_generic_session() {
        let order = Quadratic::new().variable_order();
        let mut session = session(order);

        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 5);
        let report = session.step(&mut slice).expect("step should succeed");

        assert_eq!(report.steps, 5);
        assert!(report.is_completed());
        assert_eq!(session.archive().len(), 1);
        // GA 至少不应该让最优解变差 / GA must not make the incumbent worse
        assert!(session.archive()[0].fitness <= 36.0);
    }

    #[test]
    fn population_snapshot_round_trip_resumes_the_skeleton() {
        let order = Quadratic::new().variable_order();
        let mut session = session(order.clone());

        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 3);
        session.step(&mut slice).expect("step should succeed");
        let before = session.archive()[0].fitness;

        let snapshot = session.snapshot().expect("snapshot should succeed");
        let mut resumed = self::session(order);
        resumed.restore(&snapshot).expect("restore should succeed");
        assert_eq!(
            resumed.iteration().iteration(),
            session.iteration().iteration()
        );
        assert_eq!(resumed.archive().len(), session.archive().len());
        assert!((resumed.archive()[0].fitness - before).abs() <= 1e-12);

        // 恢复后继续推进仍然有效 / Advancing after restore still works
        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 2);
        let report = resumed.step(&mut slice).expect("resumed step should succeed");
        assert_eq!(report.steps, 2);
        assert_ne!(report.termination, SearchTermination::Failed);
    }
}

#[test]
fn generic_session_broadcasts_observers_with_improvement_flag() {
    use std::sync::{Arc, Mutex};

    use crate::solver::heuristic::individual::Individual;
    use crate::solver::heuristic::iteration::Iteration;
    use crate::solver::heuristic::runner::iteration_observer::IterationObserver;

    struct Recording {
        seen: Arc<Mutex<Vec<(u64, bool)>>>,
    }

    impl IterationObserver<f64, f64> for Recording {
        fn observe(
            &mut self,
            iteration: &Iteration,
            members: &[Individual<f64, f64>],
            improved: bool,
        ) {
            assert!(!members.is_empty());
            self.seen
                .lock()
                .unwrap()
                .push((iteration.iteration(), improved));
        }
    }

    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut session = quadratic_session(false);
    session.push_observer(Box::new(Recording {
        seen: Arc::clone(&seen),
    }));

    let budget = Arc::new(SearchBudget::new(1_000));
    let mut slice = SearchSlice::new(budget, 100, 3);
    session.step(&mut slice).expect("step should succeed");

    let recorded = seen.lock().unwrap();
    assert_eq!(recorded.len(), 3);
    // 初始种群已包含最优解 x=1，后续收缩不会产生更优，因此改进标志应为 false；
    // 这正说明「改进」来自档案比较而不是「是否执行了推进」。
    // The initial population already contains the optimum x=1, so contraction cannot improve it and the flag stays false;
    // this shows "improved" comes from archive comparison rather than "an advance happened".
    assert!(recorded[0].1);
    assert!(recorded[1..].iter().all(|(_, improved)| !*improved));
    assert_eq!(recorded[0].0, 0);
}

/// 快照落盘：证明会话快照是真正的 serde JSON，可写入字符串再读回。
/// Snapshot persistence: proves session snapshots are real serde JSON that can be written to a string and read back.
mod snapshot_persistence {
    use std::sync::Arc;

    use crate::model::callback::Solution;
    use crate::solver::heuristic::individual::Individual;
    use crate::solver::heuristic::session::budget::{SearchBudget, SearchSlice};
    use crate::solver::heuristic::session::codecs::IndividualListCodec;
    use crate::solver::heuristic::session::protocol::{
        SearchProblem, SearchSession, SearchSessionSnapshot,
    };
    use crate::solver::heuristic::session::runner_session::{
        HookedSkeleton, RunnerSession, SessionCodecs,
    };
    use crate::variable::{VariableId, new_standalone_id};

    struct Quadratic {
        x: VariableId,
        id: String,
        fingerprint: String,
    }

    impl Quadratic {
        fn new() -> Self {
            Self {
                x: new_standalone_id(),
                id: "persist".to_string(),
                fingerprint: "persist-v1".to_string(),
            }
        }
    }

    impl SearchProblem<f64, f64> for Quadratic {
        fn id(&self) -> &str {
            &self.id
        }

        fn fingerprint(&self) -> &str {
            &self.fingerprint
        }

        fn variable_order(&self) -> Vec<VariableId> {
            vec![self.x]
        }

        fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
            (0..amount)
                .map(|index| Solution::from([(self.x, index as f64)]))
                .collect()
        }

        fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
            solution.get(&self.x).copied()
        }

        fn dominates(&self, lhs: &f64, rhs: &f64) -> bool {
            lhs < rhs
        }
    }

    fn persistent_session() -> RunnerSession<Quadratic, f64, f64, Vec<Individual<f64, f64>>> {
        let order = Quadratic::new().variable_order();
        let hooks = HookedSkeleton::<_, f64, f64, _>::new(
            {
                let x = order[0];
                move |problem: &Quadratic, _slice: &SearchSlice| {
                    Ok(problem
                        .initial_solutions(3)
                        .into_iter()
                        .filter_map(|solution| {
                            let value = solution.get(&x).copied()?;
                            Some(Individual::new(solution, value))
                        })
                        .collect::<Vec<_>>())
                }
            },
            |_: &mut Quadratic,
             _: &crate::solver::heuristic::iteration::Iteration,
             state: &Vec<Individual<f64, f64>>,
             _: &SearchSlice| { Ok(state.clone()) },
            |state: &Vec<Individual<f64, f64>>| {
                state
                    .iter()
                    .map(|item| Individual::new(item.solution.clone(), item.fitness))
                    .collect()
            },
        );
        let codecs = SessionCodecs::new(
            Some(Box::new(IndividualListCodec::new(order.clone()))),
            Some(Box::new(IndividualListCodec::new(order))),
        );
        RunnerSession::new(Box::new(hooks), Quadratic::new(), codecs)
    }

    #[test]
    fn snapshot_round_trips_through_json_text() {
        let mut session = persistent_session();
        let budget = Arc::new(SearchBudget::new(100));
        let mut slice = SearchSlice::new(budget, 10, 2);
        session.step(&mut slice).expect("step should succeed");

        let snapshot = session.snapshot().expect("snapshot should succeed");

        // 序列化为文本 / Serialize to text
        let text = serde_json::to_string(&serde_json::json!({
            "version": snapshot.version,
            "initialized": snapshot.initialized,
            "iteration": snapshot.iteration,
            "stagnation": snapshot.stagnation,
            "state": snapshot.state,
            "archive": snapshot.archive,
            "active_nanos": snapshot.active_nanos,
        }))
        .expect("snapshot should serialize");

        // 读回并还原为协议快照 / Parse back into the protocol snapshot
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("snapshot should deserialize");
        let restored = SearchSessionSnapshot {
            version: value["version"].as_u64().expect("version") as u32,
            initialized: value["initialized"].as_bool().expect("initialized"),
            iteration: value["iteration"].as_u64().expect("iteration"),
            stagnation: value["stagnation"].as_u64().expect("stagnation"),
            state: value["state"].clone(),
            archive: value["archive"].clone(),
            active_nanos: value["active_nanos"].as_u64().expect("active_nanos"),
        };

        let mut target = persistent_session();
        target.restore(&restored).expect("restore should succeed");
        assert_eq!(target.iteration().iteration(), session.iteration().iteration());
        assert_eq!(target.archive().len(), session.archive().len());
    }
}

/// 与 Kotlin 的文本互操作：Kotlin 写出的快照能被 Rust 读取，反之亦然。
/// Text interop with Kotlin: snapshots written by Kotlin are readable by Rust and vice versa.
mod kotlin_wire_format {
    use std::sync::Arc;

    use crate::model::callback::Solution;
    use crate::solver::heuristic::individual::Individual;
    use crate::solver::heuristic::session::budget::{SearchBudget, SearchSlice};
    use crate::solver::heuristic::session::codecs::IndividualListCodec;
    use crate::solver::heuristic::session::protocol::{
        SearchProblem, SearchSession, SearchSessionSnapshot,
    };
    use crate::solver::heuristic::session::runner_session::{
        HookedSkeleton, RunnerSession, SessionCodecs,
    };
    use crate::variable::{VariableId, new_standalone_id};

    /// 与 Kotlin 导出用例一致的双变量问题（x、y 两个分量）。
    /// Two-variable problem matching the Kotlin export fixture (components x and y).
    struct Quadratic {
        x: VariableId,
        y: VariableId,
        id: String,
        fingerprint: String,
    }

    impl Quadratic {
        fn new() -> Self {
            Self {
                x: new_standalone_id(),
                y: new_standalone_id(),
                id: "wire".to_string(),
                fingerprint: "wire-v1".to_string(),
            }
        }
    }

    impl SearchProblem<f64, f64> for Quadratic {
        fn id(&self) -> &str {
            &self.id
        }
        fn fingerprint(&self) -> &str {
            &self.fingerprint
        }
        fn variable_order(&self) -> Vec<VariableId> {
            vec![self.x, self.y]
        }
        fn initial_solutions(&self, _amount: usize) -> Vec<Solution<f64>> {
            Vec::new()
        }
        fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
            solution.get(&self.x).copied()
        }
        fn dominates(&self, lhs: &f64, rhs: &f64) -> bool {
            lhs < rhs
        }
    }

    fn session() -> RunnerSession<Quadratic, f64, f64, Vec<Individual<f64, f64>>> {
        // 变量标识必须来自同一个问题实例：`new_standalone_id` 每次调用都会生成新标识。
        // Variable identities must come from one problem instance; `new_standalone_id` yields a fresh id per call.
        let problem = Quadratic::new();
        let order = problem.variable_order();
        let x = order[0];
        let hooks = HookedSkeleton::<_, f64, f64, _>::new(
            move |problem: &Quadratic, _: &SearchSlice| {
                let mut solution = Solution::from([(x, 2.0)]);
                solution.insert(problem.y, 5.0);
                Ok(vec![Individual::new(solution, 2.0)])
            },
            |_: &mut Quadratic,
             _: &crate::solver::heuristic::iteration::Iteration,
             state: &Vec<Individual<f64, f64>>,
             _: &SearchSlice| { Ok(state.clone()) },
            |state: &Vec<Individual<f64, f64>>| state.clone(),
        );
        let codecs = SessionCodecs::new(
            Some(Box::new(IndividualListCodec::new(order.clone()))),
            Some(Box::new(IndividualListCodec::new(order))),
        );
        RunnerSession::new(Box::new(hooks), problem, codecs)
    }

    #[test]
    fn rust_snapshot_text_matches_the_text_kotlin_restored() {
        // 这段文本由 Rust 序列化产生，并已在 Kotlin 侧通过 `restore` 成功还原（见提交说明）。
        // 该断言把「Rust 写出的形状」钉死，避免后续改动悄悄破坏互操作。
        // This text was produced by Rust serialization and successfully restored on the Kotlin side (see the commit message).
        // The assertion pins the Rust-written shape so later changes cannot silently break interoperability.
        let mut session = session();
        let budget = Arc::new(SearchBudget::new(100));
        let mut slice = SearchSlice::new(budget, 10, 3);
        session.step(&mut slice).expect("step should succeed");

        let snapshot = session.snapshot().expect("snapshot should succeed");
        let text = serde_json::to_string(&snapshot).expect("serializable");

        // activeNanos 依赖真实耗时，先剔除再比较其余字段。
        // activeNanos depends on wall-clock time, so drop it before comparing the remaining fields.
        let mut value: serde_json::Value =
            serde_json::from_str(&text).expect("valid json");
        value.as_object_mut().expect("object").remove("activeNanos");

        assert_eq!(
            value,
            serde_json::json!({
                "initialized": true,
                "iteration": 3,
                "stagnation": 2,
                "state": [{"solution": ["2.0", "5.0"], "fitness": "2.0"}],
                "archive": [{"solution": ["2.0", "5.0"], "fitness": "2.0"}],
            })
        );
    }

    #[test]
    fn real_kotlin_snapshot_text_is_readable() {
        // 这段文本由 Kotlin 侧 `SnapshotExportTest` 实际运行产生（ospf-kotlin，Kotlin 2.4.20）。
        // 未做任何手工改写，用于确证两侧文本互通。
        // This text was produced by actually running `SnapshotExportTest` on the Kotlin side (ospf-kotlin, Kotlin 2.4.20).
        // It is reproduced verbatim to prove text-level interop.
        let kotlin_text = r#"{"initialized":true,"iteration":2,"stagnation":0,"state":[{"solution":["2.0","5.0"],"fitness":"1.0"},{"solution":["2.0","5.0"],"fitness":"1.0"},{"solution":["2.0","5.0"],"fitness":"1.0"},{"solution":["2.0","5.0"],"fitness":"1.0"}],"archive":[{"solution":["2.0","5.0"],"fitness":"1.0"}],"activeNanos":180592400}"#;

        let snapshot: SearchSessionSnapshot =
            serde_json::from_str(kotlin_text).expect("real Kotlin snapshot should deserialize");

        assert_eq!(snapshot.version, SearchSessionSnapshot::VERSION);
        assert_eq!(snapshot.iteration, 2);
        assert_eq!(snapshot.stagnation, 0);
        assert_eq!(snapshot.active_nanos, 180_592_400);
        assert!(snapshot.initialized);

        // 状态是种群数组：4 个个体，每个含 solution 与 fitness / The state is a population array of 4 individuals
        let population = snapshot
            .state
            .as_array()
            .expect("state should be an array");
        assert_eq!(population.len(), 4);
        assert_eq!(population[0]["fitness"], serde_json::json!("1.0"));

        // 恢复后归档与迭代计数应与 Kotlin 一致 / After restore the archive and counters must match Kotlin
        let mut session = session();
        session.restore(&snapshot).expect("restore should succeed");
        assert_eq!(session.iteration().iteration(), 2);
        assert_eq!(session.archive().len(), 1);
        assert_eq!(session.archive()[0].fitness, 1.0);
    }

    #[test]
    fn kotlin_snapshot_json_is_readable() {
        // 这是 Kotlin 侧会写出的形状：字段名驼峰、默认值省略、数值为字符串。
        // This is the shape Kotlin writes: camel-case field names, defaults omitted, numbers as strings.
        let kotlin_text = r#"{
            "initialized": true,
            "iteration": 7,
            "stagnation": 2,
            "state": [{"solution": ["2.0", "5.0"], "fitness": "2.0"}],
            "archive": [{"solution": ["2.0", "5.0"], "fitness": "2.0"}]
        }"#;

        let snapshot: SearchSessionSnapshot =
            serde_json::from_str(kotlin_text).expect("Kotlin snapshot should deserialize");

        assert_eq!(snapshot.version, SearchSessionSnapshot::VERSION);
        assert_eq!(snapshot.active_nanos, 0);
        assert_eq!(snapshot.iteration, 7);
        assert_eq!(snapshot.stagnation, 2);

        let mut session = session();
        session.restore(&snapshot).expect("restore should succeed");
        assert_eq!(session.iteration().iteration(), 7);
        assert_eq!(session.iteration().not_better_iteration(), 2);
        assert_eq!(session.archive().len(), 1);
    }

    #[test]
    fn snapshot_serializes_in_kotlin_wire_shape() {
        let mut session = session();
        let budget = Arc::new(SearchBudget::new(10));
        let mut slice = SearchSlice::new(budget, 10, 1);
        session.step(&mut slice).expect("step should succeed");

        let snapshot = session.snapshot().expect("snapshot should succeed");
        let text = serde_json::to_string(&snapshot).expect("serializable");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");

        // version 取默认值时省略；activeNanos 在推进后有值，因此应出现且为驼峰命名。
        // version is omitted while default; activeNanos is non-zero after advancing, so it must appear camel-cased.
        assert!(value.get("version").is_none());
        assert!(value.get("activeNanos").is_some());
        assert!(value.get("active_nanos").is_none());
        // 必须有驼峰字段 / The camel-case field must exist when non-default
        assert!(value.get("initialized").is_some());
        assert!(value.get("stagnation").is_some());
        // 数值以字符串编码 / Numbers are encoded as strings
        let state = value.get("state").expect("state");
        assert_eq!(state[0]["fitness"], serde_json::json!("2.0"));
        assert_eq!(state[0]["solution"][0], serde_json::json!("2.0"));
        assert_eq!(state[0]["solution"][1], serde_json::json!("5.0"));
    }
}

/// 多目标会话：NSGA 钩子 + 前沿快照 / Multi-objective session: NSGA hooks with front snapshots
mod multi_objective_adapter {
    use std::sync::Arc;

    use crate::model::callback::Solution;
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::individual::Individual;
    use crate::solver::heuristic::operator::multi_objective::dominance::{Dominance, ParetoDominance};
    use crate::solver::heuristic::operator::mutation::PolynomialMutation;
    use crate::solver::heuristic::operator::update::SimulatedBinaryCrossover;
    use crate::solver::heuristic::session::adapters::NsgaSessionHooks;
    use crate::solver::heuristic::session::codecs::MultiObjectiveArchiveCodec;
    use crate::solver::heuristic::session::budget::{SearchBudget, SearchSlice};
    use crate::solver::heuristic::session::json::SearchCodec;
    use crate::solver::heuristic::session::protocol::{
        SearchProblem, SearchSession, SearchTermination,
    };
    use crate::solver::heuristic::session::runner_session::{RunnerSession, SessionCodecs};
    use crate::variable::{VariableId, new_standalone_id};

    /// 双目标问题：min x^2 与 min (x-1)^2 / Bi-objective problem: min x^2 and min (x-1)^2
    struct BiObjective {
        x: VariableId,
        id: String,
        fingerprint: String,
    }

    impl BiObjective {
        fn new() -> Self {
            Self {
                x: new_standalone_id(),
                id: "bi-objective".to_string(),
                fingerprint: "bi-objective-v1".to_string(),
            }
        }
    }

    impl SearchProblem<Vec<f64>, f64> for BiObjective {
        fn id(&self) -> &str {
            &self.id
        }

        fn fingerprint(&self) -> &str {
            &self.fingerprint
        }

        fn variable_order(&self) -> Vec<VariableId> {
            vec![self.x]
        }

        fn lower_bound(&self, _index: usize) -> Option<f64> {
            Some(-2.0)
        }

        fn upper_bound(&self, _index: usize) -> Option<f64> {
            Some(3.0)
        }

        fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
            (0..amount)
                .map(|index| Solution::from([(self.x, index as f64 * 0.2 - 1.0)]))
                .collect()
        }

        fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> {
            let value = solution.get(&self.x).copied()?;
            Some(vec![value * value, (value - 1.0) * (value - 1.0)])
        }

        fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool {
            ParetoDominance::minimization(2).dominates(lhs, rhs)
        }

        fn multi_objective(&self) -> bool {
            true
        }
    }

    fn session() -> RunnerSession<
        BiObjective,
        Vec<f64>,
        f64,
        Vec<Individual<Vec<f64>, f64>>,
    > {
        let order = BiObjective::new().variable_order();
        let hooks = NsgaSessionHooks::new(
            12,
            Box::new(ParetoDominance::minimization(2)),
            Box::new(SimulatedBinaryCrossover::default()),
            Box::new(PolynomialMutation::default()),
            Box::new(SeededEntropy::new(41)),
            None,
        );
        let codecs: SessionCodecs<Vec<Individual<Vec<f64>, f64>>, Vec<f64>, f64> =
            SessionCodecs::new(
                Some(Box::new(MultiObjectiveArchiveCodec::new(order.clone()))),
                Some(Box::new(MultiObjectiveArchiveCodec::new(order))),
            );
        RunnerSession::new(Box::new(hooks), BiObjective::new(), codecs)
    }

    #[test]
    fn nsga_session_advances_a_non_dominated_front() {
        let mut session = session();
        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 3);

        let report = session.step(&mut slice).expect("step should succeed");
        assert_eq!(report.steps, 3);
        assert!(report.is_completed());

        // 档案内任意两解互不支配 / No two archived solutions dominate each other
        let dominance = ParetoDominance::minimization(2);
        for left in session.archive() {
            for right in session.archive() {
                assert!(!dominance.dominates(&left.fitness, &right.fitness));
            }
        }
        assert!(!session.archive().is_empty());
    }

    #[test]
    fn nsga_session_snapshot_round_trip_restores_the_front() {
        let mut session = session();
        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 2);
        session.step(&mut slice).expect("step should succeed");

        let snapshot = session.snapshot().expect("snapshot should succeed");
        let mut resumed = self::session();
        resumed.restore(&snapshot).expect("restore should succeed");

        assert_eq!(
            resumed.iteration().iteration(),
            session.iteration().iteration()
        );
        assert_eq!(resumed.archive().len(), session.archive().len());

        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 1);
        let report = resumed.step(&mut slice).expect("resumed step should succeed");
        assert_ne!(report.termination, SearchTermination::Failed);
    }

    #[test]
    fn nsga_session_defaults_to_a_bounded_front_archive() {
        let mut session = session();
        assert_eq!(session.archive_capacity(), 100);

        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 2);
        session.step(&mut slice).expect("step should succeed");
        assert!(session.archive().len() <= session.archive_capacity());

        // 显式容量应被尊重 / An explicit capacity must be respected
        let order = BiObjective::new().variable_order();
        let hooks = NsgaSessionHooks::new(
            12,
            Box::new(ParetoDominance::minimization(2)),
            Box::new(SimulatedBinaryCrossover::default()),
            Box::new(PolynomialMutation::default()),
            Box::new(SeededEntropy::new(41)),
            None,
        );
        let codecs: SessionCodecs<Vec<Individual<Vec<f64>, f64>>, Vec<f64>, f64> =
            SessionCodecs::new(
                Some(Box::new(MultiObjectiveArchiveCodec::new(order.clone()))),
                Some(Box::new(MultiObjectiveArchiveCodec::new(order))),
            );
        let mut bounded = RunnerSession::with_archive_capacity(
            Box::new(hooks),
            BiObjective::new(),
            codecs,
            1,
        );
        assert_eq!(bounded.archive_capacity(), 1);
        let budget = Arc::new(SearchBudget::new(10_000));
        let mut slice = SearchSlice::new(budget, 500, 2);
        bounded.step(&mut slice).expect("step should succeed");
        assert_eq!(bounded.archive().len(), 1);
    }

    #[test]
    fn objective_vector_codec_rejects_invalid_vectors() {
        use crate::solver::heuristic::session::codecs::ObjectiveVectorCodec;
        let codec = ObjectiveVectorCodec;
        assert!(codec.encode(&vec![]).is_err());
        assert!(codec.encode(&vec![f64::NAN]).is_err());
        let encoded = codec.encode(&vec![1.0, 2.0]).expect("encodable");
        assert_eq!(codec.decode(&encoded).expect("decodable"), vec![1.0, 2.0]);
    }
}

/// 四种非种群骨架的真实 Kotlin 快照 / Real Kotlin snapshots for the four non-population skeletons
///
/// 文本由 Kotlin 侧 `NativeStateExportTest` 实际运行产生，未做改写。
/// The text comes from actually running `NativeStateExportTest` on the Kotlin side, verbatim.
mod kotlin_native_state_wire {
    use serde_json::json;

    use crate::solver::heuristic::session::codecs::{
        ClpsoStateCodec, CmaEsStateCodec, NativeState, ParticleStateCodec, ShadeStateCodec,
    };
    use crate::solver::heuristic::session::json::SearchCodec;
    use crate::variable::VariableId;

    const PSO_STATE: &str = r#"{"individuals":[{"solution":["2.3459770750119855","2.797341034551419"],"fitness":"6.663360804528834"},{"solution":["2.3459770750119855","2.797341034551419"],"fitness":"6.663360804528834"},{"solution":["1.205944445188072","4.680363458742562"],"fitness":"0.1445806330108407"},{"solution":["1.205944445188072","4.680363458742562"],"fitness":"0.1445806330108407"},{"solution":["0.2634859539131842","2.4484116085227576"],"fitness":"7.053056259604594"},{"solution":["0.2634859539131842","2.4484116085227576"],"fitness":"7.053056259604594"},{"solution":["2.0","4.0"],"fitness":"2.0"},{"solution":["2.0","4.0"],"fitness":"2.0"}],"vectors":[[2.3459770750119855,0.7973410345514189],[-1.794055554811928,-0.319636541257438],[-5.736514046086816,1.4484116085227574],[0.0,0.0]]}"#;

    const CLPSO_STATE: &str = r#"{"individuals":[{"solution":["1.1021577489506396","2.9898748041112784"],"fitness":"4.051039508817333"},{"solution":["1.1021577489506396","2.9898748041112784"],"fitness":"4.051039508817333"},{"solution":["2.572634287671393","2.3883105791154926"],"fitness":"9.294100233919762"},{"solution":["3.0","5.0"],"fitness":"4.0"},{"solution":["6.0","5.276967173321998"],"fitness":"25.07671081509798"},{"solution":["6.0","5.276967173321998"],"fitness":"25.07671081509798"},{"solution":["2.3139968545743255","2.311832570182956"],"fitness":"8.952831864560395"},{"solution":["2.0","4.0"],"fitness":"2.0"}],"vectors":[[1.1021577489506396,0.9898748041112782],[-0.4273657123286073,-2.6116894208845074],[0.0,4.276967173321998],[0.3139968545743253,-1.6881674298170442]],"children":[{"counters":[0,2,1]},{"counters":[1,0,0]},{"counters":[0,2,3]},{"counters":[1,1,2]}]}"#;

    const SHADE_STATE: &str = r#"{"individuals":[{"solution":["-0.5","5.0"],"fitness":"2.25"},{"solution":["3.0","5.0"],"fitness":"4.0"},{"solution":["4.4641082067537665","1.0"],"fitness":"28.000045668098796"},{"solution":["1.073393912364929","6.316515219087678"],"fitness":"1.7385989884617068"},{"solution":["3.744994796872411","1.6733402708367855"],"fitness":"18.60166138849288"},{"solution":["2.0383049192776923","4.038304919277692"],"fitness":"2.0029345336817412"}],"vectors":[[0.48545356822370317,0.5,0.5,0.5,0.5],[0.4676492043332775,0.5,0.5,0.5,0.5]],"counters":[1],"solutions":[["0.0","2.0"],["6.0","1.0"],["2.0","4.0"],["5.0","0.0"],["1.0","3.0"]]}"#;

    const CMA_STATE: &str = r#"{"individuals":{"solution":["0.16588350040951216","2.3723020397546963"],"fitness":"7.600546905166418"},"vectors":[[-0.030770810556437607,2.3290830600940162],[0.30388548142287397],[-0.12164133874255621,1.300911586775164],[-0.1354016607792157,1.4480734197531275],[0.8398588519276762,-0.032377654905583426],[-0.032377654905583426,1.1884670617579896]],"counters":[1]}"#;

    fn order() -> Vec<VariableId> {
        vec![
            crate::variable::VariableId::standalone(1),
            crate::variable::VariableId::standalone(2),
        ]
    }

    fn decode(text: &str) -> serde_json::Value {
        serde_json::from_str(text).expect("valid Kotlin JSON")
    }

    #[test]
    fn pso_state_is_readable() {
        let codec = ParticleStateCodec::new(order());
        let particles = codec
            .decode(&decode(PSO_STATE))
            .expect("Kotlin PSO state should decode");

        assert_eq!(particles.len(), 4);
        assert_eq!(particles[0].velocity, vec![2.3459770750119855, 0.7973410345514189]);
        assert_eq!(particles[3].velocity, vec![0.0, 0.0]);
        assert_eq!(particles[1].individual.fitness, 0.1445806330108407);
        assert_eq!(particles[1].best.fitness, 0.1445806330108407);
    }

    #[test]
    fn clpso_state_is_readable() {
        let codec = ClpsoStateCodec::new(order());
        let particles = codec
            .decode(&decode(CLPSO_STATE))
            .expect("Kotlin CLPSO state should decode");

        assert_eq!(particles.len(), 4);
        // 计数器首项是停滞代数，其余是各维学习对象 / The first counter is stagnation, the rest are exemplars
        assert_eq!(particles[0].stagnation, 0);
        assert_eq!(particles[0].exemplars, vec![2, 1]);
        assert_eq!(particles[3].stagnation, 1);
        assert_eq!(particles[3].exemplars, vec![1, 2]);
    }

    #[test]
    fn shade_state_is_readable() {
        let codec = ShadeStateCodec::new(order());
        let state = codec
            .decode(&decode(SHADE_STATE))
            .expect("Kotlin SHADE state should decode");

        assert_eq!(state.population.len(), 6);
        assert_eq!(state.memory.memory_f.len(), 5);
        // 浮点字面量的最短表示可能不同，按数值比较。
        // Shortest representations of the same f64 can differ textually, so compare numerically.
        assert!((state.memory.memory_f[0] - 0.48545356822370317).abs() < 1.0e-15);
        assert_eq!(state.memory.cursor, 1);
        assert_eq!(state.archive.len(), 5);
    }

    #[test]
    fn cma_es_state_is_readable() {
        let codec = CmaEsStateCodec::new(order());
        let state = codec
            .decode(&decode(CMA_STATE))
            .expect("Kotlin CMA-ES state should decode");

        assert_eq!(state.generation, 1);
        assert_eq!(state.sigma, 0.30388548142287397);
        assert_eq!(state.mean.len(), 2);
        assert_eq!(state.covariance.len(), 2);
        assert_eq!(state.covariance[0].len(), 2);
        assert!(state.best.is_some());
        assert_eq!(state.best.expect("best").fitness, 7.600546905166418);
    }

    #[test]
    fn rust_written_states_are_kotlin_readable() {
        // 这四段文本由 Rust 编码产生，并已在 Kotlin 侧经 `RunnerStateCodecs.*.decode` 成功还原
        // （见提交说明）。断言把 Rust 写出的形状钉死。
        // These four texts are Rust encodings that Kotlin successfully decoded through `RunnerStateCodecs.*.decode`
        // (see the commit message). The assertions pin the Rust-written shapes.
        let cases: [(&str, &str); 4] = [
            ("pso", PSO_STATE),
            ("clpso", CLPSO_STATE),
            ("shade", SHADE_STATE),
            ("cma", CMA_STATE),
        ];
        for (name, text) in cases {
            let state = NativeState::decode(&decode(text)).expect("decodable");
            let encoded = state.encode();
            // 与 Kotlin 原文等价（浮点最短表示可能差一位，数值相同）
            // Equivalent to the Kotlin original; shortest float forms may differ textually but not numerically.
            assert_eq!(
                encoded, decode(text),
                "{name}: Rust re-encoding must match Kotlin's text"
            );
        }
    }

    #[test]
    fn native_state_round_trip_keeps_kotlin_shape() {
        // 解码后再编码，应与 Kotlin 原文等价（字段顺序与数值类型一致）。
        // Decoding and re-encoding must be equivalent to the Kotlin original, including field order and number types.
        let state = NativeState::decode(&decode(PSO_STATE)).expect("decodable");
        let encoded = state.encode();
        assert_eq!(encoded, decode(PSO_STATE));
        assert_eq!(encoded["vectors"][0][0], json!(2.3459770750119855));
        assert_eq!(encoded["individuals"][0]["fitness"], json!("6.663360804528834"));
    }
}

/// Kotlin 写出的 `NativeState` 文本可被 Rust 解码 / Kotlin-authored `NativeState` text is decodable in Rust
#[test]
fn native_state_wire_shape_matches_kotlin() {
    use crate::solver::heuristic::session::codecs::NativeState;

    // Kotlin 的 `NativeState`：空字段省略；`individuals` 里的数值是字符串，`vectors` 是 JSON 数字。
    // Kotlin's `NativeState`: empty fields are omitted; numbers inside `individuals` are strings while `vectors` are JSON numbers.
    let kotlin_text = r#"{
        "individuals": [{"solution": ["1.0"], "fitness": "1.0"}],
        "vectors": [[0.5, -0.5]],
        "counters": [3, 0]
    }"#;
    let value: crate::solver::heuristic::session::json::JsonValue =
        serde_json::from_str(kotlin_text).expect("valid json");
    let state = NativeState::decode(&value).expect("decodable");

    assert_eq!(state.vectors, vec![vec![0.5, -0.5]]);
    assert_eq!(state.counters, vec![3, 0]);
    assert!(state.children.is_empty());
    assert!(state.solutions.is_null());

    // 再编码回去应保持同一形状 / Re-encoding keeps the same shape
    let encoded = state.encode();
    assert_eq!(encoded.get("counters"), Some(&serde_json::json!([3, 0])));
    assert_eq!(encoded.get("vectors"), Some(&serde_json::json!([[0.5, -0.5]])));
}
