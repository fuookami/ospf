//! 首个满足条件函数符号 / First-satisfying function symbol

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
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, new_standalone_id};
use super::super::{

    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
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

/// 默认大 M 常量 / Default big-M constant
const DEFAULT_BIG_M: f64 = 1_000_000.0;
/// 大 M 策略 / Big-M policy
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

/// 首个满足条件函数 / First-satisfying function
///
/// 返回第一个条件为真的表达式值。
/// Return the first expression value whose condition is true.
///
/// 数学形式 / Mathematical Form:
/// - result = polynomials[i]，其中 conditions[i] 为首个为真的条件
/// - result = polynomials[i] where conditions[i] is the first true condition
#[derive(Debug, Clone)]
pub struct FirstFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 中间符号 ID / Intermediate symbol ID
    id: IntermediateSymbolId,
    /// 多项式列表 / Polynomial list
    polynomials: Vec<Linear<V>>,
    /// 条件二值变量列表 / Condition binary variable list
    conditions: Vec<BinaryVariableItem>,
    /// 结果连续变量 / Result continuous variable
    result_var: ContinuousVariableItem,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> FirstFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新函数 / Create a new function
    pub fn new(
        id: u64,
        name: &str,
        polynomials: Vec<Linear<V>>,
        conditions: Vec<BinaryVariableItem>,
    ) -> Self {
        let result_var = ContinuousVariableItem::create(new_standalone_id(), name);

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            conditions,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取多项式列表 / Get the polynomial list
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 获取条件二值变量列表 / Get the condition binary variable list
    pub fn condition_variables(&self) -> &[BinaryVariableItem] {
        &self.conditions
    }
}

impl<V> FirstFunction<V>
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
        if self.polynomials.len() != self.conditions.len() {
            return Err(ModelError::InvalidConstraint(format!(
                "first `{}` polynomial count {} does not match condition count {}",
                self.id.name,
                self.polynomials.len(),
                self.conditions.len()
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "first result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut condition_indices = Vec::with_capacity(self.conditions.len());
        for condition in &self.conditions {
            let condition_index = symbol_to_index
                .get(&(condition.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "first condition variable id {}",
                        condition.id().unique_id()
                    ))
                })?;
            condition_indices.push(condition_index);
        }

        let mut constraints = Vec::with_capacity(self.polynomials.len() * 2 + 2);

        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let mut upper_monomials = Vec::with_capacity(polynomial.monomials().len() + i + 2);
            upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "first upper result coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "first `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "first upper polynomial coefficient")?,
                    monomial.var_index(),
                ));
            }
            for prior_condition_index in condition_indices.iter().take(i) {
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-big_m, "first upper prior condition coefficient")?,
                    *prior_condition_index,
                ));
            }
            upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(big_m, "first upper active condition coefficient")?,
                condition_indices[i],
            ));

            let mut lower_monomials = Vec::with_capacity(polynomial.monomials().len() + i + 2);
            lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "first lower result coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "first `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "first lower polynomial coefficient")?,
                    monomial.var_index(),
                ));
            }
            for prior_condition_index in condition_indices.iter().take(i) {
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(big_m, "first lower prior condition coefficient")?,
                    *prior_condition_index,
                ));
            }
            lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-big_m, "first lower active condition coefficient")?,
                condition_indices[i],
            ));

            let constant = to_f64(polynomial.constant_term()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "first `{}` polynomial constant cannot be converted to f64",
                    self.id.name
                ))
            })?;

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        upper_monomials,
                        convert_f64_to_v::<V>(-constant, "first upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(big_m, "first upper rhs")?,
                ),
                &format!("{}_first_ub_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        lower_monomials,
                        convert_f64_to_v::<V>(-constant, "first lower constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(-big_m, "first lower rhs")?,
                ),
                &format!("{}_first_lb_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
        }

        let mut zero_upper_monomials = Vec::with_capacity(condition_indices.len() + 1);
        zero_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "first zero upper result coefficient")?,
            result_index,
        ));
        for condition_index in &condition_indices {
            zero_upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-big_m, "first zero upper condition coefficient")?,
                *condition_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    zero_upper_monomials,
                    convert_f64_to_v::<V>(0.0, "first zero upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "first zero upper rhs")?,
            ),
            &format!("{}_first_zero_ub", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut zero_lower_monomials = Vec::with_capacity(condition_indices.len() + 1);
        zero_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "first zero lower result coefficient")?,
            result_index,
        ));
        for condition_index in &condition_indices {
            zero_lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(big_m, "first zero lower condition coefficient")?,
                *condition_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    zero_lower_monomials,
                    convert_f64_to_v::<V>(0.0, "first zero lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "first zero lower rhs")?,
            ),
            &format!("{}_first_zero_lb", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl<V> Display for FirstFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "first({})", self.id.name)
    }
}

impl<V> DynSymbol for FirstFunction<V>
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

impl<V> Symbol for FirstFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for FirstFunction<V>
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
        format!("first({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for FirstFunction<V>
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
        for var in &self.conditions {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        for (polynomial, condition_var) in self.polynomials.iter().zip(&self.conditions) {
            let condition = match token_table
                .find_by_id(condition_var.id())
                .and_then(|token| token.get_result())
            {
                Some(v) => v,
                None if zero_if_none => from_f64(0.0)?,
                None => return None,
            };
            if to_f64(&condition)?.abs() > f64::EPSILON {
                return evaluate_linear(polynomial, token_table, zero_if_none);
            }
        }

        if zero_if_none { from_f64(0.0) } else { None }
    }
}

impl<V> LinearIntermediateSymbol<V> for FirstFunction<V>
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
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange};

    #[test]
    fn first_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(50_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let c = BinaryVariableItem::create(VariableId::standalone(50_001), "c");
        let f: FirstFunction<f64> = FirstFunction::new(
            6000,
            "first_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            vec![c.clone()],
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
            .expect("first constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "first_bound_first_ub_0")
            .expect("first upper constraint should exist");
        let condition_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("condition term should exist");

        // 2x + 1 with x in [-2, 3] => range [-3, 7], therefore M = 7.
        assert!((upper.inequality.rhs - 7.0).abs() <= 1e-9);
        assert!((*condition_term.coefficient() - 7.0).abs() <= 1e-9);
    }

    #[test]
    fn first_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(50_010), "x");
        let c = BinaryVariableItem::create(VariableId::standalone(50_011), "c");
        let f: FirstFunction<f64> = FirstFunction::new(
            6001,
            "first_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            vec![c.clone()],
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
            .expect("first constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "first_default_first_ub_0")
            .expect("first upper constraint should exist");
        let condition_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("condition term should exist");

        assert!((upper.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!(
            (*condition_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9,
            "first fallback coefficient={}, rhs={}",
            *condition_term.coefficient(),
            upper.inequality.rhs
        );
    }
}
