//! 实例隔离训练、冻结评估与轨迹重放 / Instance-isolated training, frozen evaluation, and trajectory replay

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::archive::ObjectiveComponents;
use crate::solver::hyper_heuristic::config::{ComponentSpec, HyperHeuristicConfig, LearningMode};
use crate::solver::hyper_heuristic::persistence::{PolicyArchive, atomic_write};
use crate::solver::hyper_heuristic::registry::SearchRegistry;
use crate::solver::hyper_heuristic::selection::{SelectionPolicy, SelectionTransition};
use crate::solver::hyper_heuristic::solver::{HyperHeuristicSolver, RunOptions, SearchResult};
use crate::solver::hyper_heuristic::{boundary, invalid};

/// 一个独立的问题实例 / An independent problem instance
#[derive(Debug, Clone)]
pub struct TrainingCase {
    /// 稳定实例 ID / Stable instance ID
    pub id: String,
    /// 显式问题工厂与输入 / Explicit problem factory and input
    pub problem: ComponentSpec,
}

/// 必须先按问题实例划分，再分配随机种子 / Split problem instances before assigning random seeds
#[derive(Debug, Clone, Default)]
pub struct TrainingSplit {
    /// 允许学习的实例 / Instances eligible for learning
    pub training: Vec<TrainingCase>,
    /// 冻结验证实例 / Frozen validation instances
    pub validation: Vec<TrainingCase>,
    /// 冻结测试实例 / Frozen test instances
    pub test: Vec<TrainingCase>,
}

impl TrainingSplit {
    /// 拒绝重复 ID 或跨集合重复的问题定义 / Reject duplicate IDs or repeated problem definitions across splits
    pub fn validate(&self) -> Result<()> {
        let mut ids = BTreeSet::new();
        let mut definitions = Vec::new();
        for (group, cases) in [&self.training, &self.validation, &self.test].iter().enumerate() {
            for case in *cases {
                if case.id.is_empty() || !ids.insert(&case.id) { return Err(invalid("duplicate or empty training instance ID")); }
                if definitions.iter().any(|(previous, spec)| *previous != group && *spec == &case.problem) {
                    return Err(invalid("problem instance leaks across training splits"));
                }
                definitions.push((group, &case.problem));
            }
        }
        if self.training.is_empty() { return Err(invalid("training instances are required")); }
        Ok(())
    }
}

/// 一次实例/种子运行的观察 / Observation for one instance/seed run
#[derive(Debug, Clone)]
pub struct TrainingObservation<O, V> {
    /// 问题实例 ID / Problem instance ID
    pub instance: String,
    /// 随机种子 / Random seed
    pub seed: i64,
    /// 包含失败前成本与有效档案的结果 / Result including costs and valid archive before failure
    pub result: Option<SearchResult<O, V>>,
    /// 失败诊断 / Failure diagnostic
    pub failure: Option<String>,
}

/// 分离训练与留出集的报告 / Report separating training from holdouts
pub struct TrainingReport<O, V> {
    /// 最终学习策略 / Final learned policy
    pub policy: PolicyArchive,
    /// 训练观察 / Training observations
    pub training: Vec<TrainingObservation<O, V>>,
    /// 冻结验证观察 / Frozen validation observations
    pub validation: Vec<TrainingObservation<O, V>>,
    /// 冻结测试观察 / Frozen test observations
    pub test: Vec<TrainingObservation<O, V>>,
}

/// 实例级表格策略训练器 / Instance-level tabular policy trainer
pub struct PolicyTrainer<O, V> { registry: SearchRegistry<O, V> }

