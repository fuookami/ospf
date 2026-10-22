//! 可恢复的类型安全算子工厂 / Recoverable type-safe operator factories

use std::collections::BTreeMap;
use std::sync::Arc;
use crate::model::callback::Solution;
use crate::solver::heuristic::Iteration;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel};
use crate::solver::heuristic::operator::evolution::ParameterSource;
use crate::solver::heuristic::operator::mutation::Mutation;
use crate::solver::heuristic::operator::neighborhood::Neighborhood;
use crate::solver::heuristic::operator::update::SolutionUpdate;
use crate::solver::heuristic::result::Result;
use crate::solver::hyper_heuristic::components::RecoverableComponent;
use crate::solver::hyper_heuristic::config::ComponentSpec;
use crate::solver::hyper_heuristic::invalid;

/// 带显式恢复协议的更新算子 / Update operator with an explicit recovery protocol
pub trait UpdateComponent<V: HeuristicValue>: SolutionUpdate<V> + RecoverableComponent {}
impl<V: HeuristicValue, T: SolutionUpdate<V> + RecoverableComponent> UpdateComponent<V> for T {}
/// 带显式恢复协议的变异算子 / Mutation operator with an explicit recovery protocol
pub trait MutationComponent<V: HeuristicValue>: Mutation<V> + RecoverableComponent {}
impl<V: HeuristicValue, T: Mutation<V> + RecoverableComponent> MutationComponent<V> for T {}
/// 带显式恢复协议的邻域算子 / Neighborhood operator with an explicit recovery protocol
pub trait NeighborhoodComponent<V: HeuristicValue>: Neighborhood<V> + RecoverableComponent {}
impl<V: HeuristicValue, T: Neighborhood<V> + RecoverableComponent> NeighborhoodComponent<V> for T {}
/// 带显式恢复协议的参数源 / Parameter source with an explicit recovery protocol
pub trait ParameterComponent: ParameterSource + RecoverableComponent {}
impl<T: ParameterSource + RecoverableComponent> ParameterComponent for T {}

type Factory<T> = Arc<dyn Fn(&ComponentSpec) -> Result<Box<T>> + Send + Sync>;

/// 独立算子工厂，不在线程间共享算子实例 / Independent operator factories without sharing instances across threads
pub struct OperatorRegistry<V> {
    pub(crate) updates: BTreeMap<String, Factory<dyn UpdateComponent<V>>>,
    pub(crate) mutations: BTreeMap<String, Factory<dyn MutationComponent<V>>>,
    pub(crate) neighborhoods: BTreeMap<String, Factory<dyn NeighborhoodComponent<V>>>,
    pub(crate) parameters: BTreeMap<String, Factory<dyn ParameterComponent>>,
}

impl<V> Clone for OperatorRegistry<V> {
    fn clone(&self) -> Self { Self { updates: self.updates.clone(), mutations: self.mutations.clone(),
        neighborhoods: self.neighborhoods.clone(), parameters: self.parameters.clone() } }
}

impl<V: HeuristicValue> Default for OperatorRegistry<V> {
    fn default() -> Self { Self::new() }
}

impl<V: HeuristicValue> OperatorRegistry<V> {
    /// 创建自定义工厂目录；原生装配另提供内置默认目录 / Create custom factories; native assembly also provides built-in defaults
    pub fn new() -> Self { Self { updates: BTreeMap::new(), mutations: BTreeMap::new(), neighborhoods: BTreeMap::new(), parameters: BTreeMap::new() } }

    /// 注册更新工厂 / Register an update factory
    pub fn register_update(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn UpdateComponent<V>>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.updates, id.into(), Arc::new(factory)) }

    /// 注册变异工厂 / Register a mutation factory
    pub fn register_mutation(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn MutationComponent<V>>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.mutations, id.into(), Arc::new(factory)) }

    /// 注册邻域工厂 / Register a neighborhood factory
    pub fn register_neighborhood(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn NeighborhoodComponent<V>>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.neighborhoods, id.into(), Arc::new(factory)) }

    /// 注册参数源工厂 / Register a parameter-source factory
    pub fn register_parameter(
        &mut self,
        id: impl Into<String>,
        factory: impl Fn(&ComponentSpec) -> Result<Box<dyn ParameterComponent>> + Send + Sync + 'static,
    ) -> Result<()> { insert(&mut self.parameters, id.into(), Arc::new(factory)) }
}

fn insert<T>(map: &mut BTreeMap<String, T>, id: String, factory: T) -> Result<()> {
    if id.is_empty() || map.contains_key(&id) { return Err(invalid("empty or duplicate operator factory ID")); }
    map.insert(id, factory); Ok(())
}

pub(crate) struct Stateless<T>(pub T);
impl<T> RecoverableComponent for Stateless<T> { fn stateless(&self) -> bool { true } }

impl<V: HeuristicValue> SolutionUpdate<V> for Stateless<Box<dyn SolutionUpdate<V>>> {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> { self.0.update(model, iteration, parents, entropy) }
}

impl<V: HeuristicValue> Mutation<V> for Stateless<Box<dyn Mutation<V>>> {
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        solution: &Solution<V>,
        rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> { self.0.mutate(model, iteration, solution, rate, entropy) }
}

impl<V: HeuristicValue> Neighborhood<V> for Stateless<Box<dyn Neighborhood<V>>> {
    fn neighbor(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        solution: &Solution<V>,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> { self.0.neighbor(model, iteration, solution, entropy) }
}

impl ParameterSource for Stateless<Box<dyn ParameterSource>> {
    fn value(&mut self, iteration: &Iteration, entropy: &mut dyn Entropy) -> f64 { self.0.value(iteration, entropy) }
}
