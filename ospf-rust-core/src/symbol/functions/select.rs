//! 二值选择函数 / Binary selector function

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
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange, new_group_id,
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
            "select cannot convert {context}={value} into the model value type"
        ))
        .into()
    })
}

fn validate_bounds<V: ToPrimitive>(bounds: &(V, V), context: &str) -> Result<(f64, f64)> {
    let lower = to_f64(&bounds.0).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("select {context} lower bound is invalid"))
    })?;
    let upper = to_f64(&bounds.1).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("select {context} upper bound is invalid"))
    })?;
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(ModelError::InvalidConstraint(format!(
            "select {context} requires finite ordered bounds, got [{lower}, {upper}]"
        ))
        .into());
    }
    Ok((lower, upper))
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

fn difference<V>(left: &Linear<V>, right: &Linear<V>, context: &str) -> Result<Linear<V>>
where
    V: Clone + ToPrimitive + FromPrimitive,
{
    let mut monomials = Vec::with_capacity(left.monomials().len() + right.monomials().len());
    for monomial in left.monomials() {
        let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!("select {context} coefficient is invalid"))
        })?;
        monomials.push(LinearMonomial::new(
            from_f64(coefficient, context)?,
            monomial.var_index(),
        ));
    }
    for monomial in right.monomials() {
        let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!("select {context} coefficient is invalid"))
        })?;
        monomials.push(LinearMonomial::new(
            from_f64(-coefficient, context)?,
            monomial.var_index(),
        ));
    }
    let constant = to_f64(left.constant_term()).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("select {context} constant is invalid"))
    })? - to_f64(right.constant_term()).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("select {context} constant is invalid"))
    })?;
    if !constant.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "select {context} constant is not finite"
        ))
        .into());
    }
    Ok(Linear::new(monomials, from_f64(constant, context)?))
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
            "select auxiliary token `{}` has incompatible range",
            token.name()
        ))
        .into());
    }
    token.set_range(VariableRange::bounded(lower, upper));
    Ok(())
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
            "select {context} requires finite bounds for every input variable"
        ))
    })?;
    if actual.0 < bounds.0 || actual.1 > bounds.1 {
        return Err(ModelError::InvalidConstraint(format!(
            "select {context} declared bounds [{}, {}] do not cover inferred range [{}, {}]",
            bounds.0, bounds.1, actual.0, actual.1
        ))
        .into());
    }
    Ok(())
}

/// 按二值变量在两个线性式间选择的函数 / Select between two linear expressions using a binary variable.
///
/// 精确表达为 `otherwise + selector * (then - otherwise)`；差值界由两分支的显式有限界推出，
/// 并使用精确掩码约束。`IfThenElseFunction` 是兼容别名。
/// The exact relation is `otherwise + selector * (then - otherwise)`. The finite difference
/// bounds are derived from the supplied branch bounds and used by the masking constraints.
#[derive(Debug, Clone)]
pub struct SelectFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    selector: BinaryVariableItem,
    then_value: Linear<V>,
    otherwise_value: Linear<V>,
    then_bounds: (V, V),
    otherwise_bounds: (V, V),
    result_bounds: (f64, f64),
    masked_difference: MaskingFunction<V>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

/// 双分支条件取值函数别名 / Alias for two-branch conditional selection.
pub type IfThenElseFunction<V = f64> = SelectFunction<V>;

