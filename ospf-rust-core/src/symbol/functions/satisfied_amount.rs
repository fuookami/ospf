//! Satisfied amount function symbol.

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use num_traits::{FromPrimitive, ToPrimitive};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, new_standalone_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};

fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
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

/// Counts the number of satisfied indicator constraints.
#[derive(Debug, Clone)]
pub struct SatisfiedAmountFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    indicators: Vec<BinaryVariableItem>,
    result_var: ContinuousVariableItem,
    amount_lower: Option<usize>,
    amount_upper: Option<usize>,
    declared_dependency_ids: Vec<u64>,
    _marker: std::marker::PhantomData<V>,
}

impl<V> SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self {
        let result_var = ContinuousVariableItem::create(new_standalone_id(), name);

        Self {
            id: IntermediateSymbolId::new(id, name),
            indicators,
            result_var,
            amount_lower: None,
            amount_upper: None,
            declared_dependency_ids: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建满足数量函数。
    /// Create a satisfied-amount function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, indicators: Vec<BinaryVariableItem>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
    }

    /// 使用自动 ID 与自动名称创建满足数量函数。
    /// Create a satisfied-amount function with an auto id and auto-generated name.
    pub fn auto(indicators: Vec<BinaryVariableItem>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("satisfied_amount", id);
        Self::new(id, &name, indicators)
    }

    /// 限制满足数量范围。
    /// Limit the satisfied-count range.
    pub fn with_amount_range(mut self, lower: Option<usize>, upper: Option<usize>) -> Self {
        self.amount_lower = lower;
        self.amount_upper = upper;
        self
    }

    /// 至少一个 indicator 被满足。
    /// At least one indicator is satisfied.
    pub fn any(indicators: Vec<BinaryVariableItem>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("any", id);
        Self::new(id, &name, indicators).with_amount_range(Some(1), None)
    }

    /// 带名称的 any 构造器。
    /// Named constructor for any.
    pub fn named_any(name: impl AsRef<str>, indicators: Vec<BinaryVariableItem>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
        .with_amount_range(Some(1), None)
    }

    /// 全部 indicator 被满足。
    /// All indicators are satisfied.
    pub fn all(indicators: Vec<BinaryVariableItem>) -> Self {
        let amount = indicators.len();
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("all", id);
        Self::new(id, &name, indicators).with_amount_range(Some(amount), Some(amount))
    }

    /// 带名称的 all 构造器。
    /// Named constructor for all.
    pub fn named_all(name: impl AsRef<str>, indicators: Vec<BinaryVariableItem>) -> Self {
        let amount = indicators.len();
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
        .with_amount_range(Some(amount), Some(amount))
    }

    /// 至少 `amount` 个 indicator 被满足。
    /// At least `amount` indicators are satisfied.
    pub fn at_least(indicators: Vec<BinaryVariableItem>, amount: usize) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("at_least", id);
        Self::new(id, &name, indicators).with_amount_range(Some(amount), None)
    }

    /// 带名称的 at_least 构造器。
    /// Named constructor for at_least.
    pub fn named_at_least(
        name: impl AsRef<str>,
        indicators: Vec<BinaryVariableItem>,
        amount: usize,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
        .with_amount_range(Some(amount), None)
    }

    /// 不是全部 indicator 都被满足。
    /// Not all indicators are satisfied.
    pub fn not_all(indicators: Vec<BinaryVariableItem>) -> Self {
        let upper = indicators.len().saturating_sub(1);
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("not_all", id);
        Self::new(id, &name, indicators).with_amount_range(None, Some(upper))
    }

    /// 带名称的 not_all 构造器。
    /// Named constructor for not_all.
    pub fn named_not_all(name: impl AsRef<str>, indicators: Vec<BinaryVariableItem>) -> Self {
        let upper = indicators.len().saturating_sub(1);
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
        .with_amount_range(None, Some(upper))
    }

    /// 满足数量位于 `[lower, upper]`。
    /// Satisfied count is within `[lower, upper]`.
    pub fn numerable(indicators: Vec<BinaryVariableItem>, lower: usize, upper: usize) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("numerable", id);
        Self::new(id, &name, indicators).with_amount_range(Some(lower), Some(upper))
    }

    /// 带名称的 numerable 构造器。
    /// Named constructor for numerable.
    pub fn named_numerable(
        name: impl AsRef<str>,
        indicators: Vec<BinaryVariableItem>,
        lower: usize,
        upper: usize,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
        .with_amount_range(Some(lower), Some(upper))
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicators
    }

    pub fn amount_range(&self) -> (Option<usize>, Option<usize>) {
        (self.amount_lower, self.amount_upper)
    }
}

