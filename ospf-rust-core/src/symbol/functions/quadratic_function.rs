//! 二次输入函数符号包装 / Quadratic-input function symbol wrappers
//!
//! # Rust 扩展说明 / Rust Extension Note
//!
//! 本文件包含 Quadratic*Function 包装器，提供基础函数符号的二次多项式视图 / This file contains Quadratic*Function wrappers that provide quadratic polynomial views of base function symbols
//! 这些是 Rust 特有的扩展，没有直接的 1:1 Kotlin 文件对应 / These are Rust-specific extensions that don't have direct 1:1 Kotlin file counterparts
//!
//! Kotlin 有 4 个独立的 QuadraticXxx.kt 文件 / Kotlin has 4 independent QuadraticXxx.kt files:
//! - QuadraticLinear.kt → `quadratic_linear.rs`
//! - QuadraticMin.kt → `quadratic_min.rs`
//! - QuadraticMaskingRange.kt → `quadratic_masking_range.rs`
//! - QuadraticInStepRange.kt → `quadratic_in_step_range.rs`
//!
//! 本文件中剩余的 14 个 Quadratic* 类型是 Rust 扩展 / The remaining 14 Quadratic* types in this file are Rust extensions
//! 它们将二次多项式视图与基础函数符号组合在一起，为了便利和向后兼容而保留在此 / that combine quadratic polynomial views with base function symbols, kept here for convenience and backward compatibility

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality, QuadraticConstraint};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
#[cfg(test)]
use crate::variable::VariableId;
use crate::variable::{BinaryVariableItem, ContinuousVariableItem};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticFunctionSymbol,
};
use super::big_m::{
    infer_big_m_for_quadratic_polynomials, infer_quadratic_abs_bound_from_tokens,
    infer_quadratic_bounds_from_tokens, infer_quadratic_difference_abs_bound_from_tokens,
    infer_quadratic_shifted_abs_bound_from_tokens, tighten_token_range,
};
use super::quadratic_linear::*;
use super::{
    AbsBranchBigM, AbsFunction, BinaryzationFunction, BinaryzationMethod,
    BivariateLinearPiecewiseFunction, CosFunction, InequalityFunction, InequalityKind,
    MaskingFunction, MaxFunction, ModFunction, Point2, RoundingFunction, RoundingKind,
    SigmoidFunction, SigmoidPrecision, SinFunction, SlackFunction, SlackRangeFunction, Triangle3,
    UnivariateLinearPiecewiseFunction,
};

/// 二次输入的二值化函数符号 / Quadratic-input binaryzation function symbol
#[derive(Debug, Clone)]
pub struct QuadraticBinaryzationFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: BinaryzationFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticBinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入二值化函数 / Create a new quadratic-input binaryzation function
    pub fn new(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        threshold: V,
        big_m: V,
        method: BinaryzationMethod,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 101),
            &format!("{}_bridge", name),
            input.clone(),
        );
        // QuadraticLinearFunction registers a helper only for genuine
        // quadratic inputs.  Keep a pure-linear/constant input direct so the
        // inner binaryization never references an unregistered bridge token.
        let bridge_input = bridge.input_linear_polynomial().unwrap_or_else(|| {
            Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge.result_variable().index(),
                )],
                from_f64(0.0).expect("convert 0.0"),
            )
        });
        let inner = BinaryzationFunction::new(id, name, bridge_input, threshold, big_m, method);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用指定 Big-M 创建 / Create with specified Big-M
    pub fn with_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: V) -> Self {
        Self::new(
            id,
            name,
            input,
            from_f64(0.0).expect("convert 0.0"),
            big_m,
            BinaryzationMethod::BigM,
        )
    }

    /// 使用指定阈值创建 / Create with specified threshold
    pub fn with_threshold(id: u64, name: &str, input: Quadratic<V>, threshold: V) -> Self {
        Self::new(
            id,
            name,
            input,
            threshold,
            from_f64(0.0).expect("convert 0.0"),
            BinaryzationMethod::Threshold,
        )
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &BinaryVariableItem {
        self.inner.result_variable()
    }

    fn mapped_input(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>> {
        if !self.bridge.has_quadratic_terms() {
            return self.bridge.input_linear_polynomial().ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "quadratic binaryzation input cannot be represented as linear".to_string(),
                )
                .into()
            });
        }

        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic binaryzation bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        Ok(Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        ))
    }
}