impl<V> SelectFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建精确二值选择符号 / Create an exact binary selector symbol.
    pub fn new(
        id: u64,
        name: &str,
        selector: BinaryVariableItem,
        then_value: Linear<V>,
        then_bounds: (V, V),
        otherwise_value: Linear<V>,
        otherwise_bounds: (V, V),
    ) -> Result<Self> {
        let (then_lower, then_upper) = validate_bounds(&then_bounds, "then branch")?;
        let (otherwise_lower, otherwise_upper) =
            validate_bounds(&otherwise_bounds, "otherwise branch")?;
        let difference_bounds = (then_lower - otherwise_upper, then_upper - otherwise_lower);
        let result_bounds = (
            then_lower.min(otherwise_lower),
            then_upper.max(otherwise_upper),
        );
        if !difference_bounds.0.is_finite() || !difference_bounds.1.is_finite() {
            return Err(ModelError::InvalidConstraint(format!(
                "select `{name}` difference range is not finite"
            ))
            .into());
        }
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            &format!("{name}_result"),
            VariableRange::bounded(result_bounds.0, result_bounds.1),
        );
        let mask_m = difference_bounds
            .0
            .abs()
            .max(difference_bounds.1.abs())
            .max(1.0);
        let masked_difference = MaskingFunction::with_big_m(
            next_auto_intermediate_symbol_id(),
            &format!("{name}_masked_difference"),
            difference(&then_value, &otherwise_value, "branch difference")?,
            selector.clone(),
            from_f64(mask_m, "difference bound")?,
        );
        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            selector,
            then_value,
            otherwise_value,
            then_bounds,
            otherwise_bounds,
            result_bounds,
            masked_difference,
            result_var,
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

    /// 获取选择器变量 / Get the selector variable.
    pub fn selector_variable(&self) -> &BinaryVariableItem {
        &self.selector
    }

    /// 获取 then 分支表达式 / Get the then-branch expression.
    pub fn then_polynomial(&self) -> &Linear<V> {
        &self.then_value
    }

    /// 获取 otherwise 分支表达式 / Get the otherwise-branch expression.
    pub fn otherwise_polynomial(&self) -> &Linear<V> {
        &self.otherwise_value
    }

    fn validate_selector(&self, tokens: &[Token<V>]) -> Result<()> {
        let token = tokens
            .iter()
            .find(|token| token.id() == self.selector.id())
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "select selector `{}`",
                    self.selector.id().unique_id()
                ))
            })?;
        if !token.var_type().is_binary() {
            return Err(ModelError::InvalidConstraint(format!(
                "select `{}` selector token is not binary",
                self.id.name
            ))
            .into());
        }
        Ok(())
    }
}

impl<V> SelectFunction<V>
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
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "select result variable `{}`",
                    self.result_var.id().unique_id()
                ))
            })?;
        let mask_index = symbol_to_index
            .get(&(self.masked_difference.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "select masked value `{}`",
                    self.masked_difference.result_variable().id().unique_id()
                ))
            })?;
        let mut constraints = self
            .masked_difference
            .mechanism_constraints(symbol_to_index)?;
        for constraint in &mut constraints {
            constraint.set_from(Arc::new(self.clone()));
        }

        let mut monomials = vec![
            LinearMonomial::new(from_f64(1.0, "result coefficient")?, result_index),
            LinearMonomial::new(from_f64(-1.0, "masked difference coefficient")?, mask_index),
        ];
        for monomial in self.otherwise_value.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "select `{}` otherwise coefficient is invalid",
                    self.id.name
                ))
            })?;
            monomials.push(LinearMonomial::new(
                from_f64(-coefficient, "otherwise coefficient")?,
                monomial.var_index(),
            ));
        }
        let constant = -to_f64(self.otherwise_value.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "select `{}` otherwise constant is invalid",
                self.id.name
            ))
        })?;
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(monomials, from_f64(constant, "otherwise constant")?),
                ConstraintRelation::Equal,
                from_f64(0.0, "selection equality rhs")?,
            ),
            &format!("{}_select_eq", self.id.name),
            Arc::new(self.clone()),
        ));
        Ok(constraints)
    }
}

impl<V> Display for SelectFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "select({})", self.id.name)
    }
}

impl<V> DynSymbol for SelectFunction<V>
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

