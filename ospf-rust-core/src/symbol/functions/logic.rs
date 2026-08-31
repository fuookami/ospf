//! 逻辑函数符号
//! Logic Function Symbols

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::flatten::{Linear, LinearMonomial, Quadratic};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::token::{Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    LogicFunctionSymbol,
};
use super::big_m::{BigMPolicy, infer_big_m_for_polynomials, infer_linear_abs_bound_from_tokens};

fn evaluate_linear(
    poly: &Linear<f64>,
    token_table: &dyn TokenList<f64>,
    zero_if_none: bool,
) -> Option<f64> {
    let mut value = *poly.constant_term();
    for monomial in poly.monomials() {
        let term_value = match token_table
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => 0.0,
            None => return None,
        };
        value += monomial.coefficient() * term_value;
    }
    Some(value)
}

fn as_binary(value: f64) -> f64 {
    if value.abs() <= f64::EPSILON {
        0.0
    } else {
        1.0
    }
}

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);
const NONZERO_TOLERANCE: f64 = f64::EPSILON * 16.0;
const STRICT_NONZERO_BOUNDARY: f64 = NONZERO_TOLERANCE + f64::EPSILON * 16.0;

fn nonzero_indicator_inequalities(
    polynomial: &Linear<f64>,
    indicator_index: usize,
    side_index: usize,
    big_m: f64,
    name_prefix: &str,
) -> Vec<(LinearInequality<f64>, String)> {
    let mut constraints = Vec::with_capacity(4);

    let mut ub_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
    for monomial in polynomial.monomials() {
        ub_monomials.push(LinearMonomial::new(
            *monomial.coefficient(),
            monomial.var_index(),
        ));
    }
    ub_monomials.push(LinearMonomial::new(-big_m, indicator_index));
    constraints.push((
        LinearInequality::new(
            Linear::new(ub_monomials, *polynomial.constant_term()),
            ConstraintRelation::LessEqual,
            NONZERO_TOLERANCE,
        ),
        format!("{}_band_ub", name_prefix),
    ));

    let mut lb_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
    for monomial in polynomial.monomials() {
        lb_monomials.push(LinearMonomial::new(
            *monomial.coefficient(),
            monomial.var_index(),
        ));
    }
    lb_monomials.push(LinearMonomial::new(big_m, indicator_index));
    constraints.push((
        LinearInequality::new(
            Linear::new(lb_monomials, *polynomial.constant_term()),
            ConstraintRelation::GreaterEqual,
            -NONZERO_TOLERANCE,
        ),
        format!("{}_band_lb", name_prefix),
    ));

    let mut out_lb_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
    for monomial in polynomial.monomials() {
        out_lb_monomials.push(LinearMonomial::new(
            *monomial.coefficient(),
            monomial.var_index(),
        ));
    }
    out_lb_monomials.push(LinearMonomial::new(-big_m, indicator_index));
    out_lb_monomials.push(LinearMonomial::new(-big_m, side_index));
    constraints.push((
        LinearInequality::new(
            Linear::new(out_lb_monomials, *polynomial.constant_term()),
            ConstraintRelation::GreaterEqual,
            STRICT_NONZERO_BOUNDARY - 2.0 * big_m,
        ),
        format!("{}_out_lb", name_prefix),
    ));

    let mut out_ub_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
    for monomial in polynomial.monomials() {
        out_ub_monomials.push(LinearMonomial::new(
            *monomial.coefficient(),
            monomial.var_index(),
        ));
    }
    out_ub_monomials.push(LinearMonomial::new(big_m, indicator_index));
    out_ub_monomials.push(LinearMonomial::new(-big_m, side_index));
    constraints.push((
        LinearInequality::new(
            Linear::new(out_ub_monomials, *polynomial.constant_term()),
            ConstraintRelation::LessEqual,
            -STRICT_NONZERO_BOUNDARY + big_m,
        ),
        format!("{}_out_ub", name_prefix),
    ));

    constraints
}

/// 与函数 / And Function
#[derive(Debug)]
pub struct AndFunction {
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<f64>>,
    result_var: BinaryVariableItem,
    indicator_vars: Vec<BinaryVariableItem>,
    side_vars: Vec<BinaryVariableItem>,
}

