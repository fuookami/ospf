//! 类型安全算子槽位与真实低层步反馈 / Type-safe operator slots and feedback at actual low-level steps

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::model::callback::Solution;
use crate::solver::heuristic::{Individual, Iteration};
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel};
use crate::solver::heuristic::operator::evolution::{ConstantSource, GeometricSource, LinearSource, ParameterSource};
use crate::solver::heuristic::operator::mutation::{GaussianMutation, InversionMutation, Mutation, NoMutation, PolynomialMutation, SwapMutation, UniformMutation};
use crate::solver::heuristic::operator::neighborhood::{GaussianPerturbation, Neighborhood, RandomPerturbation};
use crate::solver::heuristic::operator::update::*;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::*;
use crate::solver::heuristic::session::archive::ObjectiveComponents;
use crate::solver::hyper_heuristic::config::{AlgorithmConfig, ComponentSpec};
use crate::solver::hyper_heuristic::components::{RecoverableComponent, RewardContext, RewardPolicy, SelectorFactory};
use crate::solver::hyper_heuristic::operator_registry::*;
use crate::solver::hyper_heuristic::memory::{NotWorseAcceptance, SearchMemory};
use crate::solver::hyper_heuristic::problem::ProblemHandle;
use crate::solver::hyper_heuristic::selection::*;
use crate::solver::hyper_heuristic::{boundary, invalid};

/// 携带内层策略的高层会话边界，不反向依赖底层模块 / Upper session boundary carrying inner policies without reversing dependencies
pub trait PolicySession<O: Clone, V: HeuristicValue>: SearchSession<O, V> {
    /// 派发远程片段，只有线程代理实现；完成前不得再次访问该会话 / Dispatch a remote slice; only thread proxies implement this, with exclusive access until completion
    fn dispatch(&mut self, _slice: SearchSlice, _seeds: Vec<Solution<V>>, _initialized: bool) -> Result<()> {
        Err(invalid("session has no remote execution backend"))
    }

    /// 接收已派发片段；归并仍由协调器完成 / Receive a dispatched slice; merging remains coordinator-owned
    fn receive(&mut self) -> Result<SearchStepResult<O, V>> { Err(invalid("session has no pending remote slice")) }

    /// 导出稳定槽位路径对应的策略 / Export policies by stable slot path
    fn policies(&self) -> Result<BTreeMap<String, PolicySnapshot>> { Ok(BTreeMap::new()) }

    /// 恢复内层策略，不恢复问题实例状态 / Restore inner policies without restoring instance state
    fn restore_policies(&mut self, policies: &BTreeMap<String, PolicySnapshot>) -> Result<()> {
        if policies.is_empty() { Ok(()) } else { Err(invalid("unexpected operator policies")) }
    }

    /// 排出已完成的算子轨迹 / Drain completed operator transitions
    fn drain_transitions(&mut self) -> Vec<SelectionTransition> { Vec::new() }
}

pub(crate) struct UncontrolledSession<O, V>(pub Box<dyn SearchSession<O, V>>);

impl<O: Clone, V: HeuristicValue> SearchSession<O, V> for UncontrolledSession<O, V> {
    fn supports_import(&self) -> bool { self.0.supports_import() }
    fn step(&mut self, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> { self.0.step(slice) }
    fn import_solutions(&mut self, solutions: Vec<Solution<V>>, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        self.0.import_solutions(solutions, slice)
    }
    fn snapshot(&self) -> Result<SearchSessionSnapshot> { self.0.snapshot() }
    fn restore(&mut self, snapshot: &SearchSessionSnapshot) -> Result<()> { self.0.restore(snapshot) }
    fn iteration(&self) -> &Iteration { self.0.iteration() }
    fn close(&mut self) -> Result<()> { self.0.close() }
}

impl<O: Clone, V: HeuristicValue> PolicySession<O, V> for UncontrolledSession<O, V> {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotSnapshot {
    policy: PolicySnapshot,
    random: i64,
    components: BTreeMap<String, Value>,
}

trait SlotControl {
    fn id(&self) -> &str;
    fn used_action(&self) -> Option<&str>;
    fn choose(&mut self, context: &SelectionContext) -> Result<()>;
    fn observe(&mut self, transition: &SelectionTransition) -> Result<()>;
    fn save(&self) -> Result<SlotSnapshot>;
    fn load(&mut self, snapshot: &SlotSnapshot) -> Result<()>;
    fn policy(&self) -> Result<PolicySnapshot>;
    fn restore_policy(&mut self, snapshot: &PolicySnapshot) -> Result<()>;
    fn drain(&mut self) -> Vec<SelectionTransition>;
}

struct OperatorPool<T: RecoverableComponent + ?Sized> {
    id: String,
    actions: Vec<String>,
    candidates: Vec<Box<T>>,
    policy: Box<dyn SelectionPolicy>,
    random: SearchRandom,
    selected: usize,
    used: bool,
    before: SelectionContext,
    revision: u64,
    transitions: Vec<SelectionTransition>,
}

impl<T: RecoverableComponent + ?Sized> SlotControl for OperatorPool<T> {
    fn id(&self) -> &str { &self.id }
    fn used_action(&self) -> Option<&str> { self.used.then(|| self.actions[self.selected].as_str()) }

