//! 排序统计函数 / Order-statistic functions

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::symbol::function::big_m::infer_linear_bounds_from_tokens;
use crate::symbol::function::max::{MaxFunction, MinFunction};
use crate::symbol::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, VariableId, VariableRange, new_group_id};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul, Sub};
use std::sync::Arc;

#[derive(Debug, Clone)]
enum Wire<V> {
    Input(Linear<V>),
    Helper(ContinuousVariableItem),
}

#[derive(Debug, Clone)]
struct CompareExchange<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    left: Wire<V>,
    right: Wire<V>,
    minimum: MinFunction<V>,
    maximum: MaxFunction<V>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatisticKind {
    ArgMin,
    ArgMax,
    KthLargest(usize),
    TopKSum(usize),
}

/// 排序统计函数的共享实现 / Shared implementation for order-statistic functions.
#[derive(Debug, Clone)]
pub struct OrderStatisticCore<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<V>>,
    result_var: ContinuousVariableItem,
    kind: StatisticKind,
    extremum: Option<MinFunction<V>>,
    arg_maximum: Option<MaxFunction<V>>,
    comparators: Vec<CompareExchange<V>>,
    sorted: Vec<Wire<V>>,
}

impl<V> OrderStatisticCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>, kind: StatisticKind) -> Result<Self>
    where
        V: FromPrimitive,
    {
        match kind {
            StatisticKind::ArgMin | StatisticKind::ArgMax | StatisticKind::KthLargest(_)
                if polynomials.is_empty() =>
            {
                return Err(ModelError::InvalidConstraint(format!(
                    "{} requires at least one candidate",
                    name
                ))
                .into());
            }
            StatisticKind::KthLargest(k) if k >= polynomials.len() => {
                return Err(ModelError::InvalidConstraint(format!(
                    "{} rank {} is outside 0..{}",
                    name,
                    k,
                    polynomials.len()
                ))
                .into());
            }
            StatisticKind::TopKSum(k) if k > polynomials.len() => {
                return Err(ModelError::InvalidConstraint(format!(
                    "{} k={} exceeds candidate count {}",
                    name,
                    k,
                    polynomials.len()
                ))
                .into());
            }
            _ => {}
        }

        let group_id = new_group_id();
        let result_range = match kind {
            StatisticKind::ArgMin | StatisticKind::ArgMax => {
                VariableRange::bounded(0.0, (polynomials.len() - 1) as f64)
            }
            _ => VariableRange::<f64>::unbounded(),
        };
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            &format!("{}_result", name),
            result_range,
        );

        let mut core = Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            kind,
            extremum: None,
            arg_maximum: None,
            comparators: Vec::new(),
            sorted: Vec::new(),
        };

        match kind {
            StatisticKind::ArgMin if core.polynomials.len() > 1 => {
                core.extremum = Some(MinFunction::new(
                    crate::symbol::next_auto_intermediate_symbol_id(),
                    format!("{}_minimum", name).as_str(),
                    core.polynomials.clone(),
                    true,
                ));
            }
            StatisticKind::ArgMax if core.polynomials.len() > 1 => {
                core.arg_maximum = Some(MaxFunction::new(
                    crate::symbol::next_auto_intermediate_symbol_id(),
                    format!("{}_maximum", name).as_str(),
                    core.polynomials.clone(),
                    true,
                ));
            }
            StatisticKind::KthLargest(_) | StatisticKind::TopKSum(_) => {
                core.build_sorting_network(name)?;
            }
            StatisticKind::ArgMin | StatisticKind::ArgMax => {}
        }
        Ok(core)
    }

    fn build_sorting_network(&mut self, name: &str) -> Result<()>
    where
        V: FromPrimitive,
    {
        let mut wires: Vec<_> = self.polynomials.iter().cloned().map(Wire::Input).collect();
        let n = wires.len();
        if matches!(self.kind, StatisticKind::TopKSum(k) if k == 0 || k == n) {
            self.sorted = wires;
            return Ok(());
        }
        for pass in 0..n {
            let mut index = pass % 2;
            while index + 1 < n {
                let left = wires[index].clone();
                let right = wires[index + 1].clone();
                let placeholder = vec![Self::placeholder(&left)?, Self::placeholder(&right)?];
                let sequence = self.comparators.len();
                let minimum = MinFunction::new(
                    crate::symbol::next_auto_intermediate_symbol_id(),
                    &format!("{}_sort_min_{}", name, sequence),
                    placeholder.clone(),
                    true,
                );
                let maximum = MaxFunction::new(
                    crate::symbol::next_auto_intermediate_symbol_id(),
                    &format!("{}_sort_max_{}", name, sequence),
                    placeholder,
                    true,
                );
                wires[index] = Wire::Helper(maximum.result_variable().clone());
                wires[index + 1] = Wire::Helper(minimum.result_variable().clone());
                self.comparators.push(CompareExchange {
                    left,
                    right,
                    minimum,
                    maximum,
                });
                index += 2;
            }
        }
        self.sorted = wires;
        Ok(())
    }

    fn placeholder(wire: &Wire<V>) -> Result<Linear<V>>
    where
        V: FromPrimitive,
    {
        match wire {
            Wire::Input(polynomial) => Ok(polynomial.clone()),
            Wire::Helper(variable) => Ok(Linear::new(
                vec![LinearMonomial::new(
                    V::from_f64(1.0).ok_or_else(|| {
                        ModelError::InvalidConstraint("cannot represent coefficient 1".into())
                    })?,
                    variable.index(),
                )],
                V::from_f64(0.0).ok_or_else(|| {
                    ModelError::InvalidConstraint("cannot represent constant 0".into())
                })?,
            )),
        }
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取候选多项式 / Get the candidate polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    fn index_for(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &ContinuousVariableItem,
        role: &str,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "{} variable id {}",
                    role,
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn wire_polynomial(wire: &Wire<V>, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>>
    where
        V: FromPrimitive,
    {
        match wire {
            Wire::Input(polynomial) => Ok(polynomial.clone()),
            Wire::Helper(variable) => Ok(Linear::new(
                vec![LinearMonomial::new(
                    V::from_f64(1.0).ok_or_else(|| {
                        ModelError::InvalidConstraint("cannot represent coefficient 1".into())
                    })?,
                    Self::index_for(symbol_to_index, variable, "sort wire")?,
                )],
                V::from_f64(0.0).ok_or_else(|| {
                    ModelError::InvalidConstraint("cannot represent constant 0".into())
                })?,
            )),
        }
    }

    fn bounds_of_wire(
        wire: &Wire<V>,
        auxiliary: &[Token<V>],
        registered: &[Token<V>],
    ) -> Option<(f64, f64)>
    where
        V: ToPrimitive,
    {
        match wire {
            Wire::Input(polynomial) => infer_linear_bounds_from_tokens(polynomial, registered),
            Wire::Helper(variable) => {
                let range = auxiliary
                    .iter()
                    .find(|token| token.id() == variable.id())?
                    .range();
                Some((range.lower_bound?.to_f64()?, range.upper_bound?.to_f64()?))
            }
        }
    }

    fn refine_bounds(&self, auxiliary: &mut [Token<V>], registered: &[Token<V>])
    where
        V: FromPrimitive + ToPrimitive,
    {
        for comparator in &self.comparators {
            let Some((left_lower, left_upper)) =
                Self::bounds_of_wire(&comparator.left, auxiliary, registered)
            else {
                continue;
            };
            let Some((right_lower, right_upper)) =
                Self::bounds_of_wire(&comparator.right, auxiliary, registered)
            else {
                continue;
            };
            let minimum_bounds = (left_lower.min(right_lower), left_upper.min(right_upper));
            let maximum_bounds = (left_lower.max(right_lower), left_upper.max(right_upper));
            for (variable, (lower, upper)) in [
                (comparator.minimum.result_variable(), minimum_bounds),
                (comparator.maximum.result_variable(), maximum_bounds),
            ] {
                if let Some(token) = auxiliary
                    .iter_mut()
                    .find(|token| token.id() == variable.id())
                    && let (Some(lower), Some(upper)) = (V::from_f64(lower), V::from_f64(upper))
                {
                    token.set_range(VariableRange::bounded(lower, upper));
                }
            }
        }
        if let StatisticKind::KthLargest(k) = self.kind
            && let Some((lower, upper)) = self
                .sorted
                .get(k)
                .and_then(|wire| Self::bounds_of_wire(wire, auxiliary, registered))
            && let (Some(lower), Some(upper)) = (V::from_f64(lower), V::from_f64(upper))
            && let Some(token) = auxiliary
                .iter_mut()
                .find(|token| token.id() == self.result_var.id())
        {
            token.set_range(VariableRange::bounded(lower, upper));
        } else if let StatisticKind::TopKSum(k) = self.kind {
            let mut lower = 0.0;
            let mut upper = 0.0;
            let mut finite = true;
            for wire in self.sorted.iter().take(k) {
                match Self::bounds_of_wire(wire, auxiliary, registered) {
                    Some((wire_lower, wire_upper)) => {
                        lower += wire_lower;
                        upper += wire_upper;
                    }
                    None => finite = false,
                }
            }
            if finite
                && lower.is_finite()
                && upper.is_finite()
                && let (Some(lower), Some(upper)) = (V::from_f64(lower), V::from_f64(upper))
                && let Some(token) = auxiliary
                    .iter_mut()
                    .find(|token| token.id() == self.result_var.id())
            {
                token.set_range(VariableRange::bounded(lower, upper));
            }
        }
    }

    fn ensure_comparison_bounds(&self, registered: &[Token<V>]) -> Result<()>
    where
        V: Clone
            + Debug
            + Send
            + Sync
            + 'static
            + Add<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + Zero
            + ToPrimitive
            + FromPrimitive
            + PartialOrd,
        f64: IntoValue<V>,
    {
        let mut staged = Vec::new();
        self.register_tokens(&mut staged)?;
        self.refine_bounds(&mut staged, registered);
        for comparator in &self.comparators {
            if Self::bounds_of_wire(&comparator.left, &staged, registered).is_none()
                || Self::bounds_of_wire(&comparator.right, &staged, registered).is_none()
            {
                return Err(ModelError::InvalidConstraint(format!(
                    "{} requires finite bounds for every sort-network input",
                    self.id.name
                ))
                .into());
            }
        }
        if (self.extremum.is_some() || self.arg_maximum.is_some())
            && self
                .polynomials
                .iter()
                .any(|polynomial| infer_linear_bounds_from_tokens(polynomial, registered).is_none())
        {
            return Err(ModelError::InvalidConstraint(format!(
                "{} requires finite bounds for every arg-extremum candidate",
                self.id.name
            ))
            .into());
        }
        Ok(())
    }

    fn equality_constraint(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        terms: &[Wire<V>],
    ) -> Result<LinearConstraint<V>>
    where
        V: Clone
            + Debug
            + Send
            + Sync
            + 'static
            + Add<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + Zero
            + ToPrimitive
            + FromPrimitive
            + PartialOrd,
        f64: IntoValue<V>,
    {
        let result_index = Self::index_for(symbol_to_index, &self.result_var, "statistic result")?;
        let mut monomials = vec![LinearMonomial::new(
            V::from_i64(1).ok_or_else(|| {
                ModelError::InvalidConstraint("cannot represent coefficient 1".into())
            })?,
            result_index,
        )];
        let mut constant = V::zero();
        for wire in terms {
            match wire {
                Wire::Helper(variable) => monomials.push(LinearMonomial::new(
                    V::from_i64(-1).ok_or_else(|| {
                        ModelError::InvalidConstraint("cannot represent coefficient -1".into())
                    })?,
                    Self::index_for(symbol_to_index, variable, "statistic helper")?,
                )),
                Wire::Input(polynomial) => {
                    constant = constant - polynomial.constant_term().clone();
                    for monomial in polynomial.monomials() {
                        monomials.push(LinearMonomial::new(
                            V::zero() - monomial.coefficient().clone(),
                            monomial.var_index(),
                        ));
                    }
                }
            }
        }
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        Ok(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(monomials, constant),
                ConstraintRelation::Equal,
                V::zero(),
            ),
            &format!("{}_result", self.id.name),
            source,
        ))
    }

    fn build_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: Option<&[Token<V>]>,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Clone
            + Debug
            + Send
            + Sync
            + 'static
            + Add<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + Zero
            + ToPrimitive
            + FromPrimitive
            + PartialOrd,
        f64: IntoValue<V>,
    {
        if tokens.is_none()
            && (self.extremum.is_some()
                || self.arg_maximum.is_some()
                || !self.comparators.is_empty())
        {
            return Err(ModelError::InvalidConstraint(format!(
                "{} requires token bounds to generate exact extremum constraints",
                self.id.name
            ))
            .into());
        }
        let mut constraints = Vec::new();
        if let Some(minimum) = &self.extremum {
            constraints.extend(match tokens {
                Some(tokens) => {
                    minimum.mechanism_constraints_with_tokens(symbol_to_index, tokens)?
                }
                None => minimum.mechanism_constraints(symbol_to_index)?,
            });
            let result = Self::index_for(symbol_to_index, &self.result_var, "arg result")?;
            let selectors = minimum.selector_variables().ok_or_else(|| {
                ModelError::InvalidConstraint("argmin requires exact selectors".into())
            })?;
            let mut monomials = vec![LinearMonomial::new(
                V::from_i64(1).ok_or_else(|| {
                    ModelError::InvalidConstraint("cannot represent coefficient 1".into())
                })?,
                result,
            )];
            for (index, selector) in selectors.iter().enumerate() {
                let selector_index = symbol_to_index
                    .get(&(selector.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "argmin selector variable id {}",
                            selector.id().unique_id()
                        ))
                    })?;
                let index_coefficient = V::from_usize(index).ok_or_else(|| {
                    ModelError::InvalidConstraint("arg index cannot be represented".into())
                })?;
                let negative_one = V::from_i64(-1).ok_or_else(|| {
                    ModelError::InvalidConstraint("cannot represent coefficient -1".into())
                })?;
                monomials.push(LinearMonomial::new(
                    index_coefficient * negative_one,
                    selector_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(monomials, V::zero()),
                    ConstraintRelation::Equal,
                    V::zero(),
                ),
                &format!("{}_arg_index", self.id.name),
                Arc::new(self.clone()),
            ));
            return Ok(constraints);
        }
        if self.kind == StatisticKind::ArgMin {
            constraints.push(self.equality_constraint(symbol_to_index, &[])?);
            return Ok(constraints);
        }
        if let Some(maximum) = &self.arg_maximum {
            constraints.extend(match tokens {
                Some(tokens) => {
                    maximum.mechanism_constraints_with_tokens(symbol_to_index, tokens)?
                }
                None => maximum.mechanism_constraints(symbol_to_index)?,
            });
            let result = Self::index_for(symbol_to_index, &self.result_var, "arg result")?;
            let selectors = maximum.selector_variables().ok_or_else(|| {
                ModelError::InvalidConstraint("argmax requires exact selectors".into())
            })?;
            let mut monomials = vec![LinearMonomial::new(
                V::from_i64(1).ok_or_else(|| {
                    ModelError::InvalidConstraint("cannot represent coefficient 1".into())
                })?,
                result,
            )];
            for (index, selector) in selectors.iter().enumerate() {
                let selector_index = symbol_to_index
                    .get(&(selector.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "argmax selector variable id {}",
                            selector.id().unique_id()
                        ))
                    })?;
                let index_coefficient = V::from_usize(index).ok_or_else(|| {
                    ModelError::InvalidConstraint("arg index cannot be represented".into())
                })?;
                let negative_one = V::from_i64(-1).ok_or_else(|| {
                    ModelError::InvalidConstraint("cannot represent coefficient -1".into())
                })?;
                monomials.push(LinearMonomial::new(
                    index_coefficient * negative_one,
                    selector_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(monomials, V::zero()),
                    ConstraintRelation::Equal,
                    V::zero(),
                ),
                &format!("{}_arg_index", self.id.name),
                Arc::new(self.clone()),
            ));
            return Ok(constraints);
        }
        if self.kind == StatisticKind::ArgMax {
            constraints.push(self.equality_constraint(symbol_to_index, &[])?);
            return Ok(constraints);
        }

        for comparator in &self.comparators {
            let candidates = vec![
                Self::wire_polynomial(&comparator.left, symbol_to_index)?,
                Self::wire_polynomial(&comparator.right, symbol_to_index)?,
            ];
            let minimum = comparator.minimum.with_polynomials(candidates.clone());
            let maximum = comparator.maximum.with_polynomials(candidates);
            constraints.extend(match tokens {
                Some(tokens) => {
                    minimum.mechanism_constraints_with_tokens(symbol_to_index, tokens)?
                }
                None => minimum.mechanism_constraints(symbol_to_index)?,
            });
            constraints.extend(match tokens {
                Some(tokens) => {
                    maximum.mechanism_constraints_with_tokens(symbol_to_index, tokens)?
                }
                None => maximum.mechanism_constraints(symbol_to_index)?,
            });
        }
        let terms: Vec<_> = match self.kind {
            StatisticKind::KthLargest(k) => vec![self.sorted[k].clone()],
            StatisticKind::TopKSum(k) if k == self.polynomials.len() => {
                self.polynomials.iter().cloned().map(Wire::Input).collect()
            }
            StatisticKind::TopKSum(k) => self.sorted.iter().take(k).cloned().collect(),
            _ => Vec::new(),
        };
        constraints.push(self.equality_constraint(symbol_to_index, &terms)?);
        Ok(constraints)
    }

    fn evaluate_input_polynomial(
        polynomial: &Linear<V>,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V>
    where
        V: Clone + Add<Output = V> + Mul<Output = V> + Zero,
    {
        let mut value = polynomial.constant_term().clone();
        for monomial in polynomial.monomials() {
            let variable_value = match token_table
                .find_by_index(monomial.var_index())
                .and_then(|token| token.get_result())
            {
                Some(value) => value,
                None if zero_if_none => V::zero(),
                None => return None,
            };
            value = value + monomial.coefficient().clone() * variable_value;
        }
        Some(value)
    }

    fn semantic_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>
    where
        V: Clone
            + Debug
            + Send
            + Sync
            + 'static
            + Add<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + Zero
            + PartialOrd
            + FromPrimitive,
    {
        let values: Vec<_> = self
            .polynomials
            .iter()
            .map(|polynomial| {
                Self::evaluate_input_polynomial(polynomial, token_table, zero_if_none)
            })
            .collect::<Option<_>>()?;
        match self.kind {
            StatisticKind::ArgMin => {
                let mut best = 0;
                for index in 1..values.len() {
                    if values[index].partial_cmp(&values[best])? == Ordering::Less {
                        best = index;
                    }
                }
                return V::from_usize(best);
            }
            StatisticKind::ArgMax => {
                let mut best = 0;
                for index in 1..values.len() {
                    if values[index].partial_cmp(&values[best])? == Ordering::Greater {
                        best = index;
                    }
                }
                return V::from_usize(best);
            }
            StatisticKind::KthLargest(k) => {
                let mut sorted = values;
                Self::sort_values(&mut sorted)?;
                return Some(sorted[sorted.len() - 1 - k].clone());
            }
            StatisticKind::TopKSum(k) => {
                let mut sorted = values;
                Self::sort_values(&mut sorted)?;
                return sorted
                    .iter()
                    .rev()
                    .take(k)
                    .cloned()
                    .try_fold(V::zero(), |sum, value| Some(sum + value));
            }
        }
    }

    fn sort_values(values: &mut [V]) -> Option<()>
    where
        V: PartialOrd,
    {
        for index in 1..values.len() {
            let mut position = index;
            while position > 0
                && values[position - 1].partial_cmp(&values[position])? == Ordering::Greater
            {
                values.swap(position - 1, position);
                position -= 1;
            }
        }
        Some(())
    }
}

