//! 最小值/最大值函数符号 / Min/max function symbols

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{BigMPolicy, infer_big_m_for_polynomials};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

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

/// 最小值函数 / Minimum function
///
/// 表示 min(p1, p2, ..., pn)。
/// Represents min(p1, p2, ..., pn).
#[derive(Debug, Clone)]
pub struct MinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 输入多项式列表 / Input polynomials
    polynomials: Vec<Linear<V>>,
    /// 结果连续变量 / Result continuous variable
    result_var: ContinuousVariableItem,
    /// 选择器二值变量列表（精确模式） / Selector binary variables (exact mode)
    binary_vars: Option<Vec<BinaryVariableItem>>,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> MinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建最小值函数 / Create a minimum function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>, exact: bool) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_min", name));

        let binary_vars = if exact {
            Some(
                (0..polynomials.len())
                    .map(|i| {
                        BinaryVariableItem::create(
                            VariableId::new(group_id, i + 1),
                            &format!("{}_u{}", name, i),
                        )
                    })
                    .collect(),
            )
        } else {
            None
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            binary_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_polynomials(&self, polynomials: Vec<Linear<V>>) -> Self {
        let mut cloned = self.clone();
        cloned.polynomials = polynomials;
        cloned
    }

    pub(crate) fn mechanism_constraints_with_big_m(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero + ToPrimitive + FromPrimitive,
        f64: IntoValue<V>,
    {
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取输入多项式 / Get the input polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 是否启用精确选择约束 / Whether exact selector constraints are enabled.
    pub fn exact(&self) -> bool {
        self.binary_vars.is_some()
    }
}

impl<V> MinFunction<V>
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
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "min result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();
        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let mut upper_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
            upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "min result upper coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "min `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                let polynomial_index = monomial.var_index();
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "min polynomial upper coefficient")?,
                    polynomial_index,
                ));
            }
            let constant = to_f64(polynomial.constant_term()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "min `{}` polynomial constant cannot be converted to f64",
                    self.id.name
                ))
            })?;
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        upper_monomials,
                        convert_f64_to_v::<V>(-constant, "min upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "min upper rhs")?,
                ),
                &format!("{}_min_ub_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));

            if let Some(binary_vars) = &self.binary_vars {
                let selector_index = symbol_to_index
                    .get(&(binary_vars[i].id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "min selector variable id {}",
                            binary_vars[i].id().unique_id()
                        ))
                    })?;
                let mut lower_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "min result lower coefficient")?,
                    result_index,
                ));
                for monomial in polynomial.monomials() {
                    let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                        ModelError::InvalidConstraint(format!(
                            "min `{}` polynomial coefficient cannot be converted to f64",
                            self.id.name
                        ))
                    })?;
                    let polynomial_index = monomial.var_index();
                    lower_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(-coefficient, "min polynomial lower coefficient")?,
                        polynomial_index,
                    ));
                }
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-big_m, "min selector lower coefficient")?,
                    selector_index,
                ));
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            lower_monomials,
                            convert_f64_to_v::<V>(big_m - constant, "min lower constant")?,
                        ),
                        ConstraintRelation::GreaterEqual,
                        convert_f64_to_v::<V>(0.0, "min lower rhs")?,
                    ),
                    &format!("{}_min_lb_{}", self.id.name, i),
                    Arc::new(self.clone()),
                ));
            }
        }

        if let Some(binary_vars) = &self.binary_vars {
            let mut selector_sum_monomials = Vec::with_capacity(binary_vars.len());
            for selector in binary_vars {
                let selector_index = symbol_to_index
                    .get(&(selector.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "min selector variable id {}",
                            selector.id().unique_id()
                        ))
                    })?;
                selector_sum_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "min selector sum coefficient")?,
                    selector_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        selector_sum_monomials,
                        convert_f64_to_v::<V>(0.0, "min selector sum constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "min selector sum rhs")?,
                ),
                &format!("{}_min_selector_sum", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        Ok(constraints)
    }
}

impl<V> Display for MinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "min({})", self.id.name)
    }
}

impl<V> DynSymbol for MinFunction<V>
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

impl<V> Symbol for MinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for MinFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
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
        format!("min({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for MinFunction<V>
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
        if let Some(ref binary_vars) = self.binary_vars {
            for var in binary_vars {
                tokens.push(Token::from_generic(var.clone(), var.index()));
            }
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut min_value: Option<f64> = None;
        for polynomial in &self.polynomials {
            let value = to_f64(&evaluate_linear(polynomial, token_table, zero_if_none)?)?;
            min_value = Some(match min_value {
                Some(current) => current.min(value),
                None => value,
            });
        }
        match min_value {
            Some(v) => from_f64(v),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for MinFunction<V>
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

/// Represents max(p1, p2, ..., pn).
#[derive(Debug, Clone)]
pub struct MaxFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<V>>,
    result_var: ContinuousVariableItem,
    binary_vars: Option<Vec<BinaryVariableItem>>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> MaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建最大值函数 / Create a maximum function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>, exact: bool) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_max", name));

        let binary_vars = if exact {
            Some(
                (0..polynomials.len())
                    .map(|i| {
                        BinaryVariableItem::create(
                            VariableId::new(group_id, i + 1),
                            &format!("{}_u{}", name, i),
                        )
                    })
                    .collect(),
            )
        } else {
            None
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            binary_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_polynomials(&self, polynomials: Vec<Linear<V>>) -> Self {
        let mut cloned = self.clone();
        cloned.polynomials = polynomials;
        cloned
    }

    pub(crate) fn mechanism_constraints_with_big_m(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero + ToPrimitive + FromPrimitive,
        f64: IntoValue<V>,
    {
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取输入多项式 / Get the input polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 是否启用精确选择约束 / Whether exact selector constraints are enabled.
    pub fn exact(&self) -> bool {
        self.binary_vars.is_some()
    }
}

impl<V> MaxFunction<V>
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
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "max result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();
        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let mut lower_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
            lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "max result lower coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "max `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                let polynomial_index = monomial.var_index();
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "max polynomial lower coefficient")?,
                    polynomial_index,
                ));
            }
            let constant = to_f64(polynomial.constant_term()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "max `{}` polynomial constant cannot be converted to f64",
                    self.id.name
                ))
            })?;
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        lower_monomials,
                        convert_f64_to_v::<V>(-constant, "max lower constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "max lower rhs")?,
                ),
                &format!("{}_max_lb_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));

            if let Some(binary_vars) = &self.binary_vars {
                let selector_index = symbol_to_index
                    .get(&(binary_vars[i].id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "max selector variable id {}",
                            binary_vars[i].id().unique_id()
                        ))
                    })?;
                let mut upper_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "max result upper coefficient")?,
                    result_index,
                ));
                for monomial in polynomial.monomials() {
                    let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                        ModelError::InvalidConstraint(format!(
                            "max `{}` polynomial coefficient cannot be converted to f64",
                            self.id.name
                        ))
                    })?;
                    let polynomial_index = monomial.var_index();
                    upper_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(-coefficient, "max polynomial upper coefficient")?,
                        polynomial_index,
                    ));
                }
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(big_m, "max selector upper coefficient")?,
                    selector_index,
                ));
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            upper_monomials,
                            convert_f64_to_v::<V>(-big_m - constant, "max upper constant")?,
                        ),
                        ConstraintRelation::LessEqual,
                        convert_f64_to_v::<V>(0.0, "max upper rhs")?,
                    ),
                    &format!("{}_max_ub_{}", self.id.name, i),
                    Arc::new(self.clone()),
                ));
            }
        }

        if let Some(binary_vars) = &self.binary_vars {
            let mut selector_sum_monomials = Vec::with_capacity(binary_vars.len());
            for selector in binary_vars {
                let selector_index = symbol_to_index
                    .get(&(selector.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "max selector variable id {}",
                            selector.id().unique_id()
                        ))
                    })?;
                selector_sum_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "max selector sum coefficient")?,
                    selector_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        selector_sum_monomials,
                        convert_f64_to_v::<V>(0.0, "max selector sum constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "max selector sum rhs")?,
                ),
                &format!("{}_max_selector_sum", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        Ok(constraints)
    }
}