    fn choose(&mut self, context: &SelectionContext) -> Result<()> {
        let action = self.policy.select(context, &self.actions, &mut self.random)?;
        self.selected = self.actions.iter().position(|id| *id == action).ok_or_else(|| invalid("masked operator action"))?;
        self.before = context.clone();
        self.revision = self.policy.revision();
        self.used = false;
        Ok(())
    }

    fn observe(&mut self, transition: &SelectionTransition) -> Result<()> {
        if !self.used { return Ok(()); }
        let mut transition = transition.clone();
        transition.episode = self.id.clone();
        transition.scope = self.id.clone();
        transition.action = self.actions[self.selected].clone();
        transition.before = self.before.clone();
        transition.decision = self.before.decision;
        transition.available = self.actions.clone();
        transition.next_available = self.actions.clone();
        transition.policy_version = self.revision;
        self.transitions.push(transition.clone());
        self.policy.observe(&transition)
    }

    fn save(&self) -> Result<SlotSnapshot> {
        let components = self.candidates.iter().zip(&self.actions).filter(|(candidate, _)| !candidate.stateless())
            .map(|(candidate, id)| Ok((id.clone(), candidate.save()?))).collect::<Result<_>>()?;
        Ok(SlotSnapshot { policy: self.policy.snapshot()?, random: self.random.state() as i64, components })
    }

    fn load(&mut self, snapshot: &SlotSnapshot) -> Result<()> {
        let expected: std::collections::BTreeSet<_> = self.candidates.iter().zip(&self.actions).filter(|(candidate, _)| !candidate.stateless()).map(|(_, id)| id).collect();
        if expected != snapshot.components.keys().collect() { return Err(invalid("incompatible operator component states")); }
        for (candidate, id) in self.candidates.iter_mut().zip(&self.actions) {
            if let Some(state) = snapshot.components.get(id) { candidate.load(state)?; }
        }
        self.policy.restore(&snapshot.policy)?;
        self.random = SearchRandom::new(snapshot.random as u64);
        self.used = false;
        self.transitions.clear();
        Ok(())
    }

