//! 有界整数与线性表达式乘积 / Bounded integer and linear-expression product

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
            "integer product cannot convert {context}={value} into the model value type"
        ))
        .into()
    })
}

fn validate_bounds<V: ToPrimitive>(bounds: &(V, V), context: &str) -> Result<(f64, f64)> {
    let lower = to_f64(&bounds.0).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("integer product {context} lower bound is invalid"))
    })?;
    let upper = to_f64(&bounds.1).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("integer product {context} upper bound is invalid"))
    })?;
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(ModelError::InvalidConstraint(format!(
            "integer product {context} requires finite ordered bounds, got [{lower}, {upper}]"
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
            "integer product {context} requires finite bounds for every input variable"
        ))
    })?;
    if actual.0 < bounds.0 || actual.1 > bounds.1 {
        return Err(ModelError::InvalidConstraint(format!(
            "integer product {context} declared bounds [{}, {}] do not cover inferred range [{}, {}]",
            bounds.0, bounds.1, actual.0, actual.1
        )).into());
    }
    Ok(())
}

/// 对有界整数变量和有界线性式建模精确乘积 / Exact product of a bounded integer variable and a bounded linear expression.
///
/// 整数变量平移到零后采用二进制展开，并以原整数上界限制编码；每一位与线性式的乘积
/// 通过掩码函数精确线性化。两个输入域必须在构造时显式有界。
/// The integer is shifted to zero and binary encoded, with an upper-capacity row preventing
/// unused encodings. Each bit times the linear expression is exactly linearized by masking.
#[derive(Debug, Clone)]
pub struct IntegerProductFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    integer: IntegerVariableItem,
    integer_lower: i64,
    integer_upper: i64,
    input: Linear<V>,
    input_bounds: (V, V),
    result_var: ContinuousVariableItem,
    bits: Vec<BinaryVariableItem>,
    masks: Vec<MaskingFunction<V>>,
    result_bounds: (f64, f64),
    declared_dependency_ids: Vec<u64>,
}

