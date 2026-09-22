//! 二次掩码函数 / Quadratic masking function

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticFunctionSymbol,
};
use super::big_m::infer_quadratic_abs_bound_from_tokens;
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

/// 二次掩码函数：`y = p(x)`（`z = 1`）或 `y = 0`（`z = 0`）。
/// Quadratic masking function: `y = p(x)` when `z = 1`, otherwise `y = 0`.
///
/// The solver model uses the four standard Big-M rows:
/// `y - p + M z <= M`, `y - p - M z >= -M`, `y <= M z`, `y >= -M z`.
#[derive(Debug, Clone)]
pub struct QuadraticMaskingRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    mask_var: BinaryVariableItem,
    result_var: ContinuousVariableItem,
    big_m: V,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建二次掩码函数，默认使用 `10^6` 作为 Big-M。
    /// Create a quadratic masking function with the default Big-M `10^6`.
    pub fn new(id: u64, name: &str, input: Quadratic<V>, mask_var: BinaryVariableItem) -> Self {
        Self::with_big_m(
            id,
            name,
            input,
            mask_var,
            from_f64(DEFAULT_BIG_M).expect("convert default Big-M"),
        )
    }

    /// 创建带显式 Big-M 的二次掩码函数。
    /// Create a quadratic masking function with an explicit Big-M.
    pub fn with_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
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

    pub fn input_polynomial(&self) -> &Quadratic<V> {
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

impl<V> QuadraticMaskingRangeFunction<V>
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
                "quadratic masking `{}` Big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "quadratic masking `{}` requires a positive finite Big-M",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn resolved_big_m(&self, tokens: &[Token<V>]) -> Result<f64> {
        let configured = self.configured_big_m()?;
        Ok(infer_quadratic_abs_bound_from_tokens(&self.input, tokens)
            .filter(|bound| bound.is_finite())
            .map(|bound| configured.max(bound))
            .unwrap_or(configured))
    }

    fn indices(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<(usize, usize)> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let mask_index = symbol_to_index
            .get(&(self.mask_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking mask variable id {}",
                    self.mask_var.id().unique_id()
                ))
            })?;
        Ok((result_index, mask_index))
    }

    fn quadratic_rows(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let (result_index, mask_index) = self.indices(symbol_to_index)?;
        let one = convert_f64_to_v::<V>(1.0, "quadratic masking unit")?;
        let minus_one = convert_f64_to_v::<V>(-1.0, "quadratic masking minus unit")?;
        let m = convert_f64_to_v::<V>(big_m, "quadratic masking Big-M")?;
        let neg_m = convert_f64_to_v::<V>(-big_m, "quadratic masking negative Big-M")?;

        let mut y_minus_p = Vec::with_capacity(self.input.monomials().len() + 1);
        y_minus_p.push(QuadraticMonomial::new_linear(one.clone(), result_index));
        y_minus_p.extend(self.input.monomials().iter().map(|monomial| {
            let coefficient = monomial.coefficient().clone() * minus_one.clone();
            match monomial.var_index2() {
                Some(var_index2) => {
                    QuadraticMonomial::new_quadratic(coefficient, monomial.var_index1(), var_index2)
                }
                None => QuadraticMonomial::new_linear(coefficient, monomial.var_index1()),
            }
        }));
        let base_constant = self.input.constant().clone() * minus_one;

        let mut c1 = y_minus_p.clone();
        c1.push(QuadraticMonomial::new_linear(m.clone(), mask_index));
        let mut c2 = y_minus_p;
        c2.push(QuadraticMonomial::new_linear(neg_m.clone(), mask_index));

        let mut y_minus_mz = vec![QuadraticMonomial::new_linear(one.clone(), result_index)];
        y_minus_mz.push(QuadraticMonomial::new_linear(neg_m.clone(), mask_index));
        let mut y_plus_mz = vec![QuadraticMonomial::new_linear(one.clone(), result_index)];
        y_plus_mz.push(QuadraticMonomial::new_linear(m.clone(), mask_index));
        let owner = Arc::new(self.clone());
        Ok(vec![
            QuadraticConstraint::from_symbol(
                QuadraticInequality::new(
                    Quadratic::new(c1, base_constant.clone()),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(big_m, "quadratic masking c1 rhs")?,
                ),
                &format!("{}_masking_eq_ub", self.id.name),
                owner.clone(),
            ),
            QuadraticConstraint::from_symbol(
                QuadraticInequality::new(
                    Quadratic::new(c2, base_constant),
                    ConstraintRelation::GreaterEqual,
                    neg_m.clone(),
                ),
                &format!("{}_masking_eq_lb", self.id.name),
                owner.clone(),
            ),
            QuadraticConstraint::from_symbol(
                QuadraticInequality::new(
                    Quadratic::new(y_minus_mz, V::zero()),
                    ConstraintRelation::LessEqual,
                    V::zero(),
                ),
                &format!("{}_masking_zero_ub", self.id.name),
                owner.clone(),
            ),
            QuadraticConstraint::from_symbol(
                QuadraticInequality::new(
                    Quadratic::new(y_plus_mz, V::zero()),
                    ConstraintRelation::GreaterEqual,
                    V::zero(),
                ),
                &format!("{}_masking_zero_lb", self.id.name),
                owner,
            ),
        ])
    }

    fn linear_rows(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        if self
            .input
            .monomials()
            .iter()
            .any(|m| m.var_index2().is_some())
        {
            return Ok(Vec::new());
        }
        let (result_index, mask_index) = self.indices(symbol_to_index)?;
        let one = convert_f64_to_v::<V>(1.0, "quadratic masking unit")?;
        let m = convert_f64_to_v::<V>(big_m, "quadratic masking Big-M")?;
        let neg_m = convert_f64_to_v::<V>(-big_m, "quadratic masking negative Big-M")?;
        let minus_one = convert_f64_to_v::<V>(-1.0, "quadratic masking minus unit")?;
        let mut y_minus_p = vec![LinearMonomial::new(one.clone(), result_index)];
        y_minus_p.extend(self.input.monomials().iter().map(|monomial| {
            LinearMonomial::new(
                monomial.coefficient().clone() * minus_one.clone(),
                monomial.var_index1(),
            )
        }));
        let base_constant = self.input.constant().clone() * minus_one;
        let mut c1 = y_minus_p.clone();
        c1.push(LinearMonomial::new(m.clone(), mask_index));
        let mut c2 = y_minus_p;
        c2.push(LinearMonomial::new(neg_m.clone(), mask_index));
        let owner = Arc::new(self.clone());
        Ok(vec![
            LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(c1, base_constant.clone()),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(big_m, "quadratic masking c1 rhs")?,
                ),
                &format!("{}_masking_eq_ub", self.id.name),
                owner.clone(),
            ),
            LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(c2, base_constant),
                    ConstraintRelation::GreaterEqual,
                    neg_m.clone(),
                ),
                &format!("{}_masking_eq_lb", self.id.name),
                owner.clone(),
            ),
            LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(one.clone(), result_index),
                            LinearMonomial::new(neg_m.clone(), mask_index),
                        ],
                        V::zero(),
                    ),
                    ConstraintRelation::LessEqual,
                    V::zero(),
                ),
                &format!("{}_masking_zero_ub", self.id.name),
                owner.clone(),
            ),
            LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(one, result_index),
                            LinearMonomial::new(m, mask_index),
                        ],
                        V::zero(),
                    ),
                    ConstraintRelation::GreaterEqual,
                    V::zero(),
                ),
                &format!("{}_masking_zero_lb", self.id.name),
                owner,
            ),
        ])
    }
}

