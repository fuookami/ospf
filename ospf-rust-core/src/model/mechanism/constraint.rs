//! Constraint definitions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ModelError, Result};
use crate::model::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::symbol::IntermediateSymbol;
use ospf_rust_math::symbol::{Linear as SymbolicLinear, Quadratic as SymbolicQuadratic};

use super::ConstraintGroup;

/// Constraint relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintRelation {
    LessEqual,
    Equal,
    GreaterEqual,
}

/// Linear inequality: polynomial relation rhs.
#[derive(Debug, Clone)]
pub struct LinearInequality<V = f64> {
    pub polynomial: Linear<V>,
    pub relation: ConstraintRelation,
    pub rhs: V,
}

impl<V> LinearInequality<V> {
    pub fn new(polynomial: Linear<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }

    pub fn less_equal(polynomial: Linear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::LessEqual, rhs)
    }

    pub fn equal(polynomial: Linear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::Equal, rhs)
    }

    pub fn greater_equal(polynomial: Linear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::GreaterEqual, rhs)
    }
}

/// Quadratic inequality: polynomial relation rhs.
#[derive(Debug, Clone)]
pub struct QuadraticInequality<V = f64> {
    pub polynomial: Quadratic<V>,
    pub relation: ConstraintRelation,
    pub rhs: V,
}

impl<V> QuadraticInequality<V> {
    pub fn new(polynomial: Quadratic<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }
}

/// Generic constraint wrapper.
#[derive(Debug, Clone)]
pub struct Constraint<V, P>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    pub inequality: P,
    pub name: String,
    pub group: Option<Arc<ConstraintGroup>>,
    pub lazy: bool,
    pub priority: u32,
    pub args: Option<String>,
    pub from: Option<Arc<dyn IntermediateSymbol<V>>>,
}

impl<V, P> Constraint<V, P>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
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

    pub fn set_from(&mut self, from: Arc<dyn IntermediateSymbol<V>>) {
        self.from = Some(from);
    }

    pub fn with_group(mut self, group: Arc<ConstraintGroup>) -> Self {
        self.group = Some(group);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_args(mut self, args: impl Into<String>) -> Self {
        self.args = Some(args.into());
        self
    }

    pub fn set_lazy(&mut self, lazy: bool) {
        self.lazy = lazy;
    }
}

pub type LinearConstraint<V = f64> = Constraint<V, LinearInequality<V>>;
pub type QuadraticConstraint<V = f64> = Constraint<V, QuadraticInequality<V>>;

/// Symbolic linear inequality used by MetaModel.
#[derive(Debug, Clone)]
pub struct SymbolicLinearInequality<V = f64> {
    pub polynomial: SymbolicLinear<V>,
    pub relation: ConstraintRelation,
    pub rhs: V,
}

impl<V> SymbolicLinearInequality<V> {
    pub fn new(polynomial: SymbolicLinear<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }

    pub fn less_equal(polynomial: SymbolicLinear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::LessEqual, rhs)
    }

    pub fn equal(polynomial: SymbolicLinear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::Equal, rhs)
    }

    pub fn greater_equal(polynomial: SymbolicLinear<V>, rhs: V) -> Self {
        Self::new(polynomial, ConstraintRelation::GreaterEqual, rhs)
    }

    pub fn try_into_linear_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<LinearInequality<V>> {
        let mut monomials: Vec<LinearMonomial<V>> =
            Vec::with_capacity(self.polynomial.monomials.len());
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
            monomials.push(LinearMonomial::new(m.coefficient, var_index));
        }

        let polynomial = Linear::new(monomials, self.polynomial.constant);
        Ok(LinearInequality::new(polynomial, self.relation, self.rhs))
    }

    pub fn into_linear_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> LinearInequality<V> {
        self.try_into_linear_inequality(symbol_to_index)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to convert symbolic linear inequality to mechanism inequality: {}",
                    err
                )
            })
    }
}

/// Symbolic quadratic inequality used by MetaModel.
#[derive(Debug, Clone)]
pub struct SymbolicQuadraticInequality<V = f64> {
    pub polynomial: SymbolicQuadratic<V>,
    pub relation: ConstraintRelation,
    pub rhs: V,
}

impl<V> SymbolicQuadraticInequality<V> {
    pub fn new(polynomial: SymbolicQuadratic<V>, relation: ConstraintRelation, rhs: V) -> Self {
        Self {
            polynomial,
            relation,
            rhs,
        }
    }

    pub fn try_into_quadratic_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<QuadraticInequality<V>> {
        let mut monomials: Vec<QuadraticMonomial<V>> =
            Vec::with_capacity(self.polynomial.monomials.len());
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
                monomials.push(QuadraticMonomial::new_quadratic(
                    m.coefficient,
                    var_index1,
                    var_index2,
                ));
            } else {
                monomials.push(QuadraticMonomial::new_linear(m.coefficient, var_index1));
            }
        }

        let polynomial = Quadratic::new(monomials, self.polynomial.constant);
        Ok(QuadraticInequality::new(
            polynomial,
            self.relation,
            self.rhs,
        ))
    }

    pub fn into_quadratic_inequality(
        self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> QuadraticInequality<V> {
        self.try_into_quadratic_inequality(symbol_to_index)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to convert symbolic quadratic inequality to mechanism inequality: {}",
                    err
                )
            })
    }
}

pub type SymbolicLinearConstraint<V = f64> = Constraint<V, SymbolicLinearInequality<V>>;
pub type SymbolicQuadraticConstraint<V = f64> = Constraint<V, SymbolicQuadraticInequality<V>>;
