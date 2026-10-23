//! 决策索引查表函数 / Decision-indexed lookup function

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    next_auto_intermediate_symbol_id,
};
use super::big_m::infer_linear_bounds_from_tokens;
use super::masking::MaskingFunction;
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{
    BinaryVariableItem, ContinuousVariableItem, IntegerVariableItem, VariableId, VariableRange,
    new_group_id,
};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

fn to_f64<V: ToPrimitive>(value: &V) -> Option<f64> {
    value.to_f64()
}

fn from_f64<V: FromPrimitive>(value: f64, context: &str) -> Result<V> {
    V::from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "element cannot convert {context}={value} into the model value type"
        ))
        .into()
    })
}

fn validate_bounds<V: ToPrimitive>(lower: &V, upper: &V, context: &str) -> Result<(f64, f64)> {
    let lower = to_f64(lower).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("element {context} lower bound is invalid"))
    })?;
    let upper = to_f64(upper).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("element {context} upper bound is invalid"))
    })?;
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(ModelError::InvalidConstraint(format!(
            "element {context} requires finite ordered bounds, got [{lower}, {upper}]"
        ))
        .into());
    }
    Ok((lower, upper))
}

fn validate_declared_bounds<V>(
    poly: &Linear<V>,
    bounds: (f64, f64),
    tokens: &[Token<V>],
    context: &str,
) -> Result<()>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let actual = infer_linear_bounds_from_tokens(poly, tokens).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "element {context} requires finite bounds for every input variable"
        ))
    })?;
    if actual.0 < bounds.0 || actual.1 > bounds.1 {
        return Err(ModelError::InvalidConstraint(format!(
            "element {context} declared bounds [{}, {}] do not cover inferred range [{}, {}]",
            bounds.0, bounds.1, actual.0, actual.1
        ))
        .into());
    }
    Ok(())
}

fn evaluate_linear<V>(poly: &Linear<V>, tokens: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant_term().clone();
    for monomial in poly.monomials() {
        let term = match tokens
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(value) => value,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        value = value + monomial.coefficient().clone() * term;
    }
    Some(value)
}

fn tighten_token_range<V>(token: &mut Token<V>, lower: f64, upper: f64) -> Result<()>
where
    V: Clone + Debug + Send + Sync + 'static + PartialOrd + FromPrimitive,
{
    let expected_lower: V = from_f64(lower, "auxiliary lower bound")?;
    let expected_upper: V = from_f64(upper, "auxiliary upper bound")?;
    let old = token.range();
    let lower = match old.lower_bound {
        Some(value) if value > expected_lower => value,
        _ => expected_lower,
    };
    let upper = match old.upper_bound {
        Some(value) if value < expected_upper => value,
        _ => expected_upper,
    };
    if lower > upper {
        return Err(ModelError::InvalidConstraint(format!(
            "element auxiliary token `{}` has incompatible range",
            token.name()
        ))
        .into());
    }
    token.set_range(VariableRange::bounded(lower, upper));
    Ok(())
}

/// 一个候选查表值及其有限范围 / One candidate lookup expression and its finite range.
#[derive(Debug, Clone)]
pub struct ElementValue<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 候选线性表达式 / Candidate linear expression.
    pub expression: Linear<V>,
    /// 候选下界 / Candidate lower bound.
    pub lower_bound: V,
    /// 候选上界 / Candidate upper bound.
    pub upper_bound: V,
}

impl<V> ElementValue<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    /// 创建带有限界的查表候选 / Create a lookup candidate with finite bounds.
    pub fn new(expression: Linear<V>, lower_bound: V, upper_bound: V) -> Result<Self> {
        validate_bounds(&lower_bound, &upper_bound, "candidate")?;
        Ok(Self {
            expression,
            lower_bound,
            upper_bound,
        })
    }
}

