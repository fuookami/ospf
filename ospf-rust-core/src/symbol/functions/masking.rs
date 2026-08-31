//! Masking function symbol.
//!
//! `masking(x, m)` behaves like:
//! - `x`, when `m = 1`
//! - `0`, when `m = 0`

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::flatten::{Linear, LinearMonomial, Quadratic};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::token::{IntoValue, Token, TokenList};
#[cfg(test)]
use crate::variable::VariableId;
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, new_standalone_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const MIN_BIG_M: f64 = 1.0;

fn evaluate_linear<V>(
    poly: &Linear<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant_term().clone();
    for monomial in poly.monomials() {
        let term_value = match token_table
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        value = value + monomial.coefficient().clone() * term_value;
    }
    Some(value)
}

fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
}

fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

fn convert_f64_to_v<V>(value: f64, context: &str) -> Result<V>
where
    V: FromPrimitive,
{
    from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "failed to convert `{}` value {} from f64 into model value type",
            context, value
        ))
        .into()
    })
}

/// Represents masked expression:
/// `y = x * mask`, where `mask` is binary.
#[derive(Debug, Clone)]
pub struct MaskingFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    mask_var: BinaryVariableItem,
    result_var: ContinuousVariableItem,
    big_m: V,
    declared_dependency_ids: Vec<u64>,
}

impl<V> MaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Linear<V>, mask_var: BinaryVariableItem) -> Self {
        Self::with_big_m(
            id,
            name,
            input,
            mask_var,
            from_f64(DEFAULT_BIG_M).expect("convert default big-M"),
        )
    }

    pub fn with_big_m(
        id: u64,
        name: &str,
        input: Linear<V>,
        mask_var: BinaryVariableItem,
        big_m: V,
    ) -> Self {
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_masking", name));
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            mask_var,
            result_var,
            big_m,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.input = input;
        cloned
    }

    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    pub fn mask_variable(&self) -> &BinaryVariableItem {
        &self.mask_var
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn big_m(&self) -> &V {
        &self.big_m
    }
}

impl<V> MaskingFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn configured_big_m(&self) -> Result<f64> {
        let big_m = to_f64(&self.big_m).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "masking `{}` big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "masking `{}` requires positive finite big-M",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        let mut lower = to_f64(self.input.constant_term())?;
        let mut upper = lower;

        for monomial in self.input.monomials() {
            let token = tokens.get(monomial.var_index())?;
            let var_lower = to_f64(&token.variable.lower_bound()?)?;
            let var_upper = to_f64(&token.variable.upper_bound()?)?;
            if !var_lower.is_finite() || !var_upper.is_finite() {
                return None;
            }

            let coefficient = to_f64(monomial.coefficient())?;
            if coefficient >= 0.0 {
                lower += coefficient * var_lower;
                upper += coefficient * var_upper;
            } else {
                lower += coefficient * var_upper;
                upper += coefficient * var_lower;
            }
        }

        if !lower.is_finite() || !upper.is_finite() {
            return None;
        }
        Some(lower.abs().max(upper.abs()).max(MIN_BIG_M))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "masking result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let mask_index = symbol_to_index
            .get(&(self.mask_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "masking mask variable id {}",
                    self.mask_var.id().unique_id()
                ))
            })?;

        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "masking `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let mut y_minus_x_monomials = Vec::with_capacity(self.input.monomials().len() + 2);
        y_minus_x_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "masking y coefficient")?,
            result_index,
        ));
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "masking `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let input_index = monomial.var_index();
            y_minus_x_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "masking input coefficient")?,
                input_index,
            ));
        }

        let mut c1_monomials = y_minus_x_monomials.clone();
        c1_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "masking c1 mask coefficient")?,
            mask_index,
        ));
        let c1 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    c1_monomials,
                    convert_f64_to_v::<V>(-input_constant, "masking c1 constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "masking c1 rhs")?,
            ),
            &format!("{}_masking_eq_ub", self.id.name),
            Arc::new(self.clone()),
        );

        let mut c2_monomials = y_minus_x_monomials;
        c2_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "masking c2 mask coefficient")?,
            mask_index,
        ));
        let c2 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    c2_monomials,
                    convert_f64_to_v::<V>(-input_constant, "masking c2 constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(-big_m, "masking c2 rhs")?,
            ),
            &format!("{}_masking_eq_lb", self.id.name),
            Arc::new(self.clone()),
        );

        let c3 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "masking c3 y coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(-big_m, "masking c3 mask coefficient")?,
                            mask_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "masking c3 constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "masking c3 rhs")?,
            ),
            &format!("{}_masking_zero_ub", self.id.name),
            Arc::new(self.clone()),
        );

        let c4 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "masking c4 y coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(big_m, "masking c4 mask coefficient")?,
                            mask_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "masking c4 constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "masking c4 rhs")?,
            ),
            &format!("{}_masking_zero_lb", self.id.name),
            Arc::new(self.clone()),
        );

        Ok(vec![c1, c2, c3, c4])
    }
}

