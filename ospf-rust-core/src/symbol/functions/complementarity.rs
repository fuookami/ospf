//! 有界非负变量互斥 / Bounded nonnegative mutual exclusivity

use super::super::{Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId};
use super::big_m::infer_linear_bounds_from_tokens;
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, VariableRange, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
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
            "complementarity cannot convert {context}={value} into the model value type"
        ))
        .into()
    })
}

fn validate_bounds<V: ToPrimitive>(bounds: &(V, V), context: &str) -> Result<(f64, f64)> {
    let lower = to_f64(&bounds.0).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("complementarity {context} lower bound is invalid"))
    })?;
    let upper = to_f64(&bounds.1).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("complementarity {context} upper bound is invalid"))
    })?;
    if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || lower > upper {
        return Err(ModelError::InvalidConstraint(format!("complementarity {context} requires finite bounds with 0 <= lower <= upper, got [{lower}, {upper}]" )).into());
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
            "complementarity {context} requires finite bounds for every input variable"
        ))
    })?;
    if actual.0 < bounds.0 || actual.1 > bounds.1 {
        return Err(ModelError::InvalidConstraint(format!(
            "complementarity {context} declared bounds [{}, {}] do not cover inferred range [{}, {}]",
            bounds.0, bounds.1, actual.0, actual.1
        )).into());
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

fn evaluate_linear_from_values<V>(poly: &Linear<V>, values: &HashMap<usize, V>) -> Option<V>
where
    V: Clone + Add<Output = V> + Mul<Output = V>,
{
    let mut value = poly.constant_term().clone();
    for monomial in poly.monomials() {
        value = value + monomial.coefficient().clone() * values.get(&monomial.var_index())?.clone();
    }
    Some(value)
}

fn tighten_token_range<V>(token: &mut Token<V>) -> Result<()>
where
    V: Clone + Debug + Send + Sync + 'static + PartialOrd + FromPrimitive,
{
    let lower: V = from_f64(0.0, "selector lower bound")?;
    let upper: V = from_f64(1.0, "selector upper bound")?;
    let old = token.range();
    let lower = match old.lower_bound {
        Some(value) if value > lower => value,
        _ => lower,
    };
    let upper = match old.upper_bound {
        Some(value) if value < upper => value,
        _ => upper,
    };
    if lower > upper {
        return Err(ModelError::InvalidConstraint(format!(
            "complementarity selector `{}` has incompatible range",
            token.name()
        ))
        .into());
    }
    token.set_range(VariableRange::bounded(lower, upper));
    Ok(())
}

/// 令两个有界非负表达式至多一个为正的精确 MILP 符号。
/// Exact MILP symbol requiring at most one of two bounded nonnegative expressions to be positive.
///
/// `selector_variable()` 是内部线性化变量；`calculate_value()` 返回违规指示值：两式均为正时为 1，
/// 至少一式为零时为 0。`MutuallyExclusivePositiveFunction` 为别名。
/// `selector_variable()` is an internal linearization variable. `calculate_value()` returns a
/// violation indicator: 1 when both expressions are positive, and 0 when either is zero.
/// `MutuallyExclusivePositiveFunction` is an alias.
#[derive(Debug, Clone)]
pub struct ComplementarityFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    left: Linear<V>,
    right: Linear<V>,
    left_bounds: (V, V),
    right_bounds: (V, V),
    selector: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
}

/// 互斥正值函数别名 / Alias for mutually exclusive positive expressions.
pub type MutuallyExclusivePositiveFunction<V = f64> = ComplementarityFunction<V>;

