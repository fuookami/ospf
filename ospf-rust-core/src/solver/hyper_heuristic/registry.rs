//! 显式组件注册与原生会话装配 / Explicit component registration and native session assembly

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use serde_json::Value;
use crate::solver::heuristic::entropy::SeededEntropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::acceptance::{AlwaysAcceptance, BetterOnlyAcceptance, MetropolisAcceptance};
use crate::solver::heuristic::operator::mutation::{GaussianMutation, NoMutation, PolynomialMutation};
use crate::solver::heuristic::operator::evolution::LinearSource;
use crate::solver::heuristic::operator::neighborhood::RandomPerturbation;
use crate::solver::heuristic::operator::survivor::ElitistSurvivor;
use crate::solver::heuristic::operator::update::*;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::*;
use crate::solver::heuristic::runner::native::NativeRunner;
use crate::solver::heuristic::session::*;
use crate::solver::heuristic::session::native::NativeHooks;
use crate::solver::hyper_heuristic::config::*;
use crate::solver::hyper_heuristic::operator_registry::OperatorRegistry;
use crate::solver::hyper_heuristic::components::{RewardPolicy, SelectorFactory};
use crate::solver::hyper_heuristic::invalid;
use crate::solver::hyper_heuristic::problem::{ProblemHandle, RandomHandle};
use crate::solver::hyper_heuristic::operators::{ControlledSession, OperatorBindings, PolicySession, UncontrolledSession};
use crate::variable::VariableId;

type EvaluatedCandidates = std::rc::Rc<std::cell::RefCell<Vec<crate::solver::heuristic::Individual<Vec<f64>, f64>>>>;

/// 工厂上下文，每次调用都创建新会话 / Factory context creating a new session on every invocation
pub struct AlgorithmContext<O, V> {
    /// 算法实例配置 / Algorithm instance configuration
    pub config: AlgorithmConfig,
    /// worker 独占的问题 / Worker-owned problem
    pub problem: ProblemHandle<O, V>,
    /// 骨架实际使用的可恢复流 / Restorable stream actually used by the skeleton
    pub random: RandomHandle,
}

type ProblemFactory<O, V> = Arc<dyn Fn(&ComponentSpec, SearchRandom) -> Result<Box<dyn SearchProblem<O, V>>> + Send + Sync>;
type AlgorithmFactory<O, V> = Arc<dyn Fn(AlgorithmContext<O, V>, bool, &OperatorRegistry<V>, &BTreeMap<String, SelectorFactory>, Box<dyn RewardPolicy>) -> Result<Box<dyn PolicySession<O, V>>> + Send + Sync>;
type ArchiveCodecFactory<O, V> = Arc<dyn Fn(Vec<VariableId>) -> ArchiveCodec<O, V> + Send + Sync>;
type ObjectiveCodecFactory<O> = Arc<dyn Fn() -> Box<dyn SearchCodec<O>> + Send + Sync>;
pub(crate) type Scalarization<O> = Arc<dyn Fn(&O) -> Result<f64> + Send + Sync>;
type ScalarAdapter<O, V> = Arc<dyn Fn(AlgorithmContext<O, V>, Scalarization<O>, bool, &OperatorRegistry<V>, &BTreeMap<String, SelectorFactory>, Box<dyn RewardPolicy>) -> Result<Box<dyn PolicySession<O, V>>> + Send + Sync>;

/// 注册表只持有工厂，不共享模型或会话实例 / Registry holding factories rather than shared models or sessions
pub struct SearchRegistry<O, V> {
    /// 高层独立组件工厂 / Independent upper-level component factories
    pub components: super::components::ComponentRegistry<O, V>,
    /// 类型安全算子独立工厂 / Independent type-safe operator factories
    pub operators: OperatorRegistry<V>,
    problems: BTreeMap<String, ProblemFactory<O, V>>,
    algorithms: BTreeMap<String, AlgorithmFactory<O, V>>,
    constrained_algorithms: BTreeSet<String>,
    custom_algorithms: BTreeSet<String>,
    archive_codec: Option<ArchiveCodecFactory<O, V>>,
    objective_codec: Option<ObjectiveCodecFactory<O>>,
    scalarizations: BTreeMap<String, Scalarization<O>>,
    scalar_adapter: Option<ScalarAdapter<O, V>>,
}

impl<O, V> Clone for SearchRegistry<O, V> {
    fn clone(&self) -> Self { Self { components: self.components.clone(), operators: self.operators.clone(), problems: self.problems.clone(), algorithms: self.algorithms.clone(), constrained_algorithms: self.constrained_algorithms.clone(), custom_algorithms: self.custom_algorithms.clone(), archive_codec: self.archive_codec.clone(),
        objective_codec: self.objective_codec.clone(), scalarizations: self.scalarizations.clone(), scalar_adapter: self.scalar_adapter.clone() } }
}

impl<O: Clone + 'static, V: HeuristicValue> Default for SearchRegistry<O, V> {
    fn default() -> Self { Self::new() }
}

