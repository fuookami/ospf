//! If 函数符号
//! If Function Symbol

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
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_linear_abs_bound_from_tokens};

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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

/// If 函数 / If Function
///
/// 数学形式 / Mathematical Form:
/// - result = if condition != 0 then then_expr else else_expr
#[derive(Debug, Clone)]
pub struct IfFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    condition: Linear<V>,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
    result_var: ContinuousVariableItem,
    condition_indicator: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> IfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的 If 函数 / Create new if function
    pub fn new(
        id: u64,
        name: &str,
        condition: Linear<V>,
        then_expr: Linear<V>,
        else_expr: Linear<V>,
    ) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(
            VariableId::new(group_id, 0),
            &format!("{}_if_result", name),
        );
        let condition_indicator = BinaryVariableItem::create(
            VariableId::new(group_id, 1),
            &format!("{}_if_cond", name),
        );

        Self {
            id: IntermediateSymbolId::new(id, name),
            condition,
            then_expr,
            else_expr,
            result_var,
            condition_indicator,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 If 函数。
    /// Create an if function with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        condition: Linear<V>,
        then_expr: Linear<V>,
        else_expr: Linear<V>,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            condition,
            then_expr,
            else_expr,
        )
    }

    /// 使用自动 ID 与自动名称创建 If 函数。
    /// Create an if function with an auto id and auto-generated name.
    pub fn auto(condition: Linear<V>, then_expr: Linear<V>, else_expr: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if", id);
        Self::new(id, &name, condition, then_expr, else_expr)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn condition_polynomial(&self) -> &Linear<V> {
        &self.condition
    }

    pub fn then_polynomial(&self) -> &Linear<V> {
        &self.then_expr
    }

    pub fn else_polynomial(&self) -> &Linear<V> {
        &self.else_expr
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn condition_indicator_variable(&self) -> &BinaryVariableItem {
        &self.condition_indicator
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64>
    where
        V: ToPrimitive,
    {
        let cond_bound = infer_linear_abs_bound_from_tokens(&self.condition, tokens)?;
        let then_bound = infer_linear_abs_bound_from_tokens(&self.then_expr, tokens)?;
        let else_bound = infer_linear_abs_bound_from_tokens(&self.else_expr, tokens)?;
        let bound = cond_bound.max(then_bound).max(else_bound);
        Some((2.0 * bound).max(BIG_M_POLICY.min()))
    }
}

impl<V> IfFunction<V>
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
                    "if result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let indicator_index = symbol_to_index
            .get(&(self.condition_indicator.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if condition indicator variable id {}",
                    self.condition_indicator.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();

        // Collect f64 representations for condition
        let mut condition_monomials_f64 = Vec::with_capacity(self.condition.monomials().len());
        for monomial in self.condition.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if `{}` condition coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            condition_monomials_f64.push((coefficient, monomial.var_index()));
        }
        let condition_constant = to_f64(self.condition.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if `{}` condition constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        // Collect f64 representations for then_expr
        let mut then_monomials_f64 = Vec::with_capacity(self.then_expr.monomials().len());
        for monomial in self.then_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if `{}` then coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            then_monomials_f64.push((coefficient, monomial.var_index()));
        }
        let then_constant = to_f64(self.then_expr.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if `{}` then constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        // Collect f64 representations for else_expr
        let mut else_monomials_f64 = Vec::with_capacity(self.else_expr.monomials().len());
        for monomial in self.else_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if `{}` else coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            else_monomials_f64.push((coefficient, monomial.var_index()));
        }
        let else_constant = to_f64(self.else_expr.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if `{}` else constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        // Binaryzation constraint 1: condition - M * indicator <= 0
        let mut bin_lower_monomials = Vec::with_capacity(self.condition.monomials().len() + 1);
        for (coefficient, index) in &condition_monomials_f64 {
            bin_lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "if condition coefficient")?,
                *index,
            ));
        }
        bin_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    bin_lower_monomials,
                    convert_f64_to_v::<V>(condition_constant, "if condition constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if binaryzation lower rhs")?,
            ),
            &format!("{}_if_bin_lower", self.id.name),
            Arc::new(self.clone()),
        ));

        // Binaryzation constraint 2: -condition - M * indicator <= 0
        let mut bin_upper_monomials = Vec::with_capacity(self.condition.monomials().len() + 1);
        for (coefficient, index) in &condition_monomials_f64 {
            bin_upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "if condition coefficient")?,
                *index,
            ));
        }
        bin_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    bin_upper_monomials,
                    convert_f64_to_v::<V>(-condition_constant, "if condition constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if binaryzation upper rhs")?,
            ),
            &format!("{}_if_bin_upper", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 1: result - then_expr + M * indicator <= M
        let mut branch1_monomials = Vec::with_capacity(self.then_expr.monomials().len() + 2);
        branch1_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &then_monomials_f64 {
            branch1_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "if then coefficient")?,
                *index,
            ));
        }
        branch1_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch1_monomials,
                    convert_f64_to_v::<V>(-then_constant, "if then constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "if branching rhs")?,
            ),
            &format!("{}_if_branch_le_then", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 2: -result + then_expr + M * indicator <= M
        let mut branch2_monomials = Vec::with_capacity(self.then_expr.monomials().len() + 2);
        branch2_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &then_monomials_f64 {
            branch2_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "if then coefficient")?,
                *index,
            ));
        }
        branch2_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch2_monomials,
                    convert_f64_to_v::<V>(then_constant, "if then constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "if branching rhs")?,
            ),
            &format!("{}_if_branch_ge_then", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 3: result - else_expr - M * indicator <= 0
        let mut branch3_monomials = Vec::with_capacity(self.else_expr.monomials().len() + 2);
        branch3_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &else_monomials_f64 {
            branch3_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "if else coefficient")?,
                *index,
            ));
        }
        branch3_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch3_monomials,
                    convert_f64_to_v::<V>(-else_constant, "if else constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if branching rhs")?,
            ),
            &format!("{}_if_branch_le_else", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 4: -result + else_expr - M * indicator <= 0
        let mut branch4_monomials = Vec::with_capacity(self.else_expr.monomials().len() + 2);
        branch4_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &else_monomials_f64 {
            branch4_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "if else coefficient")?,
                *index,
            ));
        }
        branch4_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch4_monomials,
                    convert_f64_to_v::<V>(else_constant, "if else constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if branching rhs")?,
            ),
            &format!("{}_if_branch_ge_else", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl<V> Display for IfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "if({})", self.id.name)
    }
}