impl<V> IntegerProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建精确整数乘积符号 / Create an exact integer-product symbol.
    pub fn new(
        id: u64,
        name: &str,
        integer: IntegerVariableItem,
        input: Linear<V>,
        input_bounds: (V, V),
    ) -> Result<Self> {
        let lower = integer.range().lower_bound.ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "integer product `{name}` requires a finite integer lower bound"
            ))
        })?;
        let upper = integer.range().upper_bound.ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "integer product `{name}` requires a finite integer upper bound"
            ))
        })?;
        if !lower.is_finite()
            || !upper.is_finite()
            || lower.fract() != 0.0
            || upper.fract() != 0.0
            || lower > upper
            || lower.abs() > 9_007_199_254_740_991.0
            || upper.abs() > 9_007_199_254_740_991.0
        {
            return Err(ModelError::InvalidConstraint(format!(
                "integer product `{name}` requires finite integral bounds within exact f64 integer range, got [{lower}, {upper}]"
            ))
            .into());
        }
        let integer_lower = lower as i64;
        let integer_upper = upper as i64;
        let (input_lower, input_upper) = validate_bounds(&input_bounds, "input")?;
        let products = [
            lower * input_lower,
            lower * input_upper,
            upper * input_lower,
            upper * input_upper,
        ];
        let result_bounds = (
            products.iter().copied().fold(f64::INFINITY, f64::min),
            products.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        if !result_bounds.0.is_finite() || !result_bounds.1.is_finite() {
            return Err(ModelError::InvalidConstraint(format!(
                "integer product `{name}` product range is not finite"
            ))
            .into());
        }

        let width_i128 = integer_upper as i128 - integer_lower as i128;
        if width_i128 > 9_007_199_254_740_991_i128 {
            return Err(ModelError::InvalidConstraint(format!(
                "integer product `{name}` integer range is too wide for exact binary coefficients"
            ))
            .into());
        }
        let width = width_i128 as u64;
        let bit_count = if width == 0 {
            0
        } else {
            (u64::BITS - width.leading_zeros()) as usize
        };
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            &format!("{name}_product"),
            VariableRange::bounded(result_bounds.0, result_bounds.1),
        );
        let bits = (0..bit_count)
            .map(|index| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, index + 1),
                    &format!("{name}_bit_{index}"),
                )
            })
            .collect::<Vec<_>>();
        let mask_big_m = input_lower.abs().max(input_upper.abs()).max(1.0);
        let masks = bits
            .iter()
            .enumerate()
            .map(|(index, bit)| {
                Ok(MaskingFunction::with_big_m(
                    next_auto_intermediate_symbol_id(),
                    &format!("{name}_bit_mask_{index}"),
                    input.clone(),
                    bit.clone(),
                    from_f64(mask_big_m, "masking bound")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            integer,
            integer_lower,
            integer_upper,
            input,
            input_bounds,
            result_var,
            bits,
            masks,
            result_bounds,
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 设置声明依赖 / Set declared dependencies.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取整数输入 / Get the integer input.
    pub fn integer_variable(&self) -> &IntegerVariableItem {
        &self.integer
    }

    /// 获取线性输入 / Get the linear input.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取精确乘积结果变量 / Get the exact product result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取二进制位变量 / Get the binary encoding variables.
    pub fn bit_variables(&self) -> &[BinaryVariableItem] {
        &self.bits
    }

    fn validate_integer_token(&self, tokens: &[Token<V>]) -> Result<()>
    where
        V: ToPrimitive,
    {
        let token = tokens
            .iter()
            .find(|token| token.id() == self.integer.id())
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "integer product input `{}`",
                    self.integer.id().unique_id()
                ))
            })?;
        if !token.var_type().is_integer() {
            return Err(ModelError::InvalidConstraint(format!(
                "integer product `{}` input token is not an integer variable",
                self.id.name
            ))
            .into());
        }
        let range = token.range();
        let lower = range.lower_bound.as_ref().and_then(to_f64).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "integer product `{}` token requires a finite lower bound",
                self.id.name
            ))
        })?;
        let upper = range.upper_bound.as_ref().and_then(to_f64).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "integer product `{}` token requires a finite upper bound",
                self.id.name
            ))
        })?;
        if !lower.is_finite()
            || !upper.is_finite()
            || lower.fract() != 0.0
            || upper.fract() != 0.0
            || lower < self.integer_lower as f64
            || upper > self.integer_upper as f64
        {
            return Err(ModelError::InvalidConstraint(format!(
                "integer product `{}` registered integer range [{lower}, {upper}] is invalid or wider than captured range [{}, {}]",
                self.id.name, self.integer_lower, self.integer_upper
            )).into());
        }
        Ok(())
    }
}