impl<V> Symbol for SelectFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SelectFunction<V>
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
        self.validate_selector(tokens)?;
        validate_declared_bounds(
            &self.then_value,
            validate_bounds(&self.then_bounds, "then branch")?,
            tokens,
            "then branch",
        )?;
        validate_declared_bounds(
            &self.otherwise_value,
            validate_bounds(&self.otherwise_bounds, "otherwise branch")?,
            tokens,
            "otherwise branch",
        )?;
        self.mechanism(symbol_to_index)
    }
    fn register_auxiliary_tokens_with_context(
        &self,
        tokens: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        self.validate_selector(registered)?;
        validate_declared_bounds(
            &self.then_value,
            validate_bounds(&self.then_bounds, "then branch")?,
            registered,
            "then branch",
        )?;
        validate_declared_bounds(
            &self.otherwise_value,
            validate_bounds(&self.otherwise_bounds, "otherwise branch")?,
            registered,
            "otherwise branch",
        )?;
        <Self as IntermediateSymbol<V>>::register_auxiliary_tokens(self, tokens)
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
        format!("select({})", self.id.name)
    }
    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        self.validate_selector(tokens)?;
        let (then_lower, then_upper) = validate_bounds(&self.then_bounds, "then branch")?;
        let (otherwise_lower, otherwise_upper) =
            validate_bounds(&self.otherwise_bounds, "otherwise branch")?;
        validate_declared_bounds(
            &self.then_value,
            (then_lower, then_upper),
            tokens,
            "then branch",
        )?;
        validate_declared_bounds(
            &self.otherwise_value,
            (otherwise_lower, otherwise_upper),
            tokens,
            "otherwise branch",
        )?;
        for token in auxiliary {
            if token.id() == self.result_var.id() {
                tighten_token_range(token, self.result_bounds.0, self.result_bounds.1)?;
            } else if token.id() == self.masked_difference.result_variable().id() {
                tighten_token_range(
                    token,
                    (then_lower - otherwise_upper).min(0.0),
                    (then_upper - otherwise_lower).max(0.0),
                )?;
            }
        }
        Ok(())
    }
}

impl<V> FunctionSymbol<V> for SelectFunction<V>
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
        tokens.push(Token::from_generic(
            self.masked_difference.result_variable().clone(),
            self.masked_difference.result_variable().index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, tokens: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let selector = match tokens
            .find_by_id(self.selector.id())
            .and_then(|token| token.get_result())
        {
            Some(value) => value,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        let selector = to_f64(&selector)?;
        if selector == 1.0 {
            evaluate_linear(&self.then_value, tokens, zero_if_none)
        } else if selector == 0.0 {
            evaluate_linear(&self.otherwise_value, tokens, zero_if_none)
        } else {
            None
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for SelectFunction<V>
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
    fn select_evaluates_both_binary_branches_and_rejects_fractional_selector() {
        let selector = BinaryVariableItem::create(VariableId::standalone(91), "selector");
        let function = SelectFunction::new(
            91,
            "choose",
            selector.clone(),
            Linear::constant(8.0),
            (8.0, 8.0),
            Linear::constant(-3.0),
            (-3.0, -3.0),
        )
        .unwrap();
        let mut tokens = VecTokenList::<f64>::new();
        let selector_token = Token::from_generic(selector, 0);
        tokens.add_token(selector_token);
        tokens.find_by_index(0).unwrap().set_result(1.0);
        assert_eq!(function.calculate_value(&tokens, false), Some(8.0));
        tokens.find_by_index(0).unwrap().set_result(0.0);
        assert_eq!(function.calculate_value(&tokens, false), Some(-3.0));
        tokens.find_by_index(0).unwrap().set_result(0.5);
        assert_eq!(function.calculate_value(&tokens, false), None);
    }

    #[test]
    fn select_rejects_invalid_branch_bounds() {
        let selector = BinaryVariableItem::create(VariableId::standalone(92), "selector");
        assert!(
            SelectFunction::new(
                92,
                "invalid",
                selector,
                Linear::constant(1.0),
                (2.0, 1.0),
                Linear::constant(0.0),
                (0.0, 0.0),
            )
            .is_err()
        );
    }

    #[test]
    fn select_mechanism_rows_cover_both_branches() {
        let selector = BinaryVariableItem::create(VariableId::standalone(93), "selector");
        let function = SelectFunction::new(
            93,
            "choose_rows",
            selector.clone(),
            Linear::constant(8.0),
            (8.0, 8.0),
            Linear::constant(-3.0),
            (-3.0, -3.0),
        )
        .unwrap();
        let indices = HashMap::from([
            (selector.id().unique_id() as usize, 0),
            (function.result_variable().id().unique_id() as usize, 1),
            (
                function
                    .masked_difference
                    .result_variable()
                    .id()
                    .unique_id() as usize,
                2,
            ),
        ]);
        let rows = function.mechanism_constraints(&indices).unwrap();

        let then_values = [1.0, 8.0, 11.0];
        let otherwise_values = [0.0, -3.0, 0.0];
        assert!(constraints_hold(&rows, &then_values));
        assert!(constraints_hold(&rows, &otherwise_values));

        let wrong_then_result = [1.0, -3.0, 11.0];
        assert!(!constraints_hold(&rows, &wrong_then_result));
    }
}
