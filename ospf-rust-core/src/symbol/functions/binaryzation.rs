//! 二值化函数符号 / Binaryzation function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::infer_linear_shifted_abs_bound_from_tokens;
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

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

/// 二值化约束的编码方式 / Encoding method for binaryization constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryzationMethod {
    /// 使用 Big-M 约束 / Use Big-M constraints.
    BigM,
    /// 使用阈值约束 / Use threshold constraints.
    Threshold,
    /// Request solver-native indicator modeling when available.
    ///
    /// At core mechanism layer, this is currently encoded with Big-M constraints.
    Indicator,
    /// Request solver-native SOS1 modeling when available.
    ///
    /// At core mechanism layer, this is currently encoded with Big-M constraints.
    SOS1,
}

impl BinaryzationMethod {
    /// Returns the mechanism-layer equivalent method used by current core encoding.
    pub const fn mechanism_equivalent(self) -> Self {
        match self {
            Self::Threshold => Self::Threshold,
            Self::BigM | Self::Indicator | Self::SOS1 => Self::BigM,
        }
    }

    /// Returns whether caller requested a native solver feature.
    pub const fn requests_native_solver_feature(self) -> bool {
        matches!(self, Self::Indicator | Self::SOS1)
    }
}

/// Converts a value expression to binary semantics.
#[derive(Debug, Clone)]
pub struct BinaryzationFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    result_var: BinaryVariableItem,
    threshold: V,
    big_m: V,
    method: BinaryzationMethod,
    declared_dependency_ids: Vec<u64>,
}

impl<V> BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建二值化函数 / Create a binaryization function.
    pub fn new(
        id: u64,
        name: &str,
        input: Linear<V>,
        threshold: V,
        big_m: V,
        method: BinaryzationMethod,
    ) -> Self {
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_bin", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            threshold,
            big_m,
            method,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建二值化函数。
    /// Create a binaryzation function with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        input: Linear<V>,
        threshold: V,
        big_m: V,
        method: BinaryzationMethod,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            input,
            threshold,
            big_m,
            method,
        )
    }

    /// 使用自动 ID 与自动名称创建二值化函数。
    /// Create a binaryzation function with an auto id and auto-generated name.
    pub fn auto(input: Linear<V>, threshold: V, big_m: V, method: BinaryzationMethod) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("binaryzation", id);
        Self::new(id, &name, input, threshold, big_m, method)
    }

    /// 使用 Big-M 语义创建二值化函数 / Create a binaryization function with Big-M semantics.
    pub fn with_big_m(id: u64, name: &str, input: Linear<V>, big_m: V) -> Self {
        Self::new(
            id,
            name,
            input,
            from_f64(0.0).expect("convert 0.0"),
            big_m,
            BinaryzationMethod::BigM,
        )
    }

    /// 使用自动 ID 与调用方提供的名称创建 Big-M 二值化函数。
    /// Create a Big-M binaryzation function with an auto id and caller-provided name.
    pub fn named_big_m(name: impl AsRef<str>, input: Linear<V>, big_m: V) -> Self {
        Self::with_big_m(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            input,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建 Big-M 二值化函数。
    /// Create a Big-M binaryzation function with an auto id and auto-generated name.
    pub fn auto_big_m(input: Linear<V>, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("binaryzation", id);
        Self::with_big_m(id, &name, input, big_m)
    }

    /// 使用阈值语义创建二值化函数 / Create a threshold binaryization function.
    pub fn with_threshold(id: u64, name: &str, input: Linear<V>, threshold: V) -> Self {
        Self::new(
            id,
            name,
            input,
            threshold,
            from_f64(DEFAULT_BIG_M).expect("convert default big-M"),
            BinaryzationMethod::Threshold,
        )
    }

    /// 使用自动 ID 与调用方提供的名称创建阈值二值化函数。
    /// Create a threshold binaryzation function with an auto id and caller-provided name.
    pub fn named_threshold(name: impl AsRef<str>, input: Linear<V>, threshold: V) -> Self {
        Self::with_threshold(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            input,
            threshold,
        )
    }

    /// 使用自动 ID 与自动名称创建阈值二值化函数。
    /// Create a threshold binaryzation function with an auto id and auto-generated name.
    pub fn auto_threshold(input: Linear<V>, threshold: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("binaryzation", id);
        Self::with_threshold(id, &name, input, threshold)
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.input = input;
        cloned
    }

    pub(crate) fn with_big_m_value(&self, big_m: V) -> Self {
        let mut cloned = self.clone();
        cloned.big_m = big_m;
        cloned
    }

    /// 获取二值结果变量 / Get the binary result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取输入多项式 / Get the input polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取阈值 / Get the threshold.
    pub fn threshold(&self) -> &V {
        &self.threshold
    }

    /// 获取 Big-M 值 / Get the Big-M value.
    pub fn big_m(&self) -> &V {
        &self.big_m
    }

    /// 获取二值化方法 / Get the binaryization method.
    pub fn method(&self) -> BinaryzationMethod {
        self.method
    }
}

impl<V> BinaryzationFunction<V>
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
                "binaryzation `{}` big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "binaryzation `{}` requires positive finite big-M for mechanism constraint injection",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_linear_shifted_abs_bound_from_tokens(&self.input, &self.threshold, tokens)
            .map(|big_m| big_m.max(MIN_BIG_M))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_symbol_id = self.result_var.id().unique_id() as usize;
        let result_index = symbol_to_index
            .get(&result_symbol_id)
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "binary result variable id {}",
                    result_symbol_id
                ))
            })?;

        let threshold = to_f64(&self.threshold).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "binaryzation `{}` threshold cannot be converted to f64",
                self.id.name
            ))
        })?;

        let strict_eps = f64::EPSILON * 16.0;
        let (lower_rhs, upper_rhs) = match self.method.mechanism_equivalent() {
            BinaryzationMethod::Threshold => (-big_m, -strict_eps),
            BinaryzationMethod::BigM => (strict_eps - big_m, 0.0),
            BinaryzationMethod::Indicator | BinaryzationMethod::SOS1 => unreachable!(),
        };

        let mut lower_monomials = Vec::with_capacity(self.input.monomials().len() + 1);
        let mut upper_monomials = Vec::with_capacity(self.input.monomials().len() + 1);
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "binaryzation `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let converted_coefficient =
                convert_f64_to_v::<V>(coefficient, "binaryzation input coefficient")?;
            let input_index = monomial.var_index();
            lower_monomials.push(LinearMonomial::new(
                converted_coefficient.clone(),
                input_index,
            ));
            upper_monomials.push(LinearMonomial::new(converted_coefficient, input_index));
        }

        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "binaryzation `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let shifted_constant = input_constant - threshold;

        let negative_big_m = -big_m;
        let y_coefficient = convert_f64_to_v::<V>(negative_big_m, "binaryzation y coefficient")?;
        lower_monomials.push(LinearMonomial::new(y_coefficient.clone(), result_index));
        upper_monomials.push(LinearMonomial::new(y_coefficient, result_index));

        let lower_polynomial = Linear::new(
            lower_monomials,
            convert_f64_to_v::<V>(shifted_constant, "binaryzation lower constant")?,
        );
        let upper_polynomial = Linear::new(
            upper_monomials,
            convert_f64_to_v::<V>(shifted_constant, "binaryzation upper constant")?,
        );

        let lower = LinearConstraint::from_symbol(
            LinearInequality::new(
                lower_polynomial,
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(lower_rhs, "binaryzation lower rhs")?,
            ),
            &format!("{}_bin_lb", self.id.name),
            Arc::new(self.clone()),
        );
        let upper = LinearConstraint::from_symbol(
            LinearInequality::new(
                upper_polynomial,
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(upper_rhs, "binaryzation upper rhs")?,
            ),
            &format!("{}_bin_ub", self.id.name),
            Arc::new(self.clone()),
        );

        Ok(vec![lower, upper])
    }
}

