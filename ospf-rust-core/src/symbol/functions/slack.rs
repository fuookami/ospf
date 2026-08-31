//! Slack-related function symbols.
//!
//! - `SlackFunction`: absolute deviation between two expressions.
//! - `SlackRangeFunction`: distance to a closed interval.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::MaxFunction;
use super::big_m::infer_linear_difference_abs_bound_from_tokens;

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

/// Absolute slack between two expressions:
/// `slack = |left - right|`.
#[derive(Debug, Clone)]
pub struct SlackFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    left: Linear<V>,
    right: Linear<V>,
    result_var: ContinuousVariableItem,
    side_var: BinaryVariableItem,
    big_m: V,
    declared_dependency_ids: Vec<u64>,
}

impl<V> SlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, left: Linear<V>, right: Linear<V>) -> Self {
        Self::with_big_m(
            id,
            name,
            left,
            right,
            from_f64(DEFAULT_BIG_M).expect("convert default big-M"),
        )
    }

    /// 使用自动 ID 与调用方提供的名称创建松弛函数。
    /// Create a slack function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, left: Linear<V>, right: Linear<V>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            left,
            right,
        )
    }

    /// 使用自动 ID 与自动名称创建松弛函数。
    /// Create a slack function with an auto id and auto-generated name.
    pub fn auto(left: Linear<V>, right: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("slack", id);
        Self::new(id, &name, left, right)
    }

    pub fn with_target(id: u64, name: &str, left: Linear<V>, right_value: V) -> Self {
        Self::new(id, name, left, Linear::new(vec![], right_value))
    }

    /// 使用自动 ID 与调用方提供的名称创建目标值松弛函数。
    /// Create a target-value slack function with an auto id and caller-provided name.
    pub fn named_target(name: impl AsRef<str>, left: Linear<V>, right_value: V) -> Self {
        Self::with_target(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            left,
            right_value,
        )
    }

    /// 使用自动 ID 与自动名称创建目标值松弛函数。
    /// Create a target-value slack function with an auto id and auto-generated name.
    pub fn auto_target(left: Linear<V>, right_value: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("slack", id);
        Self::with_target(id, &name, left, right_value)
    }

    pub fn with_big_m(id: u64, name: &str, left: Linear<V>, right: Linear<V>, big_m: V) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(
            VariableId::new(group_id, 0),
            &format!("{}_slack", name),
        );
        let side_var = BinaryVariableItem::create(
            VariableId::new(group_id, 1),
            &format!("{}_slack_side", name),
        );
        Self {
            id: IntermediateSymbolId::new(id, name),
            left,
            right,
            result_var,
            side_var,
            big_m,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建指定 Big-M 的松弛函数。
    /// Create a slack function with custom Big-M, auto id, and caller-provided name.
    pub fn named_with_big_m(
        name: impl AsRef<str>,
        left: Linear<V>,
        right: Linear<V>,
        big_m: V,
    ) -> Self {
        Self::with_big_m(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            left,
            right,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建指定 Big-M 的松弛函数。
    /// Create a slack function with custom Big-M, auto id, and auto-generated name.
    pub fn auto_with_big_m(left: Linear<V>, right: Linear<V>, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("slack", id);
        Self::with_big_m(id, &name, left, right, big_m)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_polynomials(&self, left: Linear<V>, right: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.left = left;
        cloned.right = right;
        cloned
    }

    pub(crate) fn with_big_m_value(&self, big_m: V) -> Self {
        let mut cloned = self.clone();
        cloned.big_m = big_m;
        cloned
    }

    pub fn left_polynomial(&self) -> &Linear<V> {
        &self.left
    }

    pub fn right_polynomial(&self) -> &Linear<V> {
        &self.right
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn big_m(&self) -> &V {
        &self.big_m
    }
}

impl<V> SlackFunction<V>
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
                "slack `{}` big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "slack `{}` requires positive finite big-M",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_linear_difference_abs_bound_from_tokens(&self.left, &self.right, tokens)
            .map(|big_m| big_m.max(MIN_BIG_M))
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
                    "slack result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.side_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "slack side variable id {}",
                    self.side_var.id().unique_id()
                ))
            })?;

        let left_const = to_f64(self.left.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "slack `{}` left constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let right_const = to_f64(self.right.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "slack `{}` right constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let mut y_minus_diff_monomials =
            Vec::with_capacity(self.left.monomials().len() + self.right.monomials().len() + 1);
        y_minus_diff_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "slack y coefficient")?,
            result_index,
        ));
        for monomial in self.left.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "slack `{}` left coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let left_index = monomial.var_index();
            y_minus_diff_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "slack y-diff left coefficient")?,
                left_index,
            ));
        }
        for monomial in self.right.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "slack `{}` right coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let right_index = monomial.var_index();
            y_minus_diff_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(coefficient, "slack y-diff right coefficient")?,
                right_index,
            ));
        }
        let y_minus_diff_const = -left_const + right_const;

        let mut y_plus_diff_monomials =
            Vec::with_capacity(self.left.monomials().len() + self.right.monomials().len() + 1);
        y_plus_diff_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "slack y coefficient")?,
            result_index,
        ));
        for monomial in self.left.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "slack `{}` left coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let left_index = monomial.var_index();
            y_plus_diff_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(coefficient, "slack y+diff left coefficient")?,
                left_index,
            ));
        }
        for monomial in self.right.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "slack `{}` right coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let right_index = monomial.var_index();
            y_plus_diff_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "slack y+diff right coefficient")?,
                right_index,
            ));
        }
        let y_plus_diff_const = left_const - right_const;

        let c1 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    y_minus_diff_monomials.clone(),
                    convert_f64_to_v::<V>(y_minus_diff_const, "slack c1 constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "slack c1 rhs")?,
            ),
            &format!("{}_slack_ge_diff", self.id.name),
            Arc::new(self.clone()),
        );

        let c2 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    y_plus_diff_monomials.clone(),
                    convert_f64_to_v::<V>(y_plus_diff_const, "slack c2 constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "slack c2 rhs")?,
            ),
            &format!("{}_slack_ge_neg_diff", self.id.name),
            Arc::new(self.clone()),
        );

        let mut c3_monomials = y_minus_diff_monomials;
        c3_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "slack c3 side coefficient")?,
            side_index,
        ));
        let c3 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    c3_monomials,
                    convert_f64_to_v::<V>(y_minus_diff_const, "slack c3 constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "slack c3 rhs")?,
            ),
            &format!("{}_slack_branch_pos", self.id.name),
            Arc::new(self.clone()),
        );

        let mut c4_monomials = y_plus_diff_monomials;
        c4_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "slack c4 side coefficient")?,
            side_index,
        ));
        let c4 = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    c4_monomials,
                    convert_f64_to_v::<V>(y_plus_diff_const, "slack c4 constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "slack c4 rhs")?,
            ),
            &format!("{}_slack_branch_neg", self.id.name),
            Arc::new(self.clone()),
        );

        Ok(vec![c1, c2, c3, c4])
    }
}