impl<V> Display for MaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "max({})", self.id.name)
    }
}

impl<V> DynSymbol for MaxFunction<V>
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

impl<V> Symbol for MaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for MaxFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
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
        format!("max({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for MaxFunction<V>
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
        if let Some(ref binary_vars) = self.binary_vars {
            for var in binary_vars {
                tokens.push(Token::from_generic(var.clone(), var.index()));
            }
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut max_value: Option<f64> = None;
        for polynomial in &self.polynomials {
            let value = to_f64(&evaluate_linear(polynomial, token_table, zero_if_none)?)?;
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

impl<V> LinearIntermediateSymbol<V> for MaxFunction<V>
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
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn min_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(70_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: MinFunction<f64> = MinFunction::new(
            8000,
            "min_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact min should have binary vars")[0]
            .clone();
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
            .expect("min constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "min_bound_min_lb_0")
            .expect("min lower constraint should exist");
        let selector_term = lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        assert!((*selector_term.coefficient() + 7.0).abs() <= 1e-9);
    }

    #[test]
    fn min_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(70_010), "x");
        let f: MinFunction<f64> = MinFunction::new(
            8001,
            "min_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact min should have binary vars")[0]
            .clone();
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
            .expect("min constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "min_default_min_lb_0")
            .expect("min lower constraint should exist");
        let selector_term = lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        assert!((*selector_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }

    #[test]
    fn max_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(71_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: MaxFunction<f64> = MaxFunction::new(
            8100,
            "max_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact max should have binary vars")[0]
            .clone();
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
            .expect("max constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "max_bound_max_ub_0")
            .expect("max upper constraint should exist");
        let selector_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        assert!((*selector_term.coefficient() - 7.0).abs() <= 1e-9);
    }

    #[test]
    fn max_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(71_010), "x");
        let f: MaxFunction<f64> = MaxFunction::new(
            8101,
            "max_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact max should have binary vars")[0]
            .clone();
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
            .expect("max constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "max_default_max_ub_0")
            .expect("max upper constraint should exist");
        let selector_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        assert!((*selector_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