impl<O: ObjectiveComponents + 'static, V: HeuristicValue> PolicyTrainer<O, V> {
    /// 绑定显式组件工厂 / Bind explicit component factories
    pub fn new(registry: SearchRegistry<O, V>) -> Self { Self { registry } }

    /// 在训练实例上延续策略，留出实例始终冻结 / Continue learning over training instances while always freezing holdouts
    pub fn train(
        &self,
        config: &HyperHeuristicConfig,
        split: &TrainingSplit,
        seeds: &[i64],
        initial: Option<&PolicyArchive>,
    ) -> Result<TrainingReport<O, V>> {
        split.validate()?;
        if seeds.is_empty() { return Err(invalid("explicit training seeds are required")); }
        // 指纹去重覆盖不同 ID 和不同参数别名下的同一实例 / Fingerprints detect aliases of the same instance under different IDs or parameters
        let mut fingerprints: BTreeMap<String, usize> = BTreeMap::new();
        for (group, cases) in [&split.training, &split.validation, &split.test].iter().enumerate() {
            for case in *cases {
                use crate::solver::heuristic::session::SearchProblem;
                let problem = self.registry.problem(&case.problem, crate::solver::heuristic::session::SearchRandom::new(0))?;
                if fingerprints.get(problem.fingerprint()).is_some_and(|previous| *previous != group) {
                    return Err(invalid("problem fingerprint leaks across training splits"));
                }
                fingerprints.insert(problem.fingerprint().into(), group);
            }
        }
        let mut policy = initial.cloned();
        let mut training = Vec::new();
        for case in &split.training {
            for seed in seeds {
                let mut episode = config.clone(); episode.problem = case.problem.clone(); episode.execution.seed = *seed;
                episode.learning.mode = if policy.is_some() { LearningMode::Continue } else { LearningMode::Train };
                let (observation, learned) = observe(&self.registry, episode, case, *seed, policy.as_ref())?;
                if let Some(learned) = learned { policy = Some(learned); }
                training.push(observation);
            }
        }
        let policy = policy.ok_or_else(|| invalid("training produced no usable policy"))?;
        let evaluator = PolicyEvaluator::new(self.registry.clone());
        let validation = evaluator.evaluate(config, &split.validation, seeds, Some(&policy))?;
        let test = evaluator.evaluate(config, &split.test, seeds, Some(&policy))?;
        Ok(TrainingReport { policy, training, validation, test })
    }

    /// 仅重放显式选择的轨迹作用域，支持 Q 离策略转移 / Replay only the selected scope, including off-policy Q transitions
    pub fn replay(
        policy: &mut dyn SelectionPolicy,
        transitions: &[SelectionTransition],
        scope: &str,
    ) -> Result<()> {
        if scope.is_empty() { return Err(invalid("an explicit trajectory scope is required")); }
        for transition in transitions.iter().filter(|transition| transition.scope == scope) {
            if transition.feature_version != "progress:1" { return Err(invalid("incompatible trajectory feature version")); }
            policy.observe(transition)?;
        }
        Ok(())
    }
}

/// 公平基线比较的质量与成本报告 / Quality and cost report for fair baseline comparison
pub struct PolicyEvaluationReport<O, V> {
    /// 原始目标档案和成本观察 / Raw objective archives and cost observations
    pub observations: Vec<TrainingObservation<O, V>>,
    /// 调用方定义的质量样本 / Caller-defined quality samples
    pub quality: Vec<f64>,
    /// 失败率 / Failure rate
    pub failure_rate: f64,
    /// 总评估请求 / Total evaluation requests
    pub evaluations: u64,
    /// 总有效运行时间 / Total active runtime
    pub active_nanos: u64,
}

/// 冻结评估与等预算基线比较 / Frozen evaluation and equal-budget baseline comparison
pub struct PolicyEvaluator<O, V> { registry: SearchRegistry<O, V> }