/// 通过整数索引从线性表达式表中选择一项 / Select a linear expression from a table using an integer index.
///
/// One-hot 变量绑定索引和值；越出表范围的索引会使模型不可行，纯求值返回 `None`。`lower_index`
/// 支持负数或非零起点。`LookupFunction` 为兼容别名。
/// One-hot selectors bind the index and selected value. An index outside the table makes the model
/// infeasible and evaluates to `None`. `lower_index` supports negative or nonzero table origins.
#[derive(Debug, Clone)]
pub struct ElementFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    index: IntegerVariableItem,
    index_bounds: (i64, i64),
    lower_index: i64,
    values: Vec<ElementValue<V>>,
    selectors: Vec<BinaryVariableItem>,
    masks: Vec<MaskingFunction<V>>,
    result_var: ContinuousVariableItem,
    result_bounds: (f64, f64),
    declared_dependency_ids: Vec<u64>,
}

/// 查找函数别名 / Lookup function alias.
pub type LookupFunction<V = f64> = ElementFunction<V>;

impl<V> ElementFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建整数索引查表符号 / Create an integer-indexed lookup symbol.
    pub fn new(
        id: u64,
        name: &str,
        index: IntegerVariableItem,
        values: Vec<ElementValue<V>>,
        lower_index: i64,
    ) -> Result<Self> {
        if values.is_empty() {
            return Err(ModelError::InvalidConstraint(format!(
                "element `{name}` requires a nonempty lookup table"
            ))
            .into());
        }
        let index_lower = index.range().lower_bound.ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "element `{name}` requires a finite index lower bound"
            ))
        })?;
        let index_upper = index.range().upper_bound.ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "element `{name}` requires a finite index upper bound"
            ))
        })?;
        if !index_lower.is_finite()
            || !index_upper.is_finite()
            || index_lower.fract() != 0.0
            || index_upper.fract() != 0.0
            || index_lower > index_upper
            || index_lower.abs() > 9_007_199_254_740_991.0
            || index_upper.abs() > 9_007_199_254_740_991.0
        {
            return Err(ModelError::InvalidConstraint(format!("element `{name}` requires finite integral index bounds within exact f64 range, got [{index_lower}, {index_upper}]" )).into());
        }
        let index_bounds = (index_lower as i64, index_upper as i64);
        let value_count = i64::try_from(values.len()).map_err(|_| {
            ModelError::InvalidConstraint(format!("element `{name}` table has too many entries"))
        })?;
        let upper_index = lower_index.checked_add(value_count - 1).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "element `{name}` table index range overflows i64"
            ))
        })?;
        if lower_index.unsigned_abs() > 9_007_199_254_740_991_u64
            || upper_index.unsigned_abs() > 9_007_199_254_740_991_u64
        {
            return Err(ModelError::InvalidConstraint(format!(
                "element `{name}` table indices exceed the exact f64 integer range"
            ))
            .into());
        }
        if upper_index < index_bounds.0 || lower_index > index_bounds.1 {
            return Err(ModelError::InvalidConstraint(format!(
                "element `{name}` table has no index in the declared index domain"
            ))
            .into());
        }

        let mut lower = f64::INFINITY;
        let mut upper = f64::NEG_INFINITY;
        for (position, value) in values.iter().enumerate() {
            let (value_lower, value_upper) = validate_bounds(
                &value.lower_bound,
                &value.upper_bound,
                &format!("candidate {position}"),
            )?;
            lower = lower.min(value_lower);
            upper = upper.max(value_upper);
        }
        if !lower.is_finite() || !upper.is_finite() {
            return Err(ModelError::InvalidConstraint(format!(
                "element `{name}` output range is not finite"
            ))
            .into());
        }

        let selector_group = new_group_id();
        let selectors = (0..values.len())
            .map(|position| {
                BinaryVariableItem::create(
                    VariableId::new(selector_group, position),
                    &format!("{name}_selector_{position}"),
                )
            })
            .collect::<Vec<_>>();
        let masks = values
            .iter()
            .zip(&selectors)
            .enumerate()
            .map(|(position, (value, selector))| {
                let (value_lower, value_upper) = validate_bounds(
                    &value.lower_bound,
                    &value.upper_bound,
                    &format!("candidate {position}"),
                )?;
                let big_m = value_lower.abs().max(value_upper.abs()).max(1.0);
                Ok(MaskingFunction::with_big_m(
                    next_auto_intermediate_symbol_id(),
                    &format!("{name}_masked_{position}"),
                    value.expression.clone(),
                    selector.clone(),
                    from_f64(big_m, "candidate masking bound")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(new_group_id(), 0),
            &format!("{name}_result"),
            VariableRange::bounded(lower, upper),
        );
        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            index,
            index_bounds,
            lower_index,
            values,
            selectors,
            masks,
            result_var,
            result_bounds: (lower, upper),
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 设置声明依赖 / Set declared dependencies.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取索引变量 / Get the index variable.
    pub fn index_variable(&self) -> &IntegerVariableItem {
        &self.index
    }

    /// 获取查表值 / Get lookup entries.
    pub fn values(&self) -> &[ElementValue<V>] {
        &self.values
    }

    /// 获取表格的起始索引 / Get the first index of the table.
    pub fn lower_index(&self) -> i64 {
        self.lower_index
    }
}

impl<V> ElementFunction<V>
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
        + FromPrimitive
        + PartialOrd,
    f64: IntoValue<V>,
{
    fn validate_values(&self, tokens: &[Token<V>]) -> Result<()> {
        let index_token = tokens
            .iter()
            .find(|token| token.id() == self.index.id())
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "element index variable `{}`",
                    self.index.id().unique_id()
                ))
            })?;
        if !index_token.var_type().is_integer() {
            return Err(ModelError::InvalidConstraint(format!(
                "element `{}` index token is not an integer variable",
                self.id.name
            ))
            .into());
        }
        let index_range = index_token.range();
        let lower = index_range
            .lower_bound
            .as_ref()
            .and_then(to_f64)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "element `{}` index token requires a finite lower bound",
                    self.id.name
                ))
            })?;
        let upper = index_range
            .upper_bound
            .as_ref()
            .and_then(to_f64)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "element `{}` index token requires a finite upper bound",
                    self.id.name
                ))
            })?;
        if !lower.is_finite()
            || !upper.is_finite()
            || lower.fract() != 0.0
            || upper.fract() != 0.0
            || lower > upper
            || lower < self.index_bounds.0 as f64
            || upper > self.index_bounds.1 as f64
        {
            return Err(ModelError::InvalidConstraint(format!(
                "element `{}` registered index range [{lower}, {upper}] is nonintegral, unbounded, or wider than the captured domain [{}, {}]",
                self.id.name, self.index_bounds.0, self.index_bounds.1
            )).into());
        }
        for (position, value) in self.values.iter().enumerate() {
            let bounds = validate_bounds(
                &value.lower_bound,
                &value.upper_bound,
                &format!("candidate {position}"),
            )?;
            validate_declared_bounds(
                &value.expression,
                bounds,
                tokens,
                &format!("candidate {position}"),
            )?;
        }
        Ok(())
    }

    fn mechanism(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "element result variable `{}`",
                    self.result_var.id().unique_id()
                ))
            })?;
        let index_index = symbol_to_index
            .get(&(self.index.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "element index variable `{}`",
                    self.index.id().unique_id()
                ))
            })?;
        let mut constraints = Vec::new();
        let mut selector_sum = Vec::with_capacity(self.selectors.len());
        let mut index_binding = vec![LinearMonomial::new(
            from_f64(-1.0, "index binding coefficient")?,
            index_index,
        )];
        let mut output_binding = vec![LinearMonomial::new(
            from_f64(1.0, "output coefficient")?,
            result_index,
        )];

        for (position, (selector, mask)) in self.selectors.iter().zip(&self.masks).enumerate() {
            let selector_index = symbol_to_index
                .get(&(selector.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "element selector `{}`",
                        selector.id().unique_id()
                    ))
                })?;
            let mask_index = symbol_to_index
                .get(&(mask.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "element mask result `{}`",
                        mask.result_variable().id().unique_id()
                    ))
                })?;
            selector_sum.push(LinearMonomial::new(
                from_f64(1.0, "selector coefficient")?,
                selector_index,
            ));
            let position = i64::try_from(position).map_err(|_| {
                ModelError::InvalidConstraint("element table position exceeds i64".to_string())
            })?;
            let table_index = self.lower_index.checked_add(position).ok_or_else(|| {
                ModelError::InvalidConstraint("element table index overflow".to_string())
            })?;
            index_binding.push(LinearMonomial::new(
                from_f64(table_index as f64, "table index")?,
                selector_index,
            ));
            output_binding.push(LinearMonomial::new(
                from_f64(-1.0, "masked output coefficient")?,
                mask_index,
            ));

            let mut mask_constraints = mask.mechanism_constraints(symbol_to_index)?;
            for constraint in &mut mask_constraints {
                constraint.set_from(Arc::new(self.clone()));
            }
            constraints.extend(mask_constraints);
        }

        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(selector_sum, from_f64(0.0, "selector sum constant")?),
                ConstraintRelation::Equal,
                from_f64(1.0, "one-hot rhs")?,
            ),
            &format!("{}_one_hot", self.id.name),
            source.clone(),
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(index_binding, from_f64(0.0, "index binding constant")?),
                ConstraintRelation::Equal,
                from_f64(0.0, "index binding rhs")?,
            ),
            &format!("{}_index_binding", self.id.name),
            source.clone(),
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(output_binding, from_f64(0.0, "output binding constant")?),
                ConstraintRelation::Equal,
                from_f64(0.0, "output binding rhs")?,
            ),
            &format!("{}_output_binding", self.id.name),
            source,
        ));
        Ok(constraints)
    }
}

