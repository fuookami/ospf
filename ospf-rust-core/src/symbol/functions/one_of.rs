//! 选一函数与 If-Else 函数符号 / OneOf and IfElse function symbols

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
use crate::variable::{

    BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id, new_standalone_id,
};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_big_m_for_polynomials};

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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

/// 选一函数 / One-of function
///
/// 通过二值选择器选择一个表达式并返回加权和。
/// Select one expression by binary selectors and return weighted sum.
#[derive(Debug, Clone)]
pub struct OneOfFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 输入多项式列表 / Input polynomials
    polynomials: Vec<Linear<V>>,
    /// 结果连续变量 / Result continuous variable
    result_var: ContinuousVariableItem,
    /// 选择器二值变量列表 / Selection binary variables
    selection_vars: Vec<BinaryVariableItem>,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> OneOfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的选一函数 / Create a new one-of function
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self {
        let n = polynomials.len();
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(
            VariableId::new(group_id, 0),
            &format!("{}_result", name),
        );

        let selection_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_sel{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            selection_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 one-of 函数。
    /// Create a one-of function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, polynomials: Vec<Linear<V>>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            polynomials,
        )
    }

    /// 使用自动 ID 与自动名称创建 one-of 函数。
    /// Create a one-of function with an auto id and auto-generated name.
    pub fn auto(polynomials: Vec<Linear<V>>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("one_of", id);
        Self::new(id, &name, polynomials)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn selection_variables(&self) -> &[BinaryVariableItem] {
        &self.selection_vars
    }

    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }
}

impl<V> OneOfFunction<V>
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
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        if self.polynomials.len() != self.selection_vars.len() {
            return Err(ModelError::InvalidConstraint(format!(
                "one_of `{}` polynomial count {} does not match selection count {}",
                self.id.name,
                self.polynomials.len(),
                self.selection_vars.len()
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "one_of result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();

        for (i, (polynomial, selector)) in self
            .polynomials
            .iter()
            .zip(self.selection_vars.iter())
            .enumerate()
        {
            let selector_index = symbol_to_index
                .get(&(selector.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "one_of selector variable id {}",
                        selector.id().unique_id()
                    ))
                })?;

            let mut lower_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
            lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "one_of result lower coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "one_of `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "one_of polynomial lower coefficient")?,
                    monomial.var_index(),
                ));
            }
            lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-big_m, "one_of selector lower coefficient")?,
                selector_index,
            ));

            let mut upper_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
            upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "one_of result upper coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "one_of `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "one_of polynomial upper coefficient")?,
                    monomial.var_index(),
                ));
            }
            upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(big_m, "one_of selector upper coefficient")?,
                selector_index,
            ));

            let constant = to_f64(polynomial.constant_term()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "one_of `{}` polynomial constant cannot be converted to f64",
                    self.id.name
                ))
            })?;

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        lower_monomials,
                        convert_f64_to_v::<V>(-constant, "one_of lower constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(-big_m, "one_of lower rhs")?,
                ),
                &format!("{}_one_of_lb_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        upper_monomials,
                        convert_f64_to_v::<V>(-constant, "one_of upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(big_m, "one_of upper rhs")?,
                ),
                &format!("{}_one_of_ub_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
        }

        let mut sum_monomials = Vec::with_capacity(self.selection_vars.len());
        for selector in &self.selection_vars {
            let selector_index = symbol_to_index
                .get(&(selector.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "one_of selector variable id {}",
                        selector.id().unique_id()
                    ))
                })?;
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "one_of selector sum coefficient")?,
                selector_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    sum_monomials,
                    convert_f64_to_v::<V>(0.0, "one_of selector sum constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(1.0, "one_of selector sum rhs")?,
            ),
            &format!("{}_one_of_at_most_one", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut zero_upper_monomials = Vec::with_capacity(self.selection_vars.len() + 1);
        zero_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "one_of zero upper result coefficient")?,
            result_index,
        ));
        for selector in &self.selection_vars {
            let selector_index = symbol_to_index
                .get(&(selector.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "one_of selector variable id {}",
                        selector.id().unique_id()
                    ))
                })?;
            zero_upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-big_m, "one_of zero upper selector coefficient")?,
                selector_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    zero_upper_monomials,
                    convert_f64_to_v::<V>(0.0, "one_of zero upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "one_of zero upper rhs")?,
            ),
            &format!("{}_one_of_zero_ub", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut zero_lower_monomials = Vec::with_capacity(self.selection_vars.len() + 1);
        zero_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "one_of zero lower result coefficient")?,
            result_index,
        ));
        for selector in &self.selection_vars {
            let selector_index = symbol_to_index
                .get(&(selector.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "one_of selector variable id {}",
                        selector.id().unique_id()
                    ))
                })?;
            zero_lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(big_m, "one_of zero lower selector coefficient")?,
                selector_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    zero_lower_monomials,
                    convert_f64_to_v::<V>(0.0, "one_of zero lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "one_of zero lower rhs")?,
            ),
            &format!("{}_one_of_zero_lb", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl<V> Display for OneOfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "one_of({})", self.id.name)
    }
}