impl<V> IntegerProductFunction<V>
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
                    "integer product result variable `{}`",
                    self.result_var.id().unique_id()
                ))
            })?;
        let integer_index = symbol_to_index
            .get(&(self.integer.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "integer product input variable `{}`",
                    self.integer.id().unique_id()
                ))
            })?;
        let mut constraints = Vec::new();
        let mut product_monomials = vec![LinearMonomial::new(
            from_f64(1.0, "result coefficient")?,
            result_index,
        )];
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "integer product `{}` input coefficient is invalid",
                    self.id.name
                ))
            })?;
            product_monomials.push(LinearMonomial::new(
                from_f64(
                    -(self.integer_lower as f64) * coefficient,
                    "input coefficient",
                )?,
                monomial.var_index(),
            ));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "integer product `{}` input constant is invalid",
                self.id.name
            ))
        })?;
        let product_constant = from_f64(
            -(self.integer_lower as f64) * input_constant,
            "product constant",
        )?;
        for (weight_index, (_bit, mask)) in self.bits.iter().zip(&self.masks).enumerate() {
            let mask_id = mask.result_variable().id().unique_id() as usize;
            let mask_index = symbol_to_index.get(&mask_id).copied().ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!("integer product mask `{mask_id}`"))
            })?;
            let weight = 2_f64.powi(weight_index as i32);
            product_monomials.push(LinearMonomial::new(
                from_f64(-weight, "masked product coefficient")?,
                mask_index,
            ));

            let mut mask_constraints = mask.mechanism_constraints(symbol_to_index)?;
            for constraint in &mut mask_constraints {
                constraint.set_from(Arc::new(self.clone()));
            }
            constraints.extend(mask_constraints);
        }

        if !self.bits.is_empty() {
            let mut cap = Vec::with_capacity(self.bits.len());
            for (i, bit) in self.bits.iter().enumerate() {
                let index = symbol_to_index
                    .get(&(bit.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "integer product bit variable `{}`",
                            bit.id().unique_id()
                        ))
                    })?;
                cap.push(LinearMonomial::new(
                    from_f64(2_f64.powi(i as i32), "integer encoding weight")?,
                    index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(cap, from_f64(0.0, "integer encoding constant")?),
                    ConstraintRelation::LessEqual,
                    from_f64(
                        (self.integer_upper as i128 - self.integer_lower as i128) as f64,
                        "integer encoding capacity",
                    )?,
                ),
                &format!("{}_encoding_cap", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        let mut encoding_monomials = vec![LinearMonomial::new(
            from_f64(-1.0, "integer encoding input coefficient")?,
            integer_index,
        )];
        for (i, bit) in self.bits.iter().enumerate() {
            let bit_index = symbol_to_index
                .get(&(bit.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "integer product bit variable `{}`",
                        bit.id().unique_id()
                    ))
                })?;
            encoding_monomials.push(LinearMonomial::new(
                from_f64(2_f64.powi(i as i32), "integer encoding weight")?,
                bit_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    encoding_monomials,
                    from_f64(0.0, "integer encoding constant")?,
                ),
                ConstraintRelation::Equal,
                from_f64(-(self.integer_lower as f64), "integer encoding offset")?,
            ),
            &format!("{}_encoding_eq", self.id.name),
            Arc::new(self.clone()),
        ));

        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(product_monomials, product_constant),
                ConstraintRelation::Equal,
                from_f64(0.0, "product equality rhs")?,
            ),
            &format!("{}_product_eq", self.id.name),
            Arc::new(self.clone()),
        ));
        Ok(constraints)
    }
}

impl<V> Display for IntegerProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "integer_product({})", self.id.name)
    }
}

impl<V> DynSymbol for IntegerProductFunction<V>
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

impl<V> Symbol for IntegerProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for IntegerProductFunction<V>
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
        self.validate_integer_token(tokens)?;
        validate_declared_bounds(
            &self.input,
            validate_bounds(&self.input_bounds, "input")?,
            tokens,
            "input",
        )?;
        self.mechanism(symbol_to_index)
    }
    fn register_auxiliary_tokens_with_context(
        &self,
        tokens: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        self.validate_integer_token(registered)?;
        validate_declared_bounds(
            &self.input,
            validate_bounds(&self.input_bounds, "input")?,
            registered,
            "input",
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
        format!("integer_product({})", self.id.name)
    }
    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        self.validate_integer_token(tokens)?;
        validate_declared_bounds(
            &self.input,
            validate_bounds(&self.input_bounds, "input")?,
            tokens,
            "input",
        )?;
        for token in auxiliary {
            if self.bits.iter().any(|bit| bit.id() == token.id()) {
                token.set_range(VariableRange::bounded(
                    from_f64(0.0, "binary lower bound")?,
                    from_f64(1.0, "binary upper bound")?,
                ));
            } else if self
                .masks
                .iter()
                .any(|mask| mask.result_variable().id() == token.id())
            {
                let (lower, upper) = validate_bounds(&self.input_bounds, "input")?;
                token.set_range(VariableRange::bounded(
                    from_f64(lower.min(0.0), "masked lower bound")?,
                    from_f64(upper.max(0.0), "masked upper bound")?,
                ));
            } else if token.id() == self.result_var.id() {
                token.set_range(VariableRange::bounded(
                    from_f64(self.result_bounds.0, "result lower bound")?,
                    from_f64(self.result_bounds.1, "result upper bound")?,
                ));
            }
        }
        Ok(())
    }
}

