//! McCormick 双线性包络 / McCormick bilinear envelope

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::infer_linear_bounds_from_tokens;
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, VariableId, VariableRange, new_group_id};
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
            "McCormick envelope cannot convert {context}={value} into the model value type"
        ))
        .into()
    })
}

fn validate_bounds<V: ToPrimitive>(bounds: &(V, V), context: &str) -> Result<(f64, f64)> {
    let lower = to_f64(&bounds.0).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("McCormick {context} lower bound is invalid"))
    })?;
    let upper = to_f64(&bounds.1).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("McCormick {context} upper bound is invalid"))
    })?;
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(ModelError::InvalidConstraint(format!(
            "McCormick {context} requires finite ordered bounds, got [{lower}, {upper}]"
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
            "McCormick {context} requires finite bounds for every input variable"
        ))
    })?;
    if actual.0 < bounds.0 || actual.1 > bounds.1 {
        return Err(ModelError::InvalidConstraint(format!(
            "McCormick {context} bounds [{}, {}] do not cover inferred range [{}, {}]",
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

fn tighten_token_range<V>(token: &mut Token<V>, lower: f64, upper: f64) -> Result<()>
where
    V: Clone + Debug + Send + Sync + 'static + PartialOrd + FromPrimitive,
{
    let expected_lower: V = from_f64(lower, "result lower bound")?;
    let expected_upper: V = from_f64(upper, "result upper bound")?;
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
            "McCormick result token `{}` has incompatible range",
            token.name()
        ))
        .into());
    }
    token.set_range(VariableRange::bounded(lower, upper));
    Ok(())
}

/// 有界双线性项的 McCormick 线性松弛 / McCormick linear relaxation for a bounded bilinear term.
///
/// 结果变量只受四条包络约束限制，除退化输入域外不保证等于真实乘积。标准求值读取模型中的
/// 结果变量；[`Self::true_product_from_tokens`] 单独计算真实乘积。
/// The result variable is constrained by the four envelope inequalities and generally is not
/// equal to the true product. Standard evaluation reads the model result variable;
/// [`Self::true_product_from_tokens`] computes the actual product separately.
#[derive(Debug, Clone)]
pub struct McCormickEnvelopeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    left: Linear<V>,
    right: Linear<V>,
    left_bounds: (V, V),
    right_bounds: (V, V),
    result_bounds: (f64, f64),
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> McCormickEnvelopeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    /// 创建双线性包络函数 / Create a bilinear envelope function.
    pub fn new(
        id: u64,
        name: &str,
        left: Linear<V>,
        right: Linear<V>,
        left_bounds: (V, V),
        right_bounds: (V, V),
    ) -> Result<Self> {
        let (xl, xu) = validate_bounds(&left_bounds, "left")?;
        let (yl, yu) = validate_bounds(&right_bounds, "right")?;
        let corners = [xl * yl, xl * yu, xu * yl, xu * yu];
        let result_bounds = (
            corners.iter().copied().fold(f64::INFINITY, f64::min),
            corners.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        if !result_bounds.0.is_finite() || !result_bounds.1.is_finite() {
            return Err(ModelError::InvalidConstraint(format!(
                "McCormick `{name}` product range is not finite"
            ))
            .into());
        }
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(new_group_id(), 0),
            &format!("{name}_envelope"),
            VariableRange::bounded(result_bounds.0, result_bounds.1),
        );
        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            left,
            right,
            left_bounds,
            right_bounds,
            result_bounds,
            result_var,
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 设置声明依赖 / Set declared dependencies.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取包络结果变量 / Get the envelope result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取左输入表达式 / Get the left input expression.
    pub fn left_polynomial(&self) -> &Linear<V> {
        &self.left
    }

    /// 获取右输入表达式 / Get the right input expression.
    pub fn right_polynomial(&self) -> &Linear<V> {
        &self.right
    }

    /// 从令牌值计算真实乘积 / Compute the true product from token values.
    pub fn true_product_from_tokens(
        &self,
        tokens: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
    {
        Some(
            evaluate_linear(&self.left, tokens, zero_if_none)?
                * evaluate_linear(&self.right, tokens, zero_if_none)?,
        )
    }
}

impl<V> McCormickEnvelopeFunction<V>
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
                    "McCormick result variable `{}`",
                    self.result_var.id().unique_id()
                ))
            })?;
        let (xl, xu) = validate_bounds(&self.left_bounds, "left")?;
        let (yl, yu) = validate_bounds(&self.right_bounds, "right")?;
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        let mut constraints = Vec::with_capacity(4);

        // z - xl*y - yl*x >= -xl*yl.
        constraints.push(build_envelope_row(
            &self.id.name,
            "lower_left",
            &self.left,
            &self.right,
            result_index,
            1.0,
            -xl,
            -yl,
            ConstraintRelation::GreaterEqual,
            -xl * yl,
            source.clone(),
        )?);
        // z - xu*y - yu*x >= -xu*yu.
        constraints.push(build_envelope_row(
            &self.id.name,
            "lower_right",
            &self.left,
            &self.right,
            result_index,
            1.0,
            -xu,
            -yu,
            ConstraintRelation::GreaterEqual,
            -xu * yu,
            source.clone(),
        )?);
        // z - xu*y - yl*x <= -xu*yl.
        constraints.push(build_envelope_row(
            &self.id.name,
            "upper_left",
            &self.left,
            &self.right,
            result_index,
            1.0,
            -xu,
            -yl,
            ConstraintRelation::LessEqual,
            -xu * yl,
            source.clone(),
        )?);
        // z - xl*y - yu*x <= -xl*yu.
        constraints.push(build_envelope_row(
            &self.id.name,
            "upper_right",
            &self.left,
            &self.right,
            result_index,
            1.0,
            -xl,
            -yu,
            ConstraintRelation::LessEqual,
            -xl * yu,
            source,
        )?);
        Ok(constraints)
    }
}

