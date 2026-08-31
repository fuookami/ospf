//! 绝对值函数符号 / Abs function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_linear_abs_bound_from_tokens};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

/// 绝对值函数 / Abs Function
///
/// 数学形式 / Mathematical Form:
/// - result = |x|
#[derive(Debug, Clone)]
pub struct AbsFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    result_var: ContinuousVariableItem,
    side_var: BinaryVariableItem,
}

impl<V> AbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的绝对值函数 / Create new abs function
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_abs", name));
        let side_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_side", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            side_var,
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建绝对值函数。
    /// Create an abs function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, input: Linear<V>) -> Self {
        Self::new(next_auto_intermediate_symbol_id(), name.as_ref(), input)
    }

    /// 使用自动 ID 与自动名称创建绝对值函数。
    /// Create an abs function with an auto id and auto-generated name.
    pub fn auto(input: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("abs", id);
        Self::new(id, &name, input)
    }

    /// 获取输入线性多项式 / Get the input linear polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取绝对值结果变量 / Get the absolute-value result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取符号辅助变量 / Get the sign auxiliary variable.
    pub fn side_variable(&self) -> &BinaryVariableItem {
        &self.side_var
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64>
    where
        V: ToPrimitive,
    {
        infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| (2.0 * bound).max(BIG_M_POLICY.min()))
    }
}

impl<V> AbsFunction<V>
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
                    "abs result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.side_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "abs side variable id {}",
                    self.side_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();
        let mut input_monomials = Vec::with_capacity(self.input.monomials().len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "abs `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            input_monomials.push((coefficient, monomial.var_index()));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "abs `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        // y - x >= 0
        let mut ge_x_monomials = Vec::with_capacity(self.input.monomials().len() + 1);
        ge_x_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &input_monomials {
            ge_x_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "abs input coefficient")?,
                *index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    ge_x_monomials,
                    convert_f64_to_v::<V>(-input_constant, "abs input constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "abs rhs")?,
            ),
            &format!("{}_abs_ge_x", self.id.name),
            Arc::new(self.clone()),
        ));

        // y + x >= 0
        let mut ge_neg_x_monomials = Vec::with_capacity(self.input.monomials().len() + 1);
        ge_neg_x_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &input_monomials {
            ge_neg_x_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "abs input coefficient")?,
                *index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    ge_neg_x_monomials,
                    convert_f64_to_v::<V>(input_constant, "abs input constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "abs rhs")?,
            ),
            &format!("{}_abs_ge_neg_x", self.id.name),
            Arc::new(self.clone()),
        ));

        // y - x + M * b <= M
        let mut le_pos_branch_monomials = Vec::with_capacity(self.input.monomials().len() + 2);
        le_pos_branch_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &input_monomials {
            le_pos_branch_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "abs input coefficient")?,
                *index,
            ));
        }
        le_pos_branch_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "abs side coefficient")?,
            side_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    le_pos_branch_monomials,
                    convert_f64_to_v::<V>(-input_constant, "abs input constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "abs rhs")?,
            ),
            &format!("{}_abs_pos_branch", self.id.name),
            Arc::new(self.clone()),
        ));

        // y + x - M * b <= 0
        let mut le_neg_branch_monomials = Vec::with_capacity(self.input.monomials().len() + 2);
        le_neg_branch_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &input_monomials {
            le_neg_branch_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "abs input coefficient")?,
                *index,
            ));
        }
        le_neg_branch_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "abs side coefficient")?,
            side_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    le_neg_branch_monomials,
                    convert_f64_to_v::<V>(input_constant, "abs input constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "abs rhs")?,
            ),
            &format!("{}_abs_neg_branch", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl<V> Display for AbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "abs({})", self.id.name)
    }
}

impl<V> DynSymbol for AbsFunction<V>
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

impl<V> Symbol for AbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for AbsFunction<V>
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
        format!("abs({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for AbsFunction<V>
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
            self.side_var.clone(),
            self.side_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        from_f64(to_f64(&value)?.abs())
    }
}

impl<V> LinearIntermediateSymbol<V> for AbsFunction<V>
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
    fn abs_function_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-3.0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let abs = AbsFunction::new(100, "abs_x", poly);
        let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn abs_function_supports_f32_values() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f32>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-3.0_f32);
        tokens.add_token(tx);

        let abs: AbsFunction<f32> = AbsFunction::new(
            1001,
            "abs_f32",
            Linear::new(vec![LinearMonomial::new(2.0_f32, 0)], 1.0_f32),
        );
        let value =
            <AbsFunction<f32> as FunctionSymbol<f32>>::calculate_value(&abs, &tokens, false);
        assert_eq!(value, Some(5.0_f32));

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let constraints = abs
            .mechanism_constraints(&symbol_to_index)
            .expect("f32 abs mechanism constraints should be generated");

        assert_eq!(constraints.len(), 4);
    }

    #[test]
    fn abs_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], -1.0);
        let abs = AbsFunction::new(101, "abs_x", poly);
        let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, true);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn abs_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let abs: AbsFunction<f64> = AbsFunction::new(
            102,
            "abs_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let constraints = abs
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("abs mechanism constraints should be generated");
        let pos_branch = constraints
            .iter()
            .find(|constraint| constraint.name == "abs_bound_abs_pos_branch")
            .expect("positive branch constraint should exist");
        let side_term = pos_branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side variable term should exist");

        // 2x + 1 且 x ∈ [-2, 3] => 取值范围 [-3, 7]，绝对值上界为 7。
        // 2x + 1 with x in [-2, 3] => range [-3, 7], abs bound = 7.
        // abs 分支松弛需要覆盖分支切换，因此 M 取 2 * abs bound。
        // For abs branch relaxation, M must cover branch switch, so we use 2 * abs bound.
        assert!((pos_branch.inequality.rhs - 14.0).abs() <= 1e-9);
        assert!((*side_term.coefficient() - 14.0).abs() <= 1e-9);
    }

    #[test]
    fn abs_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let abs: AbsFunction<f64> = AbsFunction::new(
            103,
            "abs_default_m",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let constraints = abs
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("abs mechanism constraints should be generated");
        let pos_branch = constraints
            .iter()
            .find(|constraint| constraint.name == "abs_default_m_abs_pos_branch")
            .expect("positive branch constraint should exist");
        let side_term = pos_branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side variable term should exist");

        assert!((pos_branch.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*side_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