impl<V> Display for ElementFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "element({})", self.id.name)
    }
}

impl<V> DynSymbol for ElementFunction<V>
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

impl<V> Symbol for ElementFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ElementFunction<V>
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
        + FromPrimitive
        + PartialOrd,
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
    fn register_auxiliary_tokens_with_context(
        &self,
        tokens: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        self.validate_values(registered)?;
        <Self as IntermediateSymbol<V>>::register_auxiliary_tokens(self, tokens)
    }
    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.mechanism(symbol_to_index)
    }
    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.validate_values(tokens)?;
        self.mechanism(symbol_to_index)
    }
    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }
    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }
    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("element({})", self.id.name)
    }
    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        self.validate_values(tokens)?;
        for token in auxiliary {
            if token.id() == self.result_var.id() {
                tighten_token_range(token, self.result_bounds.0, self.result_bounds.1)?;
            }
            for (position, mask) in self.masks.iter().enumerate() {
                if token.id() == mask.result_variable().id() {
                    let value = &self.values[position];
                    let (lower, upper) =
                        validate_bounds(&value.lower_bound, &value.upper_bound, "candidate")?;
                    tighten_token_range(token, lower.min(0.0), upper.max(0.0))?;
                }
            }
            if self
                .selectors
                .iter()
                .any(|selector| selector.id() == token.id())
            {
                tighten_token_range(token, 0.0, 1.0)?;
            }
        }
        Ok(())
    }
}