impl AndFunction {
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<f64>>) -> Self {
        let n = polynomials.len();
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_and", name));
        let indicator_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_and_nz{}", name, i),
                )
            })
            .collect();
        let side_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, n + i + 1),
                    &format!("{}_and_side{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            indicator_vars,
            side_vars,
        }
    }

    pub fn polynomials(&self) -> &[Linear<f64>] {
        &self.polynomials
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicator_vars
    }

    pub fn side_variables(&self) -> &[BinaryVariableItem] {
        &self.side_vars
    }
}

impl AndFunction {
    fn infer_big_m_from_tokens(&self, tokens: &[Token<f64>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        if self.polynomials.len() != self.indicator_vars.len()
            || self.polynomials.len() != self.side_vars.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "and function `{}` internal auxiliary-variable size mismatch",
                self.id.name
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "and result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();
        let mut indicator_indices = Vec::with_capacity(self.indicator_vars.len());

        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let indicator_index = symbol_to_index
                .get(&(self.indicator_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "and indicator variable id {}",
                        self.indicator_vars[i].id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(self.side_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "and side variable id {}",
                        self.side_vars[i].id().unique_id()
                    ))
                })?;
            indicator_indices.push(indicator_index);

            for (inequality, name) in nonzero_indicator_inequalities(
                polynomial,
                indicator_index,
                side_index,
                big_m,
                &format!("{}_and_nz_{}", self.id.name, i),
            ) {
                constraints.push(LinearConstraint::from_symbol(
                    inequality,
                    &name,
                    source.clone(),
                ));
            }
        }

        if indicator_indices.is_empty() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(vec![LinearMonomial::new(1.0, result_index)], 0.0),
                    ConstraintRelation::Equal,
                    1.0,
                ),
                &format!("{}_and_empty", self.id.name),
                source,
            ));
            return Ok(constraints);
        }

        for (i, indicator_index) in indicator_indices.iter().copied().enumerate() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(1.0, result_index),
                            LinearMonomial::new(-1.0, indicator_index),
                        ],
                        0.0,
                    ),
                    ConstraintRelation::LessEqual,
                    0.0,
                ),
                &format!("{}_and_link_ub_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
        }

        let mut lb_monomials = Vec::with_capacity(indicator_indices.len() + 1);
        lb_monomials.push(LinearMonomial::new(1.0, result_index));
        for indicator_index in &indicator_indices {
            lb_monomials.push(LinearMonomial::new(-1.0, *indicator_index));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(lb_monomials, 0.0),
                ConstraintRelation::GreaterEqual,
                1.0 - indicator_indices.len() as f64,
            ),
            &format!("{}_and_link_lb", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl Clone for AndFunction {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            polynomials: self.polynomials.clone(),
            result_var: self.result_var.clone(),
            indicator_vars: self.indicator_vars.clone(),
            side_vars: self.side_vars.clone(),
        }
    }
}

impl Display for AndFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "and({})", self.id.name)
    }
}

impl DynSymbol for AndFunction {
    fn name(&self) -> &str {
        &self.id.name
    }

    fn display_name(&self) -> &str {
        &self.id.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.id.id as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Symbol for AndFunction {
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl IntermediateSymbol for AndFunction {
    fn category(&self) -> Category {
        Category::Linear
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol>> {
        HashSet::new()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        <Self as FunctionSymbol>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<f64>],
    ) -> Result<Vec<LinearConstraint<f64>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<f64>,
        zero_if_none: bool,
    ) -> Option<f64> {
        <Self as FunctionSymbol>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, f64>) -> Option<f64> {
        values.get(&self.result_var.index()).copied()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("and({})", self.id.name)
    }
}

impl FunctionSymbol for AndFunction {
    fn register_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for indicator_var in &self.indicator_vars {
            tokens.push(Token::from_generic(
                indicator_var.clone(),
                indicator_var.index(),
            ));
        }
        for side_var in &self.side_vars {
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<f64>, zero_if_none: bool) -> Option<f64> {
        for polynomial in &self.polynomials {
            let value = evaluate_linear(polynomial, token_table, zero_if_none)?;
            if as_binary(value) == 0.0 {
                return Some(0.0);
            }
        }
        Some(1.0)
    }
}

impl LinearIntermediateSymbol for AndFunction {
    fn to_linear_polynomial(&self) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, self.result_var.index())], 0.0)
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<f64> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl LogicFunctionSymbol for AndFunction {}

/// 或函数 / Or Function
#[derive(Debug)]
pub struct OrFunction {
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<f64>>,
    result_var: BinaryVariableItem,
    indicator_vars: Vec<BinaryVariableItem>,
    side_vars: Vec<BinaryVariableItem>,
}

