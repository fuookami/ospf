//! 约束定义。 / Constraint definitions.

use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use ospf_rust_math::symbol::{Linear as SymbolicLinear, Quadratic as SymbolicQuadratic};
use crate::error::{ModelError, Result};
use crate::model::basic::ConstraintPriority;
use crate::symbol::IntermediateSymbol;
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use super::ConstraintGroup;

/// 约束关系。 / Constraint relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintRelation {
    /// 小于等于 / Less than or equal to
    LessEqual,
    /// 等于 / Equal to
    Equal,
    /// 大于等于 / Greater than or equal to
    GreaterEqual,
}

/// 线性不等式：多项式 关系 右端项。 / Linear inequality: polynomial relation rhs.
#[derive(Debug, Clone)]
pub struct LinearInequality<V = f64> {
    /// 多项式 / Polynomial
    pub polynomial: Linear<V>,
    /// 约束关系 / Constraint relation
    pub relation: ConstraintRelation,
    /// 右端项 / Right-hand side value
    pub rhs: V,
}

impl<V> LinearInequality<V> {
    /// 创建新的线性不等式。 / Create a new linear inequality.
    pub fn new(polynomial: Linear<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }

    /// 创建小于等于的线性不等式。 / Create a less-than-or-equal-to linear inequality.
    pub fn less_equal(polynomial: Linear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::LessEqual, rhs)
    }

    /// 创建等于的线性不等式。 / Create an equal-to linear inequality.
    pub fn equal(polynomial: Linear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::Equal, rhs)
    }

    /// 创建大于等于的线性不等式。 / Create a greater-than-or-equal-to linear inequality.
    pub fn greater_equal(polynomial: Linear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::GreaterEqual, rhs)
    }
}

/// 二次不等式：多项式 关系 右端项。 / Quadratic inequality: polynomial relation rhs.
#[derive(Debug, Clone)]
pub struct QuadraticInequality<V = f64> {
    /// 多项式 / Polynomial
    pub polynomial: Quadratic<V>,
    /// 约束关系 / Constraint relation
    pub relation: ConstraintRelation,
    /// 右端项 / Right-hand side value
    pub rhs: V,
}

impl<V> QuadraticInequality<V> {
    /// 创建新的二次不等式。 / Create a new quadratic inequality.
    pub fn new(polynomial: Quadratic<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }
}

/// 通用约束包装器。 / Generic constraint wrapper.
#[derive(Debug, Clone)]
pub struct Constraint<V, P>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 不等式 / Inequality
    pub inequality: P,
    /// 约束名称 / Constraint name
    pub name: String,
    /// 约束分组 / Constraint group
    pub group: Option<Arc<ConstraintGroup>>,
    /// 是否为惰性约束 / Whether this is a lazy constraint
    pub lazy: bool,
    /// 约束优先级 / Constraint priority
    pub priority: u32,
    /// 约束参数 / Constraint arguments
    pub args: Option<String>,
    /// 来源符号 / Source symbol
    pub from: Option<Arc<dyn IntermediateSymbol<V>>>,
}

impl<V, P> Constraint<V, P>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 创建新的约束。 / Create a new constraint.
    pub fn new(inequality: P, name: &str) -> Self {
        Self {
            inequality,
            name: name.to_string(),
            group: None,
            lazy: false,
            priority: 0,
            args: None,
            from: None,
        }
    }

    /// 从符号创建约束。 / Create a constraint from a symbol.
    pub fn from_symbol(inequality: P, name: &str, from: Arc<dyn IntermediateSymbol<V>>) -> Self {
        Self {
            inequality,
            name: name.to_string(),
            group: None,
            lazy: false,
            priority: 0,
            args: None,
            from: Some(from),
        }
    }

    /// 设置来源符号。 / Set the source symbol.
    pub fn set_from(&mut self, from: Arc<dyn IntermediateSymbol<V>>) {
        self.from = Some(from);
    }

    /// 设置约束分组并返回自身。 / Set the constraint group and return self.
    pub fn with_group(mut self, group: Arc<ConstraintGroup>) -> Self {
        self.group = Some(group);
        self
    }

    /// 设置约束优先级并返回自身。 / Set the constraint priority and return self.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// 使用约束优先级类型设置优先级并返回自身。 / Set priority using constraint priority type and return self.
    pub fn with_constraint_priority(mut self, priority: ConstraintPriority) -> Self {
        self.priority = priority.into();
        self
    }

    /// 设置约束参数并返回自身。 / Set the constraint arguments and return self.
    pub fn with_args(mut self, args: impl Into<String>) -> Self {
        self.args = Some(args.into());
        self
    }

    /// 设置是否为惰性约束。 / Set whether this is a lazy constraint.
    pub fn set_lazy(&mut self, lazy: bool) {
        self.lazy = lazy;
    }
}