impl<V> FunctionSymbol<V> for ElementFunction<V>
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
        + FromPrimitive
        + PartialOrd,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for selector in &self.selectors {
            tokens.push(Token::from_generic(selector.clone(), selector.index()));
        }
        for mask in &self.masks {
            tokens.push(Token::from_generic(
                mask.result_variable().clone(),
                mask.result_variable().index(),
            ));
        }
        Ok(())
    }

    fn calculate_value(&self, tokens: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let index_value = match tokens
            .find_by_id(self.index.id())
            .and_then(|token| token.get_result())
        {
            Some(value) => value,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        let index = to_f64(&index_value)?;
        if !index.is_finite() || index.fract() != 0.0 || index.abs() > 9_007_199_254_740_991.0 {
            return None;
        }
        let index = index as i64;
        if index < self.index_bounds.0 || index > self.index_bounds.1 {
            return None;
        }
        let position = index.checked_sub(self.lower_index)?;
        if position < 0 {
            return None;
        }
        let value = self.values.get(position as usize)?;
        let (lower, upper) =
            validate_bounds(&value.lower_bound, &value.upper_bound, "candidate").ok()?;
        let result = evaluate_linear(&value.expression, tokens, zero_if_none)?;
        let result_f64 = to_f64(&result)?;
        if !result_f64.is_finite() || result_f64 < lower || result_f64 > upper {
            return None;
        }
        Some(result)
    }
}