impl<V> Display for MaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "masking({})", self.id.name)
    }
}

impl<V> DynSymbol for MaskingFunction<V>
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

impl<V> Symbol for MaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for MaskingFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
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

    fn declared_dependency_ids(&self) -> Vec<u64> {
        self.declared_dependency_ids.clone()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, self.configured_big_m()?)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = match self.infer_big_m_from_tokens(tokens) {
            Some(inferred) => inferred,
            None => self.configured_big_m()?,
        };
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("masking({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for MaskingFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mask_value = match token_table
            .find_by_id(self.mask_var.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => from_f64(0.0)?,
            None => return None,
        };
        if to_f64(&mask_value)?.abs() <= f64::EPSILON {
            return from_f64(0.0);
        }
        evaluate_linear(&self.input, token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for MaskingFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// Represents masked range variable:
/// `y in [lower * mask, upper * mask]`.
#[derive(Debug, Clone)]
pub struct MaskingRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    mask: Linear<V>,
    lower: V,
    upper: V,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> MaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, mask: Linear<V>, lower: V, upper: V) -> Self {
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_mask_range", name));
        Self {
            id: IntermediateSymbolId::new(id, name),
            mask,
            lower,
            upper,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn mask_polynomial(&self) -> &Linear<V> {
        &self.mask
    }

    pub fn lower_bound(&self) -> &V {
        &self.lower
    }

    pub fn upper_bound(&self) -> &V {
        &self.upper
    }
}

impl<V> Display for MaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "masking_range({})", self.id.name)
    }
}

impl<V> DynSymbol for MaskingRangeFunction<V>
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