impl OrFunction {
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<f64>>) -> Self {
        let n = polynomials.len();
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_or", name));
        let indicator_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_or_nz{}", name, i),
                )
            })
            .collect();
        let side_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, n + i + 1),
                    &format!("{}_or_side{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            indicator_vars,
            side_vars,
        }
    }

    pub fn polynomials(&self) -> &[Linear<f64>] {
        &self.polynomials
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicator_vars
    }

    pub fn side_variables(&self) -> &[BinaryVariableItem] {
        &self.side_vars
    }
}

impl OrFunction {
    fn infer_big_m_from_tokens(&self, tokens: &[Token<f64>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        if self.polynomials.len() != self.indicator_vars.len()
            || self.polynomials.len() != self.side_vars.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "or function `{}` internal auxiliary-variable size mismatch",
                self.id.name
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "or result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();
        let mut indicator_indices = Vec::with_capacity(self.indicator_vars.len());

        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let indicator_index = symbol_to_index
                .get(&(self.indicator_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "or indicator variable id {}",
                        self.indicator_vars[i].id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(self.side_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "or side variable id {}",
                        self.side_vars[i].id().unique_id()
                    ))
                })?;
            indicator_indices.push(indicator_index);

            for (inequality, name) in nonzero_indicator_inequalities(
                polynomial,
                indicator_index,
                side_index,
                big_m,
                &format!("{}_or_nz_{}", self.id.name, i),
            ) {
                constraints.push(LinearConstraint::from_symbol(
                    inequality,
                    &name,
                    source.clone(),
                ));
            }
        }

        if indicator_indices.is_empty() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(vec![LinearMonomial::new(1.0, result_index)], 0.0),
                    ConstraintRelation::Equal,
                    0.0,
                ),
                &format!("{}_or_empty", self.id.name),
                source,
            ));
            return Ok(constraints);
        }

        for (i, indicator_index) in indicator_indices.iter().copied().enumerate() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(1.0, result_index),
                            LinearMonomial::new(-1.0, indicator_index),
                        ],
                        0.0,
                    ),
                    ConstraintRelation::GreaterEqual,
                    0.0,
                ),
                &format!("{}_or_link_lb_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
        }

        let mut ub_monomials = Vec::with_capacity(indicator_indices.len() + 1);
        ub_monomials.push(LinearMonomial::new(1.0, result_index));
        for indicator_index in &indicator_indices {
            ub_monomials.push(LinearMonomial::new(-1.0, *indicator_index));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(ub_monomials, 0.0),
                ConstraintRelation::LessEqual,
                0.0,
            ),
            &format!("{}_or_link_ub", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl Clone for OrFunction {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            polynomials: self.polynomials.clone(),
            result_var: self.result_var.clone(),
            indicator_vars: self.indicator_vars.clone(),
            side_vars: self.side_vars.clone(),
        }
    }
}

impl Display for OrFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "or({})", self.id.name)
    }
}

impl DynSymbol for OrFunction {
    fn name(&self) -> &str {
        &self.id.name
    }

    fn display_name(&self) -> &str {
        &self.id.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.id.id as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Symbol for OrFunction {
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl IntermediateSymbol for OrFunction {
    fn category(&self) -> Category {
        Category::Linear
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol>> {
        HashSet::new()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        <Self as FunctionSymbol>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<f64>],
    ) -> Result<Vec<LinearConstraint<f64>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<f64>,
        zero_if_none: bool,
    ) -> Option<f64> {
        <Self as FunctionSymbol>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, f64>) -> Option<f64> {
        values.get(&self.result_var.index()).copied()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("or({})", self.id.name)
    }
}

impl FunctionSymbol for OrFunction {
    fn register_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for indicator_var in &self.indicator_vars {
            tokens.push(Token::from_generic(
                indicator_var.clone(),
                indicator_var.index(),
            ));
        }
        for side_var in &self.side_vars {
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<f64>, zero_if_none: bool) -> Option<f64> {
        for polynomial in &self.polynomials {
            let value = evaluate_linear(polynomial, token_table, zero_if_none)?;
            if as_binary(value) > 0.0 {
                return Some(1.0);
            }
        }
        Some(0.0)
    }
}

impl LinearIntermediateSymbol for OrFunction {
    fn to_linear_polynomial(&self) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, self.result_var.index())], 0.0)
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<f64> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl LogicFunctionSymbol for OrFunction {}

