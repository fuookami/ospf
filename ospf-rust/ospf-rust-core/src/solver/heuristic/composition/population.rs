//! 种群类算法组合 / Population algorithm compositions
//!
//! 一次组合就是「选骨架 + 填算子」：交叉、位置更新都走同一个 [`SolutionUpdate`] 槽位，因此 GA、DE、GWO、WOA 等
//! 算法共享同一套循环。
//! A composition is "pick a skeleton, fill the operators": crossover and position updates both occupy the same
//! [`SolutionUpdate`] slot, so GA, DE, GWO, WOA, and friends share one loop.

use crate::solver::heuristic::contract::RunnerSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::fitness::FitnessEvaluator;
use crate::solver::heuristic::operator::initializer::Initializer;
use crate::solver::heuristic::operator::migration::IslandMigration;
use crate::solver::heuristic::operator::mutation::Mutation;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::operator::survivor::SurvivorSelection;
use crate::solver::heuristic::operator::topology::Topology;
use crate::solver::heuristic::operator::update::SolutionUpdate;
use crate::solver::heuristic::runner::island_runner::IslandRunner;
use crate::solver::heuristic::runner::particle_swarm_runner::ParticleSwarmRunner;
use crate::solver::heuristic::runner::population_runner::PopulationRunner;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 种群类求解器类型别名 / Population solver type alias
pub type PopulationSolver<V> = RunnerSolver<PopulationRunner<V>>;

/// 粒子群求解器类型别名 / Particle swarm solver type alias
pub type ParticleSwarmSolver<V> = RunnerSolver<ParticleSwarmRunner<V>>;

/// 岛模型求解器类型别名 / Island solver type alias
pub type IslandSolver<V> = RunnerSolver<IslandRunner<V>>;

/// 种群算法配置 / Population algorithm configuration
pub struct PopulationConfig<V>
where
    V: HeuristicValue,
{
    /// 种群规模 / Population size
    pub size: usize,
    /// 变异率 / Mutation rate
    pub mutation_rate: f64,
    /// 档案容量 / Archive capacity
    pub archive_amount: usize,
    /// 终止判据 / Stopping criterion
    pub stopping: Box<dyn StoppingCriterion>,
    /// 熵源 / Entropy source
    pub entropy: Box<dyn Entropy>,
    /// 解更新算子 / Solution update operator
    pub update: Box<dyn SolutionUpdate<V>>,
    /// 变异算子 / Mutation operator
    pub mutation: Box<dyn Mutation<V>>,
    /// 环境选择算子 / Survivor selection operator
    pub survivor: Box<dyn SurvivorSelection<V, V>>,
    /// 邻域拓扑 / Neighbourhood topology
    pub topology: Box<dyn Topology>,
    /// 初始化算子 / Initializer
    pub initializer: Box<dyn Initializer<V>>,
}

impl<V> PopulationConfig<V>
where
    V: HeuristicValue,
{
    /// 创建配置 / Create a configuration
    #[expect(clippy::too_many_arguments, reason = "保留既有公开算子组合构造接口 / Preserve the existing public operator-composition constructor")]
    pub fn new(
        size: usize,
        stopping: Box<dyn StoppingCriterion>,
        entropy: Box<dyn Entropy>,
        update: Box<dyn SolutionUpdate<V>>,
        mutation: Box<dyn Mutation<V>>,
        survivor: Box<dyn SurvivorSelection<V, V>>,
        topology: Box<dyn Topology>,
        initializer: Box<dyn Initializer<V>>,
    ) -> Self {
        Self {
            size,
            mutation_rate: 0.1,
            archive_amount: 1,
            stopping,
            entropy,
            update,
            mutation,
            survivor,
            topology,
            initializer,
        }
    }

    /// 设置变异率 / Set the mutation rate
    pub fn with_mutation_rate(mut self, mutation_rate: f64) -> Self {
        self.mutation_rate = mutation_rate;
        self
    }

    /// 设置档案容量 / Set the archive capacity
    pub fn with_archive_amount(mut self, archive_amount: usize) -> Self {
        self.archive_amount = archive_amount.max(1);
        self
    }
}

/// 组装种群类求解器 / Assemble a population solver
pub fn population<V>(name: &str, config: PopulationConfig<V>) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    let PopulationConfig {
        size,
        mutation_rate,
        archive_amount,
        stopping,
        entropy,
        update,
        mutation,
        survivor,
        topology,
        initializer,
    } = config;

    let core = RunnerCore::new(
        Box::new(BestArchive::new(archive_amount)),
        stopping,
    );
    RunnerSolver::new(
        name,
        PopulationRunner::new(size, update, mutation, survivor, entropy)
            .with_mutation_rate(mutation_rate)
            .with_topology(topology)
            .with_initializer(initializer)
            .with_core(core),
    )
}