    fn policy(&self) -> Result<PolicySnapshot> { self.policy.snapshot() }
    fn restore_policy(&mut self, snapshot: &PolicySnapshot) -> Result<()> { self.policy.restore(snapshot) }
    fn drain(&mut self) -> Vec<SelectionTransition> { std::mem::take(&mut self.transitions) }
}

type Pool<T> = Rc<RefCell<OperatorPool<T>>>;

struct UpdateSlot(Pool<dyn UpdateComponent<f64>>);
struct MutationSlot(Pool<dyn MutationComponent<f64>>);
struct NeighborhoodSlot(Pool<dyn NeighborhoodComponent<f64>>);
struct ParameterSlot(Pool<dyn ParameterComponent>);

impl SolutionUpdate<f64> for UpdateSlot {
    fn update(
        &self,
        model: &dyn VariableModel<f64>,
        iteration: &Iteration,
        parents: &[Solution<f64>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<f64>> {
        let mut pool = self.0.borrow_mut();
        pool.used = true;
        pool.candidates[pool.selected].update(model, iteration, parents, entropy)
    }
}

impl Mutation<f64> for MutationSlot {
    fn mutate(
        &self,
        model: &dyn VariableModel<f64>,
        iteration: &Iteration,
        solution: &Solution<f64>,
        rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<f64> {
        let mut pool = self.0.borrow_mut();
        pool.used = true;
        pool.candidates[pool.selected].mutate(model, iteration, solution, rate, entropy)
    }
}

impl Neighborhood<f64> for NeighborhoodSlot {
    fn neighbor(
        &self,
        model: &dyn VariableModel<f64>,
        iteration: &Iteration,
        solution: &Solution<f64>,
        entropy: &mut dyn Entropy,
    ) -> Solution<f64> {
        let mut pool = self.0.borrow_mut();
        pool.used = true;
        pool.candidates[pool.selected].neighbor(model, iteration, solution, entropy)
    }
}

impl ParameterSource for ParameterSlot {
    fn value(&mut self, iteration: &Iteration, entropy: &mut dyn Entropy) -> f64 {
        let mut pool = self.0.borrow_mut();
        pool.used = true;
        let selected = pool.selected;
        pool.candidates[selected].value(iteration, entropy)
    }
}

pub(crate) struct OperatorBindings {
    update: Option<Pool<dyn UpdateComponent<f64>>>,
    mutation: Option<Pool<dyn MutationComponent<f64>>>,
    neighborhood: Option<Pool<dyn NeighborhoodComponent<f64>>>,
    parameters: BTreeMap<String, Pool<dyn ParameterComponent>>,
    pools: Vec<Rc<RefCell<dyn SlotControl>>>,
}

fn pool<T: RecoverableComponent + ?Sized + 'static>(
    id: String,
    specs: &[ComponentSpec],
    config: &AlgorithmConfig,
    random: &SearchRandom,
    learning: bool,
    factory: impl Fn(&ComponentSpec) -> Result<Box<T>>,
    selectors: &BTreeMap<String, SelectorFactory>,
) -> Result<Pool<T>> {
    if specs.iter().any(|spec| spec.version != 1) { return Err(invalid("unsupported operator component version")); }
    let actions: Vec<_> = specs.iter().map(|spec| format!("{}:{}:{}", spec.r#type, spec.version,
        super::persistence::fingerprint(&Value::Object(spec.parameters.clone())))).collect();
    if actions.iter().collect::<std::collections::BTreeSet<_>>().len() != actions.len() { return Err(invalid("duplicate operator action")); }
    let candidates = specs.iter().map(factory).collect::<Result<_>>()?;
    let policy: Box<dyn SelectionPolicy> = if let Some(factory) = selectors.get(&config.selection.r#type) {
        factory(&config.selection, actions.clone(), learning)?
    } else {
        if config.selection.integer("states", 64)? < 64 { return Err(invalid("operator progress features require at least 64 states")); }
        Box::new(TabularSelectionPolicy::new(&config.selection, actions.clone(), learning)?)
    };
    Ok(Rc::new(RefCell::new(OperatorPool { random: random.split(&format!("slot:{id}")), id, actions, candidates, policy,
        selected: 0, used: false, before: SelectionContext::default(), revision: 0, transitions: Vec::new() })))
}

impl OperatorBindings {
    pub(crate) fn validate_encoding(
        config: &AlgorithmConfig,
        encoding: SearchEncoding,
        registry: &OperatorRegistry<f64>,
    ) -> Result<()> {
        if encoding == SearchEncoding::Continuous { return Ok(()); }
        if !["genetic", "nsga-ii", "nsga-iii"].contains(&config.algorithm.r#type.as_str())
            || config.updates.is_empty() || config.mutations.is_empty()
            || config.updates.iter().any(|spec| !registry.updates.contains_key(&spec.r#type) && !["order", "cycle"].contains(&spec.r#type.as_str()))
            || config.mutations.iter().any(|spec| !registry.mutations.contains_key(&spec.r#type) && !["none", "swap", "inversion"].contains(&spec.r#type.as_str())) {
            return Err(invalid("non-continuous native search requires explicit compatible genetic or NSGA slots"));
        }
        Ok(())
    }

    pub(crate) fn new(
        config: &AlgorithmConfig,
        random: &SearchRandom,
        learning: bool,
        registry: &OperatorRegistry<f64>,
        selectors: &BTreeMap<String, SelectorFactory>,
    ) -> Result<Self> {
        let kind = config.algorithm.r#type.as_str();
        let population = super::registry::BUILTIN_ALGORITHMS[..22].contains(&kind) || ["nsga-ii", "nsga-iii"].contains(&kind);
        let trajectory = ["hill-climbing", "ns", "random-walk", "simulated-annealing"].contains(&kind);
        let parameters: &[&str] = match kind {
            "pso" | "apso" => &["inertia", "personalWeight", "socialWeight"],
            "clpso" => &["inertia"], "l-shade" => &["sizeRatio"], "simulated-annealing" => &["temperature"], _ => &[],
        };
        if (!population && (!config.updates.is_empty() || !config.mutations.is_empty()))
            || (!trajectory && !config.neighborhoods.is_empty()) || config.parameters.keys().any(|key| !parameters.contains(&key.as_str())) {
            return Err(invalid("algorithm does not support the configured operator slot"));
        }
        let mut bindings = Self { update: None, mutation: None, neighborhood: None, parameters: BTreeMap::new(), pools: Vec::new() };
        if !config.updates.is_empty() {
            let pool = pool(
                "update".into(),
                &config.updates,
                config,
                random,
                learning,
                |spec| match registry.updates.get(&spec.r#type) {
                    Some(factory) => factory(spec),
                    None => Ok(Box::new(Stateless(update(spec)?)) as Box<dyn UpdateComponent<f64>>),
                },
                selectors,
            )?;
            bindings.pools.push(pool.clone());
            bindings.update = Some(pool);
        }
        if !config.mutations.is_empty() {
            let pool = pool(
                "mutation".into(),
                &config.mutations,
                config,
                random,
                learning,
                |spec| match registry.mutations.get(&spec.r#type) {
                    Some(factory) => factory(spec),
                    None => Ok(Box::new(Stateless(mutation(spec)?)) as Box<dyn MutationComponent<f64>>),
                },
                selectors,
            )?;
            bindings.pools.push(pool.clone());
            bindings.mutation = Some(pool);
        }
        if !config.neighborhoods.is_empty() {
            let pool = pool(
                "neighborhood".into(),
                &config.neighborhoods,
                config,
                random,
                learning,
                |spec| match registry.neighborhoods.get(&spec.r#type) {
                    Some(factory) => factory(spec),
                    None => Ok(Box::new(Stateless(neighborhood(spec)?)) as Box<dyn NeighborhoodComponent<f64>>),
                },
                selectors,
            )?;
            bindings.pools.push(pool.clone());
            bindings.neighborhood = Some(pool);
        }
        for (name, specs) in &config.parameters {
            if specs.is_empty() { return Err(invalid("empty parameter-source pool")); }
            let pool = pool(
                name.clone(),
                specs,
                config,
                random,
                learning,
                |spec| match registry.parameters.get(&spec.r#type) {
                    Some(factory) => factory(spec),
                    None => Ok(Box::new(Stateless(parameter(spec)?)) as Box<dyn ParameterComponent>),
                },
                selectors,
            )?;
            bindings.pools.push(pool.clone());
            bindings.parameters.insert(name.clone(), pool);
        }
        Ok(bindings)
    }

    pub(crate) fn update(&self, default: Box<dyn SolutionUpdate<f64>>) -> Box<dyn SolutionUpdate<f64>> {
        self.update.as_ref().map(|pool| Box::new(UpdateSlot(pool.clone())) as Box<dyn SolutionUpdate<f64>>).unwrap_or(default)
    }

    pub(crate) fn mutation(&self, default: Box<dyn Mutation<f64>>) -> Box<dyn Mutation<f64>> {
        self.mutation.as_ref().map(|pool| Box::new(MutationSlot(pool.clone())) as Box<dyn Mutation<f64>>).unwrap_or(default)
    }

    pub(crate) fn neighborhood(&self, default: Box<dyn Neighborhood<f64>>) -> Box<dyn Neighborhood<f64>> {
        self.neighborhood.as_ref().map(|pool| Box::new(NeighborhoodSlot(pool.clone())) as Box<dyn Neighborhood<f64>>).unwrap_or(default)
    }

    pub(crate) fn parameter(&self, name: &str, default: Box<dyn ParameterSource>) -> Box<dyn ParameterSource> {
        self.parameters.get(name).map(|pool| Box::new(ParameterSlot(pool.clone())) as Box<dyn ParameterSource>).unwrap_or(default)
    }
}

fn update(spec: &ComponentSpec) -> Result<Box<dyn SolutionUpdate<f64>>> {
    if spec.r#type == "sbx" {
        spec.validate_parameters(&["eta"])?;
        let eta = spec.number("eta", 20.0)?;
        if eta <= 0.0 { return Err(invalid("SBX eta must be positive")); }
        return Ok(Box::new(SimulatedBinaryCrossover::new(eta)));
    }
    spec.validate_parameters(&[])?;
    Ok(match spec.r#type.as_str() {
        "single-point" => Box::new(SinglePointCrossover), "two-point" => Box::new(TwoPointCrossover),
        "uniform" => Box::new(UniformCrossover::default()), "arithmetic" => Box::new(ArithmeticCrossover),
        "blend" => Box::new(BlendCrossover::default()),
        "order" => Box::new(OrderCrossover), "cycle" => Box::new(CycleCrossover),
        "differential" => Box::new(DifferentialCrossover::default()),
        _ => return Err(invalid("unknown update operator")),
    })
}

fn mutation(spec: &ComponentSpec) -> Result<Box<dyn Mutation<f64>>> {
    if spec.r#type == "gaussian" {
        spec.validate_parameters(&["sigma"])?;
        let sigma = spec.number("sigma", 0.1)?;
        if sigma <= 0.0 { return Err(invalid("Gaussian sigma must be positive")); }
        return Ok(Box::new(GaussianMutation::new(sigma)));
    }
    spec.validate_parameters(&[])?;
    Ok(match spec.r#type.as_str() {
        "none" => Box::new(NoMutation), "polynomial" => Box::new(PolynomialMutation::default()),
        "swap" => Box::new(SwapMutation), "inversion" => Box::new(InversionMutation),
        "uniform" => Box::new(UniformMutation::default()), _ => return Err(invalid("unknown mutation operator")),
    })
}

fn neighborhood(spec: &ComponentSpec) -> Result<Box<dyn Neighborhood<f64>>> {
    if spec.r#type == "perturbation" {
        spec.validate_parameters(&["amount"])?;
        let amount = spec.integer("amount", 1)?;
        if amount == 0 { return Err(invalid("perturbation amount must be positive")); }
        return Ok(Box::new(RandomPerturbation::new(usize::try_from(amount).map_err(|_| invalid("perturbation amount overflow"))?)));
    }
    spec.validate_parameters(&[])?;
    Ok(match spec.r#type.as_str() {
        "random" => Box::new(RandomPerturbation::default()), "gaussian" => Box::new(GaussianPerturbation::default()),
        _ => return Err(invalid("unknown neighborhood operator")),
    })
}

fn parameter(spec: &ComponentSpec) -> Result<Box<dyn ParameterSource>> {
    match spec.r#type.as_str() {
        "constant" => { spec.validate_parameters(&["value"])?; Ok(Box::new(ConstantSource::new(spec.number("value", 1.0)?))) }
        "linear" => {
            spec.validate_parameters(&["from", "to", "limit"])?;
            let limit = spec.integer("limit", 1000)?;
            if limit == 0 { return Err(invalid("linear source limit must be positive")); }
            Ok(Box::new(LinearSource::new(spec.number("from", 0.9)?, spec.number("to", 0.4)?, limit)))
        }
        "geometric" => {
            spec.validate_parameters(&["initial", "ratio"])?;
            let initial = spec.number("initial", 100.0)?;
            let ratio = spec.number("ratio", 0.95)?;
            if ratio <= 0.0 { return Err(invalid("geometric ratio must be positive")); }
            Ok(Box::new(GeometricSource::new(initial, ratio)))
        }
        _ => Err(invalid("unknown parameter source")),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ControlledSnapshot {
    session: SearchSessionSnapshot,
    slots: BTreeMap<String, SlotSnapshot>,
    local_archive: Value,
    initialized: bool,
    decisions: u64,
    stagnation: u64,
    #[serde(default, alias = "reward", skip_serializing_if = "Value::is_null")]
    reward_state: Value,
    #[serde(default, skip_serializing_if = "is_zero")]
    reward_state_version: u32,
}

fn is_zero(value: &u32) -> bool { *value == 0 }

pub(crate) struct ControlledSession<O: ObjectiveComponents, V: HeuristicValue> {
    delegate: Box<dyn SearchSession<O, V>>,
    problem: ProblemHandle<O, V>,
    bindings: OperatorBindings,
    memory: SearchMemory<O, V>,
    codec: Box<dyn SearchCodec<Vec<Individual<O, V>>>>,
    initialized: bool,
    decisions: u64,
    stagnation: u64,
    reward: Box<dyn RewardPolicy>,
}

impl<O: ObjectiveComponents, V: HeuristicValue> ControlledSession<O, V> {
    pub(crate) fn wrap(
        delegate: Box<dyn SearchSession<O, V>>,
        problem: ProblemHandle<O, V>,
        bindings: OperatorBindings,
        codec: Box<dyn SearchCodec<Vec<Individual<O, V>>>>,
        reward: Box<dyn RewardPolicy>,
    ) -> Box<dyn PolicySession<O, V>>
    where O: 'static {
        Box::new(Self { delegate, problem, bindings, memory: SearchMemory::local(100, 20), codec,
            initialized: false, decisions: 0, stagnation: 0, reward })
    }

    fn context(&self) -> SelectionContext {
        SelectionContext { state: self.stagnation.min(7) as usize, decision: self.decisions, stagnation: self.stagnation,
            features: vec![0.0, self.stagnation.min(1000) as f64 / 1000.0, 0.0, 0.0, 0.0] }
    }
}

impl<O: ObjectiveComponents, V: HeuristicValue> SearchSession<O, V> for ControlledSession<O, V> {
    fn supports_import(&self) -> bool { self.delegate.supports_import() }

