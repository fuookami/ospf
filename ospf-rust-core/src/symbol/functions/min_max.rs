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

    fn deferred_structure_with_tokens(
        &self,
        _tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // MAXMIN 的即时展开不读取令牌边界：它没有覆写 `mechanism_constraints_with_tokens`，默认
        // 实现直接退回 `mechanism_constraints`，再由内部 MIN 用统一回退 Big-M 生成。因此延迟结构
        // 与即时展开天然逐行等价，不需要也不应该按令牌裁剪结构内容；只有空候选集不提供结构，
        // 因为该退化形态在即时侧同样只是一个越界等式。
        // MAXMIN's eager expansion never reads token bounds: it does not override
        // `mechanism_constraints_with_tokens`, so the default implementation falls back to
        // `mechanism_constraints`, where the inner MIN generates rows with the unified fallback
        // Big-M. A deferred structure is therefore row-identical to eager expansion by construction
        // and must not prune itself by token bounds; only an empty candidate set withholds the
        // structure because that degenerate form is a single out-of-range equality on the eager side
        // too.
        if self.polynomials().is_empty() {
            return None;
        }
        Some(Arc::new(MaxMinStructure::new(
            self.inner.name().to_string(),
            Arc::new(self.clone()),
        )))
    }
}

/// MAXMIN 的求解器无关结构描述 / Solver-neutral structure description of MAXMIN
///
/// MAXMIN 是精确 MIN 的转发符号，即时展开本身就是把公式生成委托给内部 MIN。延迟结构因此把
/// 「转发」原样保存下来：持有产生它的符号（`Arc`），物化时调用即时展开使用的同一个转发入口
/// [`IntermediateSymbol::mechanism_constraints`]，行集合与 EAGER 完全一致，不存在第二份公式。
///
/// 转发符号不额外保存 Big-M：回退 M 完全由被转发的 MIN 生成器按同一策略解析，结构里再存一份
/// 会制造第二份真相，一旦两份取值漂移就会静默偏离即时语义，因此这里只保存名称、符号 ID、结果列
/// 与选择器列，M 随生成器走。
///
/// MAXMIN is a forwarding symbol over exact MIN: its eager expansion delegates formula generation to
/// the inner MIN. The deferred structure keeps that forwarding as its content — it holds the symbol
/// that produced it (an `Arc`) and materializes through the very same forwarding entry
/// [`IntermediateSymbol::mechanism_constraints`] eager expansion uses, so the row set matches EAGER
/// exactly with no second copy of the formula.
///
/// A forwarding symbol stores no Big-M of its own: the fallback M is resolved by the forwarded MIN
/// generator under the same policy, and keeping a second copy here would create a second source of
/// truth that silently diverges from eager semantics once the two drift apart. Only the name, symbol
/// ID, result column and selector columns are kept; M travels with the generator.
#[derive(Debug)]
pub struct MaxMinStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<MaxMinFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 选择器辅助列 / Selector helper columns
    selectors: Vec<crate::variable::VariableId>,
}

