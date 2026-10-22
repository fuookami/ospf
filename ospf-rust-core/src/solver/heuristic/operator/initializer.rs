//! 初始化算子 / Initialization operators

use crate::model::callback::Solution;

use super::super::entropy::Entropy;
use super::super::model::{HeuristicValue, VariableModel, from_f64, to_f64};
use super::unit;

/// 初始化算子契约 / Initializer contract
///
/// 决定初始解如何产生。初始解是搜索的唯一入口，因此也是「模型自带初始解」与「算法随机初始化」之间的切换点。
/// Decides how initial solutions are produced. Initial solutions are the only entry point of the search, and therefore the
/// switch between "the model supplies them" and "the algorithm samples them".
pub trait Initializer<V>
where
    V: HeuristicValue,
{
    /// 生成初始解 / Generate initial solutions
    fn initialize(
        &self,
        model: &dyn VariableModel<V>,
        amount: usize,
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>>;
}

/// 直接使用模型提供的初始解 / Uses the initial solutions provided by the model
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelInitializer;

impl<V> Initializer<V> for ModelInitializer
where
    V: HeuristicValue,
{
    fn initialize(
        &self,
        model: &dyn VariableModel<V>,
        amount: usize,
        _entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        model.initial_solutions(amount)
    }
}

/// 均匀随机初始化，在变量上下界内均匀采样 / Uniform random initialization sampling within variable bounds
///
/// 只有全部变量都有有限上下界时才能按区间采样；否则回退到模型自带的初始解生成器。
/// Uniform sampling is only possible when every variable has finite bounds; otherwise it falls back to the model's own
/// initial solution generator.
#[derive(Debug, Clone, Copy, Default)]
pub struct RandomUniformInitializer;

impl<V> Initializer<V> for RandomUniformInitializer
where
    V: HeuristicValue,
{
    fn initialize(
        &self,
        model: &dyn VariableModel<V>,
        amount: usize,
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let order = model.variable_order();
        if order.is_empty() {
            return model.initial_solutions(amount);
        }

        let mut bounds = Vec::with_capacity(order.len());
        for index in 0..order.len() {
            match (model.lower_bound(index), model.upper_bound(index)) {
                (Some(lower), Some(upper))
                    if lower.is_finite() && upper.is_finite() && lower <= upper =>
                {
                    bounds.push((lower, upper));
                }
                _ => return model.initial_solutions(amount),
            }
        }

        (0..amount)
            .map(|_| {
                let mut solution = Solution::<V>::new();
                for (index, (lower, upper)) in bounds.iter().enumerate() {
                    let value = lower + unit(entropy) * (upper - lower);
                    solution.insert(order[index], from_f64(value));
                }
                solution
            })
            .collect()
    }
}

/// 反对立学习初始化，在基础初始化之上补充镜像解 / Opposition-based learning initialization augmenting a base initializer with mirrored solutions
pub struct OppositionBasedInitializer<V>
where
    V: HeuristicValue,
{
    base: Box<dyn Initializer<V>>,
}

impl<V> OppositionBasedInitializer<V>
where
    V: HeuristicValue,
{
    /// 以基础初始化算子创建 / Create from a base initializer
    pub fn new(base: Box<dyn Initializer<V>>) -> Self {
        Self { base }
    }
}

impl<V> Default for OppositionBasedInitializer<V>
where
    V: HeuristicValue,
{
    fn default() -> Self {
        Self::new(Box::new(ModelInitializer))
    }
}

impl<V> Initializer<V> for OppositionBasedInitializer<V>
where
    V: HeuristicValue,
{
    fn initialize(
        &self,
        model: &dyn VariableModel<V>,
        amount: usize,
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let originals = self.base.initialize(model, amount, entropy);
        let order = model.variable_order();
        let mirrored: Vec<Solution<V>> = originals
            .iter()
            .map(|solution| {
                let mut result = solution.clone();
                for index in 0..order.len() {
                    let (lower, upper) = model.bounds_at(index);
                    let (Some(lower), Some(upper)) = (lower, upper) else {
                        continue;
                    };
                    if !(lower.is_finite() && upper.is_finite()) {
                        continue;
                    }
                    if let Some(value) = solution.get(&order[index]) {
                        let mirror = lower + upper - to_f64(value);
                        result.insert(order[index], from_f64(mirror));
                    }
                }
                result
            })
            .collect();

        let mut all = originals;
        all.extend(mirrored);
        all
    }
}

/// 固定初始解算子，总是返回同一组数值 / Fixed initial solution operator returning the same values
///
/// 用于把一个已有解喂给另一个骨架，从而组合出两阶段算法（memetic 的基本形态）。
/// Feeds an existing solution into another skeleton, composing two-stage algorithms (the basic shape of a memetic method).
#[derive(Debug, Clone, Default)]
pub struct FixedInitializer {
    /// 初始解的各维数值 / Per-dimension values of the initial solution
    pub values: Vec<f64>,
}

impl FixedInitializer {
    /// 创建固定初始解 / Create a fixed initializer
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }
}

impl<V> Initializer<V> for FixedInitializer
where
    V: HeuristicValue,
{
    fn initialize(
        &self,
        model: &dyn VariableModel<V>,
        amount: usize,
        _entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        if self.values.is_empty() {
            return Vec::new();
        }
        let order = model.variable_order();
        let mut solution = Solution::<V>::new();
        for (index, value) in self.values.iter().enumerate() {
            if let Some(id) = order.get(index) {
                solution.insert(*id, from_f64(*value));
            }
        }
        let count = amount.max(1);
        (0..count).map(|_| solution.clone()).collect()
    }
}