impl<V> Display for BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "binary({})", self.id.name)
    }
}

impl<V> DynSymbol for BinaryzationFunction<V>
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

impl<V> Symbol for BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for BinaryzationFunction<V>
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
        format!("binary({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 必须在结构创建时固定：优先用令牌边界推断值，取不到时用配置值；两者都不可用时
        // 不提供结构，让即时展开给出配置错误，而不是把错误推迟到物化阶段。
        // The Big-M must be fixed at creation time: the token-inferred value first, the configured
        // value second; when neither is available no structure is offered so eager expansion
        // surfaces the configuration error instead of deferring it to materialization.
        let big_m = match self.infer_big_m_from_tokens(tokens) {
            Some(inferred) => inferred,
            None => self.configured_big_m().ok()?,
        };
        Some(Arc::new(BinaryzationStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_m,
        )))
    }
}

/// 二值化的求解器无关结构描述 / Solver-neutral structure description of binaryzation
///
/// 与 ABS/极值采用同一模式：持有产生它的符号（`Arc`）与创建时固定的 Big-M，物化时回调手写路径
/// 的同一个公式生成器并传入同一个 M，因此延迟物化与 EAGER 展开逐行一致（含 M 取值）。二值化
/// 没有辅助指示列，结构只涉及结果列。
///
/// Follows the same pattern as ABS and the extrema: the structure holds the symbol that produced it
/// (an `Arc`) together with the Big-M fixed at creation time and materializes through the very same
/// formula generator as the handwritten eager path with that same M, so deferred materialization
/// matches eager expansion row by row, including the M value. Binaryzation has no auxiliary
/// indicator column, so the structure only involves the result column.
#[derive(Debug)]
pub struct BinaryzationStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<BinaryzationFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 创建时固定的 Big-M / Big-M fixed at creation time
    big_m: f64,
}