/// 线性约束 / Linear constraint
pub type LinearConstraint<V = f64> = Constraint<V, LinearInequality<V>>;
/// 二次约束 / Quadratic constraint
pub type QuadraticConstraint<V = f64> = Constraint<V, QuadraticInequality<V>>;

/// 元模型使用的符号线性不等式。 / Symbolic linear inequality used by MetaModel.
#[derive(Debug, Clone)]
pub struct SymbolicLinearInequality<V = f64> {
    /// 多项式 / Polynomial
    pub polynomial: SymbolicLinear<V>,
    /// 约束关系 / Constraint relation
    pub relation: ConstraintRelation,
    /// 右端项 / Right-hand side value
    pub rhs: V,
}

impl<V> SymbolicLinearInequality<V> {
    /// 创建新的符号线性不等式。 / Create a new symbolic linear inequality.
    pub fn new(polynomial: SymbolicLinear<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }

    /// 创建小于等于的符号线性不等式。 / Create a less-than-or-equal-to symbolic linear inequality.
    pub fn less_equal(polynomial: SymbolicLinear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::LessEqual, rhs)
    }

    /// 创建等于的符号线性不等式。 / Create an equal-to symbolic linear inequality.
    pub fn equal(polynomial: SymbolicLinear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::Equal, rhs)
    }

    /// 创建大于等于的符号线性不等式。 / Create a greater-than-or-equal-to symbolic linear inequality.
    pub fn greater_equal(polynomial: SymbolicLinear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::GreaterEqual, rhs)
    }

    /// 尝试将符号线性不等式转换为机制线性不等式。 / Try to convert a symbolic linear inequality into a mechanism linear inequality.
    pub fn try_into_linear_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<LinearInequality<V>>
    where
        V: Clone + Add<Output = V>,
    {
        let mut monomial_positions: HashMap<usize, usize> = HashMap::new();
        let mut monomials: Vec<LinearMonomial<V>> = Vec::new();
        for m in self.polynomial.monomials {
            let dyn_id = m.symbol.dyn_id();
            if !dyn_id.is_standalone() {
                return Err(ModelError::InvalidConstraint(format!(
                    "symbol {} is not standalone in symbolic linear inequality",
                    dyn_id.parent_id
                ))
                .into());
            }
            let symbol_id = dyn_id.parent_id;
            let var_index = symbol_to_index.get(&symbol_id).copied().ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "symbol id {} not found during symbolic inequality conversion",
                    symbol_id
                ))
            })?;
            if let Some(position) = monomial_positions.get(&var_index) {
                let coefficient = monomials[*position].coefficient().clone() + m.coefficient;
                monomials[*position].set_coefficient(coefficient);
            } else {
                monomial_positions.insert(var_index, monomials.len());
                monomials.push(LinearMonomial::new(m.coefficient, var_index));
            }
        }

        let polynomial = Linear::new(monomials, self.polynomial.constant);
        Ok(LinearInequality::new(polynomial, self.relation, self.rhs))
    }

    /// 将符号线性不等式转换为机制线性不等式，失败时 panic。 / Convert a symbolic linear inequality into a mechanism linear inequality, panicking on failure.
    pub fn into_linear_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> LinearInequality<V>
    where
        V: Clone + Add<Output = V>,
    {
        self.try_into_linear_inequality(symbol_to_index)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to convert symbolic linear inequality to mechanism inequality: {}",
                    err
                )
            })
    }
}

