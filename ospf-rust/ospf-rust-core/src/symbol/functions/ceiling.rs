//! 向上取整函数符号 / Ceiling function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_linear_abs_bound_from_tokens};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, IntegerVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
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
const ROUNDING_EPSILON: f64 = 1e-8;

/// 向上取整函数符号 / Ceiling function symbol.
///
/// 数学形式 / Mathematical Form:
/// - result = ceil(x)
#[derive(Debug, Clone)]
pub struct CeilingFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 中间符号标识 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 输入线性多项式 / Input linear polynomial
    input: Linear<V>,
    /// 结果连续变量 / Result continuous variable
    result_var: ContinuousVariableItem,
    /// 辅助整数变量 / Auxiliary integer variable
    integer_var: IntegerVariableItem,
    /// 声明的依赖标识列表 / Declared dependency identifier list
    declared_dependency_ids: Vec<u64>,
}

impl<V> CeilingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的向上取整函数 / Create new ceiling function.
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_ceil", name));
        let integer_var =
            IntegerVariableItem::create(VariableId::new(group_id, 1), &format!("{}_int", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            integer_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动标识和调用方指定名称创建向上取整函数 / Create a ceiling function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, input: Linear<V>) -> Self {
        Self::new(next_auto_intermediate_symbol_id(), name.as_ref(), input)
    }

    /// 使用自动标识和自动生成名称创建向上取整函数 / Create a ceiling function with an auto id and auto-generated name.
    pub fn auto(input: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("ceiling", id);
        Self::new(id, &name, input)
    }

    /// 设置声明的依赖标识 / Set declared dependency identifiers.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取输入线性多项式 / Get the input linear polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取结果连续变量 / Get the result continuous variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取整数结果变量 / Get the integer result variable.
    pub fn integer_variable(&self) -> &IntegerVariableItem {
        &self.integer_var
    }
}

impl<V> CeilingFunction<V>
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
        infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| bound.max(BIG_M_POLICY.min()))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        _big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "ceiling result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let integer_index = symbol_to_index
            .get(&(self.integer_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "ceiling integer variable id {}",
                    self.integer_var.id().unique_id()
                ))
            })?;

        let mut input_monomials = Vec::with_capacity(self.input.monomials().len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "ceiling `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let input_index = monomial.var_index();
            input_monomials.push((coefficient, input_index));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "ceiling `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();

        // result_link: result_var - integer_var = 0
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "ceiling result coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(-1.0, "ceiling integer coefficient")?,
                            integer_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "ceiling equality constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "ceiling equality rhs")?,
            ),
            &format!("{}_result_link", self.id.name),
            source.clone(),
        ));

        // ceil_ub: input - integer_var <= 0  (integer_var >= input)
        let mut ceil_ub_monomials = Vec::with_capacity(input_monomials.len() + 1);
        for (coefficient, index) in &input_monomials {
            ceil_ub_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "ceiling input coefficient")?,
                *index,
            ));
        }
        ceil_ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "ceiling integer coefficient")?,
            integer_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    ceil_ub_monomials,
                    convert_f64_to_v::<V>(input_constant, "ceiling input constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "ceiling rhs")?,
            ),
            &format!("{}_ceil_ub", self.id.name),
            source.clone(),
        ));

        // ceil_lb: input - integer_var >= -1 + epsilon  (integer_var < input + 1)
        let mut ceil_lb_monomials = Vec::with_capacity(input_monomials.len() + 1);
        for (coefficient, index) in &input_monomials {
            ceil_lb_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "ceiling input coefficient")?,
                *index,
            ));
        }
        ceil_lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "ceiling integer coefficient")?,
            integer_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    ceil_lb_monomials,
                    convert_f64_to_v::<V>(input_constant, "ceiling input constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(-1.0 + ROUNDING_EPSILON, "ceiling rhs")?,
            ),
            &format!("{}_ceil_lb", self.id.name),
            source.clone(),
        ));

        Ok(constraints)
    }
}

impl<V> Display for CeilingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ceil({})", self.id.name)
    }
}

impl<V> DynSymbol for CeilingFunction<V>
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

impl<V> Symbol for CeilingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for CeilingFunction<V>
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
        Category::Nonlinear
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
        format!("ceil({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for CeilingFunction<V>
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
            self.integer_var.clone(),
            self.integer_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        let value = to_f64(&value)?;
        from_f64(value.ceil())
    }
}

