//! 二值基数函数 / Binary cardinality functions

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    next_auto_intermediate_symbol_id,
};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::Sub;
use std::sync::Arc;

const MAX_EXACT_CARDINALITY: u128 = 9_007_199_254_740_991;

fn validate_cardinality(indicator_count: usize, amount: usize, operation: &str) -> Result<()> {
    if amount > indicator_count {
        return Err(ModelError::InvalidConstraint(format!(
            "{operation} count {amount} must be between 0 and the input count {indicator_count}"
        ))
        .into());
    }
    if indicator_count as u128 > MAX_EXACT_CARDINALITY {
        return Err(ModelError::InvalidConstraint(format!(
            "{operation} input count exceeds the exact integer range supported by f64 coefficients"
        ))
        .into());
    }
    Ok(())
}

fn count_value<V>(value: usize, context: &str) -> Result<V>
where
    V: FromPrimitive + ToPrimitive,
{
    if value as u128 > MAX_EXACT_CARDINALITY {
        return Err(ModelError::InvalidConstraint(format!(
            "{context} exceeds the exact integer range supported by f64 coefficients"
        ))
        .into());
    }
    let converted = V::from_usize(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "cannot convert {context}={value} into the model value type"
        ))
    })?;
    if converted.to_usize() != Some(value) || converted.to_f64() != Some(value as f64) {
        return Err(ModelError::InvalidConstraint(format!(
            "{context}={value} cannot be represented exactly by the model value type"
        ))
        .into());
    }
    Ok(converted)
}

fn validate_indicators<V>(
    indicators: &[BinaryVariableItem],
    tokens: &[Token<V>],
    name: &str,
) -> Result<()>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    for indicator in indicators {
        let token = tokens
            .iter()
            .find(|token| token.id() == indicator.id())
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "cardinality `{name}` input variable `{}`",
                    indicator.id().unique_id()
                ))
            })?;
        if !token.var_type().is_binary() {
            return Err(ModelError::InvalidConstraint(format!(
                "cardinality `{name}` input `{}` is not registered as binary",
                indicator.name()
            ))
            .into());
        }
    }
    Ok(())
}

fn pure_count_constraint<V>(
    indicators: &[BinaryVariableItem],
    amount: usize,
    relation: ConstraintRelation,
    symbol_to_index: &HashMap<usize, usize>,
    name: &str,
    operation: &str,
) -> Result<Vec<LinearConstraint<V>>>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive + Zero + One,
{
    validate_cardinality(indicators.len(), amount, operation)?;
    let monomials = indicators
        .iter()
        .map(|indicator| {
            let index = symbol_to_index
                .get(&(indicator.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "cardinality `{name}` input variable `{}`",
                        indicator.id().unique_id()
                    ))
                })?;
            Ok(LinearMonomial::new(V::one(), index))
        })
        .collect::<Result<Vec<_>>>()?;
    let polynomial = Linear::new(monomials, V::zero());
    let inequality = LinearInequality::new(
        polynomial,
        relation,
        count_value(amount, &format!("{operation} right-hand side"))?,
    );
    Ok(vec![LinearConstraint::new(inequality, name)])
}

/// 构造纯约束 `sum(indicators) <= limit`，不引入满足指示变量。
/// Build the hard constraint `sum(indicators) <= limit` without a satisfaction indicator.
pub fn at_most_constraints<V>(
    indicators: &[BinaryVariableItem],
    limit: usize,
    symbol_to_index: &HashMap<usize, usize>,
    name: &str,
) -> Result<Vec<LinearConstraint<V>>>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive + Zero + One,
{
    pure_count_constraint(
        indicators,
        limit,
        ConstraintRelation::LessEqual,
        symbol_to_index,
        name,
        "AtMost",
    )
}