impl<V> Display for SlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "slack({})", self.id.name)
    }
}

impl<V> DynSymbol for SlackFunction<V>
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

impl<V> Symbol for SlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SlackFunction<V>
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
        format!("slack({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SlackFunction<V>
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
        tokens.push(Token::from_generic(
            self.side_var.clone(),
            self.side_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_linear(&self.left, token_table, zero_if_none)?)?;
        let right = to_f64(&evaluate_linear(&self.right, token_table, zero_if_none)?)?;
        from_f64((left - right).abs())
    }
}

impl<V> LinearIntermediateSymbol<V> for SlackFunction<V>
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

/// Distance to a closed interval `[lower, upper]`:
/// `max(lower - x, x - upper, 0)`.
///
/// This implementation reuses exact-max constraints internally.
#[derive(Debug, Clone)]
pub struct SlackRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    lower: V,
    upper: V,
    inner: MaxFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Linear<V>, lower: V, upper: V) -> Self {
        let lower_f64 = to_f64(&lower).expect("convert lower to f64");
        let upper_f64 = to_f64(&upper).expect("convert upper to f64");
        let input_constant = to_f64(input.constant_term()).expect("convert input constant to f64");

        let mut lower_minus_input_terms = Vec::with_capacity(input.monomials().len());
        let mut input_minus_upper_terms = Vec::with_capacity(input.monomials().len());
        for monomial in input.monomials() {
            let coefficient =
                to_f64(monomial.coefficient()).expect("convert input coefficient to f64");
            lower_minus_input_terms.push(LinearMonomial::new(
                from_f64(-coefficient).expect("convert lower-input coefficient"),
                monomial.var_index(),
            ));
            input_minus_upper_terms.push(LinearMonomial::new(
                from_f64(coefficient).expect("convert input-upper coefficient"),
                monomial.var_index(),
            ));
        }

        let lower_minus_input = Linear::new(
            lower_minus_input_terms,
            from_f64(lower_f64 - input_constant).expect("convert lower-input constant"),
        );
        let input_minus_upper = Linear::new(
            input_minus_upper_terms,
            from_f64(input_constant - upper_f64).expect("convert input-upper constant"),
        );
        let zero_poly = Linear::new(vec![], from_f64(0.0).expect("convert zero"));

        let inner = MaxFunction::new(
            id,
            name,
            vec![lower_minus_input, input_minus_upper, zero_poly],
            true,
        );
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            lower,
            upper,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let lower_f64 = to_f64(&self.lower).expect("convert lower to f64");
        let upper_f64 = to_f64(&self.upper).expect("convert upper to f64");
        let input_constant = to_f64(input.constant_term()).expect("convert input constant to f64");

        let mut lower_minus_input_terms = Vec::with_capacity(input.monomials().len());
        let mut input_minus_upper_terms = Vec::with_capacity(input.monomials().len());
        for monomial in input.monomials() {
            let coefficient =
                to_f64(monomial.coefficient()).expect("convert input coefficient to f64");
            lower_minus_input_terms.push(LinearMonomial::new(
                from_f64(-coefficient).expect("convert lower-input coefficient"),
                monomial.var_index(),
            ));
            input_minus_upper_terms.push(LinearMonomial::new(
                from_f64(coefficient).expect("convert input-upper coefficient"),
                monomial.var_index(),
            ));
        }

        let lower_minus_input = Linear::new(
            lower_minus_input_terms,
            from_f64(lower_f64 - input_constant).expect("convert lower-input constant"),
        );
        let input_minus_upper = Linear::new(
            input_minus_upper_terms,
            from_f64(input_constant - upper_f64).expect("convert input-upper constant"),
        );
        let zero_poly = Linear::new(vec![], from_f64(0.0).expect("convert zero"));

        let mut cloned = self.clone();
        cloned.input = input;
        cloned.inner =
            self.inner
                .with_polynomials(vec![lower_minus_input, input_minus_upper, zero_poly]);
        cloned
    }

    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    pub fn lower_bound(&self) -> &V {
        &self.lower
    }

    pub fn upper_bound(&self) -> &V {
        &self.upper
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "slack_range({})", self.id.name)
    }
}

