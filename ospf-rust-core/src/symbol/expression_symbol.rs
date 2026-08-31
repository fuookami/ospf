//! 表达式符号类型 / Expression symbol types.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Add, Mul};
use std::sync::Arc;
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use super::{

    Category, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticIntermediateSymbol,
};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::TokenList;

/// 通用表达式符号 / Generic expression symbol.
#[derive(Debug, Clone)]
pub struct ExpressionSymbol<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号标识符 / Symbol identifier.
    id: IntermediateSymbolId,
    /// 符号类别 / Symbol category.
    category: Category,
    /// 缓存值 / Cached value.
    cached_value: Option<V>,
    /// 声明的依赖 ID 列表 / Declared dependency IDs.
    declared_dependency_ids: Vec<u64>,
}

impl<V> ExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 使用指定 ID、名称和类别创建表达式符号 / Create an expression symbol with the given ID, name, and category.
    pub fn new(id: u64, name: &str, category: Category) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            category,
            cached_value: None,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用指定 ID、名称、类别和依赖 ID 创建表达式符号 / Create an expression symbol with the given ID, name, category, and dependency IDs.
    pub fn with_dependencies(
        id: u64,
        name: &str,
        category: Category,
        dependency_ids: Vec<u64>,
    ) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            category,
            cached_value: None,
            declared_dependency_ids: dependency_ids,
        }
    }
}

impl<V> Display for ExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.id.name)
    }
}

impl<V> DynSymbol for ExpressionSymbol<V>
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

impl<V> Symbol for ExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn category(&self) -> Category {
        self.category
    }

    fn cached(&self) -> bool {
        self.cached_value.is_some()
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        HashSet::new()
    }

    fn declared_dependency_ids(&self) -> Vec<u64> {
        self.declared_dependency_ids.clone()
    }

    fn flush(&self, _force: bool) {}

    fn prepare(&self, _values: &HashMap<usize, V>) -> Option<V> {
        self.cached_value.clone()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        self.id.name.clone()
    }
}

/// 通用线性表达式符号 / Generic linear expression symbol.
#[derive(Debug, Clone)]
pub struct LinearExpressionSymbol<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 内部表达式符号 / Inner expression symbol.
    inner: ExpressionSymbol<V>,
    /// 线性单项式列表 / Linear monomials.
    monomials: Vec<LinearMonomial<V>>,
    /// 常数项 / Constant term.
    constant: V,
}

impl<V> LinearExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 使用指定 ID、名称、单项式和常数项创建线性表达式符号 / Create a linear expression symbol with the given ID, name, monomials, and constant.
    pub fn new(id: u64, name: &str, monomials: Vec<LinearMonomial<V>>, constant: V) -> Self {
        Self {
            inner: ExpressionSymbol::new(id, name, Category::Linear),
            monomials,
            constant,
        }
    }

    /// 使用指定 ID、名称、单项式、常数项和依赖 ID 创建线性表达式符号 / Create a linear expression symbol with the given ID, name, monomials, constant, and dependency IDs.
    pub fn new_with_dependencies(
        id: u64,
        name: &str,
        monomials: Vec<LinearMonomial<V>>,
        constant: V,
        dependency_ids: Vec<u64>,
    ) -> Self {
        Self {
            inner: ExpressionSymbol::with_dependencies(id, name, Category::Linear, dependency_ids),
            monomials,
            constant,
        }
    }
}

impl<V> Display for LinearExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.inner)
    }
}

impl<V> DynSymbol for LinearExpressionSymbol<V>
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

impl<V> Symbol for LinearExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.inner.id()
    }
}

impl<V> IntermediateSymbol<V> for LinearExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    fn category(&self) -> Category {
        self.inner.category()
    }

    fn cached(&self) -> bool {
        self.inner.cached()
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        self.inner.dependencies()
    }

    fn declared_dependency_ids(&self) -> Vec<u64> {
        self.inner.declared_dependency_ids()
    }

    fn flush(&self, force: bool) {
        self.inner.flush(force)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        _zero_if_none: bool,
    ) -> Option<V> {
        let mut evaluated = self.constant.clone();
        for monomial in &self.monomials {
            let value = token_table
                .find_by_index(monomial.var_index())
                .and_then(|token| token.get_result())?;
            evaluated = evaluated + monomial.coefficient().clone() * value;
        }
        Some(evaluated)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let mut evaluated = self.constant.clone();
        for monomial in &self.monomials {
            let value = values.get(&monomial.var_index())?.clone();
            evaluated = evaluated + monomial.coefficient().clone() * value;
        }
        Some(evaluated)
    }

    fn to_raw_string(&self, unfold: u64) -> String {
        self.inner.to_raw_string(unfold)
    }
}