/// 构造纯约束 `sum(indicators) == amount`，不引入满足指示变量。
/// Build the hard constraint `sum(indicators) == amount` without a satisfaction indicator.
pub fn exactly_constraints<V>(
    indicators: &[BinaryVariableItem],
    amount: usize,
    symbol_to_index: &HashMap<usize, usize>,
    name: &str,
) -> Result<Vec<LinearConstraint<V>>>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive + Zero + One,
{
    pure_count_constraint(
        indicators,
        amount,
        ConstraintRelation::Equal,
        symbol_to_index,
        name,
        "Exactly",
    )
}

#[derive(Debug, Clone, Copy)]
enum CardinalityMode {
    AtMost { limit: usize },
    Exactly { amount: usize },
}

#[derive(Debug, Clone)]
struct CardinalityCore<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    indicators: Vec<BinaryVariableItem>,
    mode: CardinalityMode,
    result_var: BinaryVariableItem,
    below_var: Option<BinaryVariableItem>,
    above_var: Option<BinaryVariableItem>,
    declared_dependency_ids: Vec<u64>,
    _marker: std::marker::PhantomData<V>,
}

impl<V> CardinalityCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn new(
        id: u64,
        name: &str,
        indicators: Vec<BinaryVariableItem>,
        mode: CardinalityMode,
    ) -> Result<Self> {
        let amount = match mode {
            CardinalityMode::AtMost { limit } => limit,
            CardinalityMode::Exactly { amount } => amount,
        };
        validate_cardinality(indicators.len(), amount, "cardinality")?;

        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{name}_satisfied"));
        let mut next_index = 1;
        let below_var = if matches!(mode, CardinalityMode::Exactly { amount } if amount > 0) {
            let variable = BinaryVariableItem::create(
                VariableId::new(group_id, next_index),
                &format!("{name}_below"),
            );
            next_index += 1;
            Some(variable)
        } else {
            None
        };
        let above_var = if matches!(mode, CardinalityMode::Exactly { amount } if amount < indicators.len())
        {
            Some(BinaryVariableItem::create(
                VariableId::new(group_id, next_index),
                &format!("{name}_above"),
            ))
        } else {
            None
        };

        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            indicators,
            mode,
            result_var,
            below_var,
            above_var,
            declared_dependency_ids: Vec::new(),
            _marker: std::marker::PhantomData,
        })
    }

    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()>
    where
        V: PartialOrd + FromPrimitive + ToPrimitive,
    {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        if let Some(variable) = &self.below_var {
            tokens.push(Token::from_generic(variable.clone(), variable.index()));
        }
        if let Some(variable) = &self.above_var {
            tokens.push(Token::from_generic(variable.clone(), variable.index()));
        }
        Ok(())
    }

    fn evaluate(&self, tokens: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>
    where
        V: One + ToPrimitive + Zero,
    {
        let mut count = 0usize;
        for indicator in &self.indicators {
            let value = match tokens
                .find_by_id(indicator.id())
                .and_then(|token| token.get_result())
            {
                Some(value) => value,
                None if zero_if_none => V::zero(),
                None => return None,
            };
            let value = value.to_f64()?;
            if value == 1.0 {
                count += 1;
            } else if value != 0.0 {
                return None;
            }
        }
        let satisfied = match self.mode {
            CardinalityMode::AtMost { limit } => count <= limit,
            CardinalityMode::Exactly { amount } => count == amount,
        };
        Some(if satisfied { V::one() } else { V::zero() })
    }

    fn validate_registered(&self, tokens: &[Token<V>]) -> Result<()> {
        validate_indicators(&self.indicators, tokens, &self.id.name)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn mechanism(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        source: Arc<dyn IntermediateSymbol<V>>,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: FromPrimitive + ToPrimitive + One + Zero + Sub<Output = V>,
    {
        let mut input_indexes = Vec::with_capacity(self.indicators.len());
        for indicator in &self.indicators {
            input_indexes.push(
                symbol_to_index
                    .get(&(indicator.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "cardinality `{}` input variable `{}`",
                            self.id.name,
                            indicator.id().unique_id()
                        ))
                    })?,
            );
        }
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "cardinality `{}` result variable `{}`",
                    self.id.name,
                    self.result_var.id().unique_id()
                ))
            })?;
        let sum = Linear::new(
            input_indexes
                .iter()
                .map(|index| LinearMonomial::new(V::one(), *index))
                .collect(),
            V::zero(),
        );
        let mut constraints = Vec::new();
        let mut push_row = |polynomial: Linear<V>, relation, rhs: V, suffix: &str| {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(polynomial, relation, rhs),
                &format!("{}_{}", self.id.name, suffix),
                source.clone(),
            ));
        };

        match self.mode {
            CardinalityMode::AtMost { limit } => {
                if limit == self.indicators.len() {
                    push_row(
                        Linear::new(vec![LinearMonomial::new(V::one(), result_index)], V::zero()),
                        ConstraintRelation::Equal,
                        V::one(),
                        "always_true",
                    );
                } else {
                    let count = self.indicators.len();
                    let mut upper = sum.monomials().to_vec();
                    upper.push(LinearMonomial::new(
                        count_value::<V>(count - limit, "AtMost upper coefficient")?,
                        result_index,
                    ));
                    push_row(
                        Linear::new(upper, V::zero()),
                        ConstraintRelation::LessEqual,
                        count_value::<V>(count, "AtMost upper right-hand side")?,
                        "upper",
                    );

                    let mut lower = sum.monomials().to_vec();
                    lower.push(LinearMonomial::new(
                        count_value::<V>(limit + 1, "AtMost lower coefficient")?,
                        result_index,
                    ));
                    push_row(
                        Linear::new(lower, V::zero()),
                        ConstraintRelation::GreaterEqual,
                        count_value::<V>(limit + 1, "AtMost lower right-hand side")?,
                        "lower",
                    );
                }
            }
            CardinalityMode::Exactly { amount } => {
                let below_index = self
                    .below_var
                    .as_ref()
                    .map(|variable| {
                        symbol_to_index
                            .get(&(variable.id().unique_id() as usize))
                            .copied()
                            .ok_or_else(|| {
                                ModelError::SymbolNotRegistered(format!(
                                    "cardinality `{}` below branch `{}`",
                                    self.id.name,
                                    variable.id().unique_id()
                                ))
                            })
                    })
                    .transpose()?;
                let above_index = self
                    .above_var
                    .as_ref()
                    .map(|variable| {
                        symbol_to_index
                            .get(&(variable.id().unique_id() as usize))
                            .copied()
                            .ok_or_else(|| {
                                ModelError::SymbolNotRegistered(format!(
                                    "cardinality `{}` above branch `{}`",
                                    self.id.name,
                                    variable.id().unique_id()
                                ))
                            })
                    })
                    .transpose()?;

                let mut branch_terms = vec![LinearMonomial::new(V::one(), result_index)];
                if let Some(index) = below_index {
                    branch_terms.push(LinearMonomial::new(V::one(), index));
                }
                if let Some(index) = above_index {
                    branch_terms.push(LinearMonomial::new(V::one(), index));
                }
                push_row(
                    Linear::new(branch_terms, V::zero()),
                    ConstraintRelation::Equal,
                    V::one(),
                    "one_branch",
                );

                let count = self.indicators.len();
                let mut exact_upper = sum.monomials().to_vec();
                exact_upper.push(LinearMonomial::new(
                    count_value::<V>(count - amount, "Exactly upper coefficient")?,
                    result_index,
                ));
                push_row(
                    Linear::new(exact_upper, V::zero()),
                    ConstraintRelation::LessEqual,
                    count_value::<V>(count, "Exactly upper right-hand side")?,
                    "exact_upper",
                );

                let mut exact_lower = sum.monomials().to_vec();
                exact_lower.push(LinearMonomial::new(
                    V::zero() - count_value::<V>(amount, "Exactly lower coefficient")?,
                    result_index,
                ));
                push_row(
                    Linear::new(exact_lower, V::zero()),
                    ConstraintRelation::GreaterEqual,
                    V::zero(),
                    "exact_lower",
                );

                if let Some(index) = below_index {
                    let coefficient = count - amount + 1;
                    let mut below = sum.monomials().to_vec();
                    below.push(LinearMonomial::new(
                        count_value::<V>(coefficient, "Exactly below coefficient")?,
                        index,
                    ));
                    push_row(
                        Linear::new(below, V::zero()),
                        ConstraintRelation::LessEqual,
                        count_value::<V>(count, "Exactly below right-hand side")?,
                        "below_branch",
                    );
                }

                if let Some(index) = above_index {
                    let coefficient = amount + 1;
                    let mut above = sum.monomials().to_vec();
                    above.push(LinearMonomial::new(
                        V::zero() - count_value::<V>(coefficient, "Exactly above coefficient")?,
                        index,
                    ));
                    push_row(
                        Linear::new(above, V::zero()),
                        ConstraintRelation::GreaterEqual,
                        V::zero(),
                        "above_branch",
                    );
                }
            }
        }
        Ok(constraints)
    }
}