impl<V> ComplementarityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建有界非负表达式互斥函数 / Create a mutual-exclusivity function for bounded nonnegative expressions.
    pub fn new(
        id: u64,
        name: &str,
        left: Linear<V>,
        right: Linear<V>,
        left_bounds: (V, V),
        right_bounds: (V, V),
    ) -> Result<Self> {
        validate_bounds(&left_bounds, "left")?;
        validate_bounds(&right_bounds, "right")?;
        let selector = BinaryVariableItem::create(
            VariableId::new(new_group_id(), 0),
            &format!("{name}_selector"),
        );
        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            left,
            right,
            left_bounds,
            right_bounds,
            selector,
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 设置声明依赖 / Set declared dependencies.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取互斥选择器 / Get the exclusivity selector.
    pub fn selector_variable(&self) -> &BinaryVariableItem {
        &self.selector
    }
}

impl<V> ComplementarityFunction<V>
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
    fn mechanism(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let selector_index = symbol_to_index
            .get(&(self.selector.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "complementarity selector `{}`",
                    self.selector.id().unique_id()
                ))
            })?;
        let (_, left_upper) = validate_bounds(&self.left_bounds, "left")?;
        let (_, right_upper) = validate_bounds(&self.right_bounds, "right")?;
        let mut left_monomials = Vec::with_capacity(self.left.monomials().len() + 1);
        for monomial in self.left.monomials() {
            left_monomials.push(monomial.clone());
        }
        left_monomials.push(LinearMonomial::new(
            from_f64(-left_upper, "left upper bound")?,
            selector_index,
        ));

        let mut right_monomials = Vec::with_capacity(self.right.monomials().len() + 1);
        for monomial in self.right.monomials() {
            right_monomials.push(monomial.clone());
        }
        right_monomials.push(LinearMonomial::new(
            from_f64(right_upper, "right upper bound")?,
            selector_index,
        ));

        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        Ok(vec![
            LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(left_monomials, self.left.constant_term().clone()),
                    ConstraintRelation::LessEqual,
                    from_f64(0.0, "left exclusivity rhs")?,
                ),
                &format!("{}_left_exclusive", self.id.name),
                source.clone(),
            ),
            LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(right_monomials, self.right.constant_term().clone()),
                    ConstraintRelation::LessEqual,
                    from_f64(right_upper, "right exclusivity rhs")?,
                ),
                &format!("{}_right_exclusive", self.id.name),
                source,
            ),
        ])
    }
}

impl<V> Display for ComplementarityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "complementarity({})", self.id.name)
    }
}

impl<V> DynSymbol for ComplementarityFunction<V>
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

impl<V> Symbol for ComplementarityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ComplementarityFunction<V>
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
        validate_declared_bounds(
            &self.left,
            validate_bounds(&self.left_bounds, "left")?,
            registered,
            "left input",
        )?;
        validate_declared_bounds(
            &self.right,
            validate_bounds(&self.right_bounds, "right")?,
            registered,
            "right input",
        )?;
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
        validate_declared_bounds(
            &self.left,
            validate_bounds(&self.left_bounds, "left")?,
            tokens,
            "left input",
        )?;
        validate_declared_bounds(
            &self.right,
            validate_bounds(&self.right_bounds, "right")?,
            tokens,
            "right input",
        )?;
        self.mechanism(symbol_to_index)
    }
    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }
    fn prepare(&self, _values: &HashMap<usize, V>) -> Option<V> {
        None
    }
    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("complementarity({})", self.id.name)
    }
    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        validate_declared_bounds(
            &self.left,
            validate_bounds(&self.left_bounds, "left")?,
            tokens,
            "left input",
        )?;
        validate_declared_bounds(
            &self.right,
            validate_bounds(&self.right_bounds, "right")?,
            tokens,
            "right input",
        )?;
        for token in auxiliary {
            if token.id() == self.selector.id() {
                tighten_token_range(token)?;
            }
        }
        Ok(())
    }
}