impl<V> BinaryzationStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<BinaryzationFunction<V>>, big_m: f64) -> Self {
        let result = symbol.result_variable().id();
        Self {
            name: name.into(),
            symbol,
            result,
            big_m,
        }
    }

    /// 获取函数名称 / Get the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取结果列 / Get the result column.
    pub fn result(&self) -> &crate::variable::VariableId {
        &self.result
    }

    /// 获取固定的 Big-M / Get the fixed Big-M.
    pub fn big_m(&self) -> f64 {
        self.big_m
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for BinaryzationStructure<V>
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
    fn function_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn usage_binding(&self) -> Option<crate::model::intermediate::StructureUsageBinding> {
        // 二值化没有辅助列，只上报结果列。
        // Binaryzation has no helper columns, so only the result column is reported.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            Vec::new(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        // Big-M 进入指纹：模型边界变化导致 M 变化时，旧记录必须失效。
        // The Big-M is part of the fingerprint: when model bounds change M, old records must fail.
        Some(format!(
            "binaryzation|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            crate::model::intermediate::fingerprint_float(self.big_m)
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一个 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M so both paths stay row-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, self.big_m)
    }
}

impl<V> FunctionSymbol<V> for BinaryzationFunction<V>
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
        let input = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let threshold = to_f64(&self.threshold)?;
        let eps = f64::EPSILON * 16.0;
        let is_true = match self.method.mechanism_equivalent() {
            BinaryzationMethod::Threshold => input + eps >= threshold,
            BinaryzationMethod::BigM => input > threshold + eps,
            BinaryzationMethod::Indicator | BinaryzationMethod::SOS1 => unreachable!(),
        };
        from_f64(if is_true { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for BinaryzationFunction<V>
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
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn binaryzation_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(30_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: BinaryzationFunction<f64> = BinaryzationFunction::with_big_m(
            4000,
            "bin_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            100.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("binary constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "bin_bound_bin_lb")
            .expect("lower binary constraint should exist");
        let y_term = lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("binary y term should exist");

        // 2x + 1 with x in [-2, 3] => range [-3, 7], threshold=0 => M = 7.
        assert!((*y_term.coefficient() + 7.0).abs() <= 1e-9);
        assert!((lower.inequality.rhs + 7.0).abs() <= 1e-9);
    }

    #[test]
    fn binaryzation_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(30_100),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: BinaryzationFunction<f64> = BinaryzationFunction::with_big_m(
            4001,
            "bin_deferred",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            100.0,
        );
        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
        ];

        let structure = f
            .deferred_structure_with_tokens(&tokens)
            .expect("binaryzation should expose a deferred structure");
        assert_eq!(structure.function_name(), "bin_deferred");
        let binding = structure
            .usage_binding()
            .expect("binaryzation structure should expose a usage binding");
        assert_eq!(binding.result, f.result_variable().id());
        assert!(binding.helpers.is_empty());
        assert!(structure.fingerprint().is_some());

        let eager = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager binary constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("binaryzation structure should materialize");
        assert!(!eager.is_empty());
        assert_eq!(eager.len(), deferred.len());
        for (eager_row, deferred_row) in eager.iter().zip(deferred.iter()) {
            assert_eq!(eager_row.name, deferred_row.name);
            assert_eq!(eager_row.inequality.relation, deferred_row.inequality.relation);
            assert_eq!(eager_row.inequality.rhs, deferred_row.inequality.rhs);
            assert_eq!(
                eager_row.inequality.polynomial.constant_term(),
                deferred_row.inequality.polynomial.constant_term()
            );
        }

        // 没有令牌边界时退回配置 Big-M，两条路径仍一致。
        // Without token bounds the configured Big-M is used and both paths still agree.
        let no_tokens_structure = f
            .deferred_structure_with_tokens(&[])
            .expect("binaryzation should fall back to the configured big-M");
        let eager_default = <BinaryzationFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
            &f,
            &symbol_to_index,
        )
        .expect("eager binary constraints should be generated");
        let deferred_default = no_tokens_structure
            .materialize(&symbol_to_index)
            .expect("binaryzation structure should materialize");
        assert_eq!(eager_default.len(), deferred_default.len());
        assert_eq!(eager_default[0].name, deferred_default[0].name);
    }

    #[test]
    fn binaryzation_function_falls_back_to_configured_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(30_010), "x");
        let f: BinaryzationFunction<f64> = BinaryzationFunction::with_big_m(
            4001,
            "bin_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            13.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("binary constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "bin_default_bin_lb")
            .expect("lower binary constraint should exist");
        let y_term = lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("binary y term should exist");

        assert!((*y_term.coefficient() + 13.0).abs() <= 1e-9);
        assert!((lower.inequality.rhs + 13.0).abs() <= 1e-9);
    }

    #[test]
    fn binaryzation_native_methods_are_currently_big_m_equivalent() {
        assert_eq!(
            BinaryzationMethod::Indicator.mechanism_equivalent(),
            BinaryzationMethod::BigM
        );
        assert_eq!(
            BinaryzationMethod::SOS1.mechanism_equivalent(),
            BinaryzationMethod::BigM
        );
        assert!(BinaryzationMethod::Indicator.requests_native_solver_feature());
        assert!(BinaryzationMethod::SOS1.requests_native_solver_feature());
        assert!(!BinaryzationMethod::BigM.requests_native_solver_feature());
        assert!(!BinaryzationMethod::Threshold.requests_native_solver_feature());
    }
}
