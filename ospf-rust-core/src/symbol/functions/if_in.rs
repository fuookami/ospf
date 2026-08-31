//! If-in 函数符号 / If-in function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::infer_linear_bounds_from_tokens;
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};
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

const MIN_BIG_M: f64 = 1.0;
const STEP_EPSILON: f64 = 1e-8;

/// 检查输入值是否属于离散值集合。
/// Checks whether an input value belongs to a discrete set of values.
///
/// 数学形式 / Mathematical Form:
/// - `result = 1` if input in `{values[0], values[1], ...}`, otherwise `0`
#[derive(Debug, Clone)]
pub struct IfInFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 中间符号标识符 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 输入线性多项式 / Input linear polynomial
    input: Linear<V>,
    /// 结果二值变量 / Result binary variable
    result_var: BinaryVariableItem,
    /// 离散值集合 / Set of discrete values
    values: Vec<V>,
    /// 大 M 参数，用于机制约束松弛 / Big-M parameter for mechanism constraint relaxation
    big_m: V,
    /// 辅助变量组标识符 / Auxiliary variable group identifier
    aux_group_id: usize,
    /// 声明的依赖符号 ID 列表 / Declared dependency symbol IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> IfInFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的 if-in 函数 / Create new if-in function
    pub fn new(id: u64, name: &str, input: Linear<V>, values: Vec<V>, big_m: V) -> Self {
        let aux_group_id = new_group_id();
        let result_var = BinaryVariableItem::create(VariableId::new(aux_group_id, 0), name);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            values,
            big_m,
            aux_group_id,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 if-in 函数。
    /// Create an if-in function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, input: Linear<V>, values: Vec<V>, big_m: V) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            input,
            values,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建 if-in 函数。
    /// Create an if-in function with an auto id and auto-generated name.
    pub fn auto(input: Linear<V>, values: Vec<V>, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if_in", id);
        Self::new(id, &name, input, values, big_m)
    }

    /// 设置声明的依赖符号 ID 列表，返回修改后的自身。
    /// Set the declared dependency symbol IDs, returning the modified self.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果二值变量的引用。
    /// Get a reference to the result binary variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取输入线性多项式的引用。
    /// Get a reference to the input linear polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取离散值集合的切片。
    /// Get a slice of the discrete value set.
    pub fn values(&self) -> &[V] {
        &self.values
    }

    /// 获取大 M 参数的引用。
    /// Get a reference to the big-M parameter.
    pub fn big_m(&self) -> &V {
        &self.big_m
    }

    fn value_indicator_variable(&self, index: usize) -> BinaryVariableItem {
        BinaryVariableItem::create(
            VariableId::new(self.aux_group_id, index + 1),
            &format!("{}_ifin_val{}", self.id.name, index),
        )
    }

    fn value_side_variable(&self, count: usize, index: usize) -> BinaryVariableItem {
        BinaryVariableItem::create(
            VariableId::new(self.aux_group_id, count + index + 1),
            &format!("{}_ifin_side{}", self.id.name, index),
        )
    }
}