impl<V> FunctionSymbol<V> for ComplementarityFunction<V>
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
            self.selector.clone(),
            self.selector.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, tokens: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_linear(&self.left, tokens, zero_if_none)?)?;
        let right = to_f64(&evaluate_linear(&self.right, tokens, zero_if_none)?)?;
        let (left_lower, left_upper) = validate_bounds(&self.left_bounds, "left").ok()?;
        let (right_lower, right_upper) = validate_bounds(&self.right_bounds, "right").ok()?;
        let tolerance = f64::EPSILON * 64.0 * left.abs().max(right.abs()).max(1.0);
        if !left.is_finite()
            || !right.is_finite()
            || left < left_lower - tolerance
            || left > left_upper + tolerance
            || right < right_lower - tolerance
            || right > right_upper + tolerance
        {
            return None;
        }
        if left > tolerance && right > tolerance {
            from_f64(1.0, "violation indicator").ok()
        } else {
            Some(V::zero())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};

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
    fn complementarity_evaluates_violation_indicator_independently_of_selector() {
        let left = Linear::constant(0.0);
        let right = Linear::constant(3.0);
        let function =
            ComplementarityFunction::new(111, "exclusive", left, right, (0.0, 5.0), (0.0, 5.0))
                .unwrap();
        let mut tokens = VecTokenList::<f64>::new();
        let selector = Token::from_generic(function.selector_variable().clone(), 0);
        tokens.add_token(selector.clone());
        selector.set_result(0.0);
        assert_eq!(function.calculate_value(&tokens, false), Some(0.0));
        selector.set_result(0.5);
        assert_eq!(function.calculate_value(&tokens, false), Some(0.0));

        let violation = ComplementarityFunction::new(
            112,
            "violation",
            Linear::constant(2.0),
            Linear::constant(3.0),
            (0.0, 5.0),
            (0.0, 5.0),
        )
        .unwrap();
        assert_eq!(violation.calculate_value(&tokens, false), Some(1.0));

        let invalid = ComplementarityFunction::new(
            113,
            "negative_value",
            Linear::constant(-1.0),
            Linear::constant(0.0),
            (0.0, 5.0),
            (0.0, 5.0),
        )
        .unwrap();
        assert_eq!(invalid.calculate_value(&tokens, false), None);
        assert!(
            ComplementarityFunction::new(
                114,
                "negative_bounds",
                Linear::constant(-1.0),
                Linear::constant(0.0),
                (-1.0, 5.0),
                (0.0, 5.0),
            )
            .is_err()
        );
    }

    #[test]
    fn complementarity_constraint_is_registered_for_selector() {
        let function = ComplementarityFunction::new(
            113,
            "exclusive",
            Linear::constant(0.0),
            Linear::constant(0.0),
            (0.0, 2.0),
            (0.0, 4.0),
        )
        .unwrap();
        let mut indices = HashMap::new();
        indices.insert(function.selector_variable().id().unique_id() as usize, 0);
        assert_eq!(function.mechanism(&indices).unwrap().len(), 2);
    }

    #[test]
    fn complementarity_rows_allow_both_zero_and_reject_both_positive() {
        let both_zero = ComplementarityFunction::new(
            115,
            "both_zero",
            Linear::constant(0.0),
            Linear::constant(0.0),
            (0.0, 5.0),
            (0.0, 5.0),
        )
        .unwrap();
        let zero_indices =
            HashMap::from([(both_zero.selector_variable().id().unique_id() as usize, 0)]);
        let zero_rows = both_zero.mechanism_constraints(&zero_indices).unwrap();
        assert!(constraints_hold(&zero_rows, &[0.0]));
        assert!(constraints_hold(&zero_rows, &[1.0]));

        let both_positive = ComplementarityFunction::new(
            116,
            "both_positive",
            Linear::constant(2.0),
            Linear::constant(3.0),
            (0.0, 5.0),
            (0.0, 5.0),
        )
        .unwrap();
        let positive_indices = HashMap::from([(
            both_positive.selector_variable().id().unique_id() as usize,
            0,
        )]);
        let positive_rows = both_positive
            .mechanism_constraints(&positive_indices)
            .unwrap();
        assert!(!constraints_hold(&positive_rows, &[0.0]));
        assert!(!constraints_hold(&positive_rows, &[1.0]));
    }
}
