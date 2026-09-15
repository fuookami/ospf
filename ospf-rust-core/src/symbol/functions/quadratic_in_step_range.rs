//! 二次区间指示函数 / Quadratic interval indicator function

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticFunctionSymbol,
};
use super::big_m::{
    infer_quadratic_abs_bound_from_tokens, infer_quadratic_shifted_abs_bound_from_tokens,
};
use super::quadratic_linear::{
    convert_f64_to_v, evaluate_quadratic, evaluate_quadratic_from_values, from_f64, to_f64,
};
use crate::error::{ModelError, Result};
use crate::model::{
    ConstraintRelation, LinearConstraint, LinearInequality, QuadraticConstraint,
    QuadraticInequality,
};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, new_standalone_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const DEFAULT_OUTSIDE_TOLERANCE: f64 = 1.0e-6;

/// 二次区间指示函数：输入在闭区间内时返回输入，否则返回零。
/// Quadratic interval indicator: returns the input inside the closed interval, otherwise zero.
///
/// `inside`, `below`, and `above` form a complete three-state partition; `y` is a signed
/// continuous result variable. Values in the exterior tolerance bands are undefined and are
/// excluded by the solver model.
#[derive(Debug, Clone)]
pub struct QuadraticInStepRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    lower: V,
    upper: V,
    big_m: V,
    outside_tolerance: V,
    inside: BinaryVariableItem,
    below: BinaryVariableItem,
    above: BinaryVariableItem,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + PartialOrd,
{
    /// 创建二次区间指示函数，默认使用 `10^6` 作为 Big-M。
    /// Create a quadratic interval indicator with default Big-M `10^6`.
    pub fn new(id: u64, name: &str, input: Quadratic<V>, lower: V, upper: V) -> Self {
        Self::with_parameters(
            id,
            name,
            input,
            lower,
            upper,
            from_f64(DEFAULT_BIG_M).expect("convert default Big-M"),
            from_f64(DEFAULT_OUTSIDE_TOLERANCE).expect("convert default outside tolerance"),
        )
    }

    /// 创建带显式 Big-M 的二次区间指示函数。
    /// Create a quadratic interval indicator with an explicit Big-M.
    pub fn with_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        lower: V,
        upper: V,
        big_m: V,
    ) -> Self {
        Self::with_parameters(
            id,
            name,
            input,
            lower,
            upper,
            big_m,
            from_f64(DEFAULT_OUTSIDE_TOLERANCE).expect("convert default outside tolerance"),
        )
    }

    /// 创建带显式 Big-M 和区间外分类容差的二次区间指示函数。
    /// Create a quadratic interval indicator with explicit Big-M and exterior classification tolerance.
    pub fn with_parameters(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        lower: V,
        upper: V,
        big_m: V,
        outside_tolerance: V,
    ) -> Self {
        assert!(
            lower <= upper,
            "quadratic in-step-range requires lower <= upper"
        );
        assert!(
            outside_tolerance > from_f64(0.0).expect("convert zero outside tolerance"),
            "quadratic in-step-range outside tolerance must be positive"
        );
        let inside = BinaryVariableItem::create(new_standalone_id(), &format!("{}_inside", name));
        let below = BinaryVariableItem::create(new_standalone_id(), &format!("{}_below", name));
        let above = BinaryVariableItem::create(new_standalone_id(), &format!("{}_above", name));
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_y", name));
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            lower,
            upper,
            big_m,
            outside_tolerance,
            inside,
            below,
            above,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn input_polynomial(&self) -> &Quadratic<V> {
        &self.input
    }
    pub fn inside_indicator_variable(&self) -> &BinaryVariableItem {
        &self.inside
    }
    pub fn below_indicator_variable(&self) -> &BinaryVariableItem {
        &self.below
    }
    pub fn above_indicator_variable(&self) -> &BinaryVariableItem {
        &self.above
    }
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }
    pub fn lower_bound(&self) -> &V {
        &self.lower
    }
    pub fn upper_bound(&self) -> &V {
        &self.upper
    }
    pub fn big_m(&self) -> &V {
        &self.big_m
    }
    pub fn outside_tolerance(&self) -> &V {
        &self.outside_tolerance
    }
}

