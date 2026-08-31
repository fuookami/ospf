//! Objective function definitions.

use super::flatten::Linear;
use num_traits::{One, Zero};
use std::ops::Add;

/// Objective direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectiveCategory {
    /// Minimize.
    #[default]
    Minimum,
    /// Maximize.
    Maximum,
}

impl ObjectiveCategory {
    /// Whether minimizing.
    pub fn is_minimum(&self) -> bool {
        matches!(self, ObjectiveCategory::Minimum)
    }

    /// Whether maximizing.
    pub fn is_maximum(&self) -> bool {
        matches!(self, ObjectiveCategory::Maximum)
    }
}

/// Sub-objective.
#[derive(Debug, Clone)]
pub struct SubObjective<V = f64> {
    /// Objective direction.
    pub category: ObjectiveCategory,
    /// Polynomial.
    pub polynomial: Linear<V>,
    /// Name.
    pub name: String,
    /// Weight.
    pub weight: V,
}

impl<V> SubObjective<V> {
    /// Create sub-objective with explicit weight.
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

    /// Set weight.
    pub fn with_weight(mut self, weight: V) -> Self {
        self.weight = weight;
        self
    }
}

impl<V> SubObjective<V>
where
    V: One,
{
    /// Create sub-objective with unit default weight.
    pub fn new(category: ObjectiveCategory, polynomial: Linear<V>, name: &str) -> Self {
        Self::new_with_weight(category, polynomial, name, V::one())
    }

    /// Create minimization sub-objective.
    pub fn minimize(polynomial: Linear<V>, name: &str) -> Self {
        Self::new(ObjectiveCategory::Minimum, polynomial, name)
    }

    /// Create maximization sub-objective.
    pub fn maximize(polynomial: Linear<V>, name: &str) -> Self {
        Self::new(ObjectiveCategory::Maximum, polynomial, name)
    }
}

/// Objective.
#[derive(Debug, Clone)]
pub struct Objective<V = f64> {
    /// Objective direction.
    pub category: ObjectiveCategory,
    /// Sub-objectives.
    pub sub_objectives: Vec<SubObjective<V>>,
}

impl<V> Objective<V> {
    /// Create objective.
    pub fn new(category: ObjectiveCategory) -> Self {
        Self {
            category,
            sub_objectives: Vec::new(),
        }
    }

    /// Create minimization objective.
    pub fn minimize() -> Self {
        Self::new(ObjectiveCategory::Minimum)
    }

    /// Create maximization objective.
    pub fn maximize() -> Self {
        Self::new(ObjectiveCategory::Maximum)
    }

    /// Add sub-objective.
    pub fn add_sub_objective(&mut self, sub_objective: SubObjective<V>) {
        self.sub_objectives.push(sub_objective);
    }

    /// Sub-objective count.
    pub fn len(&self) -> usize {
        self.sub_objectives.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.sub_objectives.is_empty()
    }
}

impl<V> Objective<V>
where
    V: Zero + Clone + Add<Output = V>,
{
    /// Total weight of all sub-objectives.
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
