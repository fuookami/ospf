//! 元约束定义
//! Meta Constraint Definition

use std::sync::Arc;

use crate::model::basic::ConstraintPriority;

use super::{ConstraintGroup, LinearInequality, QuadraticInequality};

/// 不等式 trait / Inequality Trait
pub trait InequalityTrait: Clone + Send + Sync + 'static {}

impl<V: Clone + Send + Sync + 'static> InequalityTrait for LinearInequality<V> {}
impl<V: Clone + Send + Sync + 'static> InequalityTrait for QuadraticInequality<V> {}

/// 元约束 / Meta Constraint
///
/// 支持延迟求值和分组的约束。
/// Constraint supporting lazy evaluation and grouping.
#[derive(Debug, Clone)]
pub struct MetaConstraint<I: InequalityTrait> {
    /// 约束 / Constraint
    pub inequality: I,
    /// 约束名称 / Constraint name
    pub name: String,
    /// 约束组 / Constraint group
    pub group: Option<Arc<ConstraintGroup>>,
    /// 是否延迟求值 / Whether lazy evaluation
    pub lazy: bool,
    /// 优先级 / Priority
    pub priority: u32,
    pub args: Option<String>,
}

impl<I: InequalityTrait> MetaConstraint<I> {
    /// 创建新元约束 / Create new meta constraint
    pub fn new(inequality: I, name: &str) -> Self {
        Self {
            inequality,
            name: name.to_string(),
            group: None,
            lazy: false,
            priority: 0,
            args: None,
        }
    }

    /// 创建延迟求值的元约束 / Create lazy meta constraint
    pub fn lazy(inequality: I, name: &str) -> Self {
        Self {
            inequality,
            name: name.to_string(),
            group: None,
            lazy: true,
            priority: 0,
            args: None,
        }
    }

    /// 设置约束组 / Set constraint group
    pub fn with_group(mut self, group: Arc<ConstraintGroup>) -> Self {
        self.group = Some(group);
        self
    }

    /// 设置优先级 / Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// 设置语义优先级 / Set semantic priority
    pub fn with_constraint_priority(mut self, priority: ConstraintPriority) -> Self {
        self.priority = priority.into();
        self
    }

    pub fn with_args(mut self, args: impl Into<String>) -> Self {
        self.args = Some(args.into());
        self
    }

    /// 设置延迟求值 / Set lazy evaluation
    pub fn set_lazy(&mut self, lazy: bool) {
        self.lazy = lazy;
    }
}

/// 线性元约束 / Linear Meta Constraint
pub type LinearMetaConstraint<V = f64> = MetaConstraint<LinearInequality<V>>;

/// 二次元约束 / Quadratic Meta Constraint
pub type QuadraticMetaConstraint<V = f64> = MetaConstraint<QuadraticInequality<V>>;