impl<V> Display for OrderStatisticCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}({})", self.id.name, self.polynomials.len())
    }
}

impl<V> DynSymbol for OrderStatisticCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
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

impl<V> Symbol for OrderStatisticCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for OrderStatisticCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive
        + PartialOrd,
    f64: IntoValue<V>,
{
    fn category(&self) -> Category {
        Category::Linear
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        HashSet::new()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn register_auxiliary_tokens_with_context(
        &self,
        auxiliary: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, auxiliary)?;
        self.refine_bounds(auxiliary, registered);
        self.ensure_comparison_bounds(registered)
    }

    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        registered: &[Token<V>],
    ) -> Result<()> {
        self.refine_bounds(auxiliary, registered);
        self.ensure_comparison_bounds(registered)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_constraints(symbol_to_index, None)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_constraints(symbol_to_index, Some(tokens))
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        self.id.name.clone()
    }
}

impl<V> FunctionSymbol<V> for OrderStatisticCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive
        + PartialOrd,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        if let Some(minimum) = &self.extremum {
            minimum.register_tokens(tokens)?;
        }
        if let Some(maximum) = &self.arg_maximum {
            maximum.register_tokens(tokens)?;
        }
        for comparator in &self.comparators {
            comparator.minimum.register_tokens(tokens)?;
            comparator.maximum.register_tokens(tokens)?;
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.semantic_value(token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for OrderStatisticCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + One
        + Zero
        + ToPrimitive
        + FromPrimitive
        + PartialOrd,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(V::one(), self.result_var.index())],
            V::zero(),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

macro_rules! impl_order_statistic_wrapper {
    ($wrapper:ident) => {
        impl<V> Display for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.core, formatter)
            }
        }

        impl<V> DynSymbol for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn name(&self) -> &str {
                DynSymbol::name(&self.core)
            }
            fn display_name(&self) -> &str {
                DynSymbol::display_name(&self.core)
            }
            fn dyn_id(&self) -> SymbolDynId<'_> {
                DynSymbol::dyn_id(&self.core)
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl<V> Symbol for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            type Id = IntermediateSymbolId;
            fn id(&self) -> Self::Id {
                self.core.id()
            }
        }

        impl<V> IntermediateSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + Zero
                + ToPrimitive
                + FromPrimitive
                + PartialOrd,
            f64: IntoValue<V>,
        {
            fn category(&self) -> Category {
                IntermediateSymbol::category(&self.core)
            }
            fn cached(&self) -> bool {
                IntermediateSymbol::cached(&self.core)
            }
            fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
                IntermediateSymbol::dependencies(&self.core)
            }
            fn flush(&self, force: bool) {
                IntermediateSymbol::flush(&self.core, force)
            }
            fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                IntermediateSymbol::register_auxiliary_tokens(&self.core, tokens)
            }
            fn register_auxiliary_tokens_with_context(
                &self,
                auxiliary: &mut Vec<Token<V>>,
                registered: &[Token<V>],
            ) -> Result<()> {
                IntermediateSymbol::register_auxiliary_tokens_with_context(
                    &self.core, auxiliary, registered,
                )
            }
            fn refine_auxiliary_tokens(
                &self,
                auxiliary: &mut [Token<V>],
                registered: &[Token<V>],
            ) -> Result<()> {
                IntermediateSymbol::refine_auxiliary_tokens(&self.core, auxiliary, registered)
            }
            fn mechanism_constraints(
                &self,
                indexes: &HashMap<usize, usize>,
            ) -> Result<Vec<LinearConstraint<V>>> {
                IntermediateSymbol::mechanism_constraints(&self.core, indexes)
            }
            fn mechanism_constraints_with_tokens(
                &self,
                indexes: &HashMap<usize, usize>,
                tokens: &[Token<V>],
            ) -> Result<Vec<LinearConstraint<V>>> {
                IntermediateSymbol::mechanism_constraints_with_tokens(&self.core, indexes, tokens)
            }
            fn evaluate_from_tokens(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                IntermediateSymbol::evaluate_from_tokens(&self.core, token_table, zero_if_none)
            }
            fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
                IntermediateSymbol::prepare(&self.core, values)
            }
            fn to_raw_string(&self, unfold: u64) -> String {
                IntermediateSymbol::to_raw_string(&self.core, unfold)
            }
        }

        impl<V> FunctionSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + Zero
                + ToPrimitive
                + FromPrimitive
                + PartialOrd,
            f64: IntoValue<V>,
        {
            fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                FunctionSymbol::register_tokens(&self.core, tokens)
            }
            fn calculate_value(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                FunctionSymbol::calculate_value(&self.core, token_table, zero_if_none)
            }
        }

        impl<V> LinearIntermediateSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + One
                + Zero
                + ToPrimitive
                + FromPrimitive
                + PartialOrd,
            f64: IntoValue<V>,
        {
            fn to_linear_polynomial(&self) -> Linear<V> {
                LinearIntermediateSymbol::to_linear_polynomial(&self.core)
            }
            fn to_quadratic_polynomial(&self) -> Quadratic<V> {
                LinearIntermediateSymbol::to_quadratic_polynomial(&self.core)
            }
        }
    };
}

