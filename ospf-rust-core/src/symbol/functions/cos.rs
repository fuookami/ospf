//! 余弦函数符号 / Cosine function symbol

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::{ModelError, Result};
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{

    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange,
};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::sin::{build_breakpoints, build_piecewise_auxiliary_variables, build_piecewise_constraints};

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

/// 余弦函数符号 / Cosine function symbol.
///
/// 使用分段线性逼近对 `cos(input)` 建模，其中 input 为线性多项式。
/// Models `cos(input)` using piecewise-linear approximation, where input is a linear polynomial.
#[derive(Debug, Clone)]
pub struct CosFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    result_var: ContinuousVariableItem,
    offset_vars: Vec<ContinuousVariableItem>,
    selector_vars: Vec<BinaryVariableItem>,
    breakpoints: Vec<f64>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> CosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的余弦函数 / Create a new cosine function
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        let breakpoints = build_breakpoints();
        let (group_id, offset_vars, selector_vars) =
            build_piecewise_auxiliary_variables(name, "cos", &breakpoints);
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            name,
            VariableRange::bounded(-1.0, 1.0),
        );

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            offset_vars,
            selector_vars,
            breakpoints,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.input = input;
        cloned
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取输入多项式 / Get the input polynomial
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }
}

impl<V> Display for CosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cos({})", self.id.name)
    }
}

impl<V> DynSymbol for CosFunction<V>
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

impl<V> Symbol for CosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for CosFunction<V>
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
        Category::Nonlinear
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
                    "cos result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut offset_indices = Vec::with_capacity(self.offset_vars.len());
        for var in &self.offset_vars {
            let idx = symbol_to_index
                .get(&(var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "cos offset variable id {}",
                        var.id().unique_id()
                    ))
                })?;
            offset_indices.push(idx);
        }

        let mut selector_indices = Vec::with_capacity(self.selector_vars.len());
        for var in &self.selector_vars {
            let idx = symbol_to_index
                .get(&(var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "cos selector variable id {}",
                        var.id().unique_id()
                    ))
                })?;
            selector_indices.push(idx);
        }

        let values = self.breakpoints.iter().map(|x| x.cos()).collect::<Vec<_>>();
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        build_piecewise_constraints(
            &self.id.name,
            &self.input,
            symbol_to_index,
            result_index,
            &offset_indices,
            &selector_indices,
            &self.breakpoints,
            &values,
            source,
        )
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
        format!("cos({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for CosFunction<V>
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
        for var in &self.offset_vars {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        for var in &self.selector_vars {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        let value = to_f64(&value)?;
        from_f64(value.cos())
    }
}

impl<V> LinearIntermediateSymbol<V> for CosFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + One
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(V::one(), self.result_var.index())],
            V::zero(),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}
