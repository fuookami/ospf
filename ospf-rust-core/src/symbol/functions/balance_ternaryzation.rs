//! 平衡三值变量函数符号 / Balanced ternary variable function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{MIN_BIG_M, infer_linear_abs_bound_from_tokens};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

const DEFAULT_STRICT_BOUNDARY: f64 = 1e-10;

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
            Some(value) => value,
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

/// Maps a linear expression to `-1`, `0`, or `1` using a symmetric zero band.
#[derive(Debug, Clone)]
pub struct BalanceTernaryzationFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    epsilon: V,
    fallback_big_m: V,
    strict_boundary: V,
    result_var: ContinuousVariableItem,
    positive_var: BinaryVariableItem,
    negative_var: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
{
    /// 创建平衡三值化函数。`epsilon` 定义零带，`fallback_big_m` 仅在无法从变量界推断时使用。
    /// Create a balanced ternary function. `epsilon` defines the zero band and
    /// `fallback_big_m` is used only when variable bounds cannot provide a tighter value.
    pub fn new(id: u64, name: &str, input: Linear<V>, epsilon: V, fallback_big_m: V) -> Self {
        let epsilon_f64 = to_f64(&epsilon).expect("balance ternary epsilon must convert to f64");
        let fallback_big_m_f64 =
            to_f64(&fallback_big_m).expect("balance ternary fallback big-M must convert to f64");
        assert!(
            epsilon_f64.is_finite() && epsilon_f64 >= 0.0,
            "balance ternary epsilon must be finite and non-negative"
        );
        assert!(
            fallback_big_m_f64.is_finite()
                && fallback_big_m_f64 > epsilon_f64 + DEFAULT_STRICT_BOUNDARY,
            "balance ternary fallback big-M must be finite and exceed epsilon plus the strict boundary"
        );
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(VariableId::new(group_id, 0), name);

        let positive_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_pos", name));

        let negative_var =
            BinaryVariableItem::create(VariableId::new(group_id, 2), &format!("{}_neg", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            epsilon,
            fallback_big_m,
            strict_boundary: from_f64(DEFAULT_STRICT_BOUNDARY)
                .expect("convert balance ternary strict boundary"),
            result_var,
            positive_var,
            negative_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取输入多项式 / Get the input polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取零带阈值 / Get the zero-band threshold.
    pub fn epsilon(&self) -> &V {
        &self.epsilon
    }

    /// 获取回退 Big-M / Get the fallback Big-M value.
    pub fn fallback_big_m(&self) -> &V {
        &self.fallback_big_m
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取正方向指示变量 / Get the positive-direction indicator variable.
    pub fn positive_variable(&self) -> &BinaryVariableItem {
        &self.positive_var
    }

    /// 获取负方向指示变量 / Get the negative-direction indicator variable.
    pub fn negative_variable(&self) -> &BinaryVariableItem {
        &self.negative_var
    }

    fn configured_big_m(&self) -> Result<f64> {
        let epsilon = to_f64(&self.epsilon).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` epsilon cannot be converted to f64",
                self.id.name
            ))
        })?;
        let strict_boundary = to_f64(&self.strict_boundary).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` strict boundary cannot be converted to f64",
                self.id.name
            ))
        })?;
        let fallback = to_f64(&self.fallback_big_m).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` fallback big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !fallback.is_finite() || fallback <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "balance ternary `{}` fallback big-M must be finite and positive",
                self.id.name
            ))
            .into());
        }
        Ok(fallback.max(epsilon + strict_boundary).max(MIN_BIG_M))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "balance ternary result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let positive_index = symbol_to_index
            .get(&(self.positive_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "balance ternary positive variable id {}",
                    self.positive_var.id().unique_id()
                ))
            })?;
        let negative_index = symbol_to_index
            .get(&(self.negative_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "balance ternary negative variable id {}",
                    self.negative_var.id().unique_id()
                ))
            })?;

        let epsilon = to_f64(&self.epsilon).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` epsilon cannot be converted to f64",
                self.id.name
            ))
        })?;
        let strict_boundary = to_f64(&self.strict_boundary).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` strict boundary cannot be converted to f64",
                self.id.name
            ))
        })?;

        let make_input =
            |indicator_index: usize, indicator_coefficient: f64| -> Result<Linear<V>> {
                let mut monomials = self.input.monomials().to_vec();
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v(indicator_coefficient, "balance indicator coefficient")?,
                    indicator_index,
                ));
                Ok(Linear::new(monomials, self.input.constant_term().clone()))
            };
        let make_constraint = |polynomial: Linear<V>,
                               relation,
                               rhs: f64,
                               suffix: &str|
         -> Result<LinearConstraint<V>> {
            Ok(LinearConstraint::from_symbol(
                LinearInequality::new(
                    polynomial,
                    relation,
                    convert_f64_to_v(rhs, "balance constraint rhs")?,
                ),
                &format!("{}_{}", self.id.name, suffix),
                Arc::new(self.clone()),
            ))
        };

        let relation = make_constraint(
            Linear::new(
                vec![
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance result coefficient")?,
                        result_index,
                    ),
                    LinearMonomial::new(
                        convert_f64_to_v(-1.0, "balance positive coefficient")?,
                        positive_index,
                    ),
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance negative coefficient")?,
                        negative_index,
                    ),
                ],
                convert_f64_to_v(0.0, "balance result constant")?,
            ),
            ConstraintRelation::Equal,
            0.0,
            "bter_result",
        )?;
        let exclusivity = make_constraint(
            Linear::new(
                vec![
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance positive coefficient")?,
                        positive_index,
                    ),
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance negative coefficient")?,
                        negative_index,
                    ),
                ],
                convert_f64_to_v(0.0, "balance exclusivity constant")?,
            ),
            ConstraintRelation::LessEqual,
            1.0,
            "bter_exclusive",
        )?;
        let positive_lower = make_constraint(
            make_input(positive_index, -big_m)?,
            ConstraintRelation::GreaterEqual,
            epsilon + strict_boundary - big_m,
            "bter_positive_lb",
        )?;
        let positive_upper = make_constraint(
            make_input(positive_index, -big_m)?,
            ConstraintRelation::LessEqual,
            epsilon,
            "bter_positive_ub",
        )?;
        let negative_upper = make_constraint(
            make_input(negative_index, big_m)?,
            ConstraintRelation::LessEqual,
            big_m - epsilon - strict_boundary,
            "bter_negative_ub",
        )?;
        let negative_lower = make_constraint(
            make_input(negative_index, big_m)?,
            ConstraintRelation::GreaterEqual,
            -epsilon,
            "bter_negative_lb",
        )?;

        Ok(vec![
            relation,
            exclusivity,
            positive_lower,
            positive_upper,
            negative_upper,
            negative_lower,
        ])
    }
}

impl<V> Display for BalanceTernaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "bal_ternary({})", self.id.name)
    }
}

impl<V> DynSymbol for BalanceTernaryzationFunction<V>
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

impl<V> Symbol for BalanceTernaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
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
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, self.configured_big_m()?)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let epsilon = to_f64(&self.epsilon).unwrap_or(0.0);
        let strict_boundary = to_f64(&self.strict_boundary).unwrap_or(DEFAULT_STRICT_BOUNDARY);
        let big_m = infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| (bound + epsilon + strict_boundary).max(MIN_BIG_M))
            .unwrap_or(self.configured_big_m()?);
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
        format!("bal_ternary({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
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
            self.positive_var.clone(),
            self.positive_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.negative_var.clone(),
            self.negative_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let epsilon = to_f64(&self.epsilon)?;
        let strict_boundary = to_f64(&self.strict_boundary)?;
        if input >= epsilon + strict_boundary {
            from_f64(1.0)
        } else if input > epsilon {
            None
        } else if input <= -epsilon - strict_boundary {
            from_f64(-1.0)
        } else if input < -epsilon {
            None
        } else {
            from_f64(0.0)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
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