/// 返回最小候选下标 / Return the index of a minimum candidate.
#[derive(Debug, Clone)]
pub struct ArgMinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: OrderStatisticCore<V>,
}

impl<V> ArgMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建 0-based ArgMin；并列时求解器可选任一最优项 / Create a 0-based ArgMin; ties may select any optimum in the model.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Result<Self> {
        Ok(Self {
            core: OrderStatisticCore::new(id, name, polynomials, StatisticKind::ArgMin)?,
        })
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.core.result_variable()
    }
}

impl_order_statistic_wrapper!(ArgMinFunction);

/// 返回最大候选下标 / Return the index of a maximum candidate.
#[derive(Debug, Clone)]
pub struct ArgMaxFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: OrderStatisticCore<V>,
}

impl<V> ArgMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建 0-based ArgMax；并列时求解器可选任一最优项 / Create a 0-based ArgMax; ties may select any optimum in the model.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Result<Self> {
        Ok(Self {
            core: OrderStatisticCore::new(id, name, polynomials, StatisticKind::ArgMax)?,
        })
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.core.result_variable()
    }
}

impl_order_statistic_wrapper!(ArgMaxFunction);

/// 返回从大到小排序后指定 0-based 位置的值 / Return the value at a 0-based descending rank.
#[derive(Debug, Clone)]
pub struct KthLargestFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: OrderStatisticCore<V>,
}