impl<V> IfInFunction<V>
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
    fn configured_big_m(&self) -> Result<f64> {
        let big_m = to_f64(&self.big_m).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if_in `{}` big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "if_in `{}` requires positive finite big-M for mechanism constraint injection",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        let (input_lower, input_upper) = infer_linear_bounds_from_tokens(&self.input, tokens)?;
        let mut inferred = MIN_BIG_M;
        for value in &self.values {
            let value_f = to_f64(value)?;
            let lower_diff = (input_lower - value_f).abs();
            let upper_diff = (input_upper - value_f).abs();
            inferred = inferred.max(lower_diff.max(upper_diff));
        }
        Some(inferred.max(MIN_BIG_M))
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
                    "if_in result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        if self.values.is_empty() {
            return Ok(vec![LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "if_in result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "if_in constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(0.0, "if_in rhs")?,
                ),
                &format!("{}_empty", self.id.name),
                Arc::new(self.clone()),
            )]);
        }

        if !big_m.is_finite() || big_m < 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "if_in `{}` requires finite non-negative big-M",
                self.id.name
            ))
            .into());
        }

        let tolerance = STEP_EPSILON;
        let strict_boundary = tolerance + STEP_EPSILON;
        let source = Arc::new(self.clone());

        let mut base_monomials = Vec::with_capacity(self.input.monomials().len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_in `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            base_monomials.push((coefficient, monomial.var_index()));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if_in `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let value_count = self.values.len();
        let mut value_indices = Vec::with_capacity(value_count);
        let mut side_indices = Vec::with_capacity(value_count);

        for i in 0..value_count {
            let indicator_var = self.value_indicator_variable(i);
            let side_var = self.value_side_variable(value_count, i);

            let indicator_index = symbol_to_index
                .get(&(indicator_var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "if_in value indicator variable id {}",
                        indicator_var.id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(side_var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "if_in side variable id {}",
                        side_var.id().unique_id()
                    ))
                })?;
            value_indices.push(indicator_index);
            side_indices.push(side_index);
        }

        let mut constraints = Vec::with_capacity(value_count * 6 + 1);

        for (i, value) in self.values.iter().enumerate() {
            let indicator_index = value_indices[i];
            let side_index = side_indices[i];
            let value_f = to_f64(value).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_in `{}` value at index {} cannot be converted to f64",
                    self.id.name, i
                ))
            })?;
            let shifted_constant = input_constant - value_f;

            let build_value_constraint = |name_suffix: &str,
                                          relation: ConstraintRelation,
                                          rhs: f64,
                                          indicator_coeff: f64,
                                          side_coeff: f64|
             -> Result<LinearConstraint<V>> {
                let mut monomials = Vec::with_capacity(base_monomials.len() + 2);
                for (coefficient, var_index) in &base_monomials {
                    monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(*coefficient, "if_in input coefficient")?,
                        *var_index,
                    ));
                }
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(indicator_coeff, "if_in indicator coefficient")?,
                    indicator_index,
                ));
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(side_coeff, "if_in side coefficient")?,
                    side_index,
                ));

                Ok(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            monomials,
                            convert_f64_to_v::<V>(shifted_constant, "if_in constant")?,
                        ),
                        relation,
                        convert_f64_to_v::<V>(rhs, "if_in rhs")?,
                    ),
                    &format!("{}_pt{}_{}", self.id.name, i, name_suffix),
                    source.clone(),
                ))
            };

            constraints.push(build_value_constraint(
                "band_ub",
                ConstraintRelation::LessEqual,
                tolerance + big_m,
                big_m,
                0.0,
            )?);
            constraints.push(build_value_constraint(
                "band_lb",
                ConstraintRelation::GreaterEqual,
                -tolerance - big_m,
                -big_m,
                0.0,
            )?);
            constraints.push(build_value_constraint(
                "out_lb",
                ConstraintRelation::GreaterEqual,
                strict_boundary - big_m,
                big_m,
                -big_m,
            )?);
            constraints.push(build_value_constraint(
                "out_ub",
                ConstraintRelation::LessEqual,
                -strict_boundary,
                -big_m,
                -big_m,
            )?);

            // OR link lower bound: result >= b_i
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_in result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "if_in indicator coefficient")?,
                                indicator_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_in link constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "if_in link rhs")?,
                ),
                &format!("{}_or_lb_{}", self.id.name, i),
                source.clone(),
            ));
        }

        // OR link upper bound: result <= sum(b_i)
        let mut sum_monomials = Vec::with_capacity(value_indices.len() + 1);
        sum_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if_in result coefficient")?,
            result_index,
        ));
        for indicator_index in value_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "if_in indicator coefficient")?,
                indicator_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    sum_monomials,
                    convert_f64_to_v::<V>(0.0, "if_in sum constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if_in sum rhs")?,
            ),
            &format!("{}_or_ub", self.id.name),
            source,
        ));

        Ok(constraints)
    }
}

impl<V> Display for IfInFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "if_in({})", self.id.name)
    }
}

impl<V> DynSymbol for IfInFunction<V>
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

impl<V> Symbol for IfInFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for IfInFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, self.configured_big_m()?)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = match self.infer_big_m_from_tokens(tokens) {
            Some(inferred) => inferred,
            None => self.configured_big_m()?,
        };
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
        format!("if_in({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for IfInFunction<V>
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
        let count = self.values.len();
        for i in 0..count {
            let indicator_var = self.value_indicator_variable(i);
            tokens.push(Token::from_generic(
                indicator_var.clone(),
                indicator_var.index(),
            ));
        }
        for i in 0..count {
            let side_var = self.value_side_variable(count, i);
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let eps = STEP_EPSILON;

        for v in &self.values {
            let v_f = to_f64(v)?;
            if (value - v_f).abs() <= eps {
                return from_f64(1.0);
            }
        }
        from_f64(0.0)
    }
}