impl<V> MaxMinStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<MaxMinFunction<V>>) -> Self {
        let result = symbol.result_variable().id();
        let selectors = symbol
            .inner
            .selector_variables()
            .map(|selectors| selectors.iter().map(|selector| selector.id()).collect())
            .unwrap_or_default();
        Self {
            name: name.into(),
            symbol,
            result,
            selectors,
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

    /// 获取选择器辅助列 / Get the selector helper columns.
    pub fn selectors(&self) -> &[crate::variable::VariableId] {
        &self.selectors
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for MaxMinStructure<V>
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
        // 选择器列属于本结构的辅助列，参与「是否被外部引用 / 是否可省略」的判定。
        // Selector columns are helpers of this structure and take part in the
        // externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id().id,
            self.result.clone(),
            self.selectors.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        let selectors = self
            .selectors
            .iter()
            .map(|selector| selector.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "maxmin|{}|{}|{}|{}",
            self.name,
            self.symbol.id().id,
            self.result.unique_id(),
            selectors
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一转发入口，保证两条路径逐行一致。
        // Reuse the eager path's forwarding entry so both paths stay row-identical.
        <MaxMinFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
            &self.symbol,
            symbol_to_index,
        )
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

    fn deferred_structure_with_tokens(
        &self,
        _tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // 与 MAXMIN 同理：MINMAX 是精确 MAX 的转发符号，即时展开不读取令牌边界，默认实现直接
        // 退回 `mechanism_constraints`，因此延迟结构与即时展开逐行等价，不需要按令牌裁剪结构
        // 内容；空候选集不提供结构。
        // Same reasoning as MAXMIN: MINMAX forwards to exact MAX, its eager expansion never reads
        // token bounds, and the default implementation falls back to `mechanism_constraints`, so a
        // deferred structure is row-identical by construction and must not prune itself by token
        // bounds; an empty candidate set withholds the structure.
        if self.polynomials().is_empty() {
            return None;
        }
        Some(Arc::new(MinMaxStructure::new(
            self.inner.name().to_string(),
            Arc::new(self.clone()),
        )))
    }
}

/// MINMAX 的求解器无关结构描述 / Solver-neutral structure description of MINMAX
///
/// 与 [`MaxMinStructure`] 对称：MINMAX 转发到精确 MAX，结构把「转发」原样保存下来，持有产生它的
/// 符号（`Arc`），物化时调用即时展开使用的同一个转发入口
/// [`IntermediateSymbol::mechanism_constraints`]，因此延迟物化与 EAGER 展开逐行一致，且不存在
/// 第二份公式。
///
/// 与 [`MaxMinStructure`] 一样不保存 Big-M：回退 M 由被转发的 MAX 生成器按同一策略解析，结构里
/// 再存一份会制造第二份真相。
///
/// Symmetric to [`MaxMinStructure`]: MINMAX forwards to exact MAX and the structure keeps that
/// forwarding as its content, holding the symbol that produced it (an `Arc`) and materializing
/// through the very same forwarding entry [`IntermediateSymbol::mechanism_constraints`] eager
/// expansion uses, so deferred materialization matches eager expansion row by row with no second copy
/// of the formula.
///
/// Like [`MaxMinStructure`] it stores no Big-M: the fallback M is resolved by the forwarded MAX
/// generator under the same policy, and a second copy here would create a second source of truth.
#[derive(Debug)]
pub struct MinMaxStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<MinMaxFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 选择器辅助列 / Selector helper columns
    selectors: Vec<crate::variable::VariableId>,
}

impl<V> MinMaxStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<MinMaxFunction<V>>) -> Self {
        let result = symbol.result_variable().id();
        let selectors = symbol
            .inner
            .selector_variables()
            .map(|selectors| selectors.iter().map(|selector| selector.id()).collect())
            .unwrap_or_default();
        Self {
            name: name.into(),
            symbol,
            result,
            selectors,
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

    /// 获取选择器辅助列 / Get the selector helper columns.
    pub fn selectors(&self) -> &[crate::variable::VariableId] {
        &self.selectors
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for MinMaxStructure<V>
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
        // 选择器列属于本结构的辅助列，参与「是否被外部引用 / 是否可省略」的判定。
        // Selector columns are helpers of this structure and take part in the
        // externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id().id,
            self.result.clone(),
            self.selectors.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        let selectors = self
            .selectors
            .iter()
            .map(|selector| selector.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "minmax|{}|{}|{}|{}",
            self.name,
            self.symbol.id().id,
            self.result.unique_id(),
            selectors
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一转发入口，保证两条路径逐行一致。
        // Reuse the eager path's forwarding entry so both paths stay row-identical.
        <MinMaxFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
            &self.symbol,
            symbol_to_index,
        )
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
    use crate::model::{FunctionExpansionPolicy, MetaModel};
    use crate::symbol::flatten::LinearMonomial;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange};

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

    /// 构造转发符号的列映射：输入列、选择器辅助列、结果列。
    /// Build the column map of a forwarding symbol: input, selector helper and result columns.
    fn forwarding_symbol_to_index(
        binding: &crate::model::intermediate::StructureUsageBinding,
        input_ids: &[VariableId],
    ) -> HashMap<usize, usize> {
        let mut symbol_to_index = HashMap::new();
        for (offset, id) in input_ids.iter().enumerate() {
            symbol_to_index.insert(id.unique_id() as usize, offset);
        }
        for (offset, selector) in binding.helpers.iter().enumerate() {
            symbol_to_index.insert(selector.unique_id() as usize, input_ids.len() + offset);
        }
        symbol_to_index.insert(
            binding.result.unique_id() as usize,
            input_ids.len() + binding.helpers.len(),
        );
        symbol_to_index
    }

    /// 断言即时行与延迟行逐行一致 / Assert eager and deferred rows agree row by row.
    fn assert_rows_match(eager: &[LinearConstraint<f64>], deferred: &[LinearConstraint<f64>]) {
        assert!(!eager.is_empty(), "eager rows must not be empty");
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
    }

    #[test]
    fn maxmin_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_200),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let y = ContinuousVariableItem::with_range(
            VariableId::standalone(96_201),
            "y",
            VariableRange::bounded(1.0, 4.0),
        );
        let maxmin: MaxMinFunction<f64> = MaxMinFunction::new(
            30010,
            "maxmin_deferred",
            vec![
                Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            ],
        );