impl<V> KthLargestFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建精确 KthLargest 函数 / Create an exact KthLargest function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>, k: usize) -> Result<Self> {
        Ok(Self {
            core: OrderStatisticCore::new(id, name, polynomials, StatisticKind::KthLargest(k))?,
        })
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.core.result_variable()
    }
}

impl_order_statistic_wrapper!(KthLargestFunction);

/// 返回最大 k 项之和 / Return the sum of the k largest values.
#[derive(Debug, Clone)]
pub struct TopKSumFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: OrderStatisticCore<V>,
}

impl<V> TopKSumFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建精确 TopKSum；`k` 可为 0 到候选数 / Create an exact TopKSum; `k` ranges from zero to the candidate count.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>, k: usize) -> Result<Self> {
        Ok(Self {
            core: OrderStatisticCore::new(id, name, polynomials, StatisticKind::TopKSum(k))?,
        })
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.core.result_variable()
    }
}

impl_order_statistic_wrapper!(TopKSumFunction);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BasicModel;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{Continuous, ContinuousVariableItem, VariableId};
    use bigdecimal::BigDecimal;

    fn poly(index: usize) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, index)], 0.0)
    }

    fn values(entries: &[(usize, f64)]) -> VecTokenList<f64> {
        let mut tokens = VecTokenList::new();
        for (index, value) in entries {
            let variable =
                ContinuousVariableItem::create(VariableId::standalone(*index + 10_000), "x");
            let token = Token::from_generic(variable, *index);
            token.set_result(*value);
            tokens.add_token(token);
        }
        tokens
    }

    fn rows_satisfied(rows: &[LinearConstraint<f64>], values: &HashMap<usize, f64>) -> bool {
        const EPSILON: f64 = 1e-7;
        rows.iter().all(|row| {
            let mut lhs = *row.inequality.polynomial.constant_term();
            for monomial in row.inequality.polynomial.monomials() {
                lhs += *monomial.coefficient()
                    * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
            }
            match row.inequality.relation {
                ConstraintRelation::LessEqual => lhs <= row.inequality.rhs + EPSILON,
                ConstraintRelation::Equal => (lhs - row.inequality.rhs).abs() <= EPSILON,
                ConstraintRelation::GreaterEqual => lhs + EPSILON >= row.inequality.rhs,
            }
        })
    }

    fn wire_value(
        wire: &Wire<f64>,
        inputs: &HashMap<usize, f64>,
        helpers: &HashMap<usize, f64>,
    ) -> f64 {
        match wire {
            Wire::Input(polynomial) => {
                *polynomial.constant_term()
                    + polynomial
                        .monomials()
                        .iter()
                        .map(|monomial| {
                            *monomial.coefficient()
                                * inputs.get(&monomial.var_index()).copied().unwrap_or(0.0)
                        })
                        .sum::<f64>()
            }
            Wire::Helper(variable) => helpers[&(variable.id().unique_id() as usize)],
        }
    }

    fn sort_network_binary_ids(core: &OrderStatisticCore<f64>) -> Vec<usize> {
        let mut ids = HashSet::new();
        for comparator in &core.comparators {
            if let Some(selectors) = comparator.minimum.selector_variables() {
                ids.extend(
                    selectors
                        .iter()
                        .map(|selector| selector.id().unique_id() as usize),
                );
            }
            if let Some(selectors) = comparator.maximum.selector_variables() {
                ids.extend(
                    selectors
                        .iter()
                        .map(|selector| selector.id().unique_id() as usize),
                );
            }
        }
        let mut ids: Vec<_> = ids.into_iter().collect();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn arg_extrema_use_smallest_tied_index_in_semantic_evaluation() {
        let tokens = values(&[(0, -3.0), (1, 4.0), (2, -3.0)]);
        let min = ArgMinFunction::new(91, "argmin_tie", vec![poly(0), poly(1), poly(2)]).unwrap();
        let max = ArgMaxFunction::new(92, "argmax_tie", vec![poly(0), poly(1), poly(2)]).unwrap();
        assert_eq!(
            FunctionSymbol::calculate_value(&min, &tokens, false),
            Some(0.0)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&max, &tokens, false),
            Some(1.0)
        );
    }

    #[test]
    fn kth_and_top_k_cover_negative_values_and_boundaries() {
        let tokens = values(&[(0, -8.0), (1, -2.0), (2, -5.0)]);
        let kth = KthLargestFunction::new(93, "kth", vec![poly(0), poly(1), poly(2)], 1).unwrap();
        let all = TopKSumFunction::new(94, "top_all", vec![poly(0), poly(1), poly(2)], 3).unwrap();
        let none =
            TopKSumFunction::new(95, "top_none", vec![poly(0), poly(1), poly(2)], 0).unwrap();
        assert_eq!(
            FunctionSymbol::calculate_value(&kth, &tokens, false),
            Some(-5.0)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&all, &tokens, false),
            Some(-15.0)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&none, &tokens, false),
            Some(0.0)
        );
    }

    #[test]
    fn order_statistic_preflights_empty_and_invalid_ranks() {
        assert!(ArgMinFunction::<f64>::new(96, "empty_argmin", vec![]).is_err());
        assert!(KthLargestFunction::<f64>::new(97, "empty_kth", vec![], 0).is_err());
        assert!(KthLargestFunction::<f64>::new(98, "bad_k", vec![poly(0)], 1).is_err());
        assert!(TopKSumFunction::<f64>::new(99, "bad_topk", vec![poly(0)], 2).is_err());
        assert!(TopKSumFunction::<f64>::new(100, "empty_topk", vec![], 0).is_ok());
    }

    #[test]
    fn comparator_helpers_register_and_rows_resolve_by_variable_id() {
        let mut model = BasicModel::<f64>::new("sort_network");
        let indices = (0..3)
            .map(|index| {
                model
                    .register_auto_variable_with_range::<Continuous>(
                        &format!("x{}", index),
                        VariableRange::bounded(-10.0, 10.0),
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let function = Arc::new(
            KthLargestFunction::new(
                101,
                "registered_kth",
                indices.iter().copied().map(poly).collect(),
                0,
            )
            .unwrap(),
        );
        let symbol: Arc<dyn IntermediateSymbol<f64>> = function.clone();
        model.add_symbol(symbol).unwrap();
        let tokens = model.tokens();
        let indexes = tokens
            .iter()
            .map(|token| (token.id().unique_id() as usize, token.solver_index))
            .collect::<HashMap<_, _>>();
        let constraints = IntermediateSymbol::mechanism_constraints_with_tokens(
            function.as_ref(),
            &indexes,
            &tokens,
        )
        .unwrap();
        assert!(!constraints.is_empty());
        assert!(constraints.len() > tokens.len() / 2);
        assert!(IntermediateSymbol::mechanism_constraints(function.as_ref(), &indexes).is_err());
    }

    #[test]
    fn sort_network_rows_enforce_kth_result_with_id_mapped_helpers() {
        let core = OrderStatisticCore::new(
            104,
            "mapped_kth",
            vec![poly(0), poly(1)],
            StatisticKind::KthLargest(0),
        )
        .unwrap();
        let input_values = [2.5, -1.0];
        let input_tokens: Vec<_> = input_values
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let variable = ContinuousVariableItem::with_range(
                    VariableId::new(90_200, index),
                    &format!("sort_x{}", index),
                    VariableRange::bounded(-10.0, 10.0),
                );
                Token::from_generic(variable, index)
            })
            .collect();
        let mut auxiliary = Vec::new();
        IntermediateSymbol::register_auxiliary_tokens_with_context(
            &core,
            &mut auxiliary,
            &input_tokens,
        )
        .unwrap();
        for (index, token) in auxiliary.iter_mut().enumerate() {
            token.solver_index = 100 + index;
        }
        assert!(
            auxiliary
                .iter()
                .any(|token| token.variable.index() != token.solver_index)
        );

        let mut tokens = input_tokens;
        tokens.extend(auxiliary);
        let symbol_to_index: HashMap<_, _> = tokens
            .iter()
            .map(|token| (token.id().unique_id() as usize, token.solver_index))
            .collect();
        let constraints =
            IntermediateSymbol::mechanism_constraints_with_tokens(&core, &symbol_to_index, &tokens)
                .unwrap();

        let inputs: HashMap<_, _> = input_values.iter().copied().enumerate().collect();
        let mut helpers = HashMap::new();
        for comparator in &core.comparators {
            let left = wire_value(&comparator.left, &inputs, &helpers);
            let right = wire_value(&comparator.right, &inputs, &helpers);
            helpers.insert(
                comparator.minimum.result_variable().id().unique_id() as usize,
                left.min(right),
            );
            helpers.insert(
                comparator.maximum.result_variable().id().unique_id() as usize,
                left.max(right),
            );
        }
        let expected = wire_value(&core.sorted[0], &inputs, &helpers);
        assert_eq!(expected, 2.5);
        helpers.insert(core.result_var.id().unique_id() as usize, expected);

        let mut assignment: HashMap<_, _> = input_values.iter().copied().enumerate().collect();
        for (id, value) in &helpers {
            assignment.insert(symbol_to_index[id], *value);
        }
        let binary_ids = sort_network_binary_ids(&core);
        let feasible = (0..(1usize << binary_ids.len())).find_map(|mask| {
            let mut candidate = assignment.clone();
            for (bit, id) in binary_ids.iter().enumerate() {
                candidate.insert(
                    symbol_to_index[id],
                    if (mask & (1 << bit)) != 0 { 1.0 } else { 0.0 },
                );
            }
            rows_satisfied(&constraints, &candidate).then_some(candidate)
        });
        let mut feasible = feasible.expect("correct kth value should satisfy sort-network rows");
        feasible.insert(
            symbol_to_index[&(core.result_var.id().unique_id() as usize)],
            expected + 1.0,
        );
        assert!(!rows_satisfied(&constraints, &feasible));
    }

    #[test]
    fn order_statistics_preserve_big_decimal_precision() {
        let first: BigDecimal = "0.100000000000000000001".parse().unwrap();
        let second: BigDecimal = "0.100000000000000000002".parse().unwrap();
        let third: BigDecimal = "0.100000000000000000003".parse().unwrap();
        let mut tokens = VecTokenList::new();
        for (index, value) in [first.clone(), second.clone(), third.clone()]
            .into_iter()
            .enumerate()
        {
            let variable =
                ContinuousVariableItem::create(VariableId::standalone(index + 11_000), "x");
            let token = Token::from_generic(variable, index);
            token.set_result(value);
            tokens.add_token(token);
        }
        let polynomial = |index| {
            Linear::new(
                vec![LinearMonomial::new(BigDecimal::from(1), index)],
                BigDecimal::from(0),
            )
        };
        let candidates = vec![polynomial(0), polynomial(1), polynomial(2)];
        let kth = KthLargestFunction::<BigDecimal>::new(102, "precise_kth", candidates.clone(), 1)
            .unwrap();
        let top_two =
            TopKSumFunction::<BigDecimal>::new(103, "precise_top_two", candidates, 2).unwrap();
        let expected_sum: BigDecimal = "0.200000000000000000005".parse().unwrap();
        assert_eq!(
            FunctionSymbol::calculate_value(&kth, &tokens, false),
            Some(second)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&top_two, &tokens, false),
            Some(expected_sum)
        );

        let exact_input = Linear::new(
            vec![LinearMonomial::new(
                "0.000000000000000000001".parse::<BigDecimal>().unwrap(),
                7,
            )],
            "0.100000000000000000001".parse::<BigDecimal>().unwrap(),
        );
        let core = OrderStatisticCore::new(
            104,
            "exact_sum_equation",
            vec![exact_input.clone()],
            StatisticKind::TopKSum(1),
        )
        .unwrap();
        let indexes = HashMap::from([(core.result_variable().id().unique_id() as usize, 8)]);
        let constraint = core
            .equality_constraint(&indexes, &[Wire::Input(exact_input)])
            .unwrap();
        assert_eq!(
            constraint.inequality.polynomial.constant_term(),
            &"-0.100000000000000000001".parse::<BigDecimal>().unwrap()
        );
        assert_eq!(
            constraint.inequality.polynomial.monomials()[1].coefficient(),
            &"-0.000000000000000000001".parse::<BigDecimal>().unwrap()
        );
    }
}
