//! 取模函数符号 / Modulo function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, IntegerVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
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

const MOD_EPSILON: f64 = 1e-8;

/// 取模函数符号 / Modulo function symbol.
///
/// 表示取模运算 `input mod divisor`，将线性多项式输入对除数取模，
/// 结果为余数部分，同时引入整数商变量。
///
/// Represents the modulo operation `input mod divisor`, taking the remainder
/// of a linear polynomial input divided by the divisor, with an integer
/// quotient variable introduced alongside the result.
#[derive(Debug, Clone)]
pub struct ModFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 中间符号标识符 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 输入线性多项式 / Input linear polynomial
    input: Linear<V>,
    /// 除数 / Divisor
    divisor: V,
    /// 余数结果变量（连续变量） / Remainder result variable (continuous)
    result_var: ContinuousVariableItem,
    /// 商变量（整数变量） / Quotient variable (integer)
    quotient_var: IntegerVariableItem,
    /// 显式声明的依赖标识符列表 / Explicitly declared dependency identifier list
    declared_dependency_ids: Vec<u64>,
}

impl<V> ModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的取模函数符号 / Create a new modulo function symbol
    pub fn new(id: u64, name: &str, input: Linear<V>, divisor: V) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(VariableId::new(group_id, 0), name);
        let quotient_var =
            IntegerVariableItem::create(VariableId::new(group_id, 1), &format!("{}_q", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            divisor,
            result_var,
            quotient_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置显式声明的依赖标识符 / Set explicitly declared dependency identifiers
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.input = input;
        cloned
    }

    /// 获取余数结果变量的引用 / Get a reference to the remainder result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取输入线性多项式的引用 / Get a reference to the input linear polynomial
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取除数的引用 / Get a reference to the divisor
    pub fn divisor(&self) -> &V {
        &self.divisor
    }

    /// 获取商变量的引用 / Get a reference to the quotient variable
    pub fn quotient_variable(&self) -> &IntegerVariableItem {
        &self.quotient_var
    }
}

impl<V> Display for ModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "mod({})", self.id.name)
    }
}

impl<V> DynSymbol for ModFunction<V>
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

impl<V> Symbol for ModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ModFunction<V>
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
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "mod result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let quotient_index = symbol_to_index
            .get(&(self.quotient_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "mod quotient variable id {}",
                    self.quotient_var.id().unique_id()
                ))
            })?;

        let divisor = to_f64(&self.divisor).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "mod `{}` divisor cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !divisor.is_finite() || divisor.abs() <= MOD_EPSILON {
            return Err(ModelError::InvalidConstraint(format!(
                "mod `{}` requires finite non-zero divisor for mechanism injection",
                self.id.name
            ))
            .into());
        }

        let mut monomials = Vec::with_capacity(self.input.monomials().len() + 2);
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "mod `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let input_index = monomial.var_index();
            monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(coefficient, "mod input coefficient")?,
                input_index,
            ));
        }
        monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-divisor, "mod divisor coefficient")?,
            quotient_index,
        ));
        monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "mod result coefficient")?,
            result_index,
        ));

        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "mod `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let source = Arc::new(self.clone());
        let mut constraints = vec![LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    monomials,
                    convert_f64_to_v::<V>(input_constant, "mod equality constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "mod equality rhs")?,
            ),
            &format!("{}_eq", self.id.name),
            source.clone(),
        )];

        if divisor > 0.0 {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "mod result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "mod lower constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "mod lower rhs")?,
                ),
                &format!("{}_lb", self.id.name),
                source.clone(),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "mod result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "mod upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(divisor - MOD_EPSILON, "mod upper rhs")?,
                ),
                &format!("{}_ub", self.id.name),
                source,
            ));
        } else {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "mod result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "mod upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "mod upper rhs")?,
                ),
                &format!("{}_ub", self.id.name),
                source.clone(),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "mod result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "mod lower constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(divisor + MOD_EPSILON, "mod lower rhs")?,
                ),
                &format!("{}_lb", self.id.name),
                source,
            ));
        }

        Ok(constraints)
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
        format!("mod({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for ModFunction<V>
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
            self.quotient_var.clone(),
            self.quotient_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = evaluate_linear(&self.input, token_table, zero_if_none)?;
        let x = to_f64(&x)?;
        let divisor = to_f64(&self.divisor)?;

        if divisor.abs() <= f64::EPSILON {
            return if zero_if_none { Some(V::zero()) } else { None };
        }

        let quotient = (x / divisor).floor();
        from_f64(x - divisor * quotient)
    }
}

impl<V> LinearIntermediateSymbol<V> for ModFunction<V>
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