impl<V> Display for QuadraticMaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "quadratic_masking({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMaskingRangeFunction<V>
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

impl<V> Symbol for QuadraticMaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMaskingRangeFunction<V>
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
        self.linear_rows(symbol_to_index, self.configured_big_m()?)
    }
    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.linear_rows(symbol_to_index, self.resolved_big_m(tokens)?)
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
        self.quadratic_rows(symbol_to_index, self.configured_big_m()?)
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
        self.quadratic_rows(symbol_to_index, self.resolved_big_m(tokens)?)
    }
    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }
    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        // Missing masks are treated as the disabled branch, matching direct evaluation.
        // 缺失掩码按关闭分支处理，与直接求值保持一致。
        let mask = values.get(&self.mask_var.index());
        if mask
            .and_then(to_f64)
            .map(|value| value.abs() <= f64::EPSILON)
            .unwrap_or(true)
        {
            return from_f64(0.0);
        }
        evaluate_quadratic_from_values(&self.input, values)
    }
    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("quadratic_masking({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMaskingRangeFunction<V>
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
            None => from_f64(0.0)?,
        };
        if to_f64(&mask_value)?.abs() <= f64::EPSILON {
            return from_f64(0.0);
        }
        evaluate_quadratic(&self.input, token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMaskingRangeFunction<V>
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

impl<V> QuadraticFunctionSymbol<V> for QuadraticMaskingRangeFunction<V>
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