/// 非函数 / Not Function
#[derive(Debug)]
pub struct NotFunction {
    id: IntermediateSymbolId,
    polynomial: Linear<f64>,
    result_var: BinaryVariableItem,
    indicator_var: BinaryVariableItem,
    side_var: BinaryVariableItem,
}

impl NotFunction {
    pub fn new(id: u64, name: &str, polynomial: Linear<f64>) -> Self {
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_not", name));
        let indicator_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_not_nz", name));
        let side_var =
            BinaryVariableItem::create(VariableId::new(group_id, 2), &format!("{}_not_side", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomial,
            result_var,
            indicator_var,
            side_var,
        }
    }

    pub fn polynomial(&self) -> &Linear<f64> {
        &self.polynomial
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    pub fn indicator_variable(&self) -> &BinaryVariableItem {
        &self.indicator_var
    }

    pub fn side_variable(&self) -> &BinaryVariableItem {
        &self.side_var
    }
}

impl NotFunction {
    fn infer_big_m_from_tokens(&self, tokens: &[Token<f64>]) -> Option<f64> {
        infer_linear_abs_bound_from_tokens(&self.polynomial, tokens)
            .map(|bound| bound.max(BIG_M_POLICY.min()))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "not result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let indicator_index = symbol_to_index
            .get(&(self.indicator_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "not indicator variable id {}",
                    self.indicator_var.id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.side_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "not side variable id {}",
                    self.side_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();
        for (inequality, name) in nonzero_indicator_inequalities(
            &self.polynomial,
            indicator_index,
            side_index,
            big_m,
            &format!("{}_not_nz", self.id.name),
        ) {
            constraints.push(LinearConstraint::from_symbol(
                inequality,
                &name,
                source.clone(),
            ));
        }

        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(1.0, result_index),
                        LinearMonomial::new(1.0, indicator_index),
                    ],
                    0.0,
                ),
                ConstraintRelation::Equal,
                1.0,
            ),
            &format!("{}_not_link", self.id.name),
            source,
        ));

        Ok(constraints)
    }
}

impl Clone for NotFunction {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            polynomial: self.polynomial.clone(),
            result_var: self.result_var.clone(),
            indicator_var: self.indicator_var.clone(),
            side_var: self.side_var.clone(),
        }
    }
}

impl Display for NotFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "not({})", self.id.name)
    }
}

impl DynSymbol for NotFunction {
    fn name(&self) -> &str {
        &self.id.name
    }

    fn display_name(&self) -> &str {
        &self.id.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.id.id as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Symbol for NotFunction {
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl IntermediateSymbol for NotFunction {
    fn category(&self) -> Category {
        Category::Linear
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol>> {
        HashSet::new()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        <Self as FunctionSymbol>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<f64>],
    ) -> Result<Vec<LinearConstraint<f64>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<f64>,
        zero_if_none: bool,
    ) -> Option<f64> {
        <Self as FunctionSymbol>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, f64>) -> Option<f64> {
        values.get(&self.result_var.index()).copied()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("not({})", self.id.name)
    }
}

impl FunctionSymbol for NotFunction {
    fn register_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.indicator_var.clone(),
            self.indicator_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.side_var.clone(),
            self.side_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<f64>, zero_if_none: bool) -> Option<f64> {
        let value = evaluate_linear(&self.polynomial, token_table, zero_if_none)?;
        if as_binary(value) == 0.0 {
            Some(1.0)
        } else {
            Some(0.0)
        }
    }
}

impl LinearIntermediateSymbol for NotFunction {
    fn to_linear_polynomial(&self) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, self.result_var.index())], 0.0)
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<f64> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl LogicFunctionSymbol for NotFunction {}

/// 异或函数 / Xor Function
#[derive(Debug)]
pub struct XorFunction {
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<f64>>,
    result_var: BinaryVariableItem,
    indicator_vars: Vec<BinaryVariableItem>,
    side_vars: Vec<BinaryVariableItem>,
}

impl XorFunction {
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<f64>>) -> Self {
        assert!(
            polynomials.len() >= 2,
            "XorFunction requires at least two input polynomials.",
        );
        let n = polynomials.len();
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_xor", name));
        let indicator_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_xor_nz{}", name, i),
                )
            })
            .collect();
        let side_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, n + i + 1),
                    &format!("{}_xor_side{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            indicator_vars,
            side_vars,
        }
    }

    pub fn polynomials(&self) -> &[Linear<f64>] {
        &self.polynomials
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicator_vars
    }

    pub fn side_variables(&self) -> &[BinaryVariableItem] {
        &self.side_vars
    }
}