impl<V> DynSymbol for IfFunction<V>
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

impl<V> Symbol for IfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for IfFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, DEFAULT_BIG_M)
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
        format!("if({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for IfFunction<V>
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
        tokens.push(Token::from_generic(
            self.condition_indicator.clone(),
            self.condition_indicator.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let condition_value = evaluate_linear(&self.condition, token_table, zero_if_none)?;
        let cond_f64 = to_f64(&condition_value)?;
        let eps = f64::EPSILON * 16.0;
        if cond_f64.abs() > eps {
            evaluate_linear(&self.then_expr, token_table, zero_if_none)
        } else {
            evaluate_linear(&self.else_expr, token_table, zero_if_none)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for IfFunction<V>
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
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn if_function_calculate_value_true() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(5.0);
        tokens.add_token(tx);

        // condition: x - 2 (at x=5, condition=3, nonzero => use then_expr)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0);
        // then_expr: 2*x + 1 (at x=5, value=11)
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        // else_expr: x + 3 (at x=5, value=8)
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(200, "my_if", condition, then_expr, else_expr);
        let value = <IfFunction as FunctionSymbol>::calculate_value(&if_func, &tokens, false);
        assert_eq!(value, Some(11.0));
    }

    #[test]
    fn if_function_calculate_value_false() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        // condition: x - 2 (at x=2, condition=0 => use else_expr)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0);
        // then_expr: 2*x + 1 (at x=2, value=5)
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        // else_expr: x + 3 (at x=2, value=5)
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(201, "my_if", condition, then_expr, else_expr);
        let value = <IfFunction as FunctionSymbol>::calculate_value(&if_func, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn if_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        // no result set, so zero_if_none will use 0.0
        tokens.add_token(tx);

        // condition: x (at x=0 with zero_if_none, condition=0 => use else_expr)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        // then_expr: 2*x + 10
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 10.0);
        // else_expr: x + 3
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(202, "my_if", condition, then_expr, else_expr);
        let value = <IfFunction as FunctionSymbol>::calculate_value(&if_func, &tokens, true);
        assert_eq!(value, Some(3.0));
    }

    #[test]
    fn if_function_supports_f32_values() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f32>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(5.0_f32);
        tokens.add_token(tx);

        let condition: Linear<f32> =
            Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], -2.0_f32);
        let then_expr: Linear<f32> =
            Linear::new(vec![LinearMonomial::new(2.0_f32, 0)], 1.0_f32);
        let else_expr: Linear<f32> =
            Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], 3.0_f32);

        let if_func: IfFunction<f32> =
            IfFunction::new(203, "if_f32", condition, then_expr, else_expr);
        let value =
            <IfFunction<f32> as FunctionSymbol<f32>>::calculate_value(&if_func, &tokens, false);
        assert_eq!(value, Some(11.0_f32));

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let constraints = if_func
            .mechanism_constraints(&symbol_to_index)
            .expect("f32 if mechanism constraints should be generated");

        assert_eq!(constraints.len(), 6);
    }

    #[test]
    fn if_function_generates_expected_constraints() {
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0);
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(204, "if_test", condition, then_expr, else_expr);

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let constraints = if_func
            .mechanism_constraints(&symbol_to_index)
            .unwrap();

        // 2 binaryzation + 4 branching = 6 constraints
        assert_eq!(constraints.len(), 6);

        // Verify constraint names
        assert!(constraints
            .iter()
            .any(|c| c.name == "if_test_if_bin_lower"));
        assert!(constraints
            .iter()
            .any(|c| c.name == "if_test_if_bin_upper"));
        assert!(constraints
            .iter()
            .any(|c| c.name == "if_test_if_branch_le_then"));
        assert!(constraints
            .iter()
            .any(|c| c.name == "if_test_if_branch_ge_then"));
        assert!(constraints
            .iter()
            .any(|c| c.name == "if_test_if_branch_le_else"));
        assert!(constraints
            .iter()
            .any(|c| c.name == "if_test_if_branch_ge_else"));
    }

    #[test]
    fn if_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        // condition: 2x + 1 with x in [-2, 3] => range [-3, 7], abs bound = 7
        let condition = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        // then_expr: x with x in [-2, 3] => range [-2, 3], abs bound = 3
        let then_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        // else_expr: -x + 5 with x in [-2, 3] => range [2, 7], abs bound = 7
        let else_expr = Linear::new(vec![LinearMonomial::new(-1.0, 0)], 5.0);

        let if_func: IfFunction<f64> =
            IfFunction::new(205, "if_bound", condition, then_expr, else_expr);

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(if_func.result_variable().clone(), 1),
            Token::from_generic(if_func.condition_indicator_variable().clone(), 2),
        ];

        let constraints = if_func
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if mechanism constraints should be generated");

        // M = 2 * max(7, 3, 7) = 14
        let bin_lower = constraints
            .iter()
            .find(|constraint| constraint.name == "if_bound_if_bin_lower")
            .expect("binaryzation lower constraint should exist");
        let indicator_term = bin_lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("indicator variable term should exist");

        assert!((bin_lower.inequality.rhs - 0.0).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() + 14.0).abs() <= 1e-9);
    }

    #[test]
    fn if_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let then_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);

        let if_func: IfFunction<f64> =
            IfFunction::new(206, "if_default_m", condition, then_expr, else_expr);

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(if_func.result_variable().clone(), 1),
            Token::from_generic(if_func.condition_indicator_variable().clone(), 2),
        ];

        let constraints = if_func
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if mechanism constraints should be generated");
        let bin_lower = constraints
            .iter()
            .find(|constraint| constraint.name == "if_default_m_if_bin_lower")
            .expect("binaryzation lower constraint should exist");
        let indicator_term = bin_lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("indicator variable term should exist");

        assert!((bin_lower.inequality.rhs - 0.0).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