impl<V> LinearIntermediateSymbol<V> for CeilingFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + One
        + ToPrimitive
        + FromPrimitive,
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
    use std::collections::HashMap;

    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn ceiling_function_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.3);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let ceil = CeilingFunction::new(100, "ceil_x", poly);
        let value = <CeilingFunction as FunctionSymbol>::calculate_value(&ceil, &tokens, false);
        assert_eq!(value, Some(3.0));
    }

    #[test]
    fn ceiling_function_calculate_value_negative() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-2.3);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let ceil = CeilingFunction::new(101, "ceil_x", poly);
        let value = <CeilingFunction as FunctionSymbol>::calculate_value(&ceil, &tokens, false);
        assert_eq!(value, Some(-2.0));
    }

    #[test]
    fn ceiling_function_calculate_value_integer_input() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(5.0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let ceil = CeilingFunction::new(102, "ceil_x", poly);
        let value = <CeilingFunction as FunctionSymbol>::calculate_value(&ceil, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn ceiling_function_supports_f32_values() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f32>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.7_f32);
        tokens.add_token(tx);

        let ceil: CeilingFunction<f32> = CeilingFunction::new(
            1001,
            "ceil_f32",
            Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], 0.0_f32),
        );
        let value =
            <CeilingFunction<f32> as FunctionSymbol<f32>>::calculate_value(&ceil, &tokens, false);
        assert_eq!(value, Some(3.0_f32));

        let result_id = ceil.result_variable().id().unique_id() as usize;
        let integer_id = ceil.integer_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (integer_id, 2usize)]);
        let constraints = ceil
            .mechanism_constraints(&symbol_to_index)
            .expect("f32 ceiling mechanism constraints should be generated");

        assert_eq!(constraints.len(), 3);
    }

    #[test]
    fn ceiling_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let ceil = CeilingFunction::new(103, "ceil_x", poly);
        let value = <CeilingFunction as FunctionSymbol>::calculate_value(&ceil, &tokens, true);
        assert_eq!(value, Some(0.0));
    }

    #[test]
    fn ceiling_function_with_linear_input() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(1.2);
        tokens.add_token(tx);

        // 2x + 1 => 2*1.2 + 1 = 3.4 => ceil(3.4) = 4
        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let ceil = CeilingFunction::new(104, "ceil_x", poly);
        let value = <CeilingFunction as FunctionSymbol>::calculate_value(&ceil, &tokens, false);
        assert_eq!(value, Some(4.0));
    }

    #[test]
    fn ceiling_function_mechanism_constraints_structure() {
        let ceil: CeilingFunction<f64> = CeilingFunction::new(
            200,
            "ceil_struct",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <CeilingFunction<f64> as FunctionSymbol<f64>>::register_tokens(&ceil, &mut aux_tokens)
            .expect("ceiling tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }

        let constraints = ceil
            .mechanism_constraints(&symbol_to_index)
            .expect("ceiling constraints should be generated");

        assert_eq!(constraints.len(), 3);

        let result_link = constraints
            .iter()
            .find(|c| c.name == "ceil_struct_result_link")
            .expect("result_link constraint should exist");
        assert_eq!(result_link.inequality.relation, ConstraintRelation::Equal);

        let ceil_ub = constraints
            .iter()
            .find(|c| c.name == "ceil_struct_ceil_ub")
            .expect("ceil_ub constraint should exist");
        assert_eq!(ceil_ub.inequality.relation, ConstraintRelation::LessEqual);

        let ceil_lb = constraints
            .iter()
            .find(|c| c.name == "ceil_struct_ceil_lb")
            .expect("ceil_lb constraint should exist");
        assert_eq!(
            ceil_lb.inequality.relation,
            ConstraintRelation::GreaterEqual
        );
        assert!((ceil_lb.inequality.rhs - (-1.0 + ROUNDING_EPSILON)).abs() <= 1e-9);
    }

    #[test]
    fn ceiling_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let ceil: CeilingFunction<f64> = CeilingFunction::new(
            3000,
            "ceil_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let mut aux_tokens = Vec::new();
        <CeilingFunction<f64> as FunctionSymbol<f64>>::register_tokens(&ceil, &mut aux_tokens)
            .expect("ceiling tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = ceil
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("ceiling constraints should be generated");

        assert_eq!(constraints.len(), 3);

        let ceil_ub = constraints
            .iter()
            .find(|c| c.name == "ceil_bound_ceil_ub")
            .expect("ceil_ub constraint should exist");
        assert_eq!(ceil_ub.inequality.relation, ConstraintRelation::LessEqual);
    }

    #[test]
    fn ceiling_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_010), "x");
        let ceil: CeilingFunction<f64> = CeilingFunction::new(
            3001,
            "ceil_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <CeilingFunction<f64> as FunctionSymbol<f64>>::register_tokens(&ceil, &mut aux_tokens)
            .expect("ceiling tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = ceil
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("ceiling constraints should be generated");

        assert_eq!(constraints.len(), 3);

        let ceil_ub = constraints
            .iter()
            .find(|c| c.name == "ceil_default_ceil_ub")
            .expect("ceil_ub constraint should exist");
        assert_eq!(ceil_ub.inequality.relation, ConstraintRelation::LessEqual);
    }
}