/// 精确指示输入中至多 `limit` 个为 1 的函数。
/// Exact indicator that at most `limit` inputs equal 1.
#[derive(Debug, Clone)]
pub struct AtMostFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CardinalityCore<V>,
}

impl<V> AtMostFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(
        id: u64,
        name: &str,
        indicators: Vec<BinaryVariableItem>,
        limit: usize,
    ) -> Result<Self> {
        Ok(Self {
            core: CardinalityCore::new(id, name, indicators, CardinalityMode::AtMost { limit })?,
        })
    }

    pub fn named(
        name: impl AsRef<str>,
        indicators: Vec<BinaryVariableItem>,
        limit: usize,
    ) -> Result<Self> {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
            limit,
        )
    }

    pub fn auto(indicators: Vec<BinaryVariableItem>, limit: usize) -> Result<Self> {
        let id = next_auto_intermediate_symbol_id();
        Self::new(id, &format!("at_most_{id}"), indicators, limit)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.core.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn indicators(&self) -> &[BinaryVariableItem] {
        &self.core.indicators
    }

    pub fn limit(&self) -> usize {
        match self.core.mode {
            CardinalityMode::AtMost { limit } => limit,
            CardinalityMode::Exactly { .. } => unreachable!(),
        }
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.core.result_var
    }
}