    fn step(&mut self, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        let start = slice.used();
        let parent = Arc::new(slice.clone());
        let mut report = SearchStepResult::empty(SearchTermination::SliceComplete);
        while report.steps < slice.steps() && slice.remaining() > 0 && !slice.expired() {
            let before = self.context();
            if self.initialized { for pool in &self.bindings.pools { pool.borrow_mut().choose(&before)?; } }
            let mut child = (*SearchSlice::child(&parent, slice.remaining(), 1)).clone();
            let outcome = boundary(|| self.delegate.step(&mut child));
            let mut result = match outcome {
                Ok(result) => result,
                Err(error) => { let mut result = SearchStepResult::empty(SearchTermination::Failed); result.diagnostics.push(error.to_string()); result },
            };
            let (improved, contribution) = self.memory.contribution(&self.problem, self.memory.archive(), &result.candidates);
            self.stagnation = if improved { 0 } else { self.stagnation.saturating_add(1) };
            self.decisions += 1;
            if self.initialized {
                let event = SelectionTransition { episode: String::new(), decision: before.decision, action: String::new(), before,
                    after: self.context(), available: Vec::new(), next_available: Vec::new(), reward: 0.0,
                    evaluations: child.used(), terminal: result.termination != SearchTermination::SliceComplete || slice.global().remaining() == 0,
                    policy_version: 0, feature_version: "progress:1".into(), scope: String::new(), steps: result.steps,
                    failed: result.termination == SearchTermination::Failed, merged_improvement: false, elapsed_nanos: 0, previous_action: None };
                // 只对真实调用的槽位计奖，初始化与未使用槽位不推进奖励状态。
                // Credit only invoked slots; initialization and unused slots must not advance reward state.
                let mut feedback_failed = false;
                for pool in &self.bindings.pools {
                    let mut pool = pool.borrow_mut();
                    let Some(action) = pool.used_action() else { continue; };
                    let action = format!("{}/{action}", pool.id());
                    let mut event = event.clone();
                    event.reward = match boundary(|| {
                        let reward = self.reward.reward(&RewardContext { improved, contribution, evaluations: child.used(), action })?;
                        if !reward.is_finite() { return Err(invalid("non-finite operator reward")); }
                        Ok(reward)
                    }) {
                        Ok(value) => value,
                        Err(error) => {
                            feedback_failed = true; result.diagnostics.push(error.to_string());
                            event.failed = true; event.terminal = true; 0.0
                        }
                    };
                    // 反馈异常不能丢弃已执行候选或阻止其他已用槽位反馈。
                    // Feedback failures must not discard evaluated candidates or skip other used slots.
                    if let Err(error) = boundary(|| pool.observe(&event)) {
                        feedback_failed = true; result.diagnostics.push(error.to_string());
                    }
                }
                if feedback_failed { result.termination = SearchTermination::Failed; }
            }
            self.memory.offer(&self.problem, result.candidates.clone(), &mut NotWorseAcceptance);
            self.initialized |= result.steps > 0;
            report.candidates.extend(result.candidates);
            report.steps += result.steps;
            report.termination = result.termination;
            report.diagnostics.extend(result.diagnostics);
            if report.termination != SearchTermination::SliceComplete || result.steps == 0 { break; }
        }
        report.evaluations = slice.used().saturating_sub(start);
        Ok(report)
    }

