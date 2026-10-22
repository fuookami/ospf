//! 配置、双层学习、并行与恢复示例 / Configuration, two-level learning, parallelism, and recovery example

use ospf_rust_core::model::callback::Solution;
use ospf_rust_core::solver::heuristic::session::SearchProblem;
use ospf_rust_core::solver::hyper_heuristic::{
    AlgorithmConfig, ComponentSpec, ConfigurationFormats, ExecutionMode, HyperHeuristicConfig,
    HyperHeuristicSolver, ImportMode, RunOptions, SearchRegistry,
};
use ospf_rust_core::variable::{VariableId, new_standalone_id};

struct Quadratic { x: VariableId }

impl SearchProblem<f64, f64> for Quadratic {
    fn id(&self) -> &str { "quadratic" }
    fn fingerprint(&self) -> &str { "quadratic:min:(x-1)^2:bounds[-10,10]:v1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn lower_bound(&self, _: usize) -> Option<f64> { Some(-10.0) }
    fn upper_bound(&self, _: usize) -> Option<f64> { Some(10.0) }

    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        (0..amount).map(|index| Solution::from([(self.x, 2.0 + 6.0 * index as f64 / amount.max(1) as f64)])).collect()
    }

    fn objective(&self, solution: &Solution<f64>) -> Option<f64> { Some((solution[&self.x] - 1.0).powi(2)) }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn valid_objective(&self, value: &f64) -> bool { value.is_finite() }
    fn feasible(&self, solution: &Solution<f64>) -> bool {
        solution.get(&self.x).is_some_and(|value| value.is_finite() && (-10.0..=10.0).contains(value))
    }
    fn stateless(&self) -> bool { true }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = SearchRegistry::with_builtins();
    // 每次工厂调用都拥有新的变量身份和回调实例 / Every factory call owns fresh variable identities and callbacks
    registry.register_problem("quadratic", |spec, _| {
        if spec.version != 1 || !spec.parameters.is_empty() { return Err(
            ospf_rust_core::solver::heuristic::result::HeuristicError::InvalidArgument("unsupported quadratic definition".into())
        ); }
        Ok(Box::new(Quadratic { x: new_standalone_id() }))
    })?;

    let mut genetic = AlgorithmConfig::new("ga", "genetic");
    genetic.size = 8;
    genetic.updates = vec![ComponentSpec::new("single-point"), ComponentSpec::new("uniform")];
    genetic.mutations = vec![ComponentSpec::new("gaussian"), ComponentSpec::new("uniform")];
    genetic.selection = ComponentSpec::new("ucb1");
    genetic.import_mode = ImportMode::Disabled;
    let mut swarm = AlgorithmConfig::new("swarm", "pso");
    swarm.size = 8;
    swarm.import_mode = ImportMode::Disabled;
    let mut config = HyperHeuristicConfig::new(ComponentSpec::new("quadratic"), 256)
        .algorithm(genetic).algorithm(swarm);
    config.execution.mode = ExecutionMode::Synchronous;
    config.execution.parallelism = 2;
    config.execution.seed = 42;
    config.selection = ComponentSpec::new("ucb1");
    config.budget.slice_evaluations = 32;
    config.persistence.enabled = true;

    let config = ConfigurationFormats::decode(&ConfigurationFormats::encode(&config)?)?;
    let solver = HyperHeuristicSolver::new(config, registry)?;
    let mut original = solver.open()?;
    original.advance()?;
    let checkpoint = original.checkpoint()?;
    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&checkpoint), policy: None })?;
    let expected = original.run()?;
    let actual = resumed.run()?;
    assert_eq!(expected.evaluations, actual.evaluations);
    assert_eq!(expected.best().map(|value| value.fitness), actual.best().map(|value| value.fitness));
    assert_eq!(original.export_policy()?, resumed.export_policy()?);
    println!("evaluations={}, objective_calls={}, decisions={}, steps={}",
        actual.evaluations, actual.objective_calls, actual.decisions, actual.steps);
    println!("best={:?}, termination={}", actual.best().map(|value| value.fitness), actual.termination);
    original.close()?;
    resumed.close()?;
    Ok(())
}
