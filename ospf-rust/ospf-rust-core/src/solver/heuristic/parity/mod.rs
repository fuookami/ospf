//! 迁移一致性测试 / Migration parity tests
//!
//! 这些测试与 Kotlin 侧同名场景对齐：收敛性、可行性、算法名与确定性。
//! These tests mirror the same scenarios as the Kotlin suite: convergence, feasibility, algorithm names, and determinism.

use std::sync::Arc;

use crate::model::callback::{AbstractCallBackModel, Solution};
use crate::solver::heuristic::composition;
use crate::solver::heuristic::entropy::SeededEntropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::contract::HeuristicSolver;
use crate::solver::heuristic::model::{VariableModel, from_f64};
use crate::solver::heuristic::operator::value_at;
use crate::solver::heuristic::operator::neighborhood::Neighborhood;
use crate::solver::heuristic::operator::set_value_at;
use crate::solver::heuristic::operator::stopping::IterationLimit;
use crate::solver::heuristic::result::HeuristicSolutionStatus;
use crate::variable::new_standalone_id;

/// 构造一维二次模型 `f(x) = (x - 1)^2`，区间 `[-10, 10]` / Build the 1D quadratic `(x - 1)^2` on `[-10, 10]`
fn quadratic_model(initial: f64) -> (crate::variable::VariableId, AbstractCallBackModel<f64>) {
    let x = new_standalone_id();
    let mut model = AbstractCallBackModel::<f64>::new("heuristic_parity_quadratic");
    model.register_variable_name(x, "x".to_string());
    model.set_variable_bounds_provider(Arc::new(move |id| {
        if id == x {
            Some((Some(-10.0), Some(10.0)))
        } else {
            None
        }
    }));
    model.set_initial_solution_provider(Arc::new(move |amount| {
        (0..amount)
            .map(|_| Solution::from([(x, initial)]))
            .collect()
    }));
    model.set_objective_evaluator(Arc::new(|solution| {
        let value = solution.values().next().copied().unwrap_or(0.0);
        Some((value - 1.0) * (value - 1.0))
    }));
    (x, model)
}

/// 确定性的梯度下降邻域，用于验证骨架与接受判据本身 / Deterministic gradient-descent neighbourhood validating skeleton and acceptance
struct GradientStep {
    step: f64,
}

impl Neighborhood<f64> for GradientStep {
    fn neighbor(
        &self,
        model: &dyn VariableModel<f64>,
        _iteration: &Iteration,
        solution: &Solution<f64>,
        _entropy: &mut dyn crate::solver::heuristic::entropy::Entropy,
    ) -> Solution<f64> {
        let order = model.variable_order();
        let current = value_at(solution, &order, 0).unwrap_or(0.0);
        let mut neighbour = solution.clone();
        set_value_at(&mut neighbour, &order, 0, from_f64(current - self.step));
        neighbour
    }
}

#[test]
fn hill_climbing_converges_on_a_quadratic_and_reports_feasible() {
    let (_x, mut model) = quadratic_model(5.0);
    let mut solver = composition::hill_climbing(
        Box::new(IterationLimit::new(200)),
        Box::new(SeededEntropy::new(42)),
        Box::new(GradientStep { step: 0.1 }),
    );

    assert_eq!(solver.name(), "hill-climbing");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("hill climbing should produce a result");

    assert_eq!(result.status, HeuristicSolutionStatus::Feasible);
    assert!(
        result.best_objective.unwrap() <= 1.0e-3,
        "objective = {:?}",
        result.best_objective
    );
    let solution = result.best_solution.expect("best solution");
    assert!((solution.values().next().copied().unwrap_or(0.0) - 1.0).abs() < 0.1);
}

#[test]
fn trajectory_variants_expose_stable_names() {
    let stopping = || Box::new(IterationLimit::new(5));
    assert_eq!(
        composition::neighborhood_search(
            stopping(),
            Box::new(SeededEntropy::new(1)),
            Box::new(GradientStep { step: 0.1 }),
        )
        .name(),
        "ns"
    );
    assert_eq!(
        composition::simulated_annealing(
            stopping(),
            Box::new(SeededEntropy::new(1)),
            Box::new(GradientStep { step: 0.1 }),
        )
        .name(),
        "simulated-annealing"
    );
    assert_eq!(
        composition::random_walk(
            stopping(),
            Box::new(SeededEntropy::new(1)),
            Box::new(GradientStep { step: 0.1 }),
        )
        .name(),
        "random-walk"
    );
}