impl<V> LinearIntermediateSymbol<V> for LinearExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(self.monomials.clone(), self.constant.clone())
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        let quadratic_monomials: Vec<QuadraticMonomial<V>> = self
            .monomials
            .iter()
            .map(|m| QuadraticMonomial::new_linear(m.coefficient().clone(), m.var_index()))
            .collect();
        Quadratic::new(quadratic_monomials, self.constant.clone())
    }
}

/// 通用二次表达式符号 / Generic quadratic expression symbol.
#[derive(Debug, Clone)]
pub struct QuadraticExpressionSymbol<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 内部表达式符号 / Inner expression symbol.
    inner: ExpressionSymbol<V>,
    /// 二次单项式列表 / Quadratic monomials.
    monomials: Vec<QuadraticMonomial<V>>,
    /// 常数项 / Constant term.
    constant: V,
}

impl<V> QuadraticExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 使用指定 ID、名称、单项式和常数项创建二次表达式符号 / Create a quadratic expression symbol with the given ID, name, monomials, and constant.
    pub fn new(id: u64, name: &str, monomials: Vec<QuadraticMonomial<V>>, constant: V) -> Self {
        Self {
            inner: ExpressionSymbol::new(id, name, Category::Quadratic),
            monomials,
            constant,
        }
    }

    /// 使用指定 ID、名称、单项式、常数项和依赖 ID 创建二次表达式符号 / Create a quadratic expression symbol with the given ID, name, monomials, constant, and dependency IDs.
    pub fn new_with_dependencies(
        id: u64,
        name: &str,
        monomials: Vec<QuadraticMonomial<V>>,
        constant: V,
        dependency_ids: Vec<u64>,
    ) -> Self {
        Self {
            inner: ExpressionSymbol::with_dependencies(
                id,
                name,
                Category::Quadratic,
                dependency_ids,
            ),
            monomials,
            constant,
        }
    }
}

impl<V> Display for QuadraticExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.inner)
    }
}

impl<V> DynSymbol for QuadraticExpressionSymbol<V>
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

impl<V> Symbol for QuadraticExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.inner.id()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    fn category(&self) -> Category {
        self.inner.category()
    }

    fn cached(&self) -> bool {
        self.inner.cached()
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        self.inner.dependencies()
    }

    fn declared_dependency_ids(&self) -> Vec<u64> {
        self.inner.declared_dependency_ids()
    }

    fn flush(&self, force: bool) {
        self.inner.flush(force)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        _zero_if_none: bool,
    ) -> Option<V> {
        let mut evaluated = self.constant.clone();
        for monomial in &self.monomials {
            let value1 = token_table
                .find_by_index(monomial.var_index1())
                .and_then(|token| token.get_result())?;
            let term = if let Some(var2_index) = monomial.var_index2() {
                let value2 = token_table
                    .find_by_index(var2_index)
                    .and_then(|token| token.get_result())?;
                monomial.coefficient().clone() * value1 * value2
            } else {
                monomial.coefficient().clone() * value1
            };
            evaluated = evaluated + term;
        }
        Some(evaluated)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        let mut evaluated = self.constant.clone();
        for monomial in &self.monomials {
            let value1 = values.get(&monomial.var_index1())?.clone();
            let term = if let Some(var2_index) = monomial.var_index2() {
                let value2 = values.get(&var2_index)?.clone();
                monomial.coefficient().clone() * value1 * value2
            } else {
                monomial.coefficient().clone() * value1
            };
            evaluated = evaluated + term;
        }
        Some(evaluated)
    }

    fn to_raw_string(&self, unfold: u64) -> String {
        self.inner.to_raw_string(unfold)
    }
}

impl<V> QuadraticIntermediateSymbol<V> for QuadraticExpressionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::new(self.monomials.clone(), self.constant.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::ContinuousVariableItem;

    #[test]
    fn linear_expression_evaluate_from_tokens() {
        let x = ContinuousVariableItem::auto("x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        let symbol =
            LinearExpressionSymbol::new(100, "lin_expr", vec![LinearMonomial::new(3.0, 0)], 1.0);
        let value = symbol.evaluate_from_tokens(&tokens, false);
        assert_eq!(value, Some(7.0));
    }

    #[test]
    fn quadratic_expression_evaluate_from_tokens() {
        let x = ContinuousVariableItem::auto("x");
        let y = ContinuousVariableItem::auto("y");

        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        let ty = Token::from_generic(y, 1);
        ty.set_result(3.0);
        tokens.add_token(tx);
        tokens.add_token(ty);

        let symbol = QuadraticExpressionSymbol::new(
            101,
            "quad_expr",
            vec![
                QuadraticMonomial::new_quadratic(2.0, 0, 1),
                QuadraticMonomial::new_linear(3.0, 0),
            ],
            1.0,
        );
        let value = symbol.evaluate_from_tokens(&tokens, false);
        assert_eq!(value, Some(19.0));
    }
}