fn build_envelope_row<V>(
    name: &str,
    suffix: &str,
    left: &Linear<V>,
    right: &Linear<V>,
    result_index: usize,
    result_coefficient: f64,
    right_scale: f64,
    left_scale: f64,
    relation: ConstraintRelation,
    rhs: f64,
    source: Arc<dyn IntermediateSymbol<V>>,
) -> Result<LinearConstraint<V>>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    let mut monomials = vec![LinearMonomial::new(
        from_f64(result_coefficient, "result coefficient")?,
        result_index,
    )];
    for monomial in right.monomials() {
        let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "McCormick `{name}` input coefficient is invalid"
            ))
        })?;
        monomials.push(LinearMonomial::new(
            from_f64(right_scale * coefficient, "right envelope coefficient")?,
            monomial.var_index(),
        ));
    }
    for monomial in left.monomials() {
        let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "McCormick `{name}` input coefficient is invalid"
            ))
        })?;
        monomials.push(LinearMonomial::new(
            from_f64(left_scale * coefficient, "left envelope coefficient")?,
            monomial.var_index(),
        ));
    }
    let constant = right_scale
        * to_f64(right.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!("McCormick `{name}` input constant is invalid"))
        })?
        + left_scale
            * to_f64(left.constant_term()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "McCormick `{name}` input constant is invalid"
                ))
            })?;
    if !constant.is_finite() || !rhs.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "McCormick `{name}` generated row is not finite"
        ))
        .into());
    }
    Ok(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(monomials, from_f64(constant, "envelope constant")?),
            relation,
            from_f64(rhs, "envelope rhs")?,
        ),
        &format!("{name}_mccormick_{suffix}"),
        source,
    ))
}

impl<V> Display for McCormickEnvelopeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "mccormick({})", self.id.name)
    }
}

impl<V> DynSymbol for McCormickEnvelopeFunction<V>
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

impl<V> Symbol for McCormickEnvelopeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for McCormickEnvelopeFunction<V>
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
    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }
    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("mccormick({})", self.id.name)
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
            if token.id() == self.result_var.id() {
                tighten_token_range(token, self.result_bounds.0, self.result_bounds.1)?;
            }
        }
        Ok(())
    }
}