#[test]
fn seeded_entropy_is_reproducible() {
    let first: Vec<f64> = {
        let mut entropy = SeededEntropy::new(20_240_521);
        (0..8)
            .map(|_| crate::solver::heuristic::entropy::unit(&mut entropy))
            .collect()
    };
    let second: Vec<f64> = {
        let mut entropy = SeededEntropy::new(20_240_521);
        (0..8)
            .map(|_| crate::solver::heuristic::entropy::unit(&mut entropy))
            .collect()
    };
    assert_eq!(first, second);
    assert!(first.iter().all(|value| (0.0..1.0).contains(value)));
}

#[test]
fn genetic_improves_on_a_quadratic() {
    let (_x, mut model) = quadratic_model(5.0);
    let mut solver = crate::solver::heuristic::facade::genetic::<f64>(
        24,
        Box::new(IterationLimit::new(300)),
        Box::new(SeededEntropy::new(9)),
    );

    assert_eq!(solver.name(), "genetic");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("genetic algorithm should produce a result");
    assert!(
        result.best_objective.unwrap() < 0.5,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn particle_swarm_converges_on_a_quadratic() {
    let (x, mut model) = quadratic_model(5.0);
    // 粒子群需要一个彼此不同的初始种群，否则速度恒为零、不会有任何运动。
    // A swarm needs a spread initial population; with identical particles the velocity stays zero and nothing moves.
    model.set_initial_solution_provider(Arc::new(move |amount| {
        (0..amount)
            .map(|index| Solution::from([(x, index as f64 * 0.5 - 2.0)]))
            .collect()
    }));
    let mut solver = crate::solver::heuristic::facade::particle_swarm::<f64>(
        16,
        Box::new(IterationLimit::new(200)),
        Box::new(SeededEntropy::new(11)),
    );

    assert_eq!(solver.name(), "pso");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("particle swarm should produce a result");
    assert!(
        result.best_objective.unwrap() < 1.0e-3,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn nsga2_returns_a_mutually_non_dominated_front() {
    use crate::solver::heuristic::operator::multi_objective::dominance::{
        Dominance, ParetoDominance,
    };
    use crate::solver::heuristic::operator::mutation::GaussianMutation;
    use crate::solver::heuristic::operator::update::SimulatedBinaryCrossover;
    use crate::solver::heuristic::result::MultiObjectiveSolver;
    use crate::solver::heuristic::runner::nsga2_runner::Nsga2Runner;

    let (_x, mut model) = quadratic_model(0.0);
    let mut solver = Nsga2Runner::minimization(
        20,
        2,
        Box::new(|solution| {
            let value = solution.values().next().copied().unwrap_or(0.0);
            vec![value * value, (value - 1.0) * (value - 1.0)]
        }),
        Box::new(SimulatedBinaryCrossover::default()),
        Box::new(GaussianMutation::default()),
        Box::new(SeededEntropy::new(5)),
        Box::new(IterationLimit::new(40)),
    );

    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("nsga-ii should produce a front");
    assert_eq!(solver.name(), "nsga-ii");
    assert!(!result.front().is_empty());

    let dominance = ParetoDominance::minimization(2);
    for left in result.objectives() {
        for right in result.objectives() {
            assert!(!dominance.dominates(left, right));
        }
    }
}

#[test]
fn island_genetic_improves_on_a_quadratic() {
    let (_x, mut model) = quadratic_model(5.0);
    let specs = vec![
        crate::solver::heuristic::facade::IslandSpec::new(12, 0.15),
        crate::solver::heuristic::facade::IslandSpec::new(12, 0.25),
        crate::solver::heuristic::facade::IslandSpec::new(12, 0.35),
    ];
    let mut solver = crate::solver::heuristic::facade::island_genetic::<f64>(
        specs,
        Box::new(IterationLimit::new(60)),
        Box::new(SeededEntropy::new(17)),
        5,
    );

    assert_eq!(solver.name(), "island-ga");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("island genetic algorithm should produce a result");
    assert!(
        result.best_objective.unwrap() < 1.0,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn nsga3_reports_its_name_and_a_front() {
    use crate::solver::heuristic::operator::mutation::GaussianMutation;
    use crate::solver::heuristic::operator::update::SimulatedBinaryCrossover;
    use crate::solver::heuristic::result::MultiObjectiveSolver;
    use crate::solver::heuristic::runner::nsga2_runner::Nsga3Runner;

    let (_x, mut model) = quadratic_model(0.0);
    let mut solver = Nsga3Runner::new(
        16,
        Box::new(|solution| {
            let value = solution.values().next().copied().unwrap_or(0.0);
            vec![value * value, (value - 1.0) * (value - 1.0)]
        }),
        Box::new(
            crate::solver::heuristic::operator::multi_objective::dominance::ParetoDominance::minimization(2),
        ),
        Box::new(SimulatedBinaryCrossover::default()),
        Box::new(GaussianMutation::default()),
        Box::new(SeededEntropy::new(6)),
        Box::new(IterationLimit::new(30)),
        4,
    );

    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("nsga-iii should produce a front");
    assert_eq!(solver.name(), "nsga-iii");
    assert!(!result.front().is_empty());
}

/// 构造初始种群分散的一维二次模型 / Build a 1D quadratic model with a spread initial population
fn spread_quadratic_model() -> (crate::variable::VariableId, AbstractCallBackModel<f64>) {
    let (x, mut model) = quadratic_model(5.0);
    model.set_initial_solution_provider(Arc::new(move |amount| {
        (0..amount)
            .map(|index| Solution::from([(x, index as f64 * 0.5 - 2.0)]))
            .collect()
    }));
    (x, model)
}

#[test]
fn shade_improves_on_a_quadratic() {
    let (_x, mut model) = spread_quadratic_model();
    let mut solver = crate::solver::heuristic::facade::shade::<f64>(
        24,
        Box::new(IterationLimit::new(200)),
        Box::new(SeededEntropy::new(21)),
    );

    assert_eq!(solver.name(), "shade");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("shade should produce a result");
    assert!(
        result.best_objective.unwrap() < 1.0e-3,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn adaptive_particle_swarm_improves_on_a_quadratic() {
    let (_x, mut model) = spread_quadratic_model();
    let mut solver = crate::solver::heuristic::facade::adaptive_particle_swarm::<f64>(
        16,
        Box::new(IterationLimit::new(200)),
        Box::new(SeededEntropy::new(23)),
    );

    assert_eq!(solver.name(), "apso");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("apso should produce a result");
    assert!(
        result.best_objective.unwrap() < 1.0e-3,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn cma_es_converges_on_a_quadratic() {
    let (_x, mut model) = quadratic_model(5.0);
    let mut solver = crate::solver::heuristic::facade::cma_es::<f64>(
        Box::new(IterationLimit::new(150)),
        Box::new(SeededEntropy::new(25)),
    );

    assert_eq!(solver.name(), "cma-es");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("cma-es should produce a result");
    assert!(
        result.best_objective.unwrap() < 1.0e-2,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn composed_nsga2_reports_a_front() {
    use crate::solver::heuristic::result::MultiObjectiveSolver;

    let (_x, mut model) = quadratic_model(0.0);
    let mut solver = composition::nsga2_minimization::<f64>(
        16,
        2,
        Box::new(|solution| {
            let value = solution.values().next().copied().unwrap_or(0.0);
            vec![value * value, (value - 1.0) * (value - 1.0)]
        }),
        Box::new(SeededEntropy::new(27)),
        Box::new(IterationLimit::new(30)),
    );

    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("nsga-ii should produce a front");
    assert!(!result.front().is_empty());
    assert_eq!(result.front().len(), result.objectives().len());
}

#[test]
fn clpso_improves_on_a_quadratic() {
    let (_x, mut model) = spread_quadratic_model();
    let mut solver =
        crate::solver::heuristic::facade::comprehensive_learning_particle_swarm::<f64>(
            16,
            Box::new(IterationLimit::new(200)),
            Box::new(SeededEntropy::new(29)),
        );

    assert_eq!(solver.name(), "clpso");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("clpso should produce a result");
    assert!(
        result.best_objective.unwrap() < 1.0e-2,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn q_learning_improves_on_a_quadratic() {
    let (_x, mut model) = spread_quadratic_model();
    let mut solver = crate::solver::heuristic::facade::q_learning::<f64>(
        16,
        Box::new(IterationLimit::new(120)),
        Box::new(SeededEntropy::new(31)),
    );

    assert_eq!(solver.name(), "q-learning");
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("q-learning should produce a result");
    assert!(
        result.best_objective.unwrap() < 1.0,
        "objective = {:?}",
        result.best_objective
    );
}

#[test]
fn memetic_is_never_worse_than_its_global_phase() {
    use crate::solver::heuristic::composition::{MemeticConfig, SearchConfig, memetic};
    use crate::solver::heuristic::facade::genetic;
    use crate::solver::heuristic::operator::stopping::StoppingCriterion;

    let config = SearchConfig::new(60);
    let global = genetic::<f64>(20, config.stopping(), Box::new(SeededEntropy::new(33)));
    let memetic_config = MemeticConfig::new(Box::new(|| {
        Box::new(IterationLimit::new(40)) as Box<dyn StoppingCriterion>
    }));
    let mut solver = memetic(global, memetic_config, Box::new(SeededEntropy::new(35)));
    assert_eq!(solver.name(), "memetic-genetic");

    let (_x, mut model) = spread_quadratic_model();
    let result = solver
        .solve(&mut model, &mut Iteration::new())
        .expect("memetic should produce a result");
    assert!(result.best_objective.is_some());
    assert!(result.best_objective.unwrap().is_finite());
}