impl<V> DynSymbol for OneOfFunction<V>
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

impl<V> Symbol for OneOfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for OneOfFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, DEFAULT_BIG_M)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
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
        format!("one_of({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for OneOfFunction<V>
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
        for var in &self.selection_vars {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut result = 0.0_f64;
        let mut has_selection = false;

        for (polynomial, selection_var) in self.polynomials.iter().zip(&self.selection_vars) {
            let selected = match token_table
                .find_by_id(selection_var.id())
                .and_then(|token| token.get_result())
            {
                Some(v) => v,
                None if zero_if_none => from_f64(0.0)?,
                None => return None,
            };
            let selected = to_f64(&selected)?;
            if selected.abs() <= f64::EPSILON {
                continue;
            }

            has_selection = true;
            let value = to_f64(&evaluate_linear(polynomial, token_table, zero_if_none)?)?;
            result += selected * value;
        }

        if has_selection || zero_if_none {
            from_f64(result)
        } else {
            None
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for OneOfFunction<V>
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

/// Conditional expression result.
#[derive(Debug, Clone)]
pub struct IfElseFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    condition: BinaryVariableItem,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> IfElseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(
        id: u64,
        name: &str,
        condition: BinaryVariableItem,
        then_expr: Linear<V>,
        else_expr: Linear<V>,
    ) -> Self {
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_result", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            condition,
            then_expr,
            else_expr,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 if-else 函数。
    /// Create an if-else function with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        condition: BinaryVariableItem,
        then_expr: Linear<V>,
        else_expr: Linear<V>,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            condition,
            then_expr,
            else_expr,
        )
    }

    /// 使用自动 ID 与自动名称创建 if-else 函数。
    /// Create an if-else function with an auto id and auto-generated name.
    pub fn auto(condition: BinaryVariableItem, then_expr: Linear<V>, else_expr: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if_else", id);
        Self::new(id, &name, condition, then_expr, else_expr)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn condition_variable(&self) -> &BinaryVariableItem {
        &self.condition
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn then_expression(&self) -> &Linear<V> {
        &self.then_expr
    }

    pub fn else_expression(&self) -> &Linear<V> {
        &self.else_expr
    }
}

impl<V> IfElseFunction<V>
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
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_big_m_for_polynomials(
            &[self.then_expr.clone(), self.else_expr.clone()],
            tokens,
            BIG_M_POLICY.min(),
        )
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
                    "if_else result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let condition_index = symbol_to_index
            .get(&(self.condition.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_else condition variable id {}",
                    self.condition.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::with_capacity(4);

        let mut then_upper_monomials = Vec::with_capacity(self.then_expr.monomials().len() + 2);
        then_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if_else then upper result coefficient")?,
            result_index,
        ));
        for monomial in self.then_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_else `{}` then coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            then_upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "if_else then upper polynomial coefficient")?,
                monomial.var_index(),
            ));
        }
        then_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "if_else then upper condition coefficient")?,
            condition_index,
        ));
        let then_constant = to_f64(self.then_expr.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if_else `{}` then constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    then_upper_monomials,
                    convert_f64_to_v::<V>(-then_constant, "if_else then upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "if_else then upper rhs")?,
            ),
            &format!("{}_if_then_ub", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut then_lower_monomials = Vec::with_capacity(self.then_expr.monomials().len() + 2);
        then_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if_else then lower result coefficient")?,
            result_index,
        ));
        for monomial in self.then_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_else `{}` then coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            then_lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "if_else then lower polynomial coefficient")?,
                monomial.var_index(),
            ));
        }
        then_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if_else then lower condition coefficient")?,
            condition_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    then_lower_monomials,
                    convert_f64_to_v::<V>(-then_constant, "if_else then lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(-big_m, "if_else then lower rhs")?,
            ),
            &format!("{}_if_then_lb", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut else_upper_monomials = Vec::with_capacity(self.else_expr.monomials().len() + 2);
        else_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if_else else upper result coefficient")?,
            result_index,
        ));
        for monomial in self.else_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_else `{}` else coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            else_upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "if_else else upper polynomial coefficient")?,
                monomial.var_index(),
            ));
        }
        else_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if_else else upper condition coefficient")?,
            condition_index,
        ));
        let else_constant = to_f64(self.else_expr.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if_else `{}` else constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    else_upper_monomials,
                    convert_f64_to_v::<V>(-else_constant, "if_else else upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if_else else upper rhs")?,
            ),
            &format!("{}_if_else_ub", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut else_lower_monomials = Vec::with_capacity(self.else_expr.monomials().len() + 2);
        else_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if_else else lower result coefficient")?,
            result_index,
        ));
        for monomial in self.else_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_else `{}` else coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            else_lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "if_else else lower polynomial coefficient")?,
                monomial.var_index(),
            ));
        }
        else_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "if_else else lower condition coefficient")?,
            condition_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    else_lower_monomials,
                    convert_f64_to_v::<V>(-else_constant, "if_else else lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "if_else else lower rhs")?,
            ),
            &format!("{}_if_else_lb", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl<V> Display for IfElseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "if_else({})", self.id.name)
    }
}

