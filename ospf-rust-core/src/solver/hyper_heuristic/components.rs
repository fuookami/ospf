//! 可恢复的高层业务扩展点 / Recoverable upper-level business extension points

use std::collections::BTreeMap;
use std::sync::Arc;
use serde_json::Value;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::Result;
use crate::solver::hyper_heuristic::config::{ComponentSpec, HyperHeuristicConfig};
use crate::solver::hyper_heuristic::memory::{AcceptancePolicy, AlwaysAcceptance, NotWorseAcceptance};
use crate::solver::hyper_heuristic::selection::{SelectionContext, SelectionPolicy, TabularSelectionPolicy};
use crate::solver::hyper_heuristic::invalid;

/// 显式恢复协议；有状态组件不得伪装为无状态 / Explicit recovery protocol; stateful components must not claim statelessness
pub trait RecoverableComponent {
    /// 是否显式声明无状态 / Whether explicitly declared stateless
    fn stateless(&self) -> bool { false }
    /// 保存非缓存状态 / Save non-cache state
    fn save(&self) -> Result<Value> {
        if self.stateless() { Ok(Value::Null) } else { Err(invalid("component is not recoverable")) }
    }
    /// 恢复非缓存状态 / Restore non-cache state
    fn load(&mut self, state: &Value) -> Result<()> {
        if self.stateless() && state.is_null() { Ok(()) } else { Err(invalid("component state is not recoverable")) }
    }
}

/// 特征抽取的业务输入 / Business input for feature extraction
pub struct SearchProgress {
    /// 已派发决策 / Dispatched decisions
    pub decision: u64,
    /// 已消费评估请求 / Consumed evaluation requests
    pub evaluations: u64,
    /// 全局评估上限 / Global evaluation limit
    pub evaluation_limit: Option<u64>,
    /// 连续停滞数 / Consecutive stagnant outcomes
    pub stagnation: u64,
    /// 工作集解多样性 / Working-set solution diversity
    pub diversity: f64,
    /// 改善比例 / Improvement ratio
    pub success_ratio: f64,
    /// 最近成本占片段额度比例 / Latest cost relative to slice allowance
    pub cost_ratio: f64,
}

/// 可注册特征提取器 / Registerable feature extractor
pub trait FeatureExtractor: RecoverableComponent {
    /// 策略兼容性版本 / Policy compatibility version
    fn version(&self) -> &str;
    /// 不执行目标评估的特征抽取 / Extract features without objective evaluation
    fn extract(&self, progress: &SearchProgress) -> Result<SelectionContext>;
}

/// 相对派发档案的奖励输入 / Reward input relative to the dispatch archive
pub struct RewardContext {
    /// 是否改善 / Whether improved
    pub improved: bool,
    /// 贡献比例 / Contribution ratio
    pub contribution: f64,
    /// 实际评估成本 / Actual evaluation cost
    pub evaluations: u64,
    /// 稳定动作 ID；内层为槽位/动作 / Stable action ID; inner rewards use slot/action
    pub action: String,
}

/// 可注册奖励策略 / Registerable reward policy
pub trait RewardPolicy: RecoverableComponent {
    /// 计算有限奖励 / Compute a finite reward
    fn reward(&mut self, context: &RewardContext) -> Result<f64>;
}

/// 不可超出的片段上限，也作为分配器返回值 / Slice ceilings that also serve as the allocator result
#[derive(Clone, Copy)]
pub struct SliceAllocation {
    /// 评估请求额度 / Evaluation request allowance
    pub evaluations: u64,
    /// 完整低层步额度 / Complete low-level step allowance
    pub steps: u64,
}

/// 可注册的预算分配器 / Registerable budget allocator
pub trait BudgetAllocator: RecoverableComponent {
    /// 在硬上限内分配，不得返回零额度 / Allocate within hard ceilings without returning zero allowance
    fn allocate(&mut self, ceiling: SliceAllocation, progress: &SelectionContext) -> Result<SliceAllocation>;
}

/// 可注册的工作集交换时机 / Registerable working-set exchange schedule
pub trait ExchangePolicy: RecoverableComponent {
    /// 是否要求原生导入支持 / Whether native import support is required
    fn requires_import(&self) -> bool { true }
    /// 本次派发最多导入的工作集解数量 / Maximum working-set solutions imported for this dispatch
    fn amount(&mut self, decision: u64) -> Result<usize>;
}

struct ProgressFeatures;
impl RecoverableComponent for ProgressFeatures { fn stateless(&self) -> bool { true } }
impl FeatureExtractor for ProgressFeatures {
    fn version(&self) -> &str { "progress:1" }
    fn extract(&self, progress: &SearchProgress) -> Result<SelectionContext> {
        let ratio = progress.evaluation_limit.filter(|limit| *limit > 0)
            .map(|limit| progress.evaluations as f64 / limit as f64).unwrap_or(0.0).clamp(0.0, 1.0);
        Ok(SelectionContext { state: (ratio * 7.0) as usize * 8 + progress.stagnation.min(7) as usize,
            features: vec![ratio, progress.stagnation.min(1000) as f64 / 1000.0, progress.diversity, progress.success_ratio, progress.cost_ratio],
            decision: progress.decision, stagnation: progress.stagnation })
    }
}