/// 精确指示输入中恰有 `amount` 个为 1 的函数。
/// Exact indicator that exactly `amount` inputs equal 1.
#[derive(Debug, Clone)]
pub struct ExactlyFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CardinalityCore<V>,
}

impl<V> ExactlyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(
        id: u64,
        name: &str,
        indicators: Vec<BinaryVariableItem>,
        amount: usize,
    ) -> Result<Self> {
        Ok(Self {
            core: CardinalityCore::new(id, name, indicators, CardinalityMode::Exactly { amount })?,
        })
    }

    pub fn named(
        name: impl AsRef<str>,
        indicators: Vec<BinaryVariableItem>,
        amount: usize,
    ) -> Result<Self> {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
            amount,
        )
    }

    pub fn auto(indicators: Vec<BinaryVariableItem>, amount: usize) -> Result<Self> {
        let id = next_auto_intermediate_symbol_id();
        Self::new(id, &format!("exactly_{id}"), indicators, amount)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.core.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn indicators(&self) -> &[BinaryVariableItem] {
        &self.core.indicators
    }

    pub fn amount(&self) -> usize {
        match self.core.mode {
            CardinalityMode::Exactly { amount } => amount,
            CardinalityMode::AtMost { .. } => unreachable!(),
        }
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.core.result_var
    }

    pub fn below_variable(&self) -> Option<&BinaryVariableItem> {
        self.core.below_var.as_ref()
    }

    pub fn above_variable(&self) -> Option<&BinaryVariableItem> {
        self.core.above_var.as_ref()
    }
}