impl<V> DynSymbol for IfElseFunction<V>
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

impl<V> Symbol for IfElseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for IfElseFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, DEFAULT_BIG_M)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
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
        format!("if_else({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for IfElseFunction<V>
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
        let condition = match token_table
            .find_by_id(self.condition.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => from_f64(0.0)?,
            None => return None,
        };

        if to_f64(&condition)?.abs() > f64::EPSILON {
            evaluate_linear(&self.then_expr, token_table, zero_if_none)
        } else {
            evaluate_linear(&self.else_expr, token_table, zero_if_none)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for IfElseFunction<V>
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
        // Quadratic by definition; keep linear proxy for now.
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
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn one_of_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(60_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: OneOfFunction<f64> = OneOfFunction::new(
            7000,
            "oneof_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
        );

        let selector = f.selection_variables()[0].clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let selector_id = selector.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (selector_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(selector, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("one_of constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "oneof_bound_one_of_ub_0")
            .expect("one_of upper constraint should exist");
        let selector_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        // 2x + 1 with x in [-2, 3] => range [-3, 7], therefore M = 7.
        assert!((upper.inequality.rhs - 7.0).abs() <= 1e-9);
        assert!((*selector_term.coefficient() - 7.0).abs() <= 1e-9);
    }

    #[test]
    fn one_of_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(60_010), "x");
        let f: OneOfFunction<f64> = OneOfFunction::new(
            7001,
            "oneof_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
        );

        let selector = f.selection_variables()[0].clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let selector_id = selector.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (selector_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(selector, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("one_of constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "oneof_default_one_of_ub_0")
            .expect("one_of upper constraint should exist");
        let selector_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        assert!((upper.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*selector_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }

    #[test]
    fn if_else_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(61_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let c = BinaryVariableItem::create(VariableId::standalone(61_001), "c");
        let f: IfElseFunction<f64> = IfElseFunction::new(
            7100,
            "ifelse_bound",
            c.clone(),
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            Linear::new(vec![], 0.0),
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let condition_id = c.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (condition_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(c, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_else constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "ifelse_bound_if_then_ub")
            .expect("if_else then-upper constraint should exist");
        let condition_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("condition term should exist");

        assert!((upper.inequality.rhs - 7.0).abs() <= 1e-9);
        assert!((*condition_term.coefficient() - 7.0).abs() <= 1e-9);
    }

    #[test]
    fn if_else_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(61_010), "x");
        let c = BinaryVariableItem::create(VariableId::standalone(61_011), "c");
        let f: IfElseFunction<f64> = IfElseFunction::new(
            7101,
            "ifelse_default",
            c.clone(),
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![], 0.0),
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let condition_id = c.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (condition_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(c, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_else constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "ifelse_default_if_then_ub")
            .expect("if_else then-upper constraint should exist");
        let condition_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("condition term should exist");

        assert!((upper.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*condition_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