impl<V> DynSymbol for SlackRangeFunction<V>
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

impl<V> Symbol for SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SlackRangeFunction<V>
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
        self.inner.mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.inner.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("slack_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SlackRangeFunction<V>
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
        self.inner.register_tokens(tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        if x < lower {
            from_f64(lower - x)
        } else if x > upper {
            from_f64(x - upper)
        } else {
            from_f64(0.0)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for SlackRangeFunction<V>
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
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        self.inner.to_quadratic_polynomial()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn slack_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(20_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let slack: SlackFunction<f64> = SlackFunction::new(
            3000,
            "slack_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            Linear::new(vec![], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <SlackFunction as FunctionSymbol>::register_tokens(&slack, &mut aux_tokens)
            .expect("slack tokens should be registered");
        let side_token = aux_tokens
            .iter()
            .find(|token| token.id() != slack.result_variable().id())
            .expect("side token should exist");

        let result_id = slack.result_variable().id().unique_id() as usize;
        let side_id = side_token.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(slack.result_variable().clone(), 1),
            side_token.clone(),
        ];

        let constraints = slack
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("slack constraints should be generated");
        let branch = constraints
            .iter()
            .find(|constraint| constraint.name == "slack_bound_slack_branch_pos")
            .expect("positive branch constraint should exist");
        let side_term = branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side term should exist");

        // 2x + 1 with x in [-2, 3] => range [-3, 7], therefore M = 7.
        assert!((branch.inequality.rhs - 7.0).abs() <= 1e-9);
        assert!((*side_term.coefficient() - 7.0).abs() <= 1e-9);
    }

    #[test]
    fn slack_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(20_010), "x");
        let slack: SlackFunction<f64> = SlackFunction::new(
            3001,
            "slack_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <SlackFunction as FunctionSymbol>::register_tokens(&slack, &mut aux_tokens)
            .expect("slack tokens should be registered");
        let side_token = aux_tokens
            .iter()
            .find(|token| token.id() != slack.result_variable().id())
            .expect("side token should exist");

        let result_id = slack.result_variable().id().unique_id() as usize;
        let side_id = side_token.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(slack.result_variable().clone(), 1),
            side_token.clone(),
        ];

        let constraints = slack
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("slack constraints should be generated");
        let branch = constraints
            .iter()
            .find(|constraint| constraint.name == "slack_default_slack_branch_pos")
            .expect("positive branch constraint should exist");
        let side_term = branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side term should exist");

        assert!((branch.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*side_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