impl XorFunction {
    fn infer_big_m_from_tokens(&self, tokens: &[Token<f64>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        if self.polynomials.len() != self.indicator_vars.len()
            || self.polynomials.len() != self.side_vars.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "xor function `{}` internal auxiliary-variable size mismatch",
                self.id.name
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "xor result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();
        let mut indicator_indices = Vec::with_capacity(self.indicator_vars.len());

        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let indicator_index = symbol_to_index
                .get(&(self.indicator_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "xor indicator variable id {}",
                        self.indicator_vars[i].id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(self.side_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "xor side variable id {}",
                        self.side_vars[i].id().unique_id()
                    ))
                })?;
            indicator_indices.push(indicator_index);

            for (inequality, name) in nonzero_indicator_inequalities(
                polynomial,
                indicator_index,
                side_index,
                big_m,
                &format!("{}_xor_nz_{}", self.id.name, i),
            ) {
                constraints.push(LinearConstraint::from_symbol(
                    inequality,
                    &name,
                    source.clone(),
                ));
            }
        }

        let mut sum_monomials = Vec::with_capacity(indicator_indices.len() + 1);
        sum_monomials.push(LinearMonomial::new(1.0, result_index));
        for indicator_index in &indicator_indices {
            sum_monomials.push(LinearMonomial::new(-1.0, *indicator_index));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(sum_monomials, 0.0),
                ConstraintRelation::LessEqual,
                0.0,
            ),
            &format!("{}_xor_sum_ub", self.id.name),
            source.clone(),
        ));

        let mut all_one_monomials = Vec::with_capacity(indicator_indices.len() + 1);
        all_one_monomials.push(LinearMonomial::new(1.0, result_index));
        for indicator_index in &indicator_indices {
            all_one_monomials.push(LinearMonomial::new(1.0, *indicator_index));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(all_one_monomials, 0.0),
                ConstraintRelation::LessEqual,
                indicator_indices.len() as f64,
            ),
            &format!("{}_xor_all_one_ub", self.id.name),
            source.clone(),
        ));

        for i in 0..indicator_indices.len() {
            for j in (i + 1)..indicator_indices.len() {
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            vec![
                                LinearMonomial::new(1.0, result_index),
                                LinearMonomial::new(-1.0, indicator_indices[i]),
                                LinearMonomial::new(1.0, indicator_indices[j]),
                            ],
                            0.0,
                        ),
                        ConstraintRelation::GreaterEqual,
                        0.0,
                    ),
                    &format!("{}_xor_diff_lb_{}_{}", self.id.name, i, j),
                    source.clone(),
                ));
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            vec![
                                LinearMonomial::new(1.0, result_index),
                                LinearMonomial::new(1.0, indicator_indices[i]),
                                LinearMonomial::new(-1.0, indicator_indices[j]),
                            ],
                            0.0,
                        ),
                        ConstraintRelation::GreaterEqual,
                        0.0,
                    ),
                    &format!("{}_xor_diff_lb_{}_{}_rev", self.id.name, i, j),
                    source.clone(),
                ));
            }
        }

        Ok(constraints)
    }
}

impl Clone for XorFunction {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            polynomials: self.polynomials.clone(),
            result_var: self.result_var.clone(),
            indicator_vars: self.indicator_vars.clone(),
            side_vars: self.side_vars.clone(),
        }
    }
}

impl Display for XorFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "xor({})", self.id.name)
    }
}

impl DynSymbol for XorFunction {
    fn name(&self) -> &str {
        &self.id.name
    }