impl<V> FunctionSymbol<V> for McCormickEnvelopeFunction<V>
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
        Ok(())
    }

    fn calculate_value(&self, tokens: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        match tokens
            .find_by_id(self.result_var.id())
            .and_then(|token| token.get_result())
        {
            Some(value) => Some(value),
            None if zero_if_none => V::from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for McCormickEnvelopeFunction<V>
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
    fn mccormick_evaluation_reads_model_result_and_true_product_separately() {
        let left = Linear::constant(0.5);
        let right = Linear::constant(0.5);
        let function =
            McCormickEnvelopeFunction::new(101, "relax", left, right, (0.0, 1.0), (0.0, 1.0))
                .unwrap();
        let mut tokens = VecTokenList::<f64>::new();
        let result_token = Token::from_generic(function.result_variable().clone(), 0);
        result_token.set_result(0.0);
        tokens.add_token(result_token);
        assert_eq!(function.calculate_value(&tokens, false), Some(0.0));
        assert_eq!(
            function.true_product_from_tokens(&tokens, false),
            Some(0.25)
        );
    }

    #[test]
    fn mccormick_accepts_negative_and_degenerate_bounds() {
        let function = McCormickEnvelopeFunction::new(
            102,
            "negative",
            Linear::constant(-2.0),
            Linear::constant(0.0),
            (-2.0, -2.0),
            (0.0, 0.0),
        )
        .unwrap();
        assert_eq!(function.result_variable().range().lower_bound, Some(0.0));
        assert_eq!(function.result_variable().range().upper_bound, Some(0.0));
        assert!(
            McCormickEnvelopeFunction::new(
                103,
                "invalid",
                Linear::constant(0.0),
                Linear::constant(0.0),
                (2.0, 1.0),
                (0.0, 1.0),
            )
            .is_err()
        );
    }

    #[test]
    fn mccormick_handles_asymmetric_bounds_and_requires_finite_input_ranges() {
        let asymmetric = McCormickEnvelopeFunction::new(
            104,
            "asymmetric",
            Linear::constant(1.0),
            Linear::constant(2.0),
            (-2.0, 3.0),
            (1.0, 4.0),
        )
        .unwrap();
        assert_eq!(asymmetric.result_variable().range().lower_bound, Some(-8.0));
        assert_eq!(asymmetric.result_variable().range().upper_bound, Some(12.0));

        let zero = ContinuousVariableItem::with_range(
            VariableId::standalone(105),
            "zero",
            VariableRange::fixed(0.0),
        );
        let unbounded = ContinuousVariableItem::auto("unbounded");
        let function = McCormickEnvelopeFunction::new(
            105,
            "zero_times_unbounded",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            (0.0, 0.0),
            (-1.0, 1.0),
        )
        .unwrap();
        let registered = vec![
            Token::from_generic(zero, 0),
            Token::from_generic(unbounded, 1),
        ];
        assert!(
            function
                .register_auxiliary_tokens_with_context(&mut Vec::new(), &registered)
                .is_err()
        );
    }

    #[test]
    fn asymmetric_mccormick_rows_contain_products_and_reject_out_of_envelope_values() {
        let left = ContinuousVariableItem::with_range(
            VariableId::standalone(106),
            "left",
            VariableRange::bounded(-2.0, 3.0),
        );
        let right = ContinuousVariableItem::with_range(
            VariableId::standalone(107),
            "right",
            VariableRange::bounded(1.0, 4.0),
        );
        let function = McCormickEnvelopeFunction::new(
            106,
            "asymmetric_rows",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            (-2.0, 3.0),
            (1.0, 4.0),
        )
        .unwrap();
        let indices = HashMap::from([
            (left.id().unique_id() as usize, 0),
            (right.id().unique_id() as usize, 1),
            (function.result_variable().id().unique_id() as usize, 2),
        ]);
        let rows = function.mechanism_constraints(&indices).unwrap();
        assert_eq!(rows.len(), 4);

        assert!(constraints_hold(&rows, &[-2.0, 4.0, -8.0]));
        assert!(constraints_hold(&rows, &[0.5, 2.5, 1.25]));
        assert!(!constraints_hold(&rows, &[0.5, 2.5, 13.0]));
    }
}