macro_rules! impl_cardinality_symbol {
    ($symbol:ident, $operation:literal) => {
        impl<V> Display for $symbol<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", $operation, self.core.id.name)
            }
        }

        impl<V> DynSymbol for $symbol<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn name(&self) -> &str {
                &self.core.id.name
            }

            fn display_name(&self) -> &str {
                &self.core.id.name
            }

            fn dyn_id(&self) -> SymbolDynId<'_> {
                SymbolDynId::standalone(self.core.id.id as usize)
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl<V> Symbol for $symbol<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            type Id = IntermediateSymbolId;

            fn id(&self) -> Self::Id {
                self.core.id.clone()
            }
        }

        impl<V> IntermediateSymbol<V> for $symbol<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + ToPrimitive
                + FromPrimitive
                + PartialOrd
                + One
                + Zero
                + Sub<Output = V>,
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
                self.core.declared_dependency_ids.clone()
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
                self.core.validate_registered(registered)?;
                <Self as IntermediateSymbol<V>>::register_auxiliary_tokens(self, tokens)
            }

            fn mechanism_constraints(
                &self,
                symbol_to_index: &HashMap<usize, usize>,
            ) -> Result<Vec<LinearConstraint<V>>> {
                self.core.mechanism(symbol_to_index, Arc::new(self.clone()))
            }

            fn mechanism_constraints_with_tokens(
                &self,
                symbol_to_index: &HashMap<usize, usize>,
                tokens: &[Token<V>],
            ) -> Result<Vec<LinearConstraint<V>>> {
                self.core.validate_registered(tokens)?;
                self.core.mechanism(symbol_to_index, Arc::new(self.clone()))
            }

            fn evaluate_from_tokens(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
            }

            fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
                self.core.prepare(values)
            }

            fn to_raw_string(&self, _unfold: u64) -> String {
                format!("{}({})", $operation, self.core.id.name)
            }
        }

        impl<V> FunctionSymbol<V> for $symbol<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + ToPrimitive
                + FromPrimitive
                + PartialOrd
                + One
                + Zero
                + Sub<Output = V>,
            f64: IntoValue<V>,
        {
            fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                self.core.register_tokens(tokens)
            }

            fn calculate_value(&self, tokens: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
                self.core.evaluate(tokens, zero_if_none)
            }
        }

        impl<V> LinearIntermediateSymbol<V> for $symbol<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + ToPrimitive
                + FromPrimitive
                + PartialOrd
                + One
                + Zero
                + Sub<Output = V>,
            f64: IntoValue<V>,
        {
            fn to_linear_polynomial(&self) -> Linear<V> {
                Linear::new(
                    vec![LinearMonomial::new(V::one(), self.core.result_var.index())],
                    V::zero(),
                )
            }

            fn to_quadratic_polynomial(&self) -> Quadratic<V> {
                Quadratic::from_linear(&self.to_linear_polynomial())
            }
        }
    };
}