impl<O: ObjectiveComponents + 'static, V: HeuristicValue> PolicyEvaluator<O, V> {
    /// 绑定实例工厂 / Bind instance factories
    pub fn new(registry: SearchRegistry<O, V>) -> Self { Self { registry } }

    /// 每个 episode 使用独立模型与冻结策略 / Use independent models and frozen policies for every episode
    pub fn evaluate(
        &self,
        config: &HyperHeuristicConfig,
        cases: &[TrainingCase],
        seeds: &[i64],
        policy: Option<&PolicyArchive>,
    ) -> Result<Vec<TrainingObservation<O, V>>> {
        if seeds.is_empty() { return Err(invalid("explicit evaluation seeds are required")); }
        let mut observations = Vec::new();
        for case in cases {
            for seed in seeds {
                let mut episode = config.clone(); episode.problem = case.problem.clone(); episode.execution.seed = *seed;
                episode.learning.mode = LearningMode::Frozen;
                observations.push(observe(&self.registry, episode, case, *seed, policy)?.0);
            }
        }
        Ok(observations)
    }

    /// 在同一显式评估额度下比较命名基线；多目标质量由调用方定义 / Compare named baselines under equal explicit evaluation limits; callers define multi-objective quality
    pub fn compare(
        &self,
        baselines: &BTreeMap<String, HyperHeuristicConfig>,
        cases: &[TrainingCase],
        seeds: &[i64],
        quality: impl Fn(&SearchResult<O, V>) -> f64,
    ) -> Result<BTreeMap<String, PolicyEvaluationReport<O, V>>> {
        let allowance = baselines.values().next().and_then(|config| config.budget.evaluations)
            .ok_or_else(|| invalid("baselines require an explicit evaluation allowance"))?;
        if baselines.values().any(|config| config.budget.evaluations != Some(allowance)) { return Err(invalid("baseline evaluation allowances must match")); }
        let mut reports = BTreeMap::new();
        for (name, config) in baselines {
            let observations = self.evaluate(config, cases, seeds, None)?;
            let mut samples = Vec::new();
            let mut evaluations = 0_u64; let mut active_nanos = 0_u64;
            for observation in &observations {
                if let Some(result) = &observation.result {
                    evaluations = evaluations.saturating_add(result.evaluations);
                    active_nanos = active_nanos.saturating_add(result.active_nanos);
                    if observation.failure.is_none() {
                        let sample = boundary(|| Ok(quality(result)))?;
                        if !sample.is_finite() { return Err(invalid("quality metric must be finite")); }
                        samples.push(sample);
                    }
                }
            }
            let failure_rate = observations.iter().filter(|observation| observation.failure.is_some()).count() as f64 / observations.len().max(1) as f64;
            reports.insert(name.clone(), PolicyEvaluationReport { observations, quality: samples, failure_rate, evaluations, active_nanos });
        }
        Ok(reports)
    }
}

fn observe<O: ObjectiveComponents + 'static, V: HeuristicValue>(
    registry: &SearchRegistry<O, V>,
    config: HyperHeuristicConfig,
    case: &TrainingCase,
    seed: i64,
    policy: Option<&PolicyArchive>,
) -> Result<(TrainingObservation<O, V>, Option<PolicyArchive>)> {
    let solver = HyperHeuristicSolver::new(config, registry.clone())?;
    let mut run = match solver.open_with(RunOptions { policy, checkpoint: None }) {
        Ok(run) => run,
        Err(error) => return Ok((TrainingObservation { instance: case.id.clone(), seed, result: None, failure: Some(error.to_string()) }, None)),
    };
    let execution = run.run();
    let result = run.result();
    let learned = run.export_policy()?;
    let closed = run.close();
    let failure = execution.err().or_else(|| closed.err()).map(|error| error.to_string());
    Ok((TrainingObservation { instance: case.id.clone(), seed, result: Some(result), failure }, Some(learned)))
}

/// 原子 JSON Lines 轨迹存储 / Atomic JSON Lines trajectory storage
pub struct TrajectoryFiles;

impl TrajectoryFiles {
    /// 写入完整轨迹并原子替换 / Write the complete trajectory and atomically replace its destination
    pub fn write(path: impl AsRef<Path>, transitions: &[SelectionTransition]) -> Result<()> {
        let mut bytes = Vec::new();
        for transition in transitions {
            serde_json::to_writer(&mut bytes, transition).map_err(|error| invalid(error.to_string()))?;
            bytes.push(b'\n');
            if bytes.len() > 64 * 1024 * 1024 { return Err(invalid("trajectory exceeds 64 MiB")); }
        }
        atomic_write(path.as_ref(), &bytes)
    }

    /// 完整读取，拒绝损坏或截断的记录 / Read complete records, rejecting corruption or truncation
    pub fn read(path: impl AsRef<Path>) -> Result<Vec<SelectionTransition>> {
        let mut text = String::new();
        File::open(path).map_err(|error| invalid(error.to_string()))?.take(64 * 1024 * 1024 + 1)
            .read_to_string(&mut text).map_err(|error| invalid(error.to_string()))?;
        if text.len() > 64 * 1024 * 1024 { return Err(invalid("trajectory exceeds 64 MiB")); }
        text.lines().filter(|line| !line.trim().is_empty()).map(|line| serde_json::from_str(line).map_err(|error| invalid(error.to_string()))).collect()
    }
}