impl<V> LinearIntermediateSymbol<V> for ElementFunction<V>
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
        + FromPrimitive
        + PartialOrd
        + One,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::VariableId;

    fn constraints_hold(constraints: &[LinearConstraint<f64>], values: &[f64]) -> bool {
        constraints.iter().all(|constraint| {
            let polynomial = &constraint.inequality.polynomial;
            let lhs = polynomial
                .monomials()
                .iter()
                .fold(*polynomial.constant_term(), |lhs, monomial| {
                    lhs + monomial.coefficient() * values[monomial.var_index()]
                });
            let rhs = constraint.inequality.rhs;
            match constraint.inequality.relation {
                ConstraintRelation::LessEqual => lhs <= rhs + 1e-10,
                ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-10,
                ConstraintRelation::GreaterEqual => lhs + 1e-10 >= rhs,
            }
        })
    }

    #[test]
    fn element_supports_negative_indices_and_rejects_out_of_range_evaluation() {
        let index = IntegerVariableItem::with_range(
            VariableId::standalone(121),
            "index",
            VariableRange::bounded(-1.0, 1.0),
        );
        let values = vec![
            ElementValue::new(Linear::constant(4.0), 4.0, 4.0).unwrap(),
            ElementValue::new(Linear::constant(7.0), 7.0, 7.0).unwrap(),
            ElementValue::new(Linear::constant(9.0), 9.0, 9.0).unwrap(),
        ];
        let function = ElementFunction::new(121, "lookup", index.clone(), values, -1).unwrap();
        let mut tokens = VecTokenList::<f64>::new();
        let index_token = Token::from_generic(index, 0);
        tokens.add_token(index_token);
        tokens.find_by_index(0).unwrap().set_result(-1.0);
        assert_eq!(function.calculate_value(&tokens, false), Some(4.0));
        tokens.find_by_index(0).unwrap().set_result(1.0);
        assert_eq!(function.calculate_value(&tokens, false), Some(9.0));
        tokens.find_by_index(0).unwrap().set_result(2.0);
        assert_eq!(function.calculate_value(&tokens, false), None);
    }

    #[test]
    fn element_rejects_empty_table_and_unbounded_index() {
        let bounded = IntegerVariableItem::with_range(
            VariableId::standalone(122),
            "index",
            VariableRange::bounded(0.0, 1.0),
        );
        assert!(ElementFunction::<f64>::new(122, "empty", bounded, Vec::new(), 0).is_err());
        let unbounded = IntegerVariableItem::auto("unbounded");
        let values = vec![ElementValue::new(Linear::constant(1.0), 1.0, 1.0).unwrap()];
        assert!(ElementFunction::new(123, "unbounded_index", unbounded, values, 0).is_err());
    }

    #[test]
    fn negative_index_expression_table_rows_select_the_matching_candidate() {
        let index = IntegerVariableItem::with_range(
            VariableId::standalone(124),
            "index",
            VariableRange::bounded(-1.0, 0.0),
        );
        let input = ContinuousVariableItem::with_range(
            VariableId::standalone(125),
            "input",
            VariableRange::bounded(-2.0, 2.0),
        );
        let values = vec![
            ElementValue::new(
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 5.0),
                3.0,
                7.0,
            )
            .unwrap(),
            ElementValue::new(
                Linear::new(vec![LinearMonomial::new(2.0, 1)], -1.0),
                -5.0,
                3.0,
            )
            .unwrap(),
        ];
        let function =
            ElementFunction::new(124, "negative_table", index.clone(), values, -1).unwrap();

        let mut indices = HashMap::from([
            (index.id().unique_id() as usize, 0),
            (input.id().unique_id() as usize, 1),
            (function.result_variable().id().unique_id() as usize, 2),
        ]);
        for (position, selector) in function.selectors.iter().enumerate() {
            indices.insert(selector.id().unique_id() as usize, 3 + position);
        }
        for (position, mask) in function.masks.iter().enumerate() {
            indices.insert(
                mask.result_variable().id().unique_id() as usize,
                5 + position,
            );
        }
        let rows = function.mechanism_constraints(&indices).unwrap();

        let at_negative_index = [-1.0, 1.0, 6.0, 1.0, 0.0, 6.0, 0.0];
        let at_zero_index = [0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        assert!(constraints_hold(&rows, &at_negative_index));
        assert!(constraints_hold(&rows, &at_zero_index));

        let wrong_candidate = [-1.0, 1.0, 1.0, 1.0, 0.0, 6.0, 0.0];
        assert!(!constraints_hold(&rows, &wrong_candidate));
    }
}