impl<V> FunctionSymbol<V> for IntegerProductFunction<V>
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
        for bit in &self.bits {
            tokens.push(Token::from_generic(bit.clone(), bit.index()));
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
        let integer = match tokens
            .find_by_id(self.integer.id())
            .and_then(|token| token.get_result())
        {
            Some(value) => value,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        let integer_f64 = to_f64(&integer)?;
        if !integer_f64.is_finite()
            || integer_f64.fract() != 0.0
            || integer_f64 < self.integer_lower as f64
            || integer_f64 > self.integer_upper as f64
        {
            return None;
        }
        Some(integer * evaluate_linear(&self.input, tokens, zero_if_none)?)
    }
}

impl<V> LinearIntermediateSymbol<V> for IntegerProductFunction<V>
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
    use crate::variable::IntegerVariableItem;

    fn constraint_holds(constraint: &LinearConstraint<f64>, values: &[f64]) -> bool {
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
    }

    #[test]
    fn bounded_integer_product_accepts_negative_and_degenerate_domains() {
        let integer = IntegerVariableItem::with_range(
            VariableId::standalone(71),
            "integer",
            VariableRange::bounded(-2.0, 2.0),
        );
        let product =
            IntegerProductFunction::new(71, "product", integer, Linear::constant(3.0), (3.0, 3.0))
                .unwrap();
        assert_eq!(product.bit_variables().len(), 3);
        assert_eq!(product.result_variable().range().lower_bound, Some(-6.0));
        assert_eq!(product.result_variable().range().upper_bound, Some(6.0));

        let fixed = IntegerVariableItem::with_range(
            VariableId::standalone(72),
            "fixed",
            VariableRange::fixed(-1.0),
        );
        let fixed_product = IntegerProductFunction::new(
            72,
            "fixed_product",
            fixed,
            Linear::constant(4.0),
            (4.0, 4.0),
        )
        .unwrap();
        assert!(fixed_product.bit_variables().is_empty());
        assert_eq!(
            fixed_product.result_variable().range().lower_bound,
            Some(-4.0)
        );
    }

    #[test]
    fn negative_integer_encoding_and_width_cap_rows_are_feasible() {
        let integer = IntegerVariableItem::with_range(
            VariableId::standalone(75),
            "negative_integer",
            VariableRange::bounded(-2.0, 0.0),
        );
        let product = IntegerProductFunction::new(
            75,
            "negative_product",
            integer.clone(),
            Linear::constant(3.0),
            (3.0, 3.0),
        )
        .unwrap();

        let mut indices = HashMap::new();
        indices.insert(integer.id().unique_id() as usize, 0);
        indices.insert(product.result_variable().id().unique_id() as usize, 1);
        for (offset, bit) in product.bit_variables().iter().enumerate() {
            indices.insert(bit.id().unique_id() as usize, 2 + offset);
        }
        for (offset, mask) in product.masks.iter().enumerate() {
            indices.insert(mask.result_variable().id().unique_id() as usize, 4 + offset);
        }
        let rows = product.mechanism_constraints(&indices).unwrap();

        let mut valid = vec![0.0; 6];
        valid[0] = -1.0;
        valid[1] = -3.0;
        valid[2] = 1.0;
        valid[4] = 3.0;
        assert!(rows.iter().all(|row| constraint_holds(row, &valid)));

        let mut over_capacity = vec![0.0; 6];
        over_capacity[2] = 1.0;
        over_capacity[3] = 1.0;
        let width_cap = &rows[rows.len() - 3];
        assert!(!constraint_holds(width_cap, &over_capacity));
    }

    #[test]
    fn bounded_integer_product_rejects_missing_or_fractional_integer_bounds() {
        let unbounded = IntegerVariableItem::auto("unbounded");
        assert!(
            IntegerProductFunction::new(
                73,
                "invalid",
                unbounded,
                Linear::constant(1.0),
                (0.0, 1.0),
            )
            .is_err()
        );
        let fractional = IntegerVariableItem::with_range(
            VariableId::standalone(74),
            "fractional",
            VariableRange::bounded(-1.5, 2.0),
        );
        assert!(
            IntegerProductFunction::new(
                74,
                "fractional_product",
                fractional,
                Linear::constant(1.0),
                (0.0, 1.0),
            )
            .is_err()
        );
    }
}