impl<V> QuadraticInStepRangeFunction<V>
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
                "quadratic in-step-range `{}` Big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "quadratic in-step-range `{}` requires a positive finite Big-M",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn configured_outside_tolerance(&self) -> Result<f64> {
        let tolerance = to_f64(&self.outside_tolerance).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "quadratic in-step-range `{}` outside tolerance cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "quadratic in-step-range `{}` requires a positive finite outside tolerance",
                self.id.name
            ))
            .into());
        }
        Ok(tolerance)
    }

    fn resolved_big_m(&self, tokens: &[Token<V>]) -> Result<f64> {
        let configured = self.configured_big_m()?;
        let tolerance = self.configured_outside_tolerance()?;
        let inferred = [
            infer_quadratic_abs_bound_from_tokens(&self.input, tokens),
            infer_quadratic_shifted_abs_bound_from_tokens(&self.input, &self.lower, tokens),
            infer_quadratic_shifted_abs_bound_from_tokens(&self.input, &self.upper, tokens),
        ]
        .into_iter()
        .flatten()
        .map(|bound| bound + tolerance)
        .filter(|bound| bound.is_finite())
        .fold(configured, f64::max);
        Ok(inferred)
    }

    fn indices(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<(usize, usize, usize, usize)> {
        let y_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(
                    "quadratic in-step-range result variable is not registered".to_string(),
                )
            })?;
        let inside_index = symbol_to_index
            .get(&(self.inside.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(
                    "quadratic in-step-range inside indicator is not registered".to_string(),
                )
            })?;
        let below_index = symbol_to_index
            .get(&(self.below.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(
                    "quadratic in-step-range below indicator is not registered".to_string(),
                )
            })?;
        let above_index = symbol_to_index
            .get(&(self.above.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(
                    "quadratic in-step-range above indicator is not registered".to_string(),
                )
            })?;
        Ok((y_index, inside_index, below_index, above_index))
    }

    fn quadratic_rows(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
        outside_tolerance: f64,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let (y_index, inside_index, below_index, above_index) = self.indices(symbol_to_index)?;
        let one = convert_f64_to_v::<V>(1.0, "quadratic in-step-range unit")?;
        let minus_one = convert_f64_to_v::<V>(-1.0, "quadratic in-step-range minus unit")?;
        let m = convert_f64_to_v::<V>(big_m, "quadratic in-step-range Big-M")?;
        let neg_m = convert_f64_to_v::<V>(-big_m, "quadratic in-step-range negative Big-M")?;
        let tolerance = convert_f64_to_v::<V>(
            outside_tolerance,
            "quadratic in-step-range outside tolerance",
        )?;
        let neg_tolerance = convert_f64_to_v::<V>(
            -outside_tolerance,
            "quadratic in-step-range negative outside tolerance",
        )?;
        let input_const = self.input.constant().clone();
        let neg_input_const = input_const.clone() * minus_one.clone();
        let owner = Arc::new(self.clone());

        let input_terms = self.input.monomials().to_vec();
        let neg_input_terms: Vec<QuadraticMonomial<V>> = self
            .input
            .monomials()
            .iter()
            .map(|monomial| {
                let coefficient = monomial.coefficient().clone() * minus_one.clone();
                match monomial.var_index2() {
                    Some(var_index2) => QuadraticMonomial::new_quadratic(
                        coefficient,
                        monomial.var_index1(),
                        var_index2,
                    ),
                    None => QuadraticMonomial::new_linear(coefficient, monomial.var_index1()),
                }
            })
            .collect();

        let make = |polynomial: Quadratic<V>,
                    relation: ConstraintRelation,
                    rhs: V,
                    name: String,
                    owner: Arc<Self>| {
            QuadraticConstraint::from_symbol(
                QuadraticInequality::new(polynomial, relation, rhs),
                &name,
                owner,
            )
        };

        let mut inside_lower = input_terms.clone();
        inside_lower.push(QuadraticMonomial::new_linear(neg_m.clone(), inside_index));
        let mut inside_upper = input_terms.clone();
        inside_upper.push(QuadraticMonomial::new_linear(m.clone(), inside_index));
        let mut below = input_terms.clone();
        below.push(QuadraticMonomial::new_linear(m.clone(), below_index));
        let mut above = input_terms;
        above.push(QuadraticMonomial::new_linear(neg_m.clone(), above_index));

        let partition = vec![
            QuadraticMonomial::new_linear(one.clone(), inside_index),
            QuadraticMonomial::new_linear(one.clone(), below_index),
            QuadraticMonomial::new_linear(one.clone(), above_index),
        ];

        let mut value_lower = vec![QuadraticMonomial::new_linear(one.clone(), y_index)];
        value_lower.extend(neg_input_terms.clone());
        value_lower.push(QuadraticMonomial::new_linear(neg_m.clone(), inside_index));
        let mut value_upper = vec![QuadraticMonomial::new_linear(one.clone(), y_index)];
        value_upper.extend(neg_input_terms);
        value_upper.push(QuadraticMonomial::new_linear(m.clone(), inside_index));
        let zero_upper = vec![
            QuadraticMonomial::new_linear(one.clone(), y_index),
            QuadraticMonomial::new_linear(neg_m.clone(), inside_index),
        ];
        let zero_lower = vec![
            QuadraticMonomial::new_linear(one.clone(), y_index),
            QuadraticMonomial::new_linear(m.clone(), inside_index),
        ];

        Ok(vec![
            make(
                Quadratic::new(inside_lower, input_const.clone()),
                ConstraintRelation::GreaterEqual,
                self.lower.clone() + neg_m.clone(),
                format!("{}_inside_lb", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(inside_upper, input_const.clone()),
                ConstraintRelation::LessEqual,
                self.upper.clone() + m.clone(),
                format!("{}_inside_ub", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(below, input_const.clone()),
                ConstraintRelation::LessEqual,
                self.lower.clone() + neg_tolerance + m.clone(),
                format!("{}_below", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(above, input_const),
                ConstraintRelation::GreaterEqual,
                self.upper.clone() + tolerance + neg_m.clone(),
                format!("{}_above", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(partition, V::zero()),
                ConstraintRelation::Equal,
                one,
                format!("{}_partition", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(value_lower, neg_input_const.clone()),
                ConstraintRelation::GreaterEqual,
                neg_m,
                format!("{}_value_lb", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(value_upper, neg_input_const),
                ConstraintRelation::LessEqual,
                m,
                format!("{}_value_ub", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(zero_upper, V::zero()),
                ConstraintRelation::LessEqual,
                V::zero(),
                format!("{}_zero_ub", self.id.name),
                owner.clone(),
            ),
            make(
                Quadratic::new(zero_lower, V::zero()),
                ConstraintRelation::GreaterEqual,
                V::zero(),
                format!("{}_zero_lb", self.id.name),
                owner,
            ),
        ])
    }

    fn linear_rows(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
        outside_tolerance: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        if self
            .input
            .monomials()
            .iter()
            .any(|m| m.var_index2().is_some())
        {
            return Ok(Vec::new());
        }
        let (y_index, inside_index, below_index, above_index) = self.indices(symbol_to_index)?;
        let one = convert_f64_to_v::<V>(1.0, "quadratic in-step-range unit")?;
        let minus_one = convert_f64_to_v::<V>(-1.0, "quadratic in-step-range minus unit")?;
        let m = convert_f64_to_v::<V>(big_m, "quadratic in-step-range Big-M")?;
        let neg_m = convert_f64_to_v::<V>(-big_m, "quadratic in-step-range negative Big-M")?;
        let tolerance = convert_f64_to_v::<V>(
            outside_tolerance,
            "quadratic in-step-range outside tolerance",
        )?;
        let neg_tolerance = convert_f64_to_v::<V>(
            -outside_tolerance,
            "quadratic in-step-range negative outside tolerance",
        )?;
        let input_const = self.input.constant().clone();
        let neg_input_const = input_const.clone() * minus_one.clone();
        let input_linear: Vec<LinearMonomial<V>> = self
            .input
            .monomials()
            .iter()
            .map(|monomial| {
                LinearMonomial::new(monomial.coefficient().clone(), monomial.var_index1())
            })
            .collect();
        let neg_input_linear: Vec<LinearMonomial<V>> = self
            .input
            .monomials()
            .iter()
            .map(|monomial| {
                LinearMonomial::new(
                    monomial.coefficient().clone() * minus_one.clone(),
                    monomial.var_index1(),
                )
            })
            .collect();
        let owner = Arc::new(self.clone());
        let make = |polynomial: Linear<V>,
                    relation: ConstraintRelation,
                    rhs: V,
                    name: String,
                    owner: Arc<Self>| {
            LinearConstraint::from_symbol(
                LinearInequality::new(polynomial, relation, rhs),
                &name,
                owner,
            )
        };

        let mut inside_lower = input_linear.clone();
        inside_lower.push(LinearMonomial::new(neg_m.clone(), inside_index));
        let mut inside_upper = input_linear.clone();
        inside_upper.push(LinearMonomial::new(m.clone(), inside_index));
        let mut below = input_linear.clone();
        below.push(LinearMonomial::new(m.clone(), below_index));
        let mut above = input_linear;
        above.push(LinearMonomial::new(neg_m.clone(), above_index));
        let partition = vec![
            LinearMonomial::new(one.clone(), inside_index),
            LinearMonomial::new(one.clone(), below_index),
            LinearMonomial::new(one.clone(), above_index),
        ];
        let mut value_lower = vec![LinearMonomial::new(one.clone(), y_index)];
        value_lower.extend(neg_input_linear.clone());
        value_lower.push(LinearMonomial::new(neg_m.clone(), inside_index));
        let mut value_upper = vec![LinearMonomial::new(one.clone(), y_index)];
        value_upper.extend(neg_input_linear);
        value_upper.push(LinearMonomial::new(m.clone(), inside_index));
        let zero_upper = vec![
            LinearMonomial::new(one.clone(), y_index),
            LinearMonomial::new(neg_m.clone(), inside_index),
        ];
        let zero_lower = vec![
            LinearMonomial::new(one.clone(), y_index),
            LinearMonomial::new(m.clone(), inside_index),
        ];
        Ok(vec![
            make(
                Linear::new(inside_lower, input_const.clone()),
                ConstraintRelation::GreaterEqual,
                self.lower.clone() + neg_m.clone(),
                format!("{}_inside_lb", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(inside_upper, input_const.clone()),
                ConstraintRelation::LessEqual,
                self.upper.clone() + m.clone(),
                format!("{}_inside_ub", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(below, input_const.clone()),
                ConstraintRelation::LessEqual,
                self.lower.clone() + neg_tolerance + m.clone(),
                format!("{}_below", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(above, input_const),
                ConstraintRelation::GreaterEqual,
                self.upper.clone() + tolerance + neg_m.clone(),
                format!("{}_above", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(partition, V::zero()),
                ConstraintRelation::Equal,
                one,
                format!("{}_partition", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(value_lower, neg_input_const.clone()),
                ConstraintRelation::GreaterEqual,
                neg_m,
                format!("{}_value_lb", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(value_upper, neg_input_const),
                ConstraintRelation::LessEqual,
                m,
                format!("{}_value_ub", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(zero_upper, V::zero()),
                ConstraintRelation::LessEqual,
                V::zero(),
                format!("{}_zero_ub", self.id.name),
                owner.clone(),
            ),
            make(
                Linear::new(zero_lower, V::zero()),
                ConstraintRelation::GreaterEqual,
                V::zero(),
                format!("{}_zero_lb", self.id.name),
                owner,
            ),
        ])
    }
}

impl<V> Display for QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "quadratic_in_step_range({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticInStepRangeFunction<V>
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

impl<V> Symbol for QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticInStepRangeFunction<V>
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
    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        self.linear_rows(
            symbol_to_index,
            self.configured_big_m()?,
            self.configured_outside_tolerance()?,
        )
    }
    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.linear_rows(
            symbol_to_index,
            self.resolved_big_m(tokens)?,
            self.configured_outside_tolerance()?,
        )
    }
    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if !self
            .input
            .monomials()
            .iter()
            .any(|m| m.var_index2().is_some())
        {
            return Ok(Vec::new());
        }
        self.quadratic_rows(
            symbol_to_index,
            self.configured_big_m()?,
            self.configured_outside_tolerance()?,
        )
    }
    fn quadratic_mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if !self
            .input
            .monomials()
            .iter()
            .any(|m| m.var_index2().is_some())
        {
            return Ok(Vec::new());
        }
        self.quadratic_rows(
            symbol_to_index,
            self.resolved_big_m(tokens)?,
            self.configured_outside_tolerance()?,
        )
    }
    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }
    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        // Re-evaluate the source polynomial; a stale result token is not authoritative.
        // 重新计算原始多项式；过期的结果令牌不具有权威性。
        let value = evaluate_quadratic_from_values(&self.input, values)?;
        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        let tolerance = to_f64(&self.outside_tolerance)?;
        let current = to_f64(&value)?;
        if current >= lower && current <= upper {
            Some(value)
        } else if current <= lower - tolerance || current >= upper + tolerance {
            from_f64(0.0)
        } else {
            None
        }
    }
    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("quadratic_in_step_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticInStepRangeFunction<V>
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
            self.inside.clone(),
            self.inside.index(),
        ));
        tokens.push(Token::from_generic(self.below.clone(), self.below.index()));
        tokens.push(Token::from_generic(self.above.clone(), self.above.index()));
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }
    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_quadratic(&self.input, token_table, zero_if_none)?;
        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        let tolerance = to_f64(&self.outside_tolerance)?;
        let current = to_f64(&value)?;
        if current >= lower && current <= upper {
            Some(value)
        } else if current <= lower - tolerance || current >= upper + tolerance {
            from_f64(0.0)
        } else {
            None
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticInStepRangeFunction<V>
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
            V::zero(),
        )
    }
    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl<V> QuadraticFunctionSymbol<V> for QuadraticInStepRangeFunction<V>
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
}