impl<V> Display for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "satisfied_amount({})", self.id.name)
    }
}

impl<V> DynSymbol for SatisfiedAmountFunction<V>
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

impl<V> Symbol for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
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
        if let (Some(lower), Some(upper)) = (self.amount_lower, self.amount_upper)
            && lower > upper
        {
            return Err(ModelError::InvalidConstraint(format!(
                "satisfied amount `{}` has invalid amount range [{}, {}]",
                self.id.name, lower, upper
            ))
            .into());
        }
        if let Some(lower) = self.amount_lower
            && lower > self.indicators.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "satisfied amount `{}` lower bound {} exceeds indicator count {}",
                self.id.name,
                lower,
                self.indicators.len()
            ))
            .into());
        }
        if let Some(upper) = self.amount_upper
            && upper > self.indicators.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "satisfied amount `{}` upper bound {} exceeds indicator count {}",
                self.id.name,
                upper,
                self.indicators.len()
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "satisfied amount result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut monomials = Vec::with_capacity(self.indicators.len() + 1);
        monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "satisfied amount result coefficient")?,
            result_index,
        ));
        for indicator in &self.indicators {
            let indicator_index = symbol_to_index
                .get(&(indicator.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "satisfied amount indicator variable id {}",
                        indicator.id().unique_id()
                    ))
                })?;
            monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "satisfied amount indicator coefficient")?,
                indicator_index,
            ));
        }

        let equality = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    monomials,
                    convert_f64_to_v::<V>(0.0, "satisfied amount constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "satisfied amount rhs")?,
            ),
            &format!("{}_sat_amount", self.id.name),
            Arc::new(self.clone()),
        );

        let mut constraints = vec![equality];
        if let Some(lower) = self.amount_lower {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "satisfied amount lower coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "satisfied amount lower constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(lower as f64, "satisfied amount lower rhs")?,
                ),
                &format!("{}_sat_amount_lb", self.id.name),
                Arc::new(self.clone()),
            ));
        }
        if let Some(upper) = self.amount_upper {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "satisfied amount upper coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "satisfied amount upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(upper as f64, "satisfied amount upper rhs")?,
                ),
                &format!("{}_sat_amount_ub", self.id.name),
                Arc::new(self.clone()),
            ));
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

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("satisfied_amount({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for var in &self.indicators {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut count = 0.0_f64;
        for indicator in &self.indicators {
            let value = match token_table
                .find_by_id(indicator.id())
                .and_then(|token| token.get_result())
            {
                Some(v) => v,
                None if zero_if_none => from_f64(0.0)?,
                None => return None,
            };
            if to_f64(&value)?.abs() > f64::EPSILON {
                count += 1.0;
            }
        }
        from_f64(count)
    }
}

impl<V> LinearIntermediateSymbol<V> for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        let monomials: Vec<_> = self
            .indicators
            .iter()
            .map(|var| LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), var.index()))
            .collect();
        Linear::new(monomials, from_f64(0.0).expect("convert 0.0"))
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{BinaryVariableItem, VariableId};

    fn binary_token(
        tokens: &mut VecTokenList<f64>,
        id: usize,
        solver_index: usize,
        name: &str,
        value: f64,
    ) -> BinaryVariableItem {
        let variable = BinaryVariableItem::create(VariableId::standalone(id), name);
        let token = Token::from_generic(variable.clone(), solver_index);
        token.set_result(value);
        tokens.add_token(token);
        variable
    }

    #[test]
    fn satisfied_amount_does_not_count_zero_indicators() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 20_000, 0, "sat_zero_b0", 0.0);
        let b1 = binary_token(&mut tokens, 20_001, 1, "sat_zero_b1", 1.0);
        let b2 = binary_token(&mut tokens, 20_002, 2, "sat_zero_b2", -1.0);

        let function = SatisfiedAmountFunction::new(9200, "sat_zero", vec![b0, b1, b2]);
        let value =
            <SatisfiedAmountFunction as FunctionSymbol>::calculate_value(&function, &tokens, false)
                .expect("satisfied amount should be evaluated");

        assert_eq!(value, 2.0);
    }
}