        let structure = maxmin
            .deferred_structure_with_tokens(&[])
            .expect("maxmin should expose a deferred structure");
        assert_eq!(structure.function_name(), "maxmin_deferred");
        let binding = structure
            .usage_binding()
            .expect("maxmin structure should expose a usage binding");
        assert_eq!(binding.result, maxmin.result_variable().id());
        assert_eq!(binding.helpers.len(), maxmin.polynomials().len());
        assert!(structure.fingerprint().is_some());
        let concrete = structure
            .as_any()
            .downcast_ref::<MaxMinStructure<f64>>()
            .expect("structure should downcast to the MAXMIN structure");
        assert_eq!(concrete.selectors(), binding.helpers.as_slice());
        assert_eq!(*concrete.result(), maxmin.result_variable().id());

        let symbol_to_index =
            forwarding_symbol_to_index(&binding, &[x.id(), y.id()]);
        let tokens = vec![Token::from_generic(x, 0), Token::from_generic(y, 1)];

        // 即时 `mechanism_constraints` 与模型实际调用的 `mechanism_constraints_with_tokens`
        // （默认转发到前者）都必须与物化结果逐行一致。
        // The eager `mechanism_constraints` and the `mechanism_constraints_with_tokens` the model
        // actually calls (which forwards to the former by default) must both match materialization
        // row by row.
        let eager = <MaxMinFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
            &maxmin,
            &symbol_to_index,
        )
        .expect("eager maxmin constraints should be generated");
        let eager_with_tokens = maxmin
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("token-context maxmin constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("maxmin structure should materialize");

        assert_rows_match(&eager, &deferred);
        assert_rows_match(&eager_with_tokens, &deferred);

        // 空候选集是退化形态，不提供结构，继续即时展开。
        // An empty candidate set is the degenerate form: no structure is offered and eager
        // expansion continues.
        let empty: MaxMinFunction<f64> = MaxMinFunction::new(30011, "maxmin_empty", Vec::new());
        assert!(empty.deferred_structure_with_tokens(&[]).is_none());
    }

    #[test]
    fn maxmin_defers_through_the_model_pipeline() {
        fn rows(policy: FunctionExpansionPolicy) -> Vec<String> {
            let mut model = MetaModel::<f64>::new("maxmin_deferred_pipeline");
            model.set_function_expansion_policy(policy);
            let x = ContinuousVariableItem::with_range(
                VariableId::standalone(95_830),
                "x",
                VariableRange::bounded(-2.0, 3.0),
            );
            let x_index = model.register_variable(x).unwrap();
            let y = ContinuousVariableItem::with_range(
                VariableId::standalone(95_831),
                "y",
                VariableRange::bounded(1.0, 4.0),
            );
            let y_index = model.register_variable(y).unwrap();
            let maxmin: MaxMinFunction<f64> = MaxMinFunction::new(
                993,
                "maxmin_pipeline",
                vec![
                    Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
                    Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
                ],
            );
            model.add_symbol(Arc::new(maxmin)).unwrap();

            let mechanism = model.try_into_mechanism_model().unwrap();
            if policy.is_deferred() {
                // 转发符号在延迟策略下不写即时行，但保留结构描述。
                // The forwarding symbol writes no eager row under a deferred policy while keeping its
                // structure description.
                assert!(mechanism.as_basic().constraints().is_empty());
                assert_eq!(mechanism.as_basic().deferred_functions().len(), 1);
                assert_eq!(
                    mechanism.as_basic().deferred_functions()[0].function_name(),
                    "maxmin_pipeline"
                );
            }

            let linear = mechanism.into_linear_triad_model();
            let mut names = linear.basic.constraint_names.clone();
            names.sort();
            names
        }

        let eager = rows(FunctionExpansionPolicy::Eager);
        assert!(eager.len() > 2, "eager rows: {eager:?}");
        // 延迟路径经物化后必须与 EAGER 得到同一批行，并使用同一套回退 Big-M。
        // The deferred path must produce the same rows as eager expansion once materialized, using
        // the same fallback Big-M.
        assert_eq!(eager, rows(FunctionExpansionPolicy::DeferredNativeFirst));
    }

    #[test]
    fn minmax_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_400),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let y = ContinuousVariableItem::with_range(
            VariableId::standalone(96_401),
            "y",
            VariableRange::bounded(1.0, 4.0),
        );
        let minmax: MinMaxFunction<f64> = MinMaxFunction::new(
            30020,
            "minmax_deferred",
            vec![
                Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            ],
        );

        let structure = minmax
            .deferred_structure_with_tokens(&[])
            .expect("minmax should expose a deferred structure");
        assert_eq!(structure.function_name(), "minmax_deferred");
        let binding = structure
            .usage_binding()
            .expect("minmax structure should expose a usage binding");
        assert_eq!(binding.result, minmax.result_variable().id());
        assert_eq!(binding.helpers.len(), minmax.polynomials().len());
        assert!(structure.fingerprint().is_some());
        let concrete = structure
            .as_any()
            .downcast_ref::<MinMaxStructure<f64>>()
            .expect("structure should downcast to the MINMAX structure");
        assert_eq!(concrete.selectors(), binding.helpers.as_slice());
        assert_eq!(*concrete.result(), minmax.result_variable().id());

        let symbol_to_index = forwarding_symbol_to_index(&binding, &[x.id(), y.id()]);
        let tokens = vec![Token::from_generic(x, 0), Token::from_generic(y, 1)];

        // 即时 `mechanism_constraints` 与模型实际调用的 `mechanism_constraints_with_tokens`
        // （默认转发到前者）都必须与物化结果逐行一致。
        // The eager `mechanism_constraints` and the `mechanism_constraints_with_tokens` the model
        // actually calls (which forwards to the former by default) must both match materialization
        // row by row.
        let eager = <MinMaxFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
            &minmax,
            &symbol_to_index,
        )
        .expect("eager minmax constraints should be generated");
        let eager_with_tokens = minmax
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("token-context minmax constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("minmax structure should materialize");

        assert_rows_match(&eager, &deferred);
        assert_rows_match(&eager_with_tokens, &deferred);

        // 空候选集是退化形态，不提供结构，继续即时展开。
        // An empty candidate set is the degenerate form: no structure is offered and eager
        // expansion continues.
        let empty: MinMaxFunction<f64> = MinMaxFunction::new(30021, "minmax_empty", Vec::new());
        assert!(empty.deferred_structure_with_tokens(&[]).is_none());
    }

    #[test]
    fn minmax_defers_through_the_model_pipeline() {
        fn rows(policy: FunctionExpansionPolicy) -> Vec<String> {
            let mut model = MetaModel::<f64>::new("minmax_deferred_pipeline");
            model.set_function_expansion_policy(policy);
            let x = ContinuousVariableItem::with_range(
                VariableId::standalone(95_840),
                "x",
                VariableRange::bounded(-2.0, 3.0),
            );
            let x_index = model.register_variable(x).unwrap();
            let y = ContinuousVariableItem::with_range(
                VariableId::standalone(95_841),
                "y",
                VariableRange::bounded(1.0, 4.0),
            );
            let y_index = model.register_variable(y).unwrap();
            let minmax: MinMaxFunction<f64> = MinMaxFunction::new(
                995,
                "minmax_pipeline",
                vec![
                    Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
                    Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
                ],
            );
            model.add_symbol(Arc::new(minmax)).unwrap();

            let mechanism = model.try_into_mechanism_model().unwrap();
            if policy.is_deferred() {
                // 转发符号在延迟策略下不写即时行，但保留结构描述。
                // The forwarding symbol writes no eager row under a deferred policy while keeping its
                // structure description.
                assert!(mechanism.as_basic().constraints().is_empty());
                assert_eq!(mechanism.as_basic().deferred_functions().len(), 1);
                assert_eq!(
                    mechanism.as_basic().deferred_functions()[0].function_name(),
                    "minmax_pipeline"
                );
            }

            let linear = mechanism.into_linear_triad_model();
            let mut names = linear.basic.constraint_names.clone();
            names.sort();
            names
        }

        let eager = rows(FunctionExpansionPolicy::Eager);
        assert!(eager.len() > 2, "eager rows: {eager:?}");
        // 延迟路径经物化后必须与 EAGER 得到同一批行，并使用同一套回退 Big-M。
        // The deferred path must produce the same rows as eager expansion once materialized, using
        // the same fallback Big-M.
        assert_eq!(eager, rows(FunctionExpansionPolicy::DeferredNativeFirst));
    }
}