impl_cardinality_symbol!(AtMostFunction, "at_most");
impl_cardinality_symbol!(ExactlyFunction, "exactly");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};

    fn indicators(count: usize, first_id: usize) -> Vec<BinaryVariableItem> {
        (0..count)
            .map(|index| {
                BinaryVariableItem::create(
                    VariableId::standalone(first_id + index),
                    &format!("b{index}"),
                )
            })
            .collect()
    }

    fn mapping_for(
        inputs: &[BinaryVariableItem],
        helpers: &[&BinaryVariableItem],
    ) -> HashMap<usize, usize> {
        let mut mapping = HashMap::new();
        for (index, variable) in inputs.iter().enumerate() {
            mapping.insert(variable.id().unique_id() as usize, index);
        }
        for (offset, variable) in helpers.iter().enumerate() {
            mapping.insert(variable.id().unique_id() as usize, inputs.len() + offset);
        }
        mapping
    }

    fn tokens_with_values(inputs: &[BinaryVariableItem], values: &[f64]) -> VecTokenList<f64> {
        let mut tokens = VecTokenList::new();
        for (index, (variable, value)) in inputs.iter().zip(values).enumerate() {
            let token = Token::from_generic(variable.clone(), index);
            token.set_result(*value);
            tokens.add_token(token);
        }
        tokens
    }

    fn constraints_hold(constraint: &LinearConstraint<f64>, values: &[f64]) -> bool {
        let polynomial = &constraint.inequality.polynomial;
        let mut lhs = *polynomial.constant_term();
        for monomial in polynomial.monomials() {
            lhs += monomial.coefficient() * values[monomial.var_index()];
        }
        let rhs = constraint.inequality.rhs;
        match constraint.inequality.relation {
            ConstraintRelation::LessEqual => lhs <= rhs + 1e-10,
            ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-10,
            ConstraintRelation::GreaterEqual => lhs + 1e-10 >= rhs,
        }
    }

    #[test]
    fn at_most_mechanism_truth_table_matches_exact_indicator() {
        let inputs = indicators(3, 21_000);
        let function = AtMostFunction::<f64>::new(1001, "at_most_one", inputs.clone(), 1).unwrap();
        let helpers = [function.result_variable()];
        let mapping = mapping_for(&inputs, &helpers);
        let rows = function.mechanism_constraints(&mapping).unwrap();
        assert_eq!(rows.len(), 2);

        for mask in 0u32..(1u32 << inputs.len()) {
            let count = mask.count_ones() as usize;
            for result in 0..=1 {
                let mut values = vec![0.0; inputs.len() + 1];
                for (index, value) in values.iter_mut().enumerate().take(inputs.len()) {
                    *value = ((mask >> index) & 1) as f64;
                }
                values[inputs.len()] = result as f64;
                let feasible = rows.iter().all(|row| constraints_hold(row, &values));
                assert_eq!(feasible, (result == (count <= 1) as usize));
            }

            let input_values = (0..inputs.len())
                .map(|index| ((mask >> index) & 1) as f64)
                .collect::<Vec<_>>();
            let tokens = tokens_with_values(&inputs, &input_values);
            assert_eq!(
                function.calculate_value(&tokens, false),
                Some(if count <= 1 { 1.0 } else { 0.0 })
            );
        }
    }

    #[test]
    fn exactly_mechanism_truth_tables_cover_zero_middle_and_all_counts() {
        let inputs = indicators(3, 22_000);
        for amount in 0..=inputs.len() {
            let function = ExactlyFunction::<f64>::new(
                1100 + amount as u64,
                &format!("exactly_{amount}"),
                inputs.clone(),
                amount,
            )
            .unwrap();
            let mut helpers = vec![function.result_variable()];
            helpers.extend(function.below_variable());
            helpers.extend(function.above_variable());
            let mapping = mapping_for(&inputs, &helpers);
            let rows = function.mechanism_constraints(&mapping).unwrap();
            let helper_count = helpers.len();

            for mask in 0u32..(1u32 << inputs.len()) {
                let count = mask.count_ones() as usize;
                let mut feasible_results = 0;
                for branch_mask in 0..(1 << helper_count) {
                    let mut values = vec![0.0; inputs.len() + helper_count];
                    for (index, value) in values.iter_mut().enumerate().take(inputs.len()) {
                        *value = ((mask >> index) & 1) as f64;
                    }
                    for index in 0..helper_count {
                        values[inputs.len() + index] = ((branch_mask >> index) & 1) as f64;
                    }
                    let feasible = rows.iter().all(|row| constraints_hold(row, &values));
                    if feasible {
                        feasible_results += 1;
                        assert_eq!(
                            values[inputs.len()],
                            if count == amount { 1.0 } else { 0.0 }
                        );
                    }
                }
                assert_eq!(feasible_results, 1, "amount={amount}, input_mask={mask}");

                let input_values = (0..inputs.len())
                    .map(|index| ((mask >> index) & 1) as f64)
                    .collect::<Vec<_>>();
                let tokens = tokens_with_values(&inputs, &input_values);
                assert_eq!(
                    function.calculate_value(&tokens, false),
                    Some(if count == amount { 1.0 } else { 0.0 })
                );
            }
        }
    }

    #[test]
    fn cardinality_handles_empty_inputs_and_rejects_nonbinary_values() {
        let at_most = AtMostFunction::<f64>::new(1201, "empty_at_most", Vec::new(), 0).unwrap();
        let exactly = ExactlyFunction::<f64>::new(1202, "empty_exactly", Vec::new(), 0).unwrap();
        let tokens = VecTokenList::<f64>::new();
        assert_eq!(at_most.calculate_value(&tokens, false), Some(1.0));
        assert_eq!(exactly.calculate_value(&tokens, false), Some(1.0));

        let inputs = indicators(1, 23_000);
        let at_most = AtMostFunction::<f64>::new(1203, "one_input", inputs.clone(), 1).unwrap();
        let tokens = tokens_with_values(&inputs, &[0.5]);
        assert_eq!(at_most.calculate_value(&tokens, false), None);
        let token = tokens.find_by_id(inputs[0].id()).unwrap();
        token.set_result(2.0);
        assert_eq!(at_most.calculate_value(&tokens, false), None);

        let continuous =
            crate::variable::ContinuousVariableItem::create(inputs[0].id(), "wrong_type");
        let wrong_token = Token::from_generic(continuous, 0);
        assert!(
            at_most
                .register_auxiliary_tokens_with_context(&mut Vec::new(), &[wrong_token])
                .is_err()
        );
    }

    #[test]
    fn pure_cardinality_helpers_build_hard_rows_and_validate_counts() {
        let group = new_group_id();
        let inputs = (0..3)
            .map(|index| {
                BinaryVariableItem::create(VariableId::new(group, index), &format!("pure_b{index}"))
            })
            .collect::<Vec<_>>();
        let mapping = inputs
            .iter()
            .enumerate()
            .map(|(index, variable)| (variable.id().unique_id() as usize, index))
            .collect::<HashMap<_, _>>();
        let at_most = at_most_constraints::<f64>(&inputs, 2, &mapping, "pure_at_most").unwrap();
        let exactly = exactly_constraints::<f64>(&inputs, 2, &mapping, "pure_exactly").unwrap();
        assert_eq!(at_most.len(), 1);
        assert_eq!(exactly.len(), 1);
        assert_eq!(
            at_most[0].inequality.relation,
            ConstraintRelation::LessEqual
        );
        assert_eq!(exactly[0].inequality.relation, ConstraintRelation::Equal);
        assert!(at_most_constraints::<f64>(&inputs, 4, &mapping, "invalid_at_most").is_err());
        assert!(exactly_constraints::<f64>(&inputs, 4, &mapping, "invalid_exactly").is_err());
    }

    #[test]
    fn pure_cardinality_helpers_use_solver_columns_for_standalone_variables() {
        let inputs = indicators(3, 24_000);
        let dummy = BinaryVariableItem::create(VariableId::standalone(24_100), "dummy");
        let mapping = HashMap::from([
            (dummy.id().unique_id() as usize, 0),
            (inputs[0].id().unique_id() as usize, 1),
            (inputs[1].id().unique_id() as usize, 2),
            (inputs[2].id().unique_id() as usize, 3),
        ]);

        let constraint = at_most_constraints::<f64>(&inputs, 2, &mapping, "offset_inputs")
            .unwrap()
            .remove(0);
        let columns = constraint
            .inequality
            .polynomial
            .monomials()
            .iter()
            .map(|monomial| monomial.var_index())
            .collect::<Vec<_>>();
        assert_eq!(columns, vec![1, 2, 3]);
        assert_eq!(columns.iter().collect::<HashSet<_>>().len(), 3);

        let incomplete = HashMap::from([
            (inputs[0].id().unique_id() as usize, 1),
            (inputs[1].id().unique_id() as usize, 2),
        ]);
        assert!(exactly_constraints::<f64>(&inputs, 2, &incomplete, "missing_column").is_err());
    }
}