impl<V> LinearIntermediateSymbol<V> for IfInFunction<V>
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
    use crate::model::{ConstraintRelation, LinearConstraint};
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    fn constraint_lhs(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> f64 {
        let mut lhs = *constraint.inequality.polynomial.constant_term();
        for monomial in constraint.inequality.polynomial.monomials() {
            lhs += *monomial.coefficient()
                * values
                    .get(&monomial.var_index())
                    .copied()
                    .unwrap_or_default();
        }
        lhs
    }

    fn satisfies(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> bool {
        let lhs = constraint_lhs(constraint, values);
        match constraint.inequality.relation {
            ConstraintRelation::LessEqual => lhs <= constraint.inequality.rhs + 1e-6,
            ConstraintRelation::Equal => (lhs - constraint.inequality.rhs).abs() <= 1e-6,
            ConstraintRelation::GreaterEqual => lhs + 1e-6 >= constraint.inequality.rhs,
        }
    }

    #[test]
    fn if_in_function_calculate_value_in_set() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_000), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(3.0);
        tokens.add_token(tx);

        let f: IfInFunction<f64> = IfInFunction::new(
            9000,
            "ifin_test",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, false);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn if_in_function_calculate_value_not_in_set() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_010), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        let f: IfInFunction<f64> = IfInFunction::new(
            9001,
            "ifin_test2",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, false);
        assert_eq!(value, Some(0.0));
    }

    #[test]
    fn if_in_function_calculate_value_with_polynomial_input() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_020), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(1.0);
        tokens.add_token(tx);

        // input = 2x + 1, when x=1 => input=3
        let f: IfInFunction<f64> = IfInFunction::new(
            9002,
            "ifin_poly",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, false);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn if_in_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_030), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        // no result set, so evaluate_linear returns None normally
        tokens.add_token(tx);

        let f: IfInFunction<f64> = IfInFunction::new(
            9003,
            "ifin_none",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![0.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, true);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn if_in_function_empty_values_forces_zero() {
        let f: IfInFunction<f64> = IfInFunction::new(
            9004,
            "ifin_empty",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![],
            100.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("empty if_in constraints should be generated");

        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].name, "ifin_empty_empty");
        assert_eq!(
            constraints[0].inequality.relation,
            ConstraintRelation::Equal
        );
        assert_eq!(constraints[0].inequality.rhs, 0.0);
    }

    #[test]
    fn if_in_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_040),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        // input = 2x + 1 with x in [-2, 3] => range [-3, 7]
        // values = [1.0, 3.0, 5.0]
        // |lower - 1| = 4, |upper - 1| = 6 => max 6
        // |lower - 3| = 6, |upper - 3| = 4 => max 6
        // |lower - 5| = 8, |upper - 5| = 2 => max 8
        // inferred M = 8
        let f: IfInFunction<f64> = IfInFunction::new(
            9005,
            "ifin_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..3 {
            let indicator = f.value_indicator_variable(i);
            let side = f.value_side_variable(3, i);
            symbol_to_index.insert(indicator.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 5 + i);
        }

        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_in constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "ifin_bound_pt2_band_ub")
            .expect("pt2 upper-band constraint should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 4)
            .expect("indicator term should exist");

        // inferred M = 8 for value=5.0 (the third value, pt2)
        let expected_m = 8.0;
        let expected_rhs = expected_m + STEP_EPSILON;
        assert!((band_ub.inequality.rhs - expected_rhs).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() - expected_m).abs() <= 1e-9);
    }

    #[test]
    fn if_in_function_falls_back_to_configured_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_050), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9006,
            "ifin_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 2.0],
            13.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..2 {
            let indicator = f.value_indicator_variable(i);
            let side = f.value_side_variable(2, i);
            symbol_to_index.insert(indicator.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 4 + i);
        }

        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_in constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "ifin_default_pt0_band_ub")
            .expect("pt0 upper-band constraint should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("indicator term should exist");

        assert!((band_ub.inequality.rhs - (13.0 + STEP_EPSILON)).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() - 13.0).abs() <= 1e-9);
    }

    #[test]
    fn if_in_function_mechanism_constraints_or_link() {
        let f: IfInFunction<f64> = IfInFunction::new(
            9007,
            "ifin_or",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..2 {
            let indicator = f.value_indicator_variable(i);
            let side = f.value_side_variable(2, i);
            symbol_to_index.insert(indicator.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 4 + i);
        }

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // 2 values * 5 constraints each (4 point + 1 or_lb) + 1 or_ub = 11
        assert_eq!(constraints.len(), 11);

        // Verify OR link lower bounds exist
        let or_lb_0 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_or_lb_0")
            .expect("or_lb_0 should exist");
        assert_eq!(
            or_lb_0.inequality.relation,
            ConstraintRelation::GreaterEqual
        );

        let or_lb_1 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_or_lb_1")
            .expect("or_lb_1 should exist");
        assert_eq!(
            or_lb_1.inequality.relation,
            ConstraintRelation::GreaterEqual
        );

        // Verify OR link upper bound exists
        let or_ub = constraints
            .iter()
            .find(|c| c.name == "ifin_or_or_ub")
            .expect("or_ub should exist");
        assert_eq!(or_ub.inequality.relation, ConstraintRelation::LessEqual);

        // Verify point band constraints
        let band_ub_0 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_pt0_band_ub")
            .expect("pt0 band_ub should exist");
        assert_eq!(band_ub_0.inequality.relation, ConstraintRelation::LessEqual);

        let band_lb_0 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_pt0_band_lb")
            .expect("pt0 band_lb should exist");
        assert_eq!(
            band_lb_0.inequality.relation,
            ConstraintRelation::GreaterEqual
        );
    }

    #[test]
    fn if_in_function_satisfies_constraints_when_result_is_one() {
        let _x = ContinuousVariableItem::create(VariableId::standalone(90_060), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9008,
            "ifin_sat",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator_0 = f.value_indicator_variable(0);
        let indicator_1 = f.value_indicator_variable(1);
        let side_0 = f.value_side_variable(2, 0);
        let side_1 = f.value_side_variable(2, 1);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        symbol_to_index.insert(indicator_0.id().unique_id() as usize, 2usize);
        symbol_to_index.insert(indicator_1.id().unique_id() as usize, 3usize);
        symbol_to_index.insert(side_0.id().unique_id() as usize, 4usize);
        symbol_to_index.insert(side_1.id().unique_id() as usize, 5usize);

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // Scenario: x=3.0, result=1, indicator_1=1, indicator_0=0
        // pt0 (value=1.0): x=3 > value, so side_0=1 (upper side)
        // pt1 (value=3.0): x=3 ≈ value, indicator_1=1, side_1 doesn't matter
        let assignment = HashMap::from([
            (0usize, 3.0_f64), // x
            (1usize, 1.0),     // result
            (2usize, 0.0),     // indicator_0
            (3usize, 1.0),     // indicator_1
            (4usize, 1.0),     // side_0 (upper: x > value[0])
            (5usize, 0.0),     // side_1
        ]);

        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &assignment)),
            "all constraints should be satisfied when x=3, result=1, indicator_1=1"
        );
    }

    #[test]
    fn if_in_function_satisfies_constraints_when_result_is_zero() {
        let _x = ContinuousVariableItem::create(VariableId::standalone(90_070), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9009,
            "ifin_zero",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator_0 = f.value_indicator_variable(0);
        let indicator_1 = f.value_indicator_variable(1);
        let side_0 = f.value_side_variable(2, 0);
        let side_1 = f.value_side_variable(2, 1);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        symbol_to_index.insert(indicator_0.id().unique_id() as usize, 2usize);
        symbol_to_index.insert(indicator_1.id().unique_id() as usize, 3usize);
        symbol_to_index.insert(side_0.id().unique_id() as usize, 4usize);
        symbol_to_index.insert(side_1.id().unique_id() as usize, 5usize);

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // Scenario: x=2.0, result=0, all indicators=0
        // pt0 (value=1.0): x=2 > value, so side_0=1 (upper side)
        // pt1 (value=3.0): x=2 < value, so side_1=0 (lower side)
        let assignment = HashMap::from([
            (0usize, 2.0_f64), // x
            (1usize, 0.0),     // result
            (2usize, 0.0),     // indicator_0
            (3usize, 0.0),     // indicator_1
            (4usize, 1.0),     // side_0 (upper: x > value[0])
            (5usize, 0.0),     // side_1 (lower: x < value[1])
        ]);

        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &assignment)),
            "all constraints should be satisfied when x=2, result=0, all indicators=0"
        );
    }

    #[test]
    fn if_in_function_violates_when_result_wrong() {
        let _x = ContinuousVariableItem::create(VariableId::standalone(90_080), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9010,
            "ifin_viol",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator_0 = f.value_indicator_variable(0);
        let indicator_1 = f.value_indicator_variable(1);
        let side_0 = f.value_side_variable(2, 0);
        let side_1 = f.value_side_variable(2, 1);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        symbol_to_index.insert(indicator_0.id().unique_id() as usize, 2usize);
        symbol_to_index.insert(indicator_1.id().unique_id() as usize, 3usize);
        symbol_to_index.insert(side_0.id().unique_id() as usize, 4usize);
        symbol_to_index.insert(side_1.id().unique_id() as usize, 5usize);

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // Scenario: x=2.0 (not in set), but result=1 (wrong!)
        // All indicators must be 0 since x doesn't match any value,
        // but result=1 violates or_ub: result <= sum(indicators) = 0
        let assignment = HashMap::from([
            (0usize, 2.0_f64), // x
            (1usize, 1.0),     // result (wrong!)
            (2usize, 0.0),     // indicator_0
            (3usize, 0.0),     // indicator_1
            (4usize, 1.0),     // side_0
            (5usize, 1.0),     // side_1
        ]);

        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &assignment)),
            "constraints should be violated when x=2, result=1, all indicators=0"
        );
    }
}