    fn import_solutions(&mut self, solutions: Vec<Solution<V>>, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        self.delegate.import_solutions(solutions, slice)
    }

    fn snapshot(&self) -> Result<SearchSessionSnapshot> {
        let mut snapshot = self.delegate.snapshot()?;
        let slots = self.bindings.pools.iter().map(|pool| {
            let pool = pool.borrow(); Ok((pool.id().into(), pool.save()?))
        }).collect::<Result<_>>()?;
        let reward_state = self.reward.save()?;
        let saved = ControlledSnapshot { session: snapshot.clone(), slots, local_archive: self.codec.encode(&self.memory.archive().to_vec())?,
            initialized: self.initialized, decisions: self.decisions, stagnation: self.stagnation,
            reward_state_version: if reward_state.is_null() { 0 } else { 1 }, reward_state };
        snapshot.state = serde_json::to_value(saved).map_err(|error| invalid(error.to_string()))?;
        Ok(snapshot)
    }

    fn restore(&mut self, snapshot: &SearchSessionSnapshot) -> Result<()> {
        let saved: ControlledSnapshot = serde_json::from_value(snapshot.state.clone()).map_err(|error| invalid(error.to_string()))?;
        if saved.reward_state_version > 1 || (saved.reward_state_version != 0
            && (saved.reward_state.is_null() || snapshot.state.get("rewardState").is_none())) {
            return Err(invalid("unsupported or missing operator reward state / 不支持或缺失的算子奖励状态"));
        }
        if saved.slots.len() != self.bindings.pools.len() || saved.initialized != snapshot.initialized
            || saved.session.initialized != saved.initialized || saved.session.iteration != snapshot.iteration {
            return Err(invalid("incompatible controlled session snapshot"));
        }
        for pool in &self.bindings.pools {
            let mut pool = pool.borrow_mut();
            let saved = saved.slots.get(pool.id()).ok_or_else(|| invalid("operator slot changed"))?;
            pool.load(saved)?;
        }
        let archive = self.codec.decode(&saved.local_archive)?;
        self.memory.restore(&self.problem, archive, Vec::new())?;
        self.delegate.restore(&saved.session)?;
        self.reward.load(&saved.reward_state)?;
        self.initialized = saved.initialized; self.decisions = saved.decisions; self.stagnation = saved.stagnation;
        Ok(())
    }

    fn iteration(&self) -> &Iteration { self.delegate.iteration() }
    fn close(&mut self) -> Result<()> { self.delegate.close() }
}

impl<O: ObjectiveComponents, V: HeuristicValue> PolicySession<O, V> for ControlledSession<O, V> {
    fn policies(&self) -> Result<BTreeMap<String, PolicySnapshot>> {
        self.bindings.pools.iter().map(|pool| { let pool = pool.borrow(); Ok((pool.id().into(), pool.policy()?)) }).collect()
    }

    fn restore_policies(&mut self, policies: &BTreeMap<String, PolicySnapshot>) -> Result<()> {
        if policies.len() != self.bindings.pools.len() { return Err(invalid("incompatible operator policy slots")); }
        for pool in &self.bindings.pools {
            let mut pool = pool.borrow_mut();
            let saved = policies.get(pool.id()).ok_or_else(|| invalid("operator policy slot changed"))?;
            pool.restore_policy(saved)?;
        }
        Ok(())
    }

    fn drain_transitions(&mut self) -> Vec<SelectionTransition> {
        self.bindings.pools.iter().flat_map(|pool| pool.borrow_mut().drain()).collect()
    }
}