    fn display_name(&self) -> &str {
        &self.id.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.id.id as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Symbol for XorFunction {
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl IntermediateSymbol for XorFunction {
    fn category(&self) -> Category {
        Category::Linear
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol>> {
        HashSet::new()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        <Self as FunctionSymbol>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<f64>],
    ) -> Result<Vec<LinearConstraint<f64>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<f64>,
        zero_if_none: bool,
    ) -> Option<f64> {
        <Self as FunctionSymbol>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, f64>) -> Option<f64> {
        values.get(&self.result_var.index()).copied()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("xor({})", self.id.name)
    }
}

impl FunctionSymbol for XorFunction {
    fn register_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for indicator_var in &self.indicator_vars {
            tokens.push(Token::from_generic(
                indicator_var.clone(),
                indicator_var.index(),
            ));
        }
        for side_var in &self.side_vars {
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<f64>, zero_if_none: bool) -> Option<f64> {
        let mut has_zero = false;
        let mut has_non_zero = false;

        for polynomial in &self.polynomials {
            let value = evaluate_linear(polynomial, token_table, zero_if_none)?;
            if as_binary(value) == 0.0 {
                has_zero = true;
            } else {
                has_non_zero = true;
            }
            if has_zero && has_non_zero {
                return Some(1.0);
            }
        }

        Some(0.0)
    }
}

impl LinearIntermediateSymbol for XorFunction {
    fn to_linear_polynomial(&self) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, self.result_var.index())], 0.0)
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<f64> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl LogicFunctionSymbol for XorFunction {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableRange};

    fn bool_poly(var_index: usize) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, var_index)], 0.0)
    }

    fn make_binary_tokens(x: f64, y: f64) -> VecTokenList<f64> {
        let x_var = BinaryVariableItem::create(VariableId::standalone(0), "x");
        let y_var = BinaryVariableItem::create(VariableId::standalone(1), "y");

        let tx = Token::from_generic(x_var, 0);
        tx.set_result(x);
        let ty = Token::from_generic(y_var, 1);
        ty.set_result(y);

        let mut tokens = VecTokenList::new();
        tokens.add_token(tx);
        tokens.add_token(ty);
        tokens
    }

    #[test]
    fn and_or_not_xor_calculate_values() {
        let tokens = make_binary_tokens(1.0, 0.0);
        let p0 = bool_poly(0);
        let p1 = bool_poly(1);

        let and_fn = AndFunction::new(100, "and_xy", vec![p0.clone(), p1.clone()]);
        let or_fn = OrFunction::new(101, "or_xy", vec![p0.clone(), p1.clone()]);
        let not_fn = NotFunction::new(102, "not_y", p1.clone());
        let xor_fn = XorFunction::new(103, "xor_xy", vec![p0, p1]);

        assert_eq!(
            <AndFunction as FunctionSymbol>::calculate_value(&and_fn, &tokens, false),
            Some(0.0)
        );
        assert_eq!(
            <OrFunction as FunctionSymbol>::calculate_value(&or_fn, &tokens, false),
            Some(1.0)
        );
        assert_eq!(
            <NotFunction as FunctionSymbol>::calculate_value(&not_fn, &tokens, false),
            Some(1.0)
        );
        assert_eq!(
            <XorFunction as FunctionSymbol>::calculate_value(&xor_fn, &tokens, false),
            Some(1.0)
        );
    }

    #[test]
    fn xor_all_equal_is_zero() {
        let tokens = make_binary_tokens(1.0, 1.0);
        let xor_fn = XorFunction::new(104, "xor_equal", vec![bool_poly(0), bool_poly(1)]);
        assert_eq!(
            <XorFunction as FunctionSymbol>::calculate_value(&xor_fn, &tokens, false),
            Some(0.0)
        );
    }

    #[test]
    fn and_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(80_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let and_fn = AndFunction::new(
            2000,
            "and_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
        );

        let mut aux_tokens = Vec::new();
        <AndFunction as FunctionSymbol>::register_tokens(&and_fn, &mut aux_tokens)
            .expect("and tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = and_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("and constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "and_bound_and_nz_0_band_ub")
            .expect("and band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(and_fn.indicator_variables()[0].id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + 7.0).abs() <= 1e-9);
    }

    #[test]
    fn and_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(80_010), "x");
        let and_fn = AndFunction::new(
            2001,
            "and_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
        );

        let mut aux_tokens = Vec::new();
        <AndFunction as FunctionSymbol>::register_tokens(&and_fn, &mut aux_tokens)
            .expect("and tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = and_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("and constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "and_default_and_nz_0_band_ub")
            .expect("and band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(and_fn.indicator_variables()[0].id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }

    #[test]
    fn not_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(81_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let not_fn = NotFunction::new(
            2100,
            "not_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let mut aux_tokens = Vec::new();
        <NotFunction as FunctionSymbol>::register_tokens(&not_fn, &mut aux_tokens)
            .expect("not tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = not_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("not constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "not_bound_not_nz_band_ub")
            .expect("not band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(not_fn.indicator_variable().id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + 7.0).abs() <= 1e-9);
    }

    #[test]
    fn not_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(81_010), "x");
        let not_fn = NotFunction::new(
            2101,
            "not_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <NotFunction as FunctionSymbol>::register_tokens(&not_fn, &mut aux_tokens)
            .expect("not tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = not_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("not constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "not_default_not_nz_band_ub")
            .expect("not band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(not_fn.indicator_variable().id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