struct ImprovementReward(bool);
impl RecoverableComponent for ImprovementReward { fn stateless(&self) -> bool { true } }
impl RewardPolicy for ImprovementReward {
    fn reward(&mut self, context: &RewardContext) -> Result<f64> {
        Ok(if self.0 { context.contribution / context.evaluations.max(1) as f64 } else if context.improved { 1.0 } else { 0.0 })
    }
}

struct FixedAllocator;
impl RecoverableComponent for FixedAllocator { fn stateless(&self) -> bool { true } }
impl BudgetAllocator for FixedAllocator {
    fn allocate(&mut self, ceiling: SliceAllocation, _: &SelectionContext) -> Result<SliceAllocation> { Ok(ceiling) }
}

struct PeriodicExchange { period: u64, amount: usize }
impl RecoverableComponent for PeriodicExchange { fn stateless(&self) -> bool { true } }
impl ExchangePolicy for PeriodicExchange {
    fn requires_import(&self) -> bool { self.period > 0 }
    fn amount(&mut self, decision: u64) -> Result<usize> {
        Ok(if self.period > 0 && decision > 0 && decision % self.period == 0 { self.amount } else { 0 })
    }
}

type Factory<T> = Arc<dyn Fn(&ComponentSpec) -> Result<Box<T>> + Send + Sync>;
pub(crate) type SelectorFactory = Arc<dyn Fn(&ComponentSpec, Vec<String>, bool) -> Result<Box<dyn SelectionPolicy>> + Send + Sync>;

/// 显式工厂注册，运行间不共享组件实例 / Explicit factory registration without sharing runtime component instances
pub struct ComponentRegistry<O, V> {
    pub(crate) selectors: BTreeMap<String, SelectorFactory>,
    features: BTreeMap<String, Factory<dyn FeatureExtractor>>,
    rewards: BTreeMap<String, Factory<dyn RewardPolicy>>,
    allocators: BTreeMap<String, Factory<dyn BudgetAllocator>>,
    exchanges: BTreeMap<String, Factory<dyn ExchangePolicy>>,
    acceptance: BTreeMap<String, Factory<dyn AcceptancePolicy<O, V>>>,
}

impl<O, V> Clone for ComponentRegistry<O, V> {
    fn clone(&self) -> Self { Self { selectors: self.selectors.clone(), features: self.features.clone(), rewards: self.rewards.clone(),
        allocators: self.allocators.clone(), exchanges: self.exchanges.clone(), acceptance: self.acceptance.clone() } }
}

impl<O: Clone + 'static, V: HeuristicValue> Default for ComponentRegistry<O, V> {
    fn default() -> Self { Self::new() }
}