/// 元模型使用的符号二次不等式。 / Symbolic quadratic inequality used by MetaModel.
#[derive(Debug, Clone)]
pub struct SymbolicQuadraticInequality<V = f64> {
    /// 多项式 / Polynomial
    pub polynomial: SymbolicQuadratic<V>,
    /// 约束关系 / Constraint relation
    pub relation: ConstraintRelation,
    /// 右端项 / Right-hand side value
    pub rhs: V,
}

impl<V> SymbolicQuadraticInequality<V> {
    /// 创建新的符号二次不等式。 / Create a new symbolic quadratic inequality.
    pub fn new(polynomial: SymbolicQuadratic<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }

    /// 尝试将符号二次不等式转换为机制二次不等式。 / Try to convert a symbolic quadratic inequality into a mechanism quadratic inequality.
    pub fn try_into_quadratic_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<QuadraticInequality<V>>
    where
        V: Clone + Add<Output = V>,
    {
        let mut monomial_positions: HashMap<(usize, Option<usize>), usize> = HashMap::new();
        let mut monomials: Vec<((usize, Option<usize>), V)> = Vec::new();
        for m in self.polynomial.monomials {
            let dyn_id1 = m.symbol1.dyn_id();
            if !dyn_id1.is_standalone() {
                return Err(ModelError::InvalidConstraint(format!(
                    "symbol {} is not standalone in symbolic quadratic inequality",
                    dyn_id1.parent_id
                ))
                .into());
            }
            let var_index1 = symbol_to_index
                .get(&dyn_id1.parent_id)
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "symbol id {} not found during symbolic quadratic inequality conversion",
                        dyn_id1.parent_id
                    ))
                })?;

            if let Some(symbol2) = m.symbol2 {
                let dyn_id2 = symbol2.dyn_id();
                if !dyn_id2.is_standalone() {
                    return Err(ModelError::InvalidConstraint(format!(
                        "symbol {} is not standalone in symbolic quadratic inequality",
                        dyn_id2.parent_id
                    ))
                    .into());
                }
                let var_index2 = symbol_to_index
                    .get(&dyn_id2.parent_id)
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "symbol id {} not found during symbolic quadratic inequality conversion",
                            dyn_id2.parent_id
                        ))
                    })?;
                let key = if var_index1 <= var_index2 {
                    (var_index1, Some(var_index2))
                } else {
                    (var_index2, Some(var_index1))
                };
                if let Some(position) = monomial_positions.get(&key) {
                    let coefficient = monomials[*position].1.clone() + m.coefficient;
                    monomials[*position].1 = coefficient;
                } else {
                    monomial_positions.insert(key, monomials.len());
                    monomials.push((key, m.coefficient));
                }
            } else {
                let key = (var_index1, None);
                if let Some(position) = monomial_positions.get(&key) {
                    let coefficient = monomials[*position].1.clone() + m.coefficient;
                    monomials[*position].1 = coefficient;
                } else {
                    monomial_positions.insert(key, monomials.len());
                    monomials.push((key, m.coefficient));
                }
            }
        }

        let monomials = monomials
            .into_iter()
            .map(|((var_index1, var_index2), coefficient)| {
                if let Some(var_index2) = var_index2 {
                    QuadraticMonomial::new_quadratic(coefficient, var_index1, var_index2)
                } else {
                    QuadraticMonomial::new_linear(coefficient, var_index1)
                }
            })
            .collect();
        let polynomial = Quadratic::new(monomials, self.polynomial.constant);
        Ok(QuadraticInequality::new(
            polynomial,
            self.relation,
            self.rhs,
        ))
    }

    /// 将符号二次不等式转换为机制二次不等式，失败时 panic。 / Convert a symbolic quadratic inequality into a mechanism quadratic inequality, panicking on failure.
    pub fn into_quadratic_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> QuadraticInequality<V>
    where
        V: Clone + Add<Output = V>,
    {
        self.try_into_quadratic_inequality(symbol_to_index)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to convert symbolic quadratic inequality to mechanism inequality: {}",
                    err
                )
            })
    }
}

/// 符号线性约束 / Symbolic linear constraint
pub type SymbolicLinearConstraint<V = f64> = Constraint<V, SymbolicLinearInequality<V>>;
/// 符号二次约束 / Symbolic quadratic constraint
pub type SymbolicQuadraticConstraint<V = f64> = Constraint<V, SymbolicQuadraticInequality<V>>;