impl<O: Clone + 'static, V: HeuristicValue> SearchRegistry<O, V> {
    /// 创建空注册表，供自定义值与目标类型扩展 / Create an empty registry for custom value and objective types
    pub fn new() -> Self { Self { components: super::components::ComponentRegistry::new(), operators: OperatorRegistry::new(), problems: BTreeMap::new(), algorithms: BTreeMap::new(), constrained_algorithms: BTreeSet::new(), custom_algorithms: BTreeSet::new(), archive_codec: None,
        objective_codec: None, scalarizations: BTreeMap::new(), scalar_adapter: None } }

    /// 注册显式无状态标量化，越小越优；其语义必须包含在问题指纹中 / Register a stateless smaller-is-better scalarization; include its semantics in the problem fingerprint
    pub fn register_scalarization(
        &mut self,
        id: impl Into<String>,
        projection: impl Fn(&O) -> Result<f64> + Send + Sync + 'static,
    ) -> Result<()> {
        let id = id.into();
        if id.is_empty() || self.scalarizations.contains_key(&id) { return Err(invalid("empty or duplicate scalarization ID")); }
        self.scalarizations.insert(id, Arc::new(projection));
        Ok(())
    }

    /// 设置无损档案编码工厂，变量身份按当前问题的规范顺序重建 / Set a lossless archive codec factory rebuilding identities in canonical order
    pub fn set_archive_codec(
        &mut self,
        factory: impl Fn(Vec<VariableId>) -> ArchiveCodec<O, V> + Send + Sync + 'static,
    ) {
        self.archive_codec = Some(Arc::new(factory));
    }

    pub(crate) fn archive_codec(&self, order: Vec<VariableId>) -> Option<ArchiveCodec<O, V>> {
        self.archive_codec.as_ref().map(|factory| factory(order))
    }

    /// 设置原始业务目标的无损编码，用于接受历史；不要求目标实现Serde / Set a lossless original-objective codec for acceptance history without requiring Serde
    pub fn set_objective_codec(&mut self, factory: impl Fn() -> Box<dyn SearchCodec<O>> + Send + Sync + 'static) {
        self.objective_codec = Some(Arc::new(factory));
    }

    pub(crate) fn objective_codec(&self) -> Option<Box<dyn SearchCodec<O>>> {
        self.objective_codec.as_ref().map(|factory| factory())
    }

    /// 注册独立问题工厂，不能返回共享模型 / Register an independent problem factory, never a shared model
    pub fn register_problem(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec, SearchRandom) -> Result<Box<dyn SearchProblem<O, V>>> + Send + Sync + 'static,
    ) -> Result<()> {
        let id = id.into();
        if id.is_empty() || self.problems.contains_key(&id) { return Err(invalid("duplicate or empty problem ID")); }
        self.problems.insert(id, Arc::new(factory));
        Ok(())
    }

    /// 注册独立会话工厂 / Register an independent session factory
    pub fn register_algorithm(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(AlgorithmContext<O, V>) -> Result<Box<dyn SearchSession<O, V>>> + Send + Sync + 'static,
    ) -> Result<()> {
        self.register_algorithm_with_constraints(id, false, factory)
    }

    /// 注册声明原生约束事务能力的自定义工厂 / Register a custom factory declaring native constraint transaction support
    pub fn register_algorithm_with_constraints(
        &mut self,
        id: impl Into<String>,
        supports_constraints: bool,
        factory: impl Fn(AlgorithmContext<O, V>) -> Result<Box<dyn SearchSession<O, V>>> + Send + Sync + 'static,
    ) -> Result<()> {
        let id = id.into();
        if id.is_empty() || self.algorithms.contains_key(&id) { return Err(invalid("duplicate or empty algorithm ID")); }
        if supports_constraints { self.constrained_algorithms.insert(id.clone()); }
        self.custom_algorithms.insert(id.clone());
        self.algorithms.insert(id, Arc::new(move |context, _, _, _, _| Ok(Box::new(UncontrolledSession(factory(context)?)))));
        Ok(())
    }

    /// 已注册算法 ID，顺序稳定 / Registered algorithm IDs in stable order
    pub fn algorithm_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.algorithms.keys().cloned().collect();
        if self.scalar_adapter.is_some() { ids.extend(BUILTIN_ALGORITHMS.iter().map(|id| (*id).into())); }
        ids.sort(); ids.dedup(); ids
    }

    pub(crate) fn problem(&self, spec: &ComponentSpec, random: SearchRandom) -> Result<ProblemHandle<O, V>> {
        if spec.version != 1 { return Err(invalid("unsupported problem component version")); }
        let factory = self.problems.get(&spec.r#type).ok_or_else(|| invalid(format!("unknown problem: {}", spec.r#type)))?;
        crate::solver::hyper_heuristic::boundary(|| factory(spec, random)).map(ProblemHandle::new)
    }

    #[cfg(test)]
    pub(crate) fn session(&self, context: AlgorithmContext<O, V>) -> Result<Box<dyn PolicySession<O, V>>>
    where
        O: crate::solver::heuristic::session::archive::ObjectiveComponents,
    {
        self.session_configured(context, true, &ComponentSpec::new("improvement"))
    }

    pub(crate) fn session_configured(
        &self,
        mut context: AlgorithmContext<O, V>,
        learning: bool,
        reward_spec: &ComponentSpec,
    ) -> Result<Box<dyn PolicySession<O, V>>>
    where
        O: crate::solver::heuristic::session::archive::ObjectiveComponents,
    {
        if ["island", "island-ga", "memetic"].contains(&context.config.algorithm.r#type.as_str()) {
            let config = &context.config;
            let sequential = config.algorithm.r#type == "memetic";
            let parameter = if sequential { "phaseSteps" } else { "migrationPeriod" };
            config.algorithm.validate_parameters(&[parameter])?;
            if config.import_mode == ImportMode::Disabled || !config.updates.is_empty() || !config.mutations.is_empty()
                || !config.neighborhoods.is_empty() || !config.parameters.is_empty() || config.scalarization.is_some() || config.constraints.is_some() {
                return Err(invalid("composites require native imports and child-local operator bindings"));
            }
            let period = config.algorithm.integer(parameter, 5)?;
            let mut children = Vec::new();
            for child in &config.children {
                let random = RandomHandle::new(SearchRandom::new(context.random.state() as u64).split(&child.id));
                let session = self.session_configured(AlgorithmContext { config: child.clone(), problem: context.problem.clone(), random: random.clone() }, learning, reward_spec)?;
                children.push((child.id.clone(), session, random));
            }
            let codec = self.archive_codec(context.problem.variable_order());
            return Ok(Box::new(super::scheduling::CompositeSearchSession::new(children, context.problem, sequential, period, codec)?));
        }
        if let Some(config) = &context.config.constraints {
            let scalar_builtin = self.scalar_adapter.is_some() && BUILTIN_ALGORITHMS.contains(&context.config.algorithm.r#type.as_str());
            if !scalar_builtin && !self.constrained_algorithms.contains(&context.config.algorithm.r#type) {
                return Err(invalid("algorithm factory does not support native constraints / 算法工厂不支持原生约束"));
            }
            let random = SearchRandom::new(context.random.state() as u64).split("constraints");
            context.problem = ProblemHandle::new(Box::new(ConstraintProblem::new(context.problem, config.clone(), random)?));
        }
        let reward = self.components.reward(reward_spec)?;
        reward.validate_mode(context.problem.multi_objective(), context.problem.has_objective_coordinates())?;
        if let Some(adapter) = &self.scalar_adapter
            && BUILTIN_ALGORITHMS.contains(&context.config.algorithm.r#type.as_str())
        {
            let id = context.config.scalarization.as_ref().ok_or_else(|| invalid("multi-objective scalar algorithms require an explicit scalarization"))?;
            let projection = self.scalarizations.get(id).ok_or_else(|| invalid("unknown scalarization"))?.clone();
            return super::boundary(|| adapter(context, projection, learning, &self.operators, &self.components.selectors, reward));
        }
        let factory = self.algorithms.get(&context.config.algorithm.r#type)
            .ok_or_else(|| invalid(format!("unknown algorithm: {}", context.config.algorithm.r#type)))?;
        if self.custom_algorithms.contains(&context.config.algorithm.r#type) {
            if !context.config.updates.is_empty() || !context.config.mutations.is_empty()
                || !context.config.neighborhoods.is_empty() || !context.config.parameters.is_empty() {
                return Err(invalid("custom factory does not expose operator bindings"));
            }
            if let Some(codec) = self.archive_codec(context.problem.variable_order()) {
                let problem = context.problem.clone();
                let local_reward = self.components.reward(reward_spec)?;
                let delegate = crate::solver::hyper_heuristic::boundary(|| factory(context, learning, &self.operators, &self.components.selectors, reward))?;
                // 自定义可恢复会话也保留控制层历史，与Kotlin普通注册协议一致 / Restorable custom sessions retain control-layer history, matching Kotlin's ordinary registration protocol.
                return Ok(ControlledSession::wrap_custom(delegate, problem, codec, local_reward));
            }
        }
        crate::solver::hyper_heuristic::boundary(|| factory(context, learning, &self.operators, &self.components.selectors, reward))
    }
}

/// 单目标内置注册入口；数值边界与既有原生 codec 一致 / Scalar built-in registration matching existing native codec boundaries
impl SearchRegistry<f64, f64> {
    /// 创建带内置原生算法的注册表 / Create a registry containing built-in native algorithms
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.set_archive_codec(|order| Box::new(IndividualListCodec::new(order)));
        registry.set_objective_codec(|| Box::new(TextNumberCodec));
        for id in BUILTIN_ALGORITHMS {
            registry.algorithms.insert((*id).into(), Arc::new(native_session));
            registry.constrained_algorithms.insert((*id).into());
        }
        registry
    }
}

impl SearchRegistry<Vec<f64>, f64> {
    /// 创建原始目标向量的 NSGA 注册表，不默认将目标求和 / Create a vector-objective NSGA registry without implicit objective summation
    pub fn with_multi_objective_builtins() -> Self {
        let mut registry = Self::new();
        registry.set_archive_codec(|order| Box::new(MultiObjectiveArchiveCodec::new(order)));
        registry.set_objective_codec(|| Box::new(ObjectiveVectorCodec));
        registry.scalar_adapter = Some(Arc::new(projected_session));
        for id in ["nsga-ii", "nsga-iii"] {
            registry.constrained_algorithms.insert(id.into());
            registry.algorithms.insert(id.into(), Arc::new(|context, learning, operators, selectors, reward| {
                let config = &context.config;
                config.algorithm.validate_parameters(&["divisions", "mutationRate"])?;
                let rate = config.algorithm.number("mutationRate", 0.1)?;
                if !(0.0..=1.0).contains(&rate) { return Err(invalid("invalid NSGA mutation rate")); }
                if !context.problem.multi_objective()
                    || config.scalarization.is_some() || !config.children.is_empty() {
                    return Err(invalid("NSGA requires a vector problem and compatible native operator slots"));
                }
                let divisions = config.algorithm.integer("divisions", 8)?;
                if divisions == 0 { return Err(invalid("reference divisions must be positive")); }
                let divisions = if config.algorithm.r#type == "nsga-iii" {
                    Some(usize::try_from(divisions).map_err(|_| invalid("reference divisions overflow"))?)
                } else { None };
                let problem = context.problem.clone();
                let dominance = Box::new(ProblemDominance(problem));
                OperatorBindings::validate_encoding(config, context.problem.encoding(), operators)?;
                let bindings = OperatorBindings::new(config, &SearchRandom::new(context.random.state() as u64), learning, operators, selectors, context.problem.encoding())?;
                let hooks = NsgaSessionHooks::new(config.size, dominance, bindings.update(Box::new(SimulatedBinaryCrossover::default())),
                    bindings.mutation(Box::new(PolynomialMutation::default())), Box::new(context.random), divisions).with_mutation_rate(rate);
                let order = context.problem.variable_order();
                let archive_codec = Box::new(MultiObjectiveArchiveCodec::new(order.clone()));
                let codec = SessionCodecs::new(Some(Box::new(MultiObjectiveArchiveCodec::new(order.clone()))),
                    Some(Box::new(MultiObjectiveArchiveCodec::new(order))));
                let session = RunnerSession::new(Box::new(hooks), context.problem.clone(), codec).with_committed_archive();
                if config.import_mode == ImportMode::Disabled { return Ok(ControlledSession::wrap(Box::new(session), context.problem, bindings, archive_codec, reward)); }
                let amount = config.size;
                let problem = context.problem.clone();
                let session = session.with_importer(move |state, candidates| {
                    let mut merged = state.clone(); merged.extend(candidates.iter().cloned());
                    Ok(crate::solver::heuristic::session::select_constrained_front(&problem, merged, amount, &ProblemDominance(problem.clone()), divisions))
                });
                Ok(ControlledSession::wrap(Box::new(session), context.problem, bindings, archive_codec, reward))
            }));
        }
        registry
    }
}

struct ProblemDominance(ProblemHandle<Vec<f64>, f64>);

impl crate::solver::heuristic::operator::multi_objective::Dominance for ProblemDominance {
    fn dominates(&self, lhs: &[f64], rhs: &[f64]) -> bool { self.0.dominates(&lhs.to_vec(), &rhs.to_vec()) }
}

struct ProjectedProblem {
    canonical: ProblemHandle<Vec<f64>, f64>,
    projection: Scalarization<Vec<f64>>,
    evaluated: EvaluatedCandidates,
    errors: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl SearchProblem<f64, f64> for ProjectedProblem {
    fn id(&self) -> &str { self.canonical.id() }
    fn fingerprint(&self) -> &str { self.canonical.fingerprint() }
    fn variable_order(&self) -> Vec<VariableId> { self.canonical.variable_order() }
    fn lower_bound(&self, index: usize) -> Option<f64> { self.canonical.lower_bound(index) }
    fn upper_bound(&self, index: usize) -> Option<f64> { self.canonical.upper_bound(index) }
    fn initial_solutions(&self, amount: usize) -> Vec<crate::model::callback::Solution<f64>> { self.canonical.initial_solutions(amount) }
    fn objective(&self, solution: &crate::model::callback::Solution<f64>) -> Option<f64> {
        let objective = self.canonical.objective(solution)?;
        if !self.canonical.valid_objective(&objective) || objective.is_empty() || objective.iter().any(|value| !value.is_finite()) { return None; }
        let scalar = match (self.projection)(&objective) {
            Ok(value) if value.is_finite() => value,
            result => { self.errors.borrow_mut().push(format!("invalid scalarization result: {result:?}")); return None; }
        };
        self.evaluated.borrow_mut().push(crate::solver::heuristic::Individual::new(solution.clone(), objective));
        Some(scalar)
    }
    fn evaluate(
        &self,
        solution: crate::model::callback::Solution<f64>,
        slice: &SearchSlice,
    ) -> Result<SearchEvaluationResult<f64, f64>> {
        // 原问题掌握缓存与请求计费，投影不能绕过它的统一评估入口。
        // The canonical problem owns caching and admission; projection must not bypass its evaluation boundary.
        let evaluated = self.canonical.evaluate(solution, slice)?;
        let candidate = if let Some(candidate) = evaluated.candidate {
            if candidate.fitness.is_empty() || candidate.fitness.iter().any(|value| !value.is_finite()) {
                return Ok(SearchEvaluationResult { admitted: evaluated.admitted, candidate: None });
            }
            self.evaluated.borrow_mut().push(candidate.clone());
            let scalar = (self.projection)(&candidate.fitness)?;
            if !scalar.is_finite() { return Err(invalid("non-finite scalarization result")); }
            Some(crate::solver::heuristic::Individual::new(candidate.solution, scalar))
        } else { None };
        Ok(SearchEvaluationResult { admitted: evaluated.admitted, candidate })
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn valid_objective(&self, objective: &f64) -> bool { objective.is_finite() }
    fn feasible(&self, solution: &crate::model::callback::Solution<f64>) -> bool { self.canonical.feasible(solution) }
    fn admissible(&self, solution: &crate::model::callback::Solution<f64>) -> bool { self.canonical.admissible(solution) }
    fn constraint_admissible(&self, solution: &crate::model::callback::Solution<f64>) -> bool { self.canonical.constraint_admissible(solution) }
    fn has_constraints(&self) -> bool { self.canonical.has_constraints() }
    fn stochastic_constraints(&self) -> bool { self.canonical.stochastic_constraints() }
    fn observe_constraints(&self, solution: &crate::model::callback::Solution<f64>) -> Result<()> { self.canonical.observe_constraints(solution) }
    fn constraint_failure(&self) -> Option<String> { self.canonical.constraint_failure() }
    fn constraint_tier(&self, solution: &crate::model::callback::Solution<f64>) -> Option<(u8, f64)> { self.canonical.constraint_tier(solution) }
    fn begin_constraints(&self) { self.canonical.begin_constraints(); }
    fn commit_constraints(&self) -> Result<()> { self.canonical.commit_constraints() }
    fn save_constraints(&self) -> Result<JsonValue> { self.canonical.save_constraints() }
    fn load_constraints(&self, state: &JsonValue) -> Result<()> { self.canonical.load_constraints(state) }
    fn rank_constraints(
        &self,
        solutions: &[crate::model::callback::Solution<f64>],
        objective: &dyn Fn(usize, usize) -> std::cmp::Ordering,
    ) -> Option<Vec<usize>> { self.canonical.rank_constraints(solutions, objective) }
    fn has_objective_coordinates(&self) -> bool { true }
    fn objective_coordinates(&self, objective: &f64) -> Result<Vec<f64>> { Ok(vec![*objective]) }
    fn has_constraint_violation(&self) -> bool { self.canonical.has_constraint_violation() }
    fn constraint_violation(&self, solution: &crate::model::callback::Solution<f64>) -> Result<f64> { self.canonical.constraint_violation(solution) }
    fn valid_solution(&self, solution: &crate::model::callback::Solution<f64>) -> bool { self.canonical.valid_solution(solution) }
    fn encoding(&self) -> SearchEncoding { self.canonical.encoding() }
    fn stateless(&self) -> bool { self.canonical.stateless() }
    fn save_state(&self) -> Result<Value> { self.canonical.save_state() }
    fn load_state(&mut self, state: &Value) -> Result<()> { self.canonical.load_state(state) }
    fn flush(&mut self) { self.canonical.flush(); }
}

struct ProjectedSession {
    delegate: Box<dyn SearchSession<f64, f64>>,
    evaluated: EvaluatedCandidates,
    errors: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl ProjectedSession {
    fn report(&self, result: Result<SearchStepResult<f64, f64>>, used: u64) -> SearchStepResult<Vec<f64>, f64> {
        let mut report = match result {
            Ok(result) => SearchStepResult { candidates: Vec::new(), evaluations: result.evaluations, steps: result.steps,
                termination: result.termination, diagnostics: result.diagnostics },
            Err(error) => SearchStepResult { candidates: Vec::new(), evaluations: used, steps: 0,
                termination: SearchTermination::Failed, diagnostics: vec![error.to_string()] },
        };
        report.candidates = self.evaluated.replace(Vec::new());
        report.diagnostics.extend(self.errors.replace(Vec::new()));
        if !report.diagnostics.is_empty() { report.termination = SearchTermination::Failed; }
        report
    }
}

impl SearchSession<Vec<f64>, f64> for ProjectedSession {
    fn supports_import(&self) -> bool { self.delegate.supports_import() }
    fn step(&mut self, slice: &mut SearchSlice) -> Result<SearchStepResult<Vec<f64>, f64>> {
        let start = slice.used();
        let result = super::boundary(|| self.delegate.step(slice));
        Ok(self.report(result, slice.used().saturating_sub(start)))
    }
    fn import_solutions(
        &mut self,
        solutions: Vec<crate::model::callback::Solution<f64>>,
        slice: &mut SearchSlice,
    ) -> Result<SearchStepResult<Vec<f64>, f64>> {
        let start = slice.used();
        let result = super::boundary(|| self.delegate.import_solutions(solutions, slice));
        Ok(self.report(result, slice.used().saturating_sub(start)))
    }
    fn snapshot(&self) -> Result<SearchSessionSnapshot> { self.delegate.snapshot() }
    fn restore(&mut self, snapshot: &SearchSessionSnapshot) -> Result<()> {
        self.delegate.restore(snapshot)?;
        self.evaluated.borrow_mut().clear(); self.errors.borrow_mut().clear();
        Ok(())
    }
    fn iteration(&self) -> &crate::solver::heuristic::Iteration { self.delegate.iteration() }
    fn close(&mut self) -> Result<()> { self.delegate.close() }
}

fn projected_session(
    mut context: AlgorithmContext<Vec<f64>, f64>,
    projection: Scalarization<Vec<f64>>,
    learning: bool,
    operators: &OperatorRegistry<f64>,
    selectors: &BTreeMap<String, SelectorFactory>,
    reward: Box<dyn RewardPolicy>,
) -> Result<Box<dyn PolicySession<Vec<f64>, f64>>> {
    let evaluated = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let canonical = context.problem;
    let codec = Box::new(MultiObjectiveArchiveCodec::new(canonical.variable_order()));
    let problem = ProblemHandle::new(Box::new(ProjectedProblem { canonical: canonical.clone(), projection,
        evaluated: evaluated.clone(), errors: errors.clone() }));
    context.config.scalarization = None;
    let (delegate, bindings) = native_session_parts(
        AlgorithmContext { config: context.config, problem, random: context.random },
        learning, operators, selectors,
    )?;
    // 受控记忆和算子反馈必须使用原目标向量，不能把标量档案当作可迁移的规范状态。
    // Controlled memory and operator credit use canonical vectors, not a projected archive masquerading as canonical state.
    let session = Box::new(ProjectedSession { delegate, evaluated, errors });
    Ok(ControlledSession::wrap(session, canonical, bindings, codec, reward))
}

/// 当前已接入统一原生生命周期的算法 / Algorithms connected to the shared native lifecycle
pub const BUILTIN_ALGORITHMS: &[&str] = &[
    "genetic", "gwo", "sca", "mvo", "hs", "wca", "soa", "warso", "evo", "gco", "de", "woa", "hho", "aoa",
    "tlbo", "ssa", "gsa", "fa", "cs", "ba", "mfo", "goa", "pso", "apso", "clpso", "shade", "l-shade", "cma-es",
    "hill-climbing", "ns", "random-walk", "simulated-annealing",
];

fn dummy_entropy() -> Box<dyn crate::solver::heuristic::entropy::Entropy> { Box::new(SeededEntropy::default()) }

fn assemble<R>(
    context: AlgorithmContext<f64, f64>,
    runner: R,
    codec: Box<dyn SearchCodec<R::State>>,
    importer: Option<SessionImporter<R::State, f64, f64>>,
) -> Result<Box<dyn SearchSession<f64, f64>>>
where
    R: NativeRunner<f64> + 'static,
    R::State: 'static,
{
    let order = context.problem.variable_order();
    let hooks = NativeHooks::new(runner, Box::new(context.random));
    let codecs = SessionCodecs::new(Some(codec), Some(Box::new(IndividualListCodec::new(order))));
    let session = RunnerSession::with_archive_capacity(
        Box::new(hooks),
        context.problem,
        codecs,
        100,
    ).with_committed_archive();
    if context.config.import_mode == ImportMode::Native && let Some(importer) = importer {
        return Ok(Box::new(session.with_importer(importer)));
    }
    Ok(Box::new(session))
}

fn native_session(
    context: AlgorithmContext<f64, f64>,
    learning: bool,
    operators: &OperatorRegistry<f64>,
    selectors: &BTreeMap<String, SelectorFactory>,
    reward: Box<dyn RewardPolicy>,
) -> Result<Box<dyn PolicySession<f64, f64>>> {
    let problem = context.problem.clone();
    let codec = Box::new(IndividualListCodec::new(problem.variable_order()));
    let (session, bindings) = native_session_parts(
        context,
        learning,
        operators,
        selectors,
    )?;
    Ok(ControlledSession::wrap(session, problem, bindings, codec, reward))
}

fn native_session_parts(
    context: AlgorithmContext<f64, f64>,
    learning: bool,
    operators: &OperatorRegistry<f64>,
    selectors: &BTreeMap<String, SelectorFactory>,
) -> Result<(Box<dyn SearchSession<f64, f64>>, OperatorBindings)> {
    let config = &context.config;
    let kind = config.algorithm.r#type.as_str();
    let spec = &config.algorithm;
    OperatorBindings::validate_encoding(config, context.problem.encoding(), operators)?;
    if (kind == "clpso" && config.size < 2) || (["de", "shade", "l-shade"].contains(&kind) && config.size < 4) {
        return Err(invalid("native population is too small for the configured algorithm"));
    }
    if !config.children.is_empty() || config.scalarization.is_some() { return Err(invalid("this native algorithm does not support children or scalarization")); }
    let bindings = OperatorBindings::new(config, &SearchRandom::new(context.random.state() as u64), learning, operators, selectors, context.problem.encoding())?;
    let order = context.problem.variable_order();
    let size = config.size;
    let session = match kind {
        "pso" | "apso" => {
            spec.validate_parameters(&["maxVelocity"])?;
            let velocity = spec.number("maxVelocity", 10_000.0)?;
            if velocity <= 0.0 { return Err(invalid("maximum velocity must be positive")); }
            let mut runner = ParticleSwarmRunner::new(size, dummy_entropy()).with_max_velocity(velocity);
            let inertia = if kind == "apso" { Box::new(LinearSource::new(0.9, 0.4, 1000)) as Box<dyn crate::solver::heuristic::operator::evolution::ParameterSource> }
                else { Box::new(crate::solver::heuristic::operator::evolution::ConstantSource::new(0.4)) };
            let personal = Box::new(crate::solver::heuristic::operator::evolution::ConstantSource::new(2.0));
            let social = Box::new(crate::solver::heuristic::operator::evolution::ConstantSource::new(2.0));
            runner = runner.with_inertia(bindings.parameter("inertia", inertia))
                .with_personal_weight(bindings.parameter("personalWeight", personal))
                .with_social_weight(bindings.parameter("socialWeight", social));
            let problem = context.problem.clone();
            assemble(context, runner, Box::new(ParticleStateCodec::new(order)), Some(Box::new(move |state, candidates| {
                ContextStateSynchronizer::new(&problem, velocity)?.particles(state, candidates)
            })))
        }
        "clpso" => {
            spec.validate_parameters(&["learningWeight", "maxVelocity", "refreshInterval", "stagnationLimit"])?;
            let mut clpso = ClpsoConfig::new(size);
            clpso.learning_weight = spec.number("learningWeight", 1.49445)?;
            clpso.max_velocity = spec.number("maxVelocity", 10_000.0)?;
            clpso.refresh_interval = u32::try_from(spec.integer("refreshInterval", 7)?).map_err(|_| invalid("refresh interval overflow"))?;
            clpso.stagnation_limit = u32::try_from(spec.integer("stagnationLimit", 5)?).map_err(|_| invalid("stagnation limit overflow"))?;
            if clpso.learning_weight <= 0.0 || clpso.max_velocity <= 0.0 || clpso.refresh_interval == 0 || clpso.stagnation_limit == 0 {
                return Err(invalid("invalid CLPSO parameters"));
            }
            let max_velocity = clpso.max_velocity;
            let clpso = ClpsoRunner::new(clpso, dummy_entropy())
                .with_inertia(bindings.parameter("inertia", Box::new(LinearSource::new(0.9, 0.4, 1000))));
            let problem = context.problem.clone();
            assemble(context, clpso, Box::new(ClpsoStateCodec::new(order)),
                Some(Box::new(move |state, candidates| {
                    ContextStateSynchronizer::new(&problem, max_velocity)?.clpso(state, candidates)
                })))
        }
        "shade" | "l-shade" => {
            let parameters: &[&str] = if kind == "shade" { &["memorySize", "minimumSize", "pBestRate", "numerics"] }
                else { &["memorySize", "minimumSize", "pBestRate", "sizeReductionSteps", "numerics"] };
            spec.validate_parameters(parameters)?;
            let mut shade = ShadeConfig::new(size);
            shade.numerics = match spec.parameters.get("numerics") {
                None => ShadeNumerics::Native,
                Some(Value::String(value)) if value == "native" => ShadeNumerics::Native,
                Some(Value::String(value)) if value == "shade-fdlibm-v1" => ShadeNumerics::FdlibmV1,
                _ => return Err(invalid("unsupported SHADE numerical contract")),
            };
            shade.memory_size = spec.integer("memorySize", 5)? as usize;
            shade.minimum_size = spec.integer("minimumSize", 4)? as usize;
            shade.p_best_rate = spec.number("pBestRate", 0.2)?;
            if shade.memory_size == 0 || shade.minimum_size < 4 || shade.minimum_size > size || !(0.0..=1.0).contains(&shade.p_best_rate) {
                return Err(invalid("invalid SHADE parameters"));
            }
            if kind == "l-shade" {
                let limit = spec.integer("sizeReductionSteps", 1000)?;
                if limit == 0 { return Err(invalid("sizeReductionSteps must be positive")); }
                shade.size_ratio = Box::new(LinearSource::new(1.0, shade.minimum_size as f64 / size as f64, limit));
            }
            shade.size_ratio = bindings.parameter("sizeRatio", shade.size_ratio);
            let problem = context.problem.clone();
            assemble(context, ShadeRunner::new(shade, dummy_entropy()), Box::new(ShadeStateCodec::new(order)), Some(Box::new(move |state, candidates| {
                ContextStateSynchronizer::new(&problem, 10_000.0)?.shade(state, candidates)
            })))
        }
        "cma-es" => {
            spec.validate_parameters(&["initialSigma", "restartOnImport", "numerics"])?;
            let numerics = match spec.parameters.get("numerics") {
                None => CmaNumerics::Native,
                Some(Value::String(value)) if value == "native" => CmaNumerics::Native,
                Some(Value::String(value)) if value == "cma-polar-fdlibm-v1" => CmaNumerics::PolarFdlibmV1,
                _ => return Err(invalid("unknown CMA numerical contract")),
            };
            let sigma = spec.number("initialSigma", 0.3)?;
            if sigma <= 0.0 { return Err(invalid("CMA-ES sigma must be positive")); }
            let restart = match spec.parameters.get("restartOnImport") {
                Some(value) => value.as_bool().ok_or_else(|| invalid("restartOnImport must be boolean"))?,
                None => false,
            };
            let import_order = order.clone();
            let import_problem = context.problem.clone();
            let importer: SessionImporter<CmaEsState<f64>, f64, f64> = Box::new(move |state, candidates| {
                if candidates.is_empty() { return Ok(state.clone()); }
                let ranked: Vec<_> = import_problem.rank_indices(candidates).into_iter().map(|index| candidates[index].clone()).collect();
                let dimension = import_order.len();
                let mean: Vec<_> = import_order.iter().map(|id| ranked.iter().map(|value| value.solution[id] / ranked.len() as f64)
                    .fold(0.0, |total, value| total + value)).collect();
                let covariance = if ranked.len() == 1 { state.covariance.clone() } else {
                    (0..dimension).map(|row| (0..dimension).map(|column| {
                        let sum = ranked.iter().map(|value| (value.solution[&import_order[row]] - mean[row]) * (value.solution[&import_order[column]] - mean[column]))
                            .fold(0.0, |total, value| total + value);
                        sum / (ranked.len() - 1) as f64 / (sigma * sigma) + if row == column { 1e-10 } else { 0.0 }
                    }).collect()).collect()
                };
                let best = state.best.as_ref().filter(|best| import_problem.compare_individuals(best, &ranked[0]) == std::cmp::Ordering::Less).cloned().unwrap_or_else(|| ranked[0].clone());
                Ok(CmaEsState { mean, sigma, covariance, path_sigma: vec![0.0; dimension], path_covariance: vec![0.0; dimension], generation: 0, best: Some(best) })
            });
            assemble(context, CmaEsRunner::new(dummy_entropy(), sigma).with_numerics(numerics), Box::new(CmaEsStateCodec::new(order)), if restart { Some(importer) } else { None })
        }
        "hill-climbing" | "ns" | "random-walk" | "simulated-annealing" => {
            spec.validate_parameters(&[])?;
            let acceptance: Box<dyn crate::solver::heuristic::operator::acceptance::Acceptance<f64, f64>> = match kind {
                "hill-climbing" => Box::new(BetterOnlyAcceptance),
                "ns" => Box::new(BetterOnlyAcceptance),
                "random-walk" => Box::new(AlwaysAcceptance),
                _ => Box::new(MetropolisAcceptance::new(bindings.parameter("temperature",
                    Box::new(crate::solver::heuristic::operator::evolution::GeometricSource::new(100.0, 0.95))))),
            };
            assemble(context, TrajectoryRunner::new(bindings.neighborhood(Box::new(RandomPerturbation::default())), acceptance, dummy_entropy()),
                Box::new(TrajectoryStateCodec::new(order)), Some(Box::new(|state, candidates| Ok(candidates.first().cloned().or_else(|| state.clone())))))
        }
        _ => {
            spec.validate_parameters(&["mutationRate"])?;
            let update: Box<dyn SolutionUpdate<f64>> = match kind {
                "genetic" => Box::new(SinglePointCrossover), "gwo" => Box::new(GreyWolfUpdate::default()),
                "sca" => Box::new(SineCosineUpdate::default()), "mvo" => Box::new(MultiVerseUpdate::default()),
                "hs" => Box::new(HarmonySearchUpdate::default()), "wca" => Box::new(WaterCycleUpdate::default()),
                "soa" => Box::new(SeagullUpdate::default()), "warso" => Box::new(WarStrategyUpdate::default()),
                "evo" => Box::new(EnergyValleyUpdate::default()), "gco" => Box::new(GerminalCenterUpdate::default()),
                "de" => Box::new(DifferentialCrossover::default()), "woa" => Box::new(WhaleOptimizationUpdate::default()),
                "hho" => Box::new(HarrisHawkUpdate::default()), "aoa" => Box::new(ArithmeticOptimizationUpdate::default()),
                "tlbo" => Box::new(TeachingLearningUpdate::default()), "ssa" => Box::new(SalpSwarmUpdate::default()),
                "gsa" => Box::new(GravitationalSearchUpdate::default()), "fa" => Box::new(FireflyUpdate::default()),
                "cs" => Box::new(CuckooSearchUpdate::default()), "ba" => Box::new(BatAlgorithmUpdate::default()),
                "mfo" => Box::new(MothFlameUpdate::default()), "goa" => Box::new(GrasshopperUpdate::default()),
                _ => return Err(invalid("unknown native algorithm")),
            };
            let mutation: Box<dyn crate::solver::heuristic::operator::mutation::Mutation<f64>> = if kind == "genetic" {
                Box::new(GaussianMutation::default())
            } else { Box::new(NoMutation) };
            let rate = spec.number("mutationRate", 0.1)?;
            if !(0.0..=1.0).contains(&rate) { return Err(invalid("invalid mutation rate")); }
            let runner = PopulationRunner::new(size, bindings.update(update), bindings.mutation(mutation), Box::new(ElitistSurvivor), dummy_entropy()).with_mutation_rate(rate);
            let problem = context.problem.clone();
            assemble(context, runner, Box::new(PopulationStateCodec::new(order)), Some(Box::new(move |state, candidates| {
                Ok(import_population(&problem, state, candidates))
            })))
        }
    }?;
    Ok((session, bindings))
}

fn import_population(
    problem: &ProblemHandle<f64, f64>,
    state: &[crate::solver::heuristic::Individual<f64, f64>],
    candidates: &[crate::solver::heuristic::Individual<f64, f64>],
) -> Vec<crate::solver::heuristic::Individual<f64, f64>> {
    let mut remaining = state.to_vec();
    remaining.extend_from_slice(candidates);
    if problem.stochastic_constraints() {
        return problem.rank_indices(&remaining).into_iter().take(state.len()).map(|index| remaining[index].clone()).collect();
    }
    let mut selected = Vec::with_capacity(state.len());
    // 与精英生存选择一致，按优度输出；原地替换最差成员会改变后续基于下标的随机选择。
    // Match elitist survival order; replacing worst entries in place changes subsequent index-based random choices.
    while selected.len() < state.len() && !remaining.is_empty() {
        let mut best = 0;
        for index in 1..remaining.len() {
            if problem.compare_individuals(&remaining[index], &remaining[best]) == std::cmp::Ordering::Less { best = index; }
        }
        selected.push(remaining.remove(best));
    }
    selected
}