/// 组装「位置更新」类算法 / Assemble a position-update algorithm
///
/// 位置更新类算法没有交叉与变异，一步迭代就是一次位置更新：位置更新占据 [`SolutionUpdate`] 槽位，变异换成
/// [`crate::solver::heuristic::operator::mutation::NoMutation`]，循环完全复用种群骨架。
/// Position-update algorithms have neither crossover nor mutation; one iteration is one position update: the update occupies
/// the [`SolutionUpdate`] slot, mutation becomes
/// [`crate::solver::heuristic::operator::mutation::NoMutation`], and the loop is the shared population skeleton.
pub fn position_update<V>(
    name: &str,
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    update: Box<dyn SolutionUpdate<V>>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    population(
        name,
        PopulationConfig::new(
            size,
            stopping,
            entropy,
            update,
            Box::new(crate::solver::heuristic::operator::mutation::NoMutation),
            Box::new(crate::solver::heuristic::operator::survivor::ElitistSurvivor),
            Box::new(crate::solver::heuristic::operator::topology::GlobalTopology),
            Box::new(crate::solver::heuristic::operator::initializer::ModelInitializer),
        ),
    )
}

/// 组装遗传算法 / Assemble a genetic algorithm
///
/// 默认使用单点交叉、高斯变异与精英环境选择，与 Kotlin 侧 `genetic` 的默认组合一致。
/// Defaults to single-point crossover, Gaussian mutation, and elitist survivor selection, matching the Kotlin `genetic`
/// defaults.
pub fn genetic<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    population(
        "genetic",
        PopulationConfig::new(
            size,
            stopping,
            entropy,
            Box::new(crate::solver::heuristic::operator::update::SinglePointCrossover),
            Box::new(crate::solver::heuristic::operator::mutation::GaussianMutation::default()),
            Box::new(crate::solver::heuristic::operator::survivor::ElitistSurvivor),
            Box::new(crate::solver::heuristic::operator::topology::GlobalTopology),
            Box::new(crate::solver::heuristic::operator::initializer::ModelInitializer),
        ),
    )
}

/// 粒子群配置 / Particle swarm configuration
pub struct SwarmConfig<V>
where
    V: HeuristicValue,
{
    /// 粒子数量 / Particle amount
    pub size: usize,
    /// 终止判据 / Stopping criterion
    pub stopping: Box<dyn StoppingCriterion>,
    /// 熵源 / Entropy source
    pub entropy: Box<dyn Entropy>,
    /// 初始化算子 / Initializer
    pub initializer: Box<dyn Initializer<V>>,
    /// 适应度评估算子 / Fitness evaluator
    pub fitness_evaluator: Option<Box<dyn FitnessEvaluator<V>>>,
}

impl<V> SwarmConfig<V>
where
    V: HeuristicValue,
{
    /// 创建粒子群配置 / Create a particle swarm configuration
    pub fn new(
        size: usize,
        stopping: Box<dyn StoppingCriterion>,
        entropy: Box<dyn Entropy>,
        initializer: Box<dyn Initializer<V>>,
    ) -> Self {
        Self {
            size,
            stopping,
            entropy,
            initializer,
            fitness_evaluator: None,
        }
    }
}

/// 组装粒子群求解器 / Assemble a particle swarm solver
pub fn particle_swarm<V>(name: &str, config: SwarmConfig<V>) -> ParticleSwarmSolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(Box::new(BestArchive::new(1)), config.stopping);
    let mut runner = ParticleSwarmRunner::new(config.size, config.entropy)
        .with_initializer(config.initializer)
        .with_core(core);
    if let Some(evaluator) = config.fitness_evaluator {
        runner = runner.with_fitness_evaluator(evaluator);
    }
    RunnerSolver::new(name, runner)
}

/// 岛模型配置 / Island-model configuration
pub struct IslandConfig<V>
where
    V: HeuristicValue,
{
    /// 各岛屿的种群骨架 / Population skeleton of each island
    pub islands: Vec<PopulationRunner<V>>,
    /// 迁移周期 / Migration period
    pub migration_period: u64,
    /// 迁移算子 / Migration operator
    pub migration: Box<dyn IslandMigration<V, V>>,
    /// 终止判据 / Stopping criterion
    pub stopping: Box<dyn StoppingCriterion>,
    /// 熵源 / Entropy source
    pub entropy: Box<dyn Entropy>,
}

/// 组装岛模型求解器 / Assemble an island solver
pub fn island<V>(name: &str, config: IslandConfig<V>) -> IslandSolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(Box::new(BestArchive::new(1)), config.stopping);
    RunnerSolver::new(
        name,
        IslandRunner::new(config.islands, config.entropy)
            .with_migration_period(config.migration_period)
            .with_migration(config.migration)
            .with_core(core),
    )
}
