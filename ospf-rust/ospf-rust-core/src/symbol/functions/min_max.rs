//! MaxMin/MinMax 函数符号 / MaxMin/MinMax function symbols

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::max::{MaxFunction, MinFunction};
use crate::error::Result;
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::ContinuousVariableItem;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

/// 多项式集合的精确最小值（对应 Kotlin `MaxMinFunction`）。
/// Exact minimum of a polynomial set (Kotlin `MaxMinFunction`).
#[derive(Debug, Clone)]
pub struct MaxMinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    inner: MinFunction<V>,
}

impl<V> MaxMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的 MaxMin 函数 / Create a new MaxMin function
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self {
        Self {
            inner: MinFunction::new(id, name, polynomials, true),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.inner = self.inner.with_declared_dependencies(dependency_ids);
        self
    }

    /// 获取结果变量 / Get the result variable
    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 获取输入多项式列表 / Get the input polynomials
    /// 获取输入多项式列表 / Get the input polynomials
    pub fn polynomials(&self) -> &[Linear<V>] {
        self.inner.polynomials()
    }
}

impl<V> Display for MaxMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "maxmin({})", self.inner.name())
    }
}

impl<V> DynSymbol for MaxMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn display_name(&self) -> &str {
        self.inner.display_name()
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        self.inner.dyn_id()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<V> Symbol for MaxMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.inner.id()
    }
}

impl<V> IntermediateSymbol<V> for MaxMinFunction<V>
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
        <MinFunction<V> as IntermediateSymbol<V>>::category(&self.inner)
    }

    fn cached(&self) -> bool {
        <MinFunction<V> as IntermediateSymbol<V>>::cached(&self.inner)
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        <MinFunction<V> as IntermediateSymbol<V>>::dependencies(&self.inner)
    }

    fn declared_dependency_ids(&self) -> Vec<u64> {
        <MinFunction<V> as IntermediateSymbol<V>>::declared_dependency_ids(&self.inner)
    }

    fn flush(&self, force: bool) {
        <MinFunction<V> as IntermediateSymbol<V>>::flush(&self.inner, force)
    }

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        <MinFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
            &self.inner,
            symbol_to_index,
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
        <MinFunction<V> as IntermediateSymbol<V>>::prepare(&self.inner, values)
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("maxmin({})", self.inner.name())
    }
}

impl<V> FunctionSymbol<V> for MaxMinFunction<V>
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
        <MinFunction<V> as FunctionSymbol<V>>::register_tokens(&self.inner, tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        <MinFunction<V> as FunctionSymbol<V>>::calculate_value(
            &self.inner,
            token_table,
            zero_if_none,
        )
    }
}

impl<V> LinearIntermediateSymbol<V> for MaxMinFunction<V>
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
        <MinFunction<V> as LinearIntermediateSymbol<V>>::to_linear_polynomial(&self.inner)
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        <MinFunction<V> as LinearIntermediateSymbol<V>>::to_quadratic_polynomial(&self.inner)
    }
}

/// 多项式集合的精确最大值（对应 Kotlin `MinMaxFunction`）。
/// Exact maximum of a polynomial set (Kotlin `MinMaxFunction`).
#[derive(Debug, Clone)]
pub struct MinMaxFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    inner: MaxFunction<V>,
}

impl<V> MinMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的 MinMax 函数 / Create a new MinMax function
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self {
        Self {
            inner: MaxFunction::new(id, name, polynomials, true),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.inner = self.inner.with_declared_dependencies(dependency_ids);
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 获取输入多项式列表 / Get the input polynomials
    pub fn polynomials(&self) -> &[Linear<V>] {
        self.inner.polynomials()
    }
}

impl<V> Display for MinMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "minmax({})", self.inner.name())
    }
}

impl<V> DynSymbol for MinMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn display_name(&self) -> &str {
        self.inner.display_name()
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        self.inner.dyn_id()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<V> Symbol for MinMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.inner.id()
    }
}

impl<V> IntermediateSymbol<V> for MinMaxFunction<V>
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
        <MaxFunction<V> as IntermediateSymbol<V>>::category(&self.inner)
    }

    fn cached(&self) -> bool {
        <MaxFunction<V> as IntermediateSymbol<V>>::cached(&self.inner)
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        <MaxFunction<V> as IntermediateSymbol<V>>::dependencies(&self.inner)
    }

    fn declared_dependency_ids(&self) -> Vec<u64> {
        <MaxFunction<V> as IntermediateSymbol<V>>::declared_dependency_ids(&self.inner)
    }

    fn flush(&self, force: bool) {
        <MaxFunction<V> as IntermediateSymbol<V>>::flush(&self.inner, force)
    }

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        <MaxFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
            &self.inner,
            symbol_to_index,
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
        <MaxFunction<V> as IntermediateSymbol<V>>::prepare(&self.inner, values)
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("minmax({})", self.inner.name())
    }
}

impl<V> FunctionSymbol<V> for MinMaxFunction<V>
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
        <MaxFunction<V> as FunctionSymbol<V>>::register_tokens(&self.inner, tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        <MaxFunction<V> as FunctionSymbol<V>>::calculate_value(
            &self.inner,
            token_table,
            zero_if_none,
        )
    }
}

impl<V> LinearIntermediateSymbol<V> for MinMaxFunction<V>
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
        <MaxFunction<V> as LinearIntermediateSymbol<V>>::to_linear_polynomial(&self.inner)
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        <MaxFunction<V> as LinearIntermediateSymbol<V>>::to_quadratic_polynomial(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::LinearMonomial;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId};

    #[test]
    fn maxmin_and_minmax_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(5.0);
        tokens.add_token(ty);

        let p1 = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let p2 = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);

        let maxmin = MaxMinFunction::new(30001, "maxmin", vec![p1.clone(), p2.clone()]);
        let minmax = MinMaxFunction::new(30002, "minmax", vec![p1, p2]);
        assert_eq!(maxmin.calculate_value(&tokens, false), Some(2.0));
        assert_eq!(minmax.calculate_value(&tokens, false), Some(5.0));
    }
}