impl<V> Display for QuadraticBinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qbinary({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticBinaryzationFunction<V>
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

impl<V> Symbol for QuadraticBinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticBinaryzationFunction<V>
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
        let mut constraints = Vec::new();
        let mapped_input = self.mapped_input(symbol_to_index)?;
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = Vec::new();
        let mapped_input = self.mapped_input(symbol_to_index)?;
        let mut mapped_inner = self.inner.with_input_polynomial(mapped_input);
        if let Some(big_m) = infer_quadratic_shifted_abs_bound_from_tokens(
            &self.input,
            self.inner.threshold(),
            tokens,
        ) {
            mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                big_m.max(MIN_BIG_M),
                "quadratic binaryzation inferred big-M",
            )?);
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qbinary({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticBinaryzationFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let threshold = to_f64(self.inner.threshold())?;
        let eps = f64::EPSILON * 16.0;
        let is_true = match self.inner.method().mechanism_equivalent() {
            BinaryzationMethod::Threshold => input + eps >= threshold,
            BinaryzationMethod::BigM => input > threshold + eps,
            BinaryzationMethod::Indicator | BinaryzationMethod::SOS1 => unreachable!(),
        };
        from_f64(if is_true { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticBinaryzationFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的不等式函数符号 / Quadratic-input inequality function symbol
#[derive(Debug, Clone)]
pub struct QuadraticInequalityFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: InequalityFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticInequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入不等式函数 / Create a new quadratic-input inequality function
    pub fn new(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        right: V,
        kind: InequalityKind,
        big_m: V,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 201),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = InequalityFunction::new(id, name, bridge_input, right, kind, big_m);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 创建小于等于不等式 / Create a less-than-or-equal inequality
    pub fn less_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self {
        Self::new(id, name, input, right, InequalityKind::LessEqual, big_m)
    }

    /// 创建大于等于不等式 / Create a greater-than-or-equal inequality
    pub fn greater_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self {
        Self::new(id, name, input, right, InequalityKind::GreaterEqual, big_m)
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &BinaryVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticInequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qineq({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticInequalityFunction<V>
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

impl<V> Symbol for QuadraticInequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticInequalityFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic inequality bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_left_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic inequality bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mut mapped_inner = self.inner.with_left_polynomial(mapped_input);
        if let Some(big_m) = infer_quadratic_shifted_abs_bound_from_tokens(
            &self.input,
            self.inner.right_value(),
            tokens,
        ) {
            mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                big_m.max(MIN_BIG_M),
                "quadratic inequality inferred big-M",
            )?);
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qineq({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticInequalityFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let right = to_f64(self.inner.right_value())?;
        let eps = f64::EPSILON * 16.0;
        let satisfied = match self.inner.inequality_kind() {
            InequalityKind::LessEqual => left <= right + eps,
            InequalityKind::GreaterEqual => left + eps >= right,
            InequalityKind::Less => left < right - eps,
            InequalityKind::Greater => left > right + eps,
            InequalityKind::Equal => (left - right).abs() <= eps,
            InequalityKind::NotEqual => (left - right).abs() > eps,
        };
        from_f64(if satisfied { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticInequalityFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的取整函数符号 / Quadratic-input rounding function symbol
#[derive(Debug, Clone)]
pub struct QuadraticRoundingFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: RoundingFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticRoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入取整函数 / Create a new quadratic-input rounding function
    pub fn new(id: u64, name: &str, input: Quadratic<V>, kind: RoundingKind) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 301),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = RoundingFunction::new(id, name, bridge_input, kind);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 创建向下取整函数 / Create a floor rounding function
    pub fn floor(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Floor)
    }

    /// 创建向上取整函数 / Create a ceil rounding function
    pub fn ceil(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Ceil)
    }

    /// 创建四舍五入函数 / Create a round function
    pub fn round(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Round)
    }

    /// 创建截断取整函数 / Create a trunc function
    pub fn trunc(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Trunc)
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticRoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qround({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticRoundingFunction<V>
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

impl<V> Symbol for QuadraticRoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticRoundingFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic rounding bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic rounding bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        match infer_quadratic_abs_bound_from_tokens(&self.input, tokens) {
            Some(big_m) => constraints.extend(
                mapped_inner
                    .mechanism_constraints_with_big_m(symbol_to_index, big_m.max(MIN_BIG_M))?,
            ),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let value = to_f64(&evaluate_quadratic_from_values(&self.input, values)?)?;
        from_f64(value.max(0.0))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qround({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticRoundingFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let output = match self.inner.rounding_kind() {
            RoundingKind::Floor => input.floor(),
            RoundingKind::Ceil => input.ceil(),
            RoundingKind::Round => input.round(),
            RoundingKind::Trunc => input.trunc(),
        };
        from_f64(output)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticRoundingFunction<V>
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
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的取模函数符号 / Quadratic-input modulo function symbol
#[derive(Debug, Clone)]
pub struct QuadraticModFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: ModFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入取模函数 / Create a new quadratic-input modulo function
    pub fn new(id: u64, name: &str, input: Quadratic<V>, divisor: V) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 401),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = ModFunction::new(id, name, bridge_input, divisor);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmod({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticModFunction<V>
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

impl<V> Symbol for QuadraticModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticModFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic mod bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmod({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticModFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let divisor = to_f64(self.inner.divisor())?;
        if divisor.abs() <= f64::EPSILON {
            return None;
        }
        let mut value = input % divisor;
        if value.abs() <= f64::EPSILON {
            value = 0.0;
        }
        if value < 0.0 {
            value += divisor.abs();
        }
        from_f64(value)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticModFunction<V>
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
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的最大值函数符号 / Quadratic-input maximum function symbol
#[derive(Debug, Clone)]
pub struct QuadraticMaxFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inputs: Vec<Quadratic<V>>,
    bridges: Vec<QuadraticLinearFunction<V>>,
    inner: MaxFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入最大值函数 / Create a new quadratic-input maximum function
    pub fn new(id: u64, name: &str, inputs: Vec<Quadratic<V>>, exact: bool) -> Self {
        let bridges: Vec<QuadraticLinearFunction<V>> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                QuadraticLinearFunction::new(
                    auxiliary_id(id, 601 + i as u64),
                    &format!("{}_bridge{}", name, i),
                    input.clone(),
                )
            })
            .collect();
        // Keep pure-linear candidates expression-only.  QuadraticLinearFunction
        // registers a helper token only for genuine quadratic inputs.
        let linear_inputs: Vec<Linear<V>> = bridges
            .iter()
            .map(|bridge| {
                bridge.input_linear_polynomial().unwrap_or_else(|| {
                    Linear::new(
                        vec![LinearMonomial::new(
                            from_f64(1.0).expect("convert 1.0"),
                            bridge.result_variable().index(),
                        )],
                        from_f64(0.0).expect("convert 0.0"),
                    )
                })
            })
            .collect();
        let inner = MaxFunction::new(id, name, linear_inputs, exact);

        Self {
            id: IntermediateSymbolId::new(id, name),
            inputs,
            bridges,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    fn mapped_inputs(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Vec<Linear<V>>> {
        self.bridges
            .iter()
            .map(|bridge| {
                if !bridge.has_quadratic_terms() {
                    return bridge.input_linear_polynomial().ok_or_else(|| {
                        ModelError::InvalidConstraint(
                            "quadratic max linear candidate cannot be represented as linear"
                                .to_string(),
                        )
                        .into()
                    });
                }

                let bridge_index = symbol_to_index
                    .get(&(bridge.result_variable().id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "quadratic max bridge variable id {}",
                            bridge.result_variable().id().unique_id()
                        ))
                    })?;
                Ok(Linear::new(
                    vec![LinearMonomial::new(
                        from_f64(1.0).expect("convert 1.0"),
                        bridge_index,
                    )],
                    from_f64(0.0).expect("convert 0.0"),
                ))
            })
            .collect()
    }
}

impl<V> Display for QuadraticMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmax({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMaxFunction<V>
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

impl<V> Symbol for QuadraticMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMaxFunction<V>
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
        let mut constraints = Vec::new();
        let mapped_inputs = self.mapped_inputs(symbol_to_index)?;
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = Vec::new();
        let mapped_inputs = self.mapped_inputs(symbol_to_index)?;
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        let big_m = infer_big_m_for_quadratic_polynomials(&self.inputs, tokens, MIN_BIG_M);
        match big_m {
            Some(big_m) => constraints
                .extend(mapped_inner.mechanism_constraints_with_big_m(symbol_to_index, big_m)?),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.quadratic_mechanism_constraints(symbol_to_index)?);
        }
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmax({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMaxFunction<V>
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
        for bridge in &self.bridges {
            bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut max_value: Option<f64> = None;
        for input in &self.inputs {
            let value = to_f64(&evaluate_quadratic(input, token_table, zero_if_none)?)?;
            max_value = Some(match max_value {
                Some(current) => current.max(value),
                None => value,
            });
        }
        match max_value {
            Some(v) => from_f64(v),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMaxFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的松弛函数符号 / Quadratic-input slack function symbol
#[derive(Debug, Clone)]
pub struct QuadraticSlackFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    left: Quadratic<V>,
    right: Quadratic<V>,
    left_bridge: QuadraticLinearFunction<V>,
    right_bridge: QuadraticLinearFunction<V>,
    inner: SlackFunction<V>,
    explicit_big_m: Option<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入松弛函数 / Create a new quadratic-input slack function
    pub fn new(id: u64, name: &str, left: Quadratic<V>, right: Quadratic<V>) -> Self {
        Self::with_optional_big_m(id, name, left, right, None)
    }

    /// 使用指定目标值创建 / Create with specified target value
    pub fn with_target(id: u64, name: &str, left: Quadratic<V>, right_value: V) -> Self {
        Self::new(id, name, left, Quadratic::new(vec![], right_value))
    }

    /// 使用指定 Big-M 创建 / Create with specified Big-M
    pub fn with_big_m(
        id: u64,
        name: &str,
        left: Quadratic<V>,
        right: Quadratic<V>,
        big_m: V,
    ) -> Self {
        Self::with_optional_big_m(id, name, left, right, Some(big_m))
    }

    fn with_optional_big_m(
        id: u64,
        name: &str,
        left: Quadratic<V>,
        right: Quadratic<V>,
        big_m: Option<V>,
    ) -> Self {
        let left_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 701),
            &format!("{}_lbridge", name),
            left.clone(),
        );
        let right_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 702),
            &format!("{}_rbridge", name),
            right.clone(),
        );
        let left_linear = left_bridge.input_linear_polynomial().unwrap_or_else(|| {
            Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    left_bridge.result_variable().index(),
                )],
                from_f64(0.0).expect("convert 0.0"),
            )
        });
        let right_linear = right_bridge.input_linear_polynomial().unwrap_or_else(|| {
            Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    right_bridge.result_variable().index(),
                )],
                from_f64(0.0).expect("convert 0.0"),
            )
        });
        let inner = match big_m.clone() {
            Some(big_m) => SlackFunction::with_big_m(id, name, left_linear, right_linear, big_m),
            None => SlackFunction::new(id, name, left_linear, right_linear),
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            left,
            right,
            left_bridge,
            right_bridge,
            inner,
            explicit_big_m: big_m,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 返回原始左右二次表达式。
    /// Return the original left and right quadratic expressions.
    pub fn input_polynomials(&self) -> (&Quadratic<V>, &Quadratic<V>) {
        (&self.left, &self.right)
    }

    /// 返回显式 Big-M；默认构造时为 `None`，约束生成会从令牌边界推导。
    /// Return the explicit Big-M; `None` means constraint generation derives it from token bounds.
    pub fn big_m(&self) -> Option<&V> {
        self.explicit_big_m.as_ref()
    }

    fn mapped_input(
        bridge: &QuadraticLinearFunction<V>,
        symbol_to_index: &HashMap<usize, usize>,
        description: &str,
    ) -> Result<Linear<V>> {
        if let Some(input) = bridge.input_linear_polynomial() {
            return Ok(input);
        }
        let bridge_index = symbol_to_index
            .get(&(bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic slack {} bridge variable id {}",
                    description,
                    bridge.result_variable().id().unique_id()
                ))
            })?;
        Ok(Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        ))
    }
}

impl<V> Display for QuadraticSlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qslack({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSlackFunction<V>
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

impl<V> Symbol for QuadraticSlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSlackFunction<V>
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
        let mapped_left = Self::mapped_input(&self.left_bridge, symbol_to_index, "left")?;
        let mapped_right = Self::mapped_input(&self.right_bridge, symbol_to_index, "right")?;
        let mut constraints = Vec::new();
        if self.left_bridge.has_quadratic_terms() {
            constraints.extend(self.left_bridge.mechanism_constraints(symbol_to_index)?);
        }
        if self.right_bridge.has_quadratic_terms() {
            constraints.extend(self.right_bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mapped_inner = self.inner.with_polynomials(mapped_left, mapped_right);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mapped_left = Self::mapped_input(&self.left_bridge, symbol_to_index, "left")?;
        let mapped_right = Self::mapped_input(&self.right_bridge, symbol_to_index, "right")?;
        let mut constraints = Vec::new();
        if self.left_bridge.has_quadratic_terms() {
            constraints.extend(self.left_bridge.mechanism_constraints(symbol_to_index)?);
        }
        if self.right_bridge.has_quadratic_terms() {
            constraints.extend(self.right_bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inner = self.inner.with_polynomials(mapped_left, mapped_right);
        if self.explicit_big_m.is_none() {
            if let Some(big_m) =
                infer_quadratic_difference_abs_bound_from_tokens(&self.left, &self.right, tokens)
            {
                mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                    big_m.max(MIN_BIG_M),
                    "quadratic slack inferred big-M",
                )?);
            }
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = Vec::new();
        if self.left_bridge.has_quadratic_terms() {
            constraints.extend(
                self.left_bridge
                    .quadratic_mechanism_constraints(symbol_to_index)?,
            );
        }
        if self.right_bridge.has_quadratic_terms() {
            constraints.extend(
                self.right_bridge
                    .quadratic_mechanism_constraints(symbol_to_index)?,
            );
        }
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let left = to_f64(&evaluate_quadratic_from_values(&self.left, values)?)?;
        let right = to_f64(&evaluate_quadratic_from_values(&self.right, values)?)?;
        from_f64((left - right).abs())
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qslack({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSlackFunction<V>
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
        if self.left_bridge.has_quadratic_terms() {
            self.left_bridge.register_tokens(tokens)?;
        }
        if self.right_bridge.has_quadratic_terms() {
            self.right_bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_quadratic(&self.left, token_table, zero_if_none)?)?;
        let right = to_f64(&evaluate_quadratic(&self.right, token_table, zero_if_none)?)?;
        from_f64((left - right).abs())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSlackFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的区间松弛函数符号 / Quadratic-input range slack function symbol
#[derive(Debug, Clone)]
pub struct QuadraticSlackRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    lower: V,
    upper: V,
    bridge: QuadraticLinearFunction<V>,
    inner: SlackRangeFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 创建新的二次输入区间松弛函数 / Create a new quadratic-input range slack function
    pub fn new(id: u64, name: &str, input: Quadratic<V>, lower: V, upper: V) -> Self {
        Self::with_optional_big_m(id, name, input, lower, upper, None)
    }

    /// 使用显式 Big-M 创建二次输入区间松弛函数。
    /// Create a quadratic range-slack function with an explicit Big-M.
    pub fn with_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        lower: V,
        upper: V,
        big_m: V,
    ) -> Self {
        Self::with_optional_big_m(id, name, input, lower, upper, Some(big_m))
    }

    fn with_optional_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        lower: V,
        upper: V,
        big_m: Option<V>,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 801),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = bridge.input_linear_polynomial().unwrap_or_else(|| {
            Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge.result_variable().index(),
                )],
                from_f64(0.0).expect("convert 0.0"),
            )
        });
        let inner = match big_m {
            Some(big_m) => SlackRangeFunction::with_big_m(
                id,
                name,
                bridge_input,
                lower.clone(),
                upper.clone(),
                big_m,
            ),
            None => SlackRangeFunction::new(id, name, bridge_input, lower.clone(), upper.clone()),
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            lower,
            upper,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn input_polynomial(&self) -> &Quadratic<V> {
        &self.input
    }

    pub fn lower_bound(&self) -> &V {
        &self.lower
    }

    pub fn upper_bound(&self) -> &V {
        &self.upper
    }

    pub fn big_m(&self) -> Option<&V> {
        self.inner.big_m()
    }

    fn mapped_input(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>> {
        if let Some(input) = self.bridge.input_linear_polynomial() {
            return Ok(input);
        }
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic slack-range bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        Ok(Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        ))
    }
}

impl<V> Display for QuadraticSlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qslack_range({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSlackRangeFunction<V>
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

impl<V> Symbol for QuadraticSlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSlackRangeFunction<V>
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
        let mapped_input = self.mapped_input(symbol_to_index)?;
        let mut constraints = Vec::new();
        if self.bridge.has_quadratic_terms() {
            constraints.extend(self.bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mapped_input = self.mapped_input(symbol_to_index)?;
        let mut constraints = Vec::new();
        if self.bridge.has_quadratic_terms() {
            constraints.extend(self.bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inner = self.inner.with_input_polynomial(mapped_input);
        if self.inner.big_m().is_none() {
            let lower_bound =
                infer_quadratic_shifted_abs_bound_from_tokens(&self.input, &self.lower, tokens);
            let upper_bound =
                infer_quadratic_shifted_abs_bound_from_tokens(&self.input, &self.upper, tokens);
            if let Some(big_m) = lower_bound.into_iter().chain(upper_bound).reduce(f64::max) {
                mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                    big_m.max(MIN_BIG_M),
                    "quadratic slack-range inferred big-M",
                )?);
            }
        }
        constraints
            .extend(mapped_inner.mechanism_constraints_with_tokens(symbol_to_index, tokens)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if self.bridge.has_quadratic_terms() {
            self.bridge.quadratic_mechanism_constraints(symbol_to_index)
        } else {
            Ok(Vec::new())
        }
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let value = to_f64(&evaluate_quadratic_from_values(&self.input, values)?)?;
        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        if value < lower {
            from_f64(lower - value)
        } else if value > upper {
            from_f64(value - upper)
        } else {
            from_f64(0.0)
        }
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qslack_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSlackRangeFunction<V>
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
        if self.bridge.has_quadratic_terms() {
            self.bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
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

impl<V> LinearIntermediateSymbol<V> for QuadraticSlackRangeFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的掩码函数符号 / Quadratic-input masking function symbol
#[derive(Debug, Clone)]
pub struct QuadraticMaskingFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: MaskingFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入掩码函数 / Create a new quadratic-input masking function
    pub fn new(id: u64, name: &str, input: Quadratic<V>, mask_var: BinaryVariableItem) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 851),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = MaskingFunction::new(id, name, bridge_input, mask_var);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用指定 Big-M 创建 / Create with specified Big-M
    pub fn with_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        mask_var: BinaryVariableItem,
        big_m: V,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 851),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = MaskingFunction::with_big_m(id, name, bridge_input, mask_var, big_m);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 获取掩码变量 / Get the mask variable
    pub fn mask_variable(&self) -> &BinaryVariableItem {
        self.inner.mask_variable()
    }
}

impl<V> Display for QuadraticMaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmasking({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMaskingFunction<V>
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

impl<V> Symbol for QuadraticMaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMaskingFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mut mapped_inner = self.inner.with_input_polynomial(mapped_input);
        if let Some(big_m) = infer_quadratic_abs_bound_from_tokens(&self.input, tokens) {
            mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                big_m.max(MIN_BIG_M),
                "quadratic masking inferred big-M",
            )?);
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let value = to_f64(&evaluate_quadratic_from_values(&self.input, values)?)?;
        from_f64(value.max(0.0))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmasking({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMaskingFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let mask = match token_table
            .find_by_id(self.mask_variable().id())
            .and_then(|token| token.get_result())
        {
            Some(v) => to_f64(&v)?,
            None if zero_if_none => 0.0,
            None => return None,
        };
        let active = if mask.abs() <= f64::EPSILON { 0.0 } else { 1.0 };
        from_f64(input * active)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMaskingFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的正弦函数符号 / Quadratic-input sine function symbol
#[derive(Debug, Clone)]
pub struct QuadraticSinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: SinFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入正弦函数 / Create a new quadratic-input sine function
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 901),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SinFunction::new(id, name, bridge_input);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticSinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qsin({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSinFunction<V>
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

impl<V> Symbol for QuadraticSinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSinFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic sin bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qsin({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSinFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(input.sin())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSinFunction<V>
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
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的余弦函数符号 / Quadratic-input cosine function symbol
#[derive(Debug, Clone)]
pub struct QuadraticCosFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: CosFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticCosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建新的二次输入余弦函数 / Create a new quadratic-input cosine function
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1001),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = CosFunction::new(id, name, bridge_input);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticCosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qcos({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticCosFunction<V>
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

impl<V> Symbol for QuadraticCosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticCosFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic cos bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qcos({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticCosFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(input.cos())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticCosFunction<V>
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
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的单变量线性分段插值函数符号 / Quadratic-input univariate linear piecewise interpolation function symbol
#[derive(Debug, Clone)]
pub struct QuadraticUnivariateLinearPiecewiseFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: UnivariateLinearPiecewiseFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticUnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 创建新的二次输入单变量分段线性插值函数 / Create a new quadratic-input univariate linear piecewise interpolation function
    pub fn new(id: u64, name: &str, input: Quadratic<V>, points: Vec<Point2<V>>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1101),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = UnivariateLinearPiecewiseFunction::new(id, name, bridge_input, points);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticUnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qulp({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticUnivariateLinearPiecewiseFunction<V>
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

impl<V> Symbol for QuadraticUnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticUnivariateLinearPiecewiseFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic univariate-piecewise bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qulp({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticUnivariateLinearPiecewiseFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let points = self.inner.points();
        if points.is_empty() {
            return from_f64(0.0);
        }
        if points.len() == 1 {
            return Some(points[0].y.clone());
        }

        let x = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let first_x = to_f64(&points[0].x)?;
        if x <= first_x {
            return Some(points[0].y.clone());
        }
        for i in 0..(points.len() - 1) {
            let x0 = to_f64(&points[i].x)?;
            let x1 = to_f64(&points[i + 1].x)?;
            if x <= x1 {
                let y0 = to_f64(&points[i].y)?;
                let y1 = to_f64(&points[i + 1].y)?;
                if (x1 - x0).abs() <= f64::EPSILON {
                    return from_f64(y1);
                }
                let ratio = (x - x0) / (x1 - x0);
                return from_f64(y0 + ratio * (y1 - y0));
            }
        }
        points.last().map(|point| point.y.clone())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticUnivariateLinearPiecewiseFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的双变量线性分段插值函数符号 / Quadratic-input bivariate linear piecewise interpolation function symbol
#[derive(Debug, Clone)]
pub struct QuadraticBivariateLinearPiecewiseFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    x_bridge: QuadraticLinearFunction<V>,
    y_bridge: QuadraticLinearFunction<V>,
    inner: BivariateLinearPiecewiseFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticBivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 创建新的二次输入双变量分段线性插值函数 / Create a new quadratic-input bivariate piecewise linear interpolation function
    pub fn new(
        id: u64,
        name: &str,
        x_input: Quadratic<V>,
        y_input: Quadratic<V>,
        triangles: Vec<Triangle3<V>>,
    ) -> Self {
        let x_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1201),
            &format!("{}_xbridge", name),
            x_input.clone(),
        );
        let y_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1202),
            &format!("{}_ybridge", name),
            y_input.clone(),
        );
        let x_linear = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                x_bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let y_linear = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                y_bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = BivariateLinearPiecewiseFunction::new(id, name, x_linear, y_linear, triangles);

        Self {
            id: IntermediateSymbolId::new(id, name),
            x_bridge,
            y_bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticBivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qblp({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticBivariateLinearPiecewiseFunction<V>
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

impl<V> Symbol for QuadraticBivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticBivariateLinearPiecewiseFunction<V>
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
        let mut constraints = self.x_bridge.mechanism_constraints(symbol_to_index)?;
        constraints.extend(self.y_bridge.mechanism_constraints(symbol_to_index)?);
        let x_index = symbol_to_index
            .get(&(self.x_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic bivariate-piecewise x bridge variable id {}",
                    self.x_bridge.result_variable().id().unique_id()
                ))
            })?;
        let y_index = symbol_to_index
            .get(&(self.y_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic bivariate-piecewise y bridge variable id {}",
                    self.y_bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_x = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                x_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_y = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                y_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomials(mapped_x, mapped_y);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = self
            .x_bridge
            .quadratic_mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.y_bridge
                .quadratic_mechanism_constraints(symbol_to_index)?,
        );
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qblp({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticBivariateLinearPiecewiseFunction<V>
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
        self.x_bridge.register_tokens(tokens)?;
        self.y_bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.inner.calculate_value(token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticBivariateLinearPiecewiseFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次正部输入的已知符号 / Known sign of a quadratic positive-part input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnownSign {
    /// 输入在有限域上恒非负，结果等于输入本身。
    /// The input is non-negative over its finite domain, so the result equals the input.
    NonNegative,
    /// 输入在有限域上恒非正，结果恒为零。
    /// The input is non-positive over its finite domain, so the result is always zero.
    NonPositive,
}

/// 二次输入的正部函数符号（max(f, 0)）/ Quadratic-input positive-part function symbol (max(f, 0))
#[derive(Debug, Clone)]
pub struct QuadraticPositivePartFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: MaxFunction<V>,
    explicit_big_m: Option<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticPositivePartFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 创建新的二次输入正部函数 / Create a new quadratic-input positive-part function
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::with_optional_big_m(id, name, input, None)
    }

    /// 使用显式 Big-M 创建二次输入正部函数。
    /// Create a quadratic positive-part function with an explicit Big-M.
    pub fn with_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: V) -> Self {
        Self::with_optional_big_m(id, name, input, Some(big_m))
    }

    fn with_optional_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        explicit_big_m: Option<V>,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1401),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = bridge.input_linear_polynomial().unwrap_or_else(|| {
            Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge.result_variable().index(),
                )],
                from_f64(0.0).expect("convert 0.0"),
            )
        });
        let zero_input = Linear::new(vec![], from_f64(0.0).expect("convert 0.0"));
        let inner = MaxFunction::new(id, name, vec![bridge_input, zero_input], true);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            explicit_big_m,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 返回原始二次输入。
    /// Return the original quadratic input.
    pub fn input_polynomial(&self) -> &Quadratic<V> {
        &self.input
    }

    /// 返回显式 Big-M；默认构造时为 `None`，约束生成会从令牌边界推导。
    /// Return the explicit Big-M; `None` means constraint generation derives it from token bounds.
    pub fn big_m(&self) -> Option<&V> {
        self.explicit_big_m.as_ref()
    }

    fn mapped_input(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>> {
        if let Some(input) = self.bridge.input_linear_polynomial() {
            return Ok(input);
        }
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic positive-part bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        Ok(Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        ))
    }

    fn explicit_big_m_f64(&self) -> Result<Option<f64>> {
        self.explicit_big_m
            .as_ref()
            .map(|value| {
                let big_m = to_f64(value).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "quadratic positive-part `{}` Big-M cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                if !big_m.is_finite() || big_m <= 0.0 {
                    return Err(ModelError::InvalidConstraint(format!(
                        "quadratic positive-part `{}` requires a positive finite Big-M",
                        self.id.name
                    ))
                    .into());
                }
                Ok(big_m)
            })
            .transpose()
    }
}

impl<V> Display for QuadraticPositivePartFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "quadratic_positive_part({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticPositivePartFunction<V>
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

impl<V> Symbol for QuadraticPositivePartFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> QuadraticPositivePartFunction<V>
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
    /// 二次输入在有限域上恒非负或恒非正时，`max(p(x), 0)` 退化为恒等或恒零。
    ///
    /// A quadratic input that is provably non-negative or non-positive over a finite
    /// domain degenerates `max(p(x), 0)` into the identity or into zero.
    fn known_input_sign(&self, tokens: &[Token<V>]) -> Option<KnownSign> {
        let (lower, upper) = infer_quadratic_bounds_from_tokens(&self.input, tokens)?;
        if lower >= 0.0 {
            Some(KnownSign::NonNegative)
        } else if upper <= 0.0 {
            Some(KnownSign::NonPositive)
        } else {
            None
        }
    }

    /// 从已注册令牌判断输入符号；选择器未注册时说明注册阶段已判定符号已知。
    ///
    /// 选择器缺席只说明"符号已知"，不携带方向；方向必须由令牌边界重新推导，因此该路径
    /// 要求调用方改用 [`IntermediateSymbol::mechanism_constraints_with_tokens`]。
    ///
    /// Determine the input sign from the registered tokens. A missing selector means the
    /// registration step already established a known sign, but it does not carry the
    /// direction; the direction has to be re-derived from token bounds, so this path
    /// requires the caller to use
    /// [`IntermediateSymbol::mechanism_constraints_with_tokens`] instead.
    fn registered_input_sign(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Option<KnownSign>> {
        let Some(selectors) = self.inner.selector_variables() else {
            return Ok(None);
        };
        if selectors
            .iter()
            .any(|var| symbol_to_index.contains_key(&(var.id().unique_id() as usize)))
        {
            return Ok(None);
        }
        Err(ModelError::InvalidConstraint(format!(
            "quadratic positive-part `{}` dropped its selectors for a sign-known input and needs the token context to rebuild the equality; use `mechanism_constraints_with_tokens`",
            self.id.name
        ))
        .into())
    }

    /// 生成符号已知时的等式约束：非负输入为 `result = mapped_input`，非正输入为 `result = 0`。
    ///
    /// Build the equality used when the input sign is known: `result = mapped_input` for a
    /// non-negative input and `result = 0` for a non-positive input.
    fn sign_known_constraint(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        mapped_input: &Linear<V>,
        sign: KnownSign,
    ) -> Result<LinearConstraint<V>> {
        let result_index = symbol_to_index
            .get(&(self.inner.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic positive-part result variable id {}",
                    self.inner.result_variable().id().unique_id()
                ))
            })?;

        let mut monomials = vec![LinearMonomial::new(
            from_f64(1.0).expect("convert 1.0"),
            result_index,
        )];
        let constant = match sign {
            KnownSign::NonNegative => {
                for monomial in mapped_input.monomials() {
                    let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                        ModelError::InvalidConstraint(format!(
                            "quadratic positive-part `{}` input coefficient cannot be converted to f64",
                            self.id.name
                        ))
                    })?;
                    monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(-coefficient, "quadratic positive-part input")?,
                        monomial.var_index(),
                    ));
                }
                let constant = to_f64(mapped_input.constant_term()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "quadratic positive-part `{}` input constant cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                convert_f64_to_v::<V>(-constant, "quadratic positive-part constant")?
            }
            KnownSign::NonPositive => from_f64(0.0).expect("convert 0.0"),
        };

        Ok(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(monomials, constant),
                ConstraintRelation::Equal,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_sign_known", self.id.name),
            Arc::new(self.clone()),
        ))
    }
}

impl<V> IntermediateSymbol<V> for QuadraticPositivePartFunction<V>
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

    fn register_auxiliary_tokens_with_context(
        &self,
        tokens: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        if self.bridge.has_quadratic_terms() {
            self.bridge.register_tokens(tokens)?;
        }
        // 输入符号已知时不需要选择器与 Big-M，只保留结果列。
        // A known input sign needs no selector or Big-M and keeps only the result column.
        if self.known_input_sign(registered).is_some() {
            let result_variable = self.inner.result_variable().clone();
            tokens.push(Token::from_generic(
                result_variable.clone(),
                result_variable.index(),
            ));
            return Ok(());
        }
        self.inner.register_tokens(tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mapped_input = self.mapped_input(symbol_to_index)?;
        let mut constraints = if self.bridge.has_quadratic_terms() {
            self.bridge.mechanism_constraints(symbol_to_index)?
        } else {
            Vec::new()
        };
        if let Some(sign) = self.registered_input_sign(symbol_to_index)? {
            constraints.push(self.sign_known_constraint(symbol_to_index, &mapped_input, sign)?);
            return Ok(constraints);
        }
        let zero_input = Linear::new(vec![], from_f64(0.0).expect("convert 0.0"));
        let mapped_inner = self.inner.with_polynomials(vec![mapped_input, zero_input]);
        if let Some(big_m) = self.explicit_big_m_f64()? {
            constraints
                .extend(mapped_inner.mechanism_constraints_with_big_m(symbol_to_index, big_m)?);
        } else {
            constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        }
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mapped_input = self.mapped_input(symbol_to_index)?;
        let mut constraints = if self.bridge.has_quadratic_terms() {
            self.bridge.mechanism_constraints(symbol_to_index)?
        } else {
            Vec::new()
        };
        if let Some(sign) = self.known_input_sign(tokens) {
            constraints.push(self.sign_known_constraint(symbol_to_index, &mapped_input, sign)?);
            return Ok(constraints);
        }
        let zero_input = Linear::new(vec![], from_f64(0.0).expect("convert 0.0"));
        let mapped_inner = self.inner.with_polynomials(vec![mapped_input, zero_input]);
        let inferred = infer_quadratic_abs_bound_from_tokens(&self.input, tokens);
        match self.explicit_big_m_f64()?.or(inferred) {
            Some(big_m) => constraints.extend(
                mapped_inner
                    .mechanism_constraints_with_big_m(symbol_to_index, big_m.max(MIN_BIG_M))?,
            ),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if self.bridge.has_quadratic_terms() {
            self.bridge.quadratic_mechanism_constraints(symbol_to_index)
        } else {
            Ok(Vec::new())
        }
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let value = to_f64(&evaluate_quadratic_from_values(&self.input, values)?)?;
        from_f64(value.max(0.0))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("quadratic_positive_part({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticPositivePartFunction<V>
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
        if self.bridge.has_quadratic_terms() {
            self.bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(x.max(0.0))
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticPositivePartFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 二次输入的绝对值函数符号（|q|）/ Quadratic-input absolute-value function symbol (|q|)
///
/// 数学形式 / Mathematical form: `result = |q|`，`q` 为二次多项式。
/// Mathematical form: `result = |q|` where `q` is a quadratic polynomial.
///
/// 组合契约 / Composition contract:
/// - 真正的二次输入先经 [`QuadraticLinearFunction`] 桥接为标量列，产生恰好一条二次等式
///   `q - bridge = 0`，再对该标量列施加线性 [`AbsFunction`] 的四条分支行；
/// - 纯线性（经 [`lift_linear_input`] 提升）输入直接退化：不注册桥接列、不产生二次约束，
///   只保留结果列与分支指示列；
/// - 桥接列只以一次项参与组合，因此组合次数恒不超过二次。
///
/// A genuine quadratic input is bridged into a scalar column by [`QuadraticLinearFunction`],
/// producing exactly one quadratic equality `q - bridge = 0`, after which the four branch rows of
/// the linear [`AbsFunction`] apply to that column; a purely linear (lifted) input degenerates
/// without a bridge column or quadratic constraint. The bridge column only takes part as a linear
/// term, so composition never exceeds degree two.
#[derive(Debug, Clone)]
pub struct QuadraticAbsFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 原始二次输入 / Original quadratic input
    input: Quadratic<V>,
    /// 二次输入桥接 / Quadratic-input bridge
    bridge: QuadraticLinearFunction<V>,
    /// 线性 ABS 分支载体（持有结果列与分支指示列）/ Linear ABS carrier (owns result and side columns)
    inner: AbsFunction<V>,
    /// 显式分支 Big-M / Explicit branch Big-M
    explicit_big_m: Option<AbsBranchBigM>,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticAbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 创建新的二次输入绝对值函数。
    ///
    /// 未提供显式 Big-M 时，约束生成先从令牌边界推断非对称分支 Big-M，无法推断时回退到
    /// [`AbsBranchBigM::fallback`]。
    ///
    /// Create a new quadratic-input absolute-value function.
    ///
    /// Without an explicit Big-M, constraint generation first infers the asymmetric branch pair
    /// from the token bounds and falls back to [`AbsBranchBigM::fallback`] when inference fails.
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::with_optional_big_m(id, name, input, None)
    }

    /// 由线性输入创建：先经共享适配 [`lift_linear_input`] 提升为退化的二次输入。
    ///
    /// 提升不新增任何 helper 列或 token，因此纯线性输入包装后只有结果列与分支指示列。
    ///
    /// Create from a linear input by lifting it through the shared adapter
    /// [`lift_linear_input`] into a degenerate quadratic input.
    ///
    /// The lift adds no helper column or token, so wrapping a purely linear input yields only the
    /// result column and the branch indicator column.
    pub fn from_linear(id: u64, name: &str, input: Linear<V>) -> Self {
        Self::new(id, name, lift_linear_input(&input))
    }

    /// 使用显式对称分支 Big-M 创建（两条分支行取同一取值）。
    ///
    /// Big-M 的合法性在约束生成阶段校验：非有限值或非正值返回
    /// [`ModelError::InvalidConstraint`]。
    ///
    /// Create with an explicit symmetric branch Big-M (both branch rows share one value).
    ///
    /// The value is validated during constraint generation: a non-finite or non-positive Big-M
    /// returns [`ModelError::InvalidConstraint`].
    pub fn with_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: V) -> Self {
        let value = to_f64(&big_m).unwrap_or(f64::NAN);
        Self::with_optional_big_m(
            id,
            name,
            input,
            Some(AbsBranchBigM {
                positive_branch: value,
                negative_branch: value,
            }),
        )
    }

    /// 使用显式非对称分支 Big-M 创建。
    ///
    /// 正分支行 `y - bridge + M_pos * b <= M_pos` 与负分支行 `y + bridge - M_neg * b <= 0`
    /// 各自只在另一侧生效，因此两个取值可以不同；候选输入取值跨正负时非对称取值明显更紧。
    ///
    /// Create with an explicit asymmetric branch Big-M pair.
    ///
    /// The positive branch row `y - bridge + M_pos * b <= M_pos` and the negative branch row
    /// `y + bridge - M_neg * b <= 0` each relax only on the other side, so the two values may
    /// differ; the pair is clearly tighter when the candidate input spans both signs.
    pub fn with_branch_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        big_m: AbsBranchBigM,
    ) -> Self {
        Self::with_optional_big_m(id, name, input, Some(big_m))
    }

    fn with_optional_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        explicit_big_m: Option<AbsBranchBigM>,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1451),
            &format!("{}_bridge", name),
            input.clone(),
        );
        // 纯线性输入没有桥接列：内层 ABS 直接作用于原始线性表达式（退化路径）。
        // A purely linear input has no bridge column: the inner ABS applies to the original
        // linear expression directly (the degenerate path).
        let bridge_input = bridge.input_linear_polynomial().unwrap_or_else(|| {
            Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge.result_variable().index(),
                )],
                from_f64(0.0).expect("convert 0.0"),
            )
        });
        let inner = AbsFunction::new(id, name, bridge_input);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            explicit_big_m,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 获取分支指示列（选择器）/ Get the branch indicator (selector) column
    pub fn side_variable(&self) -> &BinaryVariableItem {
        self.inner.side_variable()
    }

    /// 返回原始二次输入 / Return the original quadratic input
    pub fn input_polynomial(&self) -> &Quadratic<V> {
        &self.input
    }

    /// 输入是否含真正的二次项 / Whether the input carries a genuine quadratic monomial
    pub fn has_quadratic_input(&self) -> bool {
        self.bridge.has_quadratic_terms()
    }

    /// 返回桥接列；仅当 [`Self::has_quadratic_input`] 为真时该列才会注册进模型。
    /// Return the bridge column; it is registered only when [`Self::has_quadratic_input`] holds.
    pub fn bridge_variable(&self) -> &ContinuousVariableItem {
        self.bridge.result_variable()
    }

    /// 返回显式分支 Big-M；`None` 表示从令牌边界推断或回退到策略值。
    /// Return the explicit branch Big-M; `None` means inference from token bounds or policy fallback.
    pub fn big_m(&self) -> Option<AbsBranchBigM> {
        self.explicit_big_m
    }
}

impl<V> Display for QuadraticAbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "quadratic_abs({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticAbsFunction<V>
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

impl<V> Symbol for QuadraticAbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> QuadraticAbsFunction<V>
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
    /// 把二次输入映射到约束列空间：纯线性输入直接返回自身，真正的二次输入返回桥接列。
    ///
    /// Map the quadratic input into the constraint column space: a purely linear input is
    /// returned as is, while a genuine quadratic input maps to its bridge column.
    fn mapped_input(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>> {
        if let Some(input) = self.bridge.input_linear_polynomial() {
            return Ok(input);
        }
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic abs bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        Ok(Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        ))
    }

    /// 从令牌边界推断非对称分支 Big-M：`bridge ∈ [lower, upper]` 时，正分支行只需覆盖
    /// `max(0, -2 * lower)`，负分支行只需覆盖 `max(0, 2 * upper)`，因此取值跨正负时两条
    /// 分支的松弛可以明显不同。
    ///
    /// Infer the asymmetric branch Big-M pair from token bounds: for `bridge ∈ [lower, upper]`
    /// the positive branch row only has to cover `max(0, -2 * lower)` and the negative branch row
    /// `max(0, 2 * upper)`, so the two relaxations can differ clearly across both signs.
    fn inferred_branch_big_m(&self, tokens: &[Token<V>]) -> Option<AbsBranchBigM> {
        let (lower, upper) = infer_quadratic_bounds_from_tokens(&self.input, tokens)?;
        AbsBranchBigM::from_input_bounds(lower, upper)
    }

    /// 解析本次约束生成使用的分支 Big-M：显式配置优先（并校验），否则从令牌边界推断，
    /// 最后回退到策略取值。
    ///
    /// Resolve the branch Big-M used by this constraint generation: the explicit configuration
    /// wins (and is validated), then token-bound inference applies, and the policy fallback is
    /// the last resort.
    fn resolved_branch_big_m(&self, tokens: Option<&[Token<V>]>) -> Result<AbsBranchBigM> {
        if let Some(explicit) = self.explicit_big_m {
            for (branch, value) in [
                ("positive", explicit.positive_branch),
                ("negative", explicit.negative_branch),
            ] {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError::InvalidConstraint(format!(
                        "quadratic abs `{}` requires a positive finite {branch}-branch Big-M, got {value}",
                        self.id.name
                    ))
                    .into());
                }
            }
            return Ok(explicit);
        }
        Ok(tokens
            .and_then(|tokens| self.inferred_branch_big_m(tokens))
            .unwrap_or_else(AbsBranchBigM::fallback))
    }

    /// 生成线性 ABS 的四条分支行，输入已映射到约束列空间。
    ///
    /// Build the four linear ABS branch rows on the column-space mapped input.
    fn build_abs_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: AbsBranchBigM,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mapped_input = self.mapped_input(symbol_to_index)?;
        let result_index = symbol_to_index
            .get(&(self.inner.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic abs result variable id {}",
                    self.inner.result_variable().id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.inner.side_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic abs side variable id {}",
                    self.inner.side_variable().id().unique_id()
                ))
            })?;
        super::abs::generate_abs_constraints(
            &self.id.name,
            &mapped_input,
            result_index,
            side_index,
            big_m,
            Arc::new(self.clone()),
        )
    }

    /// 生成桥接行的线性部分（当前桥接不写独立线性行，保留调用点以便与即时展开对齐）。
    ///
    /// Build the linear part of the bridge rows (the bridge currently writes no standalone linear
    /// row; the call site is kept so the wrapper mirrors eager expansion).
    fn bridge_linear_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        if self.bridge.has_quadratic_terms() {
            self.bridge.mechanism_constraints(symbol_to_index)
        } else {
            Ok(Vec::new())
        }
    }
}

impl<V> IntermediateSymbol<V> for QuadraticAbsFunction<V>
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

    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        // 输入域有限时把 `0 <= y <= max(|lower|, |upper|)` 传播到结果列，只收紧不放宽。
        // Propagate `0 <= y <= max(|lower|, |upper|)` to the result column when the input domain
        // is finite; bounds are only tightened, never widened.
        let Some((lower, upper)) = infer_quadratic_bounds_from_tokens(&self.input, tokens) else {
            return Ok(());
        };
        let abs_bound = lower.abs().max(upper.abs());
        if !abs_bound.is_finite() {
            return Ok(());
        }
        for token in auxiliary.iter_mut() {
            if token.id() != self.inner.result_variable().id() {
                continue;
            }
            return tighten_token_range(token, 0.0, abs_bound);
        }
        Ok(())
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge_linear_constraints(symbol_to_index)?;
        let big_m = self.resolved_branch_big_m(None)?;
        constraints.extend(self.build_abs_constraints(symbol_to_index, big_m)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge_linear_constraints(symbol_to_index)?;
        let big_m = self.resolved_branch_big_m(Some(tokens))?;
        constraints.extend(self.build_abs_constraints(symbol_to_index, big_m)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if !self.bridge.has_quadratic_terms() {
            return Ok(Vec::new());
        }
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic abs bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let rows = self.bridge.quadratic_mechanism_constraints(symbol_to_index)?;
        // 组合入口显式校验次数不变式：桥接列必须以一次项进入桥接等式，否则代入桥接等式后该项
        // 次数会升到三次或更高，二次模型无法表达。
        // The composition entry point explicitly checks the degree invariant: the bridge column
        // must enter the bridge equality as a linear term, otherwise substituting the bridge
        // equality raises the term to degree 3 or higher, which a quadratic model cannot express.
        for row in &rows {
            guard_quadratic_composition_degree(
                &row.inequality.polynomial,
                &HashSet::from([bridge_index]),
                &self.id.name,
            )?;
        }
        Ok(rows)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let value = to_f64(&evaluate_quadratic_from_values(&self.input, values)?)?;
        from_f64(value.abs())
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("quadratic_abs({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticAbsFunction<V>
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
        // 只有真正的二次输入才注册桥接列；纯线性输入不产生额外 helper 列。
        // Only a genuine quadratic input registers the bridge column; a purely linear input adds
        // no helper column at all.
        if self.bridge.has_quadratic_terms() {
            self.bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(value.abs())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticAbsFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl<V> QuadraticFunctionSymbol<V> for QuadraticAbsFunction<V>
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

/// 二次输入的 Sigmoid 函数符号 / Quadratic-input sigmoid function symbol
#[derive(Debug, Clone)]
pub struct QuadraticSigmoidFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: SigmoidFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 创建新的二次输入 Sigmoid 函数 / Create a new quadratic-input sigmoid function
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1601),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SigmoidFunction::new(id, name, bridge_input);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用指定精度创建 / Create with specified precision
    pub fn with_precision(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        precision: SigmoidPrecision,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1601),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SigmoidFunction::with_precision(id, name, bridge_input, precision);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticSigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qsigmoid({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSigmoidFunction<V>
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

impl<V> Symbol for QuadraticSigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSigmoidFunction<V>
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic sigmoid bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qsigmoid({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSigmoidFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(SigmoidFunction::<V>::sigmoid(input))
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSigmoidFunction<V>
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
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

macro_rules! impl_quadratic_function_symbol {
    ($($ty:ident),+ $(,)?) => {
        $(
            impl<V> QuadraticFunctionSymbol<V> for $ty<V>
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
        )+
    };
}

impl_quadratic_function_symbol!(
    QuadraticBinaryzationFunction,
    QuadraticInequalityFunction,
    QuadraticRoundingFunction,
    QuadraticModFunction,
    QuadraticMaxFunction,
    QuadraticSlackFunction,
    QuadraticSlackRangeFunction,
    QuadraticMaskingFunction,
    QuadraticSinFunction,
    QuadraticCosFunction,
    QuadraticUnivariateLinearPiecewiseFunction,
    QuadraticBivariateLinearPiecewiseFunction,
    QuadraticPositivePartFunction,
    QuadraticSigmoidFunction,
);

#[cfg(test)]
mod tests {
    use super::super::{
        QuadraticInStepRangeFunction, QuadraticMaskingRangeFunction, QuadraticMinFunction,
    };
    use super::*;
    use crate::symbol::flatten::QuadraticMonomial;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableRange};

    fn token_index_map<V>(tokens: &[Token<V>]) -> HashMap<usize, usize>
    where
        V: Clone + Debug + Send + Sync + 'static,
    {
        tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
            .collect()
    }

    fn coefficient_for_index(constraint: &LinearConstraint<f64>, index: usize) -> f64 {
        *constraint
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == index)
            .expect("expected monomial should exist")
            .coefficient()
    }

    #[test]
    fn quadratic_linear_function_generates_quadratic_constraint() {
        let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0);
        let bridge = QuadraticLinearFunction::new(20001, "bridge", input);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(
            bridge.result_variable().id().unique_id() as usize,
            bridge.result_variable().index(),
        );
        let constraints = bridge
            .quadratic_mechanism_constraints(&symbol_to_index)
            .unwrap();
        assert_eq!(constraints.len(), 1);
    }

    #[test]
    fn quadratic_binaryzation_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");

        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0
        let qbin = QuadraticBinaryzationFunction::with_big_m(20011, "qbin", quad, 10.0);
        assert_eq!(qbin.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_binaryzation_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let qbin: QuadraticBinaryzationFunction<f64> = QuadraticBinaryzationFunction::new(
            20012,
            "qbin_bound",
            quad,
            1.0,
            100.0,
            BinaryzationMethod::BigM,
        );

        let mut aux_tokens = Vec::new();
        qbin.register_tokens(&mut aux_tokens)
            .expect("quadratic binaryzation tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qbin
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic binaryzation constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qbin_bound_bin_ub")
            .expect("upper binaryzation constraint should exist");
        let y_index = *symbol_to_index
            .get(&(qbin.result_variable().id().unique_id() as usize))
            .expect("binary result index should exist");

        assert!((coefficient_for_index(upper, y_index) + 3.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_inequality_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0); // 4.0
        let ineq = QuadraticInequalityFunction::less_equal(20021, "qineq", quad, 5.0, 10.0);
        assert_eq!(ineq.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_inequality_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let ineq: QuadraticInequalityFunction<f64> =
            QuadraticInequalityFunction::less_equal(20022, "qineq_bound", quad, 1.0, 100.0);

        let mut aux_tokens = Vec::new();
        ineq.register_tokens(&mut aux_tokens)
            .expect("quadratic inequality tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = ineq
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic inequality constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qineq_bound_ineq_ub")
            .expect("upper inequality constraint should exist");
        let y_index = *symbol_to_index
            .get(&(ineq.result_variable().id().unique_id() as usize))
            .expect("inequality result index should exist");

        assert!((upper.inequality.rhs - 3.0).abs() <= 1e-9);
        assert!((coefficient_for_index(upper, y_index) - 3.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_rounding_and_mod_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0

        let qround = QuadraticRoundingFunction::floor(20031, "qround", quad.clone());
        assert_eq!(qround.calculate_value(&tokens, false), Some(3.0));

        let qmod = QuadraticModFunction::new(20032, "qmod", quad, 2.0);
        assert_eq!(qmod.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_rounding_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let qtrunc: QuadraticRoundingFunction<f64> =
            QuadraticRoundingFunction::trunc(20033, "qtrunc_bound", quad);

        let mut aux_tokens = Vec::new();
        qtrunc
            .register_tokens(&mut aux_tokens)
            .expect("quadratic rounding tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let sign_index = *symbol_to_index
            .get(
                &(qtrunc
                    .inner
                    .sign_variable()
                    .expect("trunc rounding sign variable should exist")
                    .id()
                    .unique_id() as usize),
            )
            .expect("rounding sign index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qtrunc
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic rounding constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "qtrunc_bound_sign_lb")
            .expect("rounding sign lower constraint should exist");

        assert!((coefficient_for_index(lower, sign_index) + 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_min_max_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let quad1 = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0); // 4.0
        let quad2 = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0

        let qmin =
            QuadraticMinFunction::new(20041, "qmin", vec![quad1.clone(), quad2.clone()], true);
        let qmax = QuadraticMaxFunction::new(20042, "qmax", vec![quad1, quad2], true);
        assert_eq!(qmin.calculate_value(&tokens, false), Some(3.0));
        assert_eq!(qmax.calculate_value(&tokens, false), Some(4.0));
    }

    #[test]
    fn quadratic_min_infers_big_m_from_original_candidate_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let constant = Quadratic::new(vec![], 2.0);
        let qmin: QuadraticMinFunction<f64> =
            QuadraticMinFunction::new(20043, "qmin_bound", vec![quad, constant], true);

        let mut aux_tokens = Vec::new();
        qmin.register_tokens(&mut aux_tokens)
            .expect("quadratic min tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let selector_id = aux_tokens
            .iter()
            .find(|token| {
                token.id() != qmin.bridges[0].result_variable().id()
                    && token.id() != qmin.bridges[1].result_variable().id()
                    && token.id() != qmin.result_variable().id()
            })
            .expect("min selector token should exist")
            .id()
            .unique_id() as usize;
        let selector_index = *symbol_to_index
            .get(&selector_id)
            .expect("min selector index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qmin
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic min constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "qmin_bound_min_lb_0")
            .expect("min lower constraint should exist");

        assert!((coefficient_for_index(lower, selector_index) + 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_max_infers_big_m_from_original_candidate_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let constant = Quadratic::new(vec![], 2.0);
        let qmax: QuadraticMaxFunction<f64> =
            QuadraticMaxFunction::new(20044, "qmax_bound", vec![quad, constant], true);

        let mut aux_tokens = Vec::new();
        qmax.register_tokens(&mut aux_tokens)
            .expect("quadratic max tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let selector_id = aux_tokens
            .iter()
            .find(|token| {
                token.id() != qmax.bridges[0].result_variable().id()
                    && token.id() != qmax.bridges[1].result_variable().id()
                    && token.id() != qmax.result_variable().id()
            })
            .expect("max selector token should exist")
            .id()
            .unique_id() as usize;
        let selector_index = *symbol_to_index
            .get(&selector_id)
            .expect("max selector index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qmax
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic max constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qmax_bound_max_ub_0")
            .expect("max upper constraint should exist");

        assert!((coefficient_for_index(upper, selector_index) - 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_slack_and_slack_range_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let left = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0
        let right = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0); // 4.0
        let qslack = QuadraticSlackFunction::new(20051, "qslack", left.clone(), right);
        assert_eq!(qslack.calculate_value(&tokens, false), Some(1.0));

        let qslack_range = QuadraticSlackRangeFunction::new(20052, "qslack_range", left, 1.0, 2.0);
        assert_eq!(qslack_range.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_slack_infers_big_m_from_original_difference_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let left = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let right = Quadratic::new(vec![], 1.0);
        let qslack: QuadraticSlackFunction<f64> =
            QuadraticSlackFunction::new(20053, "qslack_bound", left, right);

        let mut aux_tokens = Vec::new();
        qslack
            .register_tokens(&mut aux_tokens)
            .expect("quadratic slack tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let side_id = aux_tokens
            .iter()
            .find(|token| {
                token.id() != qslack.left_bridge.result_variable().id()
                    && token.id() != qslack.right_bridge.result_variable().id()
                    && token.id() != qslack.result_variable().id()
            })
            .expect("slack side token should exist")
            .id()
            .unique_id() as usize;
        let side_index = *symbol_to_index
            .get(&side_id)
            .expect("slack side index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qslack
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic slack constraints should be generated");
        let branch = constraints
            .iter()
            .find(|constraint| constraint.name == "qslack_bound_slack_branch_pos")
            .expect("positive slack branch constraint should exist");

        assert!((branch.inequality.rhs - 3.0).abs() <= 1e-9);
        assert!((coefficient_for_index(branch, side_index) - 3.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_trigonometric_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(0.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qsin = QuadraticSinFunction::new(20061, "qsin", quad.clone());
        let qcos = QuadraticCosFunction::new(20062, "qcos", quad);
        assert_eq!(qsin.calculate_value(&tokens, false), Some(0.0));
        assert_eq!(qcos.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_univariate_piecewise_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(1.5);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let points = vec![Point2::new(0.0, 0.0), Point2::new(2.0, 4.0)];
        let qulp = QuadraticUnivariateLinearPiecewiseFunction::new(20071, "qulp", quad, points);
        assert_eq!(qulp.calculate_value(&tokens, false), Some(3.0));
    }

    #[test]
    fn quadratic_masking_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mask = BinaryVariableItem::create(VariableId::standalone(2), "m");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.5);
        tokens.add_token(tx);
        let tm = Token::from_generic(mask.clone(), 2);
        tm.set_result(1.0);
        tokens.add_token(tm);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qmask = QuadraticMaskingFunction::new(20081, "qmask", quad, mask);
        assert_eq!(qmask.calculate_value(&tokens, false), Some(2.5));
    }

    #[test]
    fn quadratic_masking_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let mask = BinaryVariableItem::create(VariableId::standalone(1), "m");
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let qmask: QuadraticMaskingFunction<f64> =
            QuadraticMaskingFunction::with_big_m(20082, "qmask_bound", quad, mask.clone(), 100.0);

        let mut aux_tokens = Vec::new();
        qmask
            .register_tokens(&mut aux_tokens)
            .expect("quadratic masking tokens should be registered");
        aux_tokens.push(Token::from_generic(mask.clone(), 1));
        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qmask
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic masking constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qmask_bound_masking_eq_ub")
            .expect("upper masking constraint should exist");

        assert!((upper.inequality.rhs - 4.0).abs() <= 1e-9);
        let mask_index = *symbol_to_index
            .get(&(mask.id().unique_id() as usize))
            .expect("mask index should exist");

        assert!((coefficient_for_index(upper, mask_index) - 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_masking_range_calculate_value() {
        let mask = BinaryVariableItem::create(VariableId::standalone(0), "m");
        let mut tokens = VecTokenList::<f64>::new();
        let tm = Token::from_generic(mask.clone(), 0);
        tm.set_result(1.0);
        tokens.add_token(tm);

        let qmask = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 1, 1)], 0.0);
        let qmask_range =
            QuadraticMaskingRangeFunction::with_big_m(20091, "qmr", qmask, mask.clone(), 10.0);
        let tx = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(1), "x"),
            1,
        );
        tx.set_result(-2.5);
        tokens.add_token(tx);

        assert_eq!(qmask_range.calculate_value(&tokens, false), Some(6.25));

        let mut tokens_off = VecTokenList::<f64>::new();
        let tm0 = Token::from_generic(mask.clone(), 0);
        tm0.set_result(0.0);
        tokens_off.add_token(tm0);
        let tx_off = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(1), "x"),
            1,
        );
        tx_off.set_result(-2.5);
        tokens_off.add_token(tx_off);
        assert_eq!(qmask_range.calculate_value(&tokens_off, false), Some(0.0));
    }

    #[test]
    fn quadratic_positive_part_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-1.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let positive_part = QuadraticPositivePartFunction::new(20101, "qpositive_part", quad);
        assert_eq!(positive_part.calculate_value(&tokens, false), Some(0.0));

        let mut tokens_pos = VecTokenList::<f64>::new();
        let tx2 = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(0), "x"),
            0,
        );
        tx2.set_result(2.0);
        tokens_pos.add_token(tx2);
        assert_eq!(positive_part.calculate_value(&tokens_pos, false), Some(2.0));
    }

    #[test]
    fn quadratic_positive_part_uses_a_sign_known_equality_when_the_input_is_non_negative() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let positive_part: QuadraticPositivePartFunction<f64> =
            QuadraticPositivePartFunction::new(20102, "qpositive_part_bound", quad);

        // 注册阶段即可判定输入非负，因此只保留桥接列与结果列，不再产生选择器。
        // Registration already proves a non-negative input, so only the bridge and the
        // result column remain and no selector is created.
        let mut aux_tokens = Vec::new();
        positive_part
            .register_auxiliary_tokens_with_context(&mut aux_tokens, std::slice::from_ref(&{
                Token::from_generic(x.clone(), 0)
            }))
            .expect("quadratic positive-part tokens should be registered");
        assert_eq!(aux_tokens.len(), 2, "bridge plus result column");

        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = positive_part
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic positive-part constraints should be generated");
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].name, "qpositive_part_bound_sign_known");

        // `result - bridge = 0`：非负输入下正部等于输入本身。
        // `result - bridge = 0`: with a non-negative input the positive part is the input.
        let result_index = *symbol_to_index
            .get(&(positive_part.result_variable().id().unique_id() as usize))
            .expect("result index should exist");
        let bridge_index = *symbol_to_index
            .get(&(positive_part.bridge.result_variable().id().unique_id() as usize))
            .expect("bridge index should exist");
        assert!((coefficient_for_index(&constraints[0], result_index) - 1.0).abs() <= 1e-9);
        assert!((coefficient_for_index(&constraints[0], bridge_index) + 1.0).abs() <= 1e-9);
        assert_eq!(constraints[0].inequality.relation, ConstraintRelation::Equal);
    }

    #[test]
    fn quadratic_positive_part_uses_a_zero_equality_when_the_input_is_non_positive() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-3.0, -1.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let positive_part: QuadraticPositivePartFunction<f64> =
            QuadraticPositivePartFunction::new(20103, "qpositive_part_negative", quad);

        let registered = vec![Token::from_generic(x, 0)];
        let mut aux_tokens = Vec::new();
        positive_part
            .register_auxiliary_tokens_with_context(&mut aux_tokens, &registered)
            .expect("quadratic positive-part tokens should be registered");
        assert_eq!(aux_tokens.len(), 1, "result column only");

        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = registered;
        tokens.extend(aux_tokens);

        let constraints = positive_part
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic positive-part constraints should be generated");
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].name, "qpositive_part_negative_sign_known");
        assert_eq!(constraints[0].inequality.relation, ConstraintRelation::Equal);
        assert!(constraints[0]
            .inequality
            .polynomial
            .monomials()
            .iter()
            .all(|monomial| monomial.var_index() != 0));
    }

    #[test]
    fn quadratic_positive_part_keeps_selectors_when_the_input_sign_is_unknown() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-1.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], -1.0);
        let positive_part: QuadraticPositivePartFunction<f64> =
            QuadraticPositivePartFunction::new(20104, "qpositive_part_mixed", quad);

        let registered = vec![Token::from_generic(x, 0)];
        let mut aux_tokens = Vec::new();
        positive_part
            .register_auxiliary_tokens_with_context(&mut aux_tokens, &registered)
            .expect("quadratic positive-part tokens should be registered");
        assert_eq!(aux_tokens.len(), 4, "bridge plus result and two selectors");

        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = registered;
        tokens.extend(aux_tokens);

        let constraints = positive_part
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic positive-part constraints should be generated");
        assert!(constraints
            .iter()
            .any(|constraint| constraint.name == "qpositive_part_mixed_max_ub_0"));
        assert!(!constraints
            .iter()
            .any(|constraint| constraint.name == "qpositive_part_mixed_sign_known"));
    }

    #[test]
    fn quadratic_in_step_range_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qstep = QuadraticInStepRangeFunction::new(20111, "qstep", quad, 0.0, 4.0);
        assert_eq!(qstep.calculate_value(&tokens, false), Some(2.0));

        let mut tokens_off_step = VecTokenList::<f64>::new();
        let tx2 = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(0), "x"),
            0,
        );
        tx2.set_result(5.0);
        tokens_off_step.add_token(tx2);
        assert_eq!(qstep.calculate_value(&tokens_off_step, false), Some(0.0));
    }

    #[test]
    fn quadratic_sigmoid_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(0.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qsigmoid = QuadraticSigmoidFunction::new(20121, "qsigmoid", quad);
        assert_eq!(qsigmoid.calculate_value(&tokens, false), Some(0.5));
    }
}
