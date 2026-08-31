//! 范围松弛函数符号 / Slack-range function symbol
//!
//! - `SlackRangeFunction`：到闭区间的距离 / distance to a closed interval

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::Result;
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::ContinuousVariableItem;
use super::super::{

    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::max::MaxFunction;

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

/// 范围松弛函数 / Slack-range function
///
/// 到闭区间 `[lower, upper]` 的距离：`max(lower - x, x - upper, 0)`。
/// Distance to a closed interval `[lower, upper]`: `max(lower - x, x - upper, 0)`.
///
/// 此实现内部复用精确最大值约束。
/// This implementation reuses exact-max constraints internally.
#[derive(Debug, Clone)]
pub struct SlackRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 输入多项式 / Input polynomial
    input: Linear<V>,
    /// 下界 / Lower bound
    lower: V,
    /// 上界 / Upper bound
    upper: V,
    /// 内部最大值函数 / Inner maximum function
    inner: MaxFunction<V>,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建新的范围松弛函数 / Create a new slack-range function
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

    /// 设置声明的依赖 ID / Set declared dependency IDs
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

    /// 获取输入多项式 / Get the input polynomial
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取下界 / Get the lower bound
    pub fn lower_bound(&self) -> &V {
        &self.lower
    }

    /// 获取上界 / Get the upper bound
    pub fn upper_bound(&self) -> &V {
        &self.upper
    }

    /// 获取结果变量 / Get the result variable
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
