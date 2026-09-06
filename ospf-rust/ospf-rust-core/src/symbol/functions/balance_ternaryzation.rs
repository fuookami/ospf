//! 平衡三值变量函数符号 / Balanced ternary variable function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

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

/// Represents a balanced ternary variable: values in {-1, 0, 1}.
#[derive(Debug, Clone)]
pub struct BalanceTernaryzationFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    result_var: ContinuousVariableItem,
    positive_var: BinaryVariableItem,
    negative_var: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
    _marker: std::marker::PhantomData<V>,
}

impl<V> BalanceTernaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建三值平衡函数 / Create a ternary-balancing function.
    pub fn new(id: u64, name: &str) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(VariableId::new(group_id, 0), name);

        let positive_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_pos", name));

        let negative_var =
            BinaryVariableItem::create(VariableId::new(group_id, 2), &format!("{}_neg", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            result_var,
            positive_var,
            negative_var,
            declared_dependency_ids: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
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
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
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

        let relation = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "balance result coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(-1.0, "balance positive coefficient")?,
                            positive_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "balance negative coefficient")?,
                            negative_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "balance relation constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "balance relation rhs")?,
            ),
            &format!("{}_bal_relation", self.id.name),
            Arc::new(self.clone()),
        );

        let exclusivity = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "balance positive exclusivity coefficient")?,
                            positive_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "balance negative exclusivity coefficient")?,
                            negative_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "balance exclusivity constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(1.0, "balance exclusivity rhs")?,
            ),
            &format!("{}_bal_exclusive", self.id.name),
            Arc::new(self.clone()),
        );

        Ok(vec![relation, exclusivity])
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
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
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
        let positive = match token_table
            .find_by_id(self.positive_var.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => from_f64(0.0)?,
            None => return None,
        };
        let negative = match token_table
            .find_by_id(self.negative_var.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => from_f64(0.0)?,
            None => return None,
        };

        let pos = if to_f64(&positive)?.abs() > f64::EPSILON {
            1.0
        } else {
            0.0
        };
        let neg = if to_f64(&negative)?.abs() > f64::EPSILON {
            1.0
        } else {
            0.0
        };
        from_f64(pos - neg)
    }
}

impl<V> LinearIntermediateSymbol<V> for BalanceTernaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![
                LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    self.positive_var.index(),
                ),
                LinearMonomial::new(
                    from_f64(-1.0).expect("convert -1.0"),
                    self.negative_var.index(),
                ),
            ],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}
