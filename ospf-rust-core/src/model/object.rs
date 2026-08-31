//! 目标函数定义 / Objective function definitions

use super::flatten::Linear;
use num_traits::{One, Zero};
use std::ops::Add;

/// 目标方向 / Objective direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum ObjectiveCategory {
    /// 最小化 / Minimize
    #[default]
    Minimum,
    /// 最大化 / Maximize
    Maximum,
}

impl ObjectiveCategory {
    /// 是否为最小化 / Whether minimizing
    pub fn is_minimum(&self) -> bool {
        matches!(self, ObjectiveCategory::Minimum)
    }

    /// 是否为最大化 / Whether maximizing
    pub fn is_maximum(&self) -> bool {
        matches!(self, ObjectiveCategory::Maximum)
    }
}

/// 子目标 / Sub-objective
#[derive(Debug, Clone)]
pub struct SubObjective<V = f64> {
    /// 目标方向 / Objective direction
    pub category: ObjectiveCategory,
    /// 多项式 / Polynomial
    pub polynomial: Linear<V>,
    /// 名称 / Name
    pub name: String,
    /// 权重 / Weight
    pub weight: V,
}

impl<V> SubObjective<V> {
    /// 创建带显式权重的子目标 / Create sub-objective with explicit weight
    pub fn new_with_weight(
        category: ObjectiveCategory,
        polynomial: Linear<V>,
        name: &str,
        weight: V,
    ) -> Self {
        Self {
            category,
            polynomial,
            name: name.to_string(),
            weight,
        }
    }

    /// 设置权重 / Set weight
    pub fn with_weight(mut self, weight: V) -> Self {
        self.weight = weight;
        self
    }
}

impl<V> SubObjective<V>
where
    V: One,
{
    /// 创建带单位默认权重的子目标 / Create sub-objective with unit default weight
    pub fn new(category: ObjectiveCategory, polynomial: Linear<V>, name: &str) -> Self {
        Self::new_with_weight(category, polynomial, name, V::one())
    }

    /// 创建最小化子目标 / Create minimization sub-objective
    pub fn minimize(polynomial: Linear<V>, name: &str) -> Self {
        Self::new(ObjectiveCategory::Minimum, polynomial, name)
    }

    /// 创建最大化子目标 / Create maximization sub-objective
    pub fn maximize(polynomial: Linear<V>, name: &str) -> Self {
        Self::new(ObjectiveCategory::Maximum, polynomial, name)
    }
}

/// 目标 / Objective
#[derive(Debug, Clone)]
pub struct Objective<V = f64> {
    /// 目标方向 / Objective direction
    pub category: ObjectiveCategory,
    /// 子目标列表 / Sub-objectives
    pub sub_objectives: Vec<SubObjective<V>>,
}

impl<V> Objective<V> {
    /// 创建目标 / Create objective
    pub fn new(category: ObjectiveCategory) -> Self {
        Self {
            category,
            sub_objectives: Vec::new(),
        }
    }

    /// 创建最小化目标 / Create minimization objective
    pub fn minimize() -> Self {
        Self::new(ObjectiveCategory::Minimum)
    }

    /// 创建最大化目标 / Create maximization objective
    pub fn maximize() -> Self {
        Self::new(ObjectiveCategory::Maximum)
    }

    /// 添加子目标 / Add sub-objective
    pub fn add_sub_objective(&mut self, sub_objective: SubObjective<V>) {
        self.sub_objectives.push(sub_objective);
    }

    /// 子目标数量 / Sub-objective count
    pub fn len(&self) -> usize {
        self.sub_objectives.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.sub_objectives.is_empty()
    }
}

impl<V> Objective<V>
where
    V: Zero + Clone + Add<Output = V>,
{
    /// 所有子目标的权重总和 / Total weight of all sub-objectives
    pub fn total_weight(&self) -> V {
        self.sub_objectives
            .iter()
            .fold(V::zero(), |acc, so| acc + so.weight.clone())
    }
}

impl<V> Default for Objective<V> {
    fn default() -> Self {
        Self {
            category: ObjectiveCategory::default(),
            sub_objectives: Vec::new(),
        }
    }
}