impl<O: Clone + 'static, V: HeuristicValue> ComponentRegistry<O, V> {
    /// 创建仅含内置组件的注册表 / Create a registry containing only built-in components
    pub fn new() -> Self {
        Self { selectors: BTreeMap::new(), features: BTreeMap::new(), rewards: BTreeMap::new(), allocators: BTreeMap::new(),
            exchanges: BTreeMap::new(), acceptance: BTreeMap::new() }
    }

    /// 注册独立选择器工厂，必须遵守 learning=false 的冻结语义 / Register an independent selector factory honoring frozen learning=false semantics
    pub fn register_selector(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec, Vec<String>, bool) -> Result<Box<dyn SelectionPolicy>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.selectors, id.into(), Arc::new(factory)) }

    /// 注册独立特征工厂 / Register an independent feature factory
    pub fn register_features(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn FeatureExtractor>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.features, id.into(), Arc::new(factory)) }

    /// 注册独立奖励工厂 / Register an independent reward factory
    pub fn register_reward(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn RewardPolicy>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.rewards, id.into(), Arc::new(factory)) }

    /// 注册独立预算分配器工厂 / Register an independent allocator factory
    pub fn register_allocator(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn BudgetAllocator>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.allocators, id.into(), Arc::new(factory)) }

    /// 注册独立交换工厂 / Register an independent exchange factory
    pub fn register_exchange(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn ExchangePolicy>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.exchanges, id.into(), Arc::new(factory)) }

    /// 注册独立工作集接受策略工厂 / Register an independent working-set acceptance factory
    pub fn register_acceptance(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn AcceptancePolicy<O, V>>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.acceptance, id.into(), Arc::new(factory)) }

    pub(crate) fn selector(&self, spec: &ComponentSpec, actions: Vec<String>, learning: bool) -> Result<Box<dyn SelectionPolicy>> {
        if let Some(factory) = self.selectors.get(&spec.r#type) { return factory(spec, actions, learning); }
        Ok(Box::new(TabularSelectionPolicy::new(spec, actions, learning)?))
    }

    pub(crate) fn reward(&self, spec: &ComponentSpec) -> Result<Box<dyn RewardPolicy>> {
        if let Some(factory) = self.rewards.get(&spec.r#type) { return factory(spec); }
        spec.validate_parameters(&[])?;
        if !["improvement", "cost-aware"].contains(&spec.r#type.as_str()) { return Err(invalid("unknown reward policy")); }
        Ok(Box::new(ImprovementReward(spec.r#type == "cost-aware")))
    }

    pub(crate) fn instantiate(&self, config: &HyperHeuristicConfig) -> Result<RuntimeComponents<O, V>> {
        let spec = &config.learning.features;
        let features = if let Some(factory) = self.features.get(&spec.r#type) { factory(spec)? }
            else { spec.validate_parameters(&[])?; if spec.r#type != "progress" { return Err(invalid("unknown feature extractor")); } Box::new(ProgressFeatures) };
        let reward = self.reward(&config.learning.reward)?;
        let spec = &config.budget.allocator;
        let allocator = if let Some(factory) = self.allocators.get(&spec.r#type) { factory(spec)? }
            else { spec.validate_parameters(&[])?; if spec.r#type != "fixed" { return Err(invalid("unknown budget allocator")); } Box::new(FixedAllocator) };
        let spec = &config.memory.acceptance;
        let acceptance = if let Some(factory) = self.acceptance.get(&spec.r#type) { factory(spec)? }
            else {
                spec.validate_parameters(&[])?;
                match spec.r#type.as_str() {
                    "not-worse" => Box::new(NotWorseAcceptance) as Box<dyn AcceptancePolicy<O, V>>,
                    "always" => Box::new(AlwaysAcceptance),
                    _ => return Err(invalid("unknown acceptance policy")),
                }
            };
        let spec = &config.memory.exchange;
        let exchange = if let Some(factory) = self.exchanges.get(&spec.r#type) { factory(spec)? }
            else {
                let (period, amount) = match spec.r#type.as_str() {
                    "none" => { spec.validate_parameters(&[])?; (0, 0) }
                    "periodic" => {
                        spec.validate_parameters(&["period", "amount"])?;
                        let period = spec.integer("period", 5)?;
                        let amount = spec.integer("amount", 1)?;
                        if period == 0 || amount == 0 { return Err(invalid("exchange period and amount must be positive")); }
                        (period, usize::try_from(amount).map_err(|_| invalid("exchange amount overflow"))?)
                    }
                    _ => return Err(invalid("unknown exchange policy")),
                };
                Box::new(PeriodicExchange { period, amount })
            };
        if features.version().is_empty() { return Err(invalid("empty feature protocol version")); }
        Ok(RuntimeComponents { features, reward, allocator, exchange, acceptance })
    }
}

fn insert<T>(map: &mut BTreeMap<String, T>, id: String, value: T) -> Result<()> {
    if id.is_empty() || map.contains_key(&id) { return Err(invalid("empty or duplicate component ID")); }
    map.insert(id, value); Ok(())
}

pub(crate) struct RuntimeComponents<O, V> {
    pub(crate) features: Box<dyn FeatureExtractor>,
    pub(crate) reward: Box<dyn RewardPolicy>,
    pub(crate) allocator: Box<dyn BudgetAllocator>,
    pub(crate) exchange: Box<dyn ExchangePolicy>,
    pub(crate) acceptance: Box<dyn AcceptancePolicy<O, V>>,
}

impl<O: Clone, V: HeuristicValue> RuntimeComponents<O, V> {
    pub(crate) fn save(&self) -> Result<BTreeMap<String, Value>> {
        let components: [(&str, &dyn RecoverableComponent); 5] = [("features", self.features.as_ref()), ("reward", self.reward.as_ref()),
            ("allocator", self.allocator.as_ref()), ("exchange", self.exchange.as_ref()), ("acceptance", self.acceptance.as_ref())];
        components.into_iter().filter(|(_, component)| !component.stateless()).map(|(id, component)| Ok((id.into(), component.save()?))).collect()
    }

    pub(crate) fn load(&mut self, saved: &BTreeMap<String, Value>) -> Result<()> {
        if self.save()?.keys().ne(saved.keys()) { return Err(invalid("incompatible recoverable components")); }
        let components: [(&str, &mut dyn RecoverableComponent); 5] = [("features", self.features.as_mut()), ("reward", self.reward.as_mut()),
            ("allocator", self.allocator.as_mut()), ("exchange", self.exchange.as_mut()), ("acceptance", self.acceptance.as_mut())];
        for (id, component) in components {
            if let Some(state) = saved.get(id) { component.load(state)?; }
        }
        Ok(())
    }
}