impl<V> Symbol for MaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for MaskingRangeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
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

    fn declared_dependency_ids(&self) -> Vec<u64> {
        self.declared_dependency_ids.clone()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "masking_range result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let lower = to_f64(&self.lower).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "masking_range `{}` lower bound cannot be converted to f64",
                self.id.name
            ))
        })?;
        let upper = to_f64(&self.upper).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "masking_range `{}` upper bound cannot be converted to f64",
                self.id.name
            ))
        })?;
        if lower > upper {
            return Err(ModelError::InvalidConstraint(format!(
                "masking_range `{}` lower bound {} is greater than upper bound {}",
                self.id.name, lower, upper
            ))
            .into());
        }

        let mut upper_monomials = Vec::with_capacity(self.mask.monomials().len() + 1);
        upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "masking_range y upper coefficient")?,
            result_index,
        ));
        for monomial in self.mask.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "masking_range `{}` mask coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let mask_index = monomial.var_index();
            upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(
                    -upper * coefficient,
                    "masking_range upper mask coefficient",
                )?,
                mask_index,
            ));
        }
        let mask_constant = to_f64(self.mask.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "masking_range `{}` mask constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let upper_constraint = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    upper_monomials,
                    convert_f64_to_v::<V>(-upper * mask_constant, "masking_range upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "masking_range upper rhs")?,
            ),
            &format!("{}_masking_range_ub", self.id.name),
            Arc::new(self.clone()),
        );

        let mut lower_monomials = Vec::with_capacity(self.mask.monomials().len() + 1);
        lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "masking_range y lower coefficient")?,
            result_index,
        ));
        for monomial in self.mask.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "masking_range `{}` mask coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let mask_index = monomial.var_index();
            lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(
                    -lower * coefficient,
                    "masking_range lower mask coefficient",
                )?,
                mask_index,
            ));
        }
        let lower_constraint = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    lower_monomials,
                    convert_f64_to_v::<V>(-lower * mask_constant, "masking_range lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "masking_range lower rhs")?,
            ),
            &format!("{}_masking_range_lb", self.id.name),
            Arc::new(self.clone()),
        );

        Ok(vec![upper_constraint, lower_constraint])
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("masking_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for MaskingRangeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mask_value = to_f64(&evaluate_linear(&self.mask, token_table, zero_if_none)?)?;
        if mask_value.abs() <= f64::EPSILON {
            return from_f64(0.0);
        }

        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        let lb = (lower * mask_value).min(upper * mask_value);
        let ub = (lower * mask_value).max(upper * mask_value);
        match token_table
            .find_by_id(self.result_var.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => from_f64(to_f64(&v)?.clamp(lb, ub)),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for MaskingRangeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn masking_range_calculate_value() {
        let mask = BinaryVariableItem::create(VariableId::standalone(1001), "m");
        let mask_poly = Linear::new(vec![LinearMonomial::new(1.0, mask.index())], 0.0);
        let f = MaskingRangeFunction::new(1100, "mr", mask_poly, -2.0, 3.0);

        let mut tokens = VecTokenList::<f64>::new();
        let tm = Token::from_generic(mask.clone(), mask.index());
        tm.set_result(1.0);
        tokens.add_token(tm);
        let ty = Token::from_generic(f.result_variable().clone(), f.result_variable().index());
        ty.set_result(2.0);
        tokens.add_token(ty);

        assert_eq!(f.calculate_value(&tokens, false), Some(2.0));

        let mut off_tokens = VecTokenList::<f64>::new();
        let tm_off = Token::from_generic(mask.clone(), mask.index());
        tm_off.set_result(0.0);
        off_tokens.add_token(tm_off);
        let ty_off = Token::from_generic(f.result_variable().clone(), f.result_variable().index());
        ty_off.set_result(2.0);
        off_tokens.add_token(ty_off);
        assert_eq!(f.calculate_value(&off_tokens, false), Some(0.0));
    }

    #[test]
    fn masking_range_generates_two_constraints() {
        let mask = BinaryVariableItem::create(VariableId::standalone(2001), "m");
        let mask_poly = Linear::new(vec![LinearMonomial::new(1.0, mask.index())], 0.0);
        let f = MaskingRangeFunction::new(2100, "mr", mask_poly, -2.0, 3.0);

        let mut symbol_to_index = std::collections::HashMap::new();
        symbol_to_index.insert(
            f.result_variable().id().unique_id() as usize,
            f.result_variable().index(),
        );
        symbol_to_index.insert(mask.id().unique_id() as usize, mask.index());

        let constraints = f.mechanism_constraints(&symbol_to_index).unwrap();
        assert_eq!(constraints.len(), 2);
    }

    #[test]
    fn masking_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(10_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let mask = BinaryVariableItem::create(VariableId::standalone(10_001), "m");
        let f: MaskingFunction<f64> = MaskingFunction::new(
            2200,
            "masking_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            mask.clone(),
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mask_id = mask.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (mask_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(mask, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("masking constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "masking_bound_masking_eq_ub")
            .expect("upper masking constraint should exist");
        let mask_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("mask term should exist");

        // 2x + 1 with x in [-2, 3] => range [-3, 7], therefore M = 7.
        assert!((upper.inequality.rhs - 7.0).abs() <= 1e-9);
        assert!(
            (*mask_term.coefficient() - 7.0).abs() <= 1e-9,
            "masking inferred coefficient={}, rhs={}",
            *mask_term.coefficient(),
            upper.inequality.rhs
        );
    }

    #[test]
    fn masking_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(10_010), "x");
        let mask = BinaryVariableItem::create(VariableId::standalone(10_011), "m");
        let f: MaskingFunction<f64> = MaskingFunction::new(
            2201,
            "masking_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            mask.clone(),
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mask_id = mask.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (mask_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(mask, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("masking constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "masking_default_masking_eq_ub")
            .expect("upper masking constraint should exist");
        let mask_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("mask term should exist");

        assert!((upper.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*mask_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
