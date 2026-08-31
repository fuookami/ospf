//! If-Then 蕴含函数符号 / If-Then implication function symbol

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
use crate::variable::BinaryVariableItem;
use super::super::{

    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::{InequalityFunction, InequalityKind};

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

fn relation_to_kind(relation: ConstraintRelation) -> InequalityKind {
    match relation {
        ConstraintRelation::LessEqual => InequalityKind::LessEqual,
        ConstraintRelation::Equal => InequalityKind::Equal,
        ConstraintRelation::GreaterEqual => InequalityKind::GreaterEqual,
    }
}

fn auxiliary_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

fn evaluate_inequality<V>(
    inequality: &LinearInequality<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<bool>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive,
{
    let left = to_f64(&evaluate_linear(
        &inequality.polynomial,
        token_table,
        zero_if_none,
    )?)?;
    let right = to_f64(&inequality.rhs)?;
    let eps = f64::EPSILON * 16.0;
    Some(match inequality.relation {
        ConstraintRelation::LessEqual => left <= right + eps,
        ConstraintRelation::Equal => (left - right).abs() <= eps,
        ConstraintRelation::GreaterEqual => left + eps >= right,
    })
}

/// 表示蕴含关系 `premise => consequence`。
/// Represents implication `premise => consequence`.
#[derive(Debug, Clone)]
pub struct IfThenFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 前提不等式 / Premise inequality
    premise: LinearInequality<V>,
    /// 结论不等式 / Consequence inequality
    consequence: LinearInequality<V>,
    /// 前提不等式指示函数 / Premise inequality indicator function
    premise_indicator: InequalityFunction<V>,
    /// 结论不等式指示函数 / Consequence inequality indicator function
    consequence_indicator: InequalityFunction<V>,
    /// 结果二值变量 / Result binary variable
    result_var: BinaryVariableItem,
    /// 是否为约束模式 / Whether constraint mode is enabled
    constraint_mode: bool,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> IfThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的条件执行函数 / Create a new if-then function
    pub fn new(
        id: u64,
        name: &str,
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
    ) -> Self {
        Self::with_mode(id, name, premise, consequence, big_m, true)
    }

    /// 使用自动 ID 与调用方提供的名称创建约束模式蕴含函数。
    /// Create a constraint-mode implication function with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            premise,
            consequence,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建约束模式蕴含函数。
    /// Create a constraint-mode implication function with an auto id and auto-generated name.
    pub fn auto(premise: LinearInequality<V>, consequence: LinearInequality<V>, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if_then", id);
        Self::new(id, &name, premise, consequence, big_m)
    }

    /// 创建指示模式的条件执行函数 / Create an indicator-mode if-then function
    pub fn indicator(
        id: u64,
        name: &str,
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
    ) -> Self {
        Self::with_mode(id, name, premise, consequence, big_m, false)
    }

    /// 使用自动 ID 与调用方提供的名称创建指示模式蕴含函数。
    /// Create an indicator-mode implication function with an auto id and caller-provided name.
    pub fn named_indicator(
        name: impl AsRef<str>,
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
    ) -> Self {
        Self::indicator(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            premise,
            consequence,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建指示模式蕴含函数。
    /// Create an indicator-mode implication function with an auto id and auto-generated name.
    pub fn auto_indicator(
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
    ) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if_then", id);
        Self::indicator(id, &name, premise, consequence, big_m)
    }

    /// 创建指定模式的条件执行函数 / Create an if-then function with specified mode
    pub fn with_mode(
        id: u64,
        name: &str,
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
        constraint_mode: bool,
    ) -> Self {
        let premise_indicator = InequalityFunction::new(
            auxiliary_id(id, 11),
            &format!("{}_premise", name),
            premise.polynomial.clone(),
            premise.rhs.clone(),
            relation_to_kind(premise.relation),
            big_m.clone(),
        );
        let consequence_indicator = InequalityFunction::new(
            auxiliary_id(id, 12),
            &format!("{}_consequence", name),
            consequence.polynomial.clone(),
            consequence.rhs.clone(),
            relation_to_kind(consequence.relation),
            big_m,
        );
        let result_var = BinaryVariableItem::auto(&format!("{}_if_then", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            premise,
            consequence,
            premise_indicator,
            consequence_indicator,
            result_var,
            constraint_mode,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取前提指示变量 / Get the premise indicator variable
    pub fn premise_indicator_variable(&self) -> &BinaryVariableItem {
        self.premise_indicator.result_variable()
    }

    /// 获取结论指示变量 / Get the consequence indicator variable
    pub fn consequence_indicator_variable(&self) -> &BinaryVariableItem {
        self.consequence_indicator.result_variable()
    }

    /// 获取前提不等式 / Get the premise inequality
    pub fn premise(&self) -> &LinearInequality<V> {
        &self.premise
    }

    /// 获取结论不等式 / Get the consequence inequality
    pub fn consequence(&self) -> &LinearInequality<V> {
        &self.consequence
    }

    /// 是否为约束模式 / Whether constraint mode is enabled
    pub fn is_constraint_mode(&self) -> bool {
        self.constraint_mode
    }
}

impl<V> Display for IfThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "if_then({})", self.id.name)
    }
}

impl<V> DynSymbol for IfThenFunction<V>
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

impl<V> Symbol for IfThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IfThenFunction<V>
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
    fn logical_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let premise_index = symbol_to_index
            .get(&(self.premise_indicator.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_then premise indicator variable id {}",
                    self.premise_indicator.result_variable().id().unique_id()
                ))
            })?;
        let consequence_index = symbol_to_index
            .get(
                &(self
                    .consequence_indicator
                    .result_variable()
                    .id()
                    .unique_id() as usize),
            )
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_then consequence indicator variable id {}",
                    self.consequence_indicator
                        .result_variable()
                        .id()
                        .unique_id()
                ))
            })?;
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_then result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();
        if self.constraint_mode {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_then premise coefficient")?,
                                premise_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "if_then consequence coefficient")?,
                                consequence_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then premise implication constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "if_then premise implication rhs")?,
                ),
                &format!("{}_if_then", self.id.name),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "if_then fixed result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "if_then fixed result constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "if_then fixed result rhs")?,
                ),
                &format!("{}_if_then_result", self.id.name),
                Arc::new(self.clone()),
            ));
        } else {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_then value result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_then value premise coefficient")?,
                                premise_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then value lower one constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(1.0, "if_then value lower one rhs")?,
                ),
                &format!("{}_if_then_value_lb1", self.id.name),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    1.0,
                                    "if_then value consequence lower coefficient",
                                )?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    -1.0,
                                    "if_then value consequence coefficient",
                                )?,
                                consequence_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then value lower two constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "if_then value lower two rhs")?,
                ),
                &format!("{}_if_then_value_lb2", self.id.name),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    1.0,
                                    "if_then value upper result coefficient",
                                )?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    1.0,
                                    "if_then value upper premise coefficient",
                                )?,
                                premise_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    -1.0,
                                    "if_then value upper consequence coefficient",
                                )?,
                                consequence_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then value upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(1.0, "if_then value upper rhs")?,
                ),
                &format!("{}_if_then_value_ub", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        Ok(constraints)
    }
}

impl<V> IntermediateSymbol<V> for IfThenFunction<V>
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
        let mut constraints = self
            .premise_indicator
            .mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.consequence_indicator
                .mechanism_constraints(symbol_to_index)?,
        );

        let premise_index = symbol_to_index
            .get(&(self.premise_indicator.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_then premise indicator variable id {}",
                    self.premise_indicator.result_variable().id().unique_id()
                ))
            })?;
        let consequence_index = symbol_to_index
            .get(
                &(self
                    .consequence_indicator
                    .result_variable()
                    .id()
                    .unique_id() as usize),
            )
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_then consequence indicator variable id {}",
                    self.consequence_indicator
                        .result_variable()
                        .id()
                        .unique_id()
                ))
            })?;
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_then result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        if self.constraint_mode {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_then premise coefficient")?,
                                premise_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "if_then consequence coefficient")?,
                                consequence_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then premise implication constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "if_then premise implication rhs")?,
                ),
                &format!("{}_if_then", self.id.name),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "if_then fixed result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "if_then fixed result constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "if_then fixed result rhs")?,
                ),
                &format!("{}_if_then_result", self.id.name),
                Arc::new(self.clone()),
            ));
        } else {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_then value result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_then value premise coefficient")?,
                                premise_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then value lower one constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(1.0, "if_then value lower one rhs")?,
                ),
                &format!("{}_if_then_value_lb1", self.id.name),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    1.0,
                                    "if_then value consequence lower coefficient",
                                )?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    -1.0,
                                    "if_then value consequence coefficient",
                                )?,
                                consequence_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then value lower two constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "if_then value lower two rhs")?,
                ),
                &format!("{}_if_then_value_lb2", self.id.name),
                Arc::new(self.clone()),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    1.0,
                                    "if_then value upper result coefficient",
                                )?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    1.0,
                                    "if_then value upper premise coefficient",
                                )?,
                                premise_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(
                                    -1.0,
                                    "if_then value upper consequence coefficient",
                                )?,
                                consequence_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_then value upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(1.0, "if_then value upper rhs")?,
                ),
                &format!("{}_if_then_value_ub", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self
            .premise_indicator
            .mechanism_constraints_with_tokens(symbol_to_index, tokens)?;
        constraints.extend(
            self.consequence_indicator
                .mechanism_constraints_with_tokens(symbol_to_index, tokens)?,
        );
        constraints.extend(self.logical_constraints(symbol_to_index)?);
        Ok(constraints)
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
        format!("if_then({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for IfThenFunction<V>
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
        self.premise_indicator.register_tokens(tokens)?;
        self.consequence_indicator.register_tokens(tokens)?;
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let premise = evaluate_inequality(&self.premise, token_table, zero_if_none)?;
        let consequence = evaluate_inequality(&self.consequence, token_table, zero_if_none)?;
        from_f64(if !premise || consequence { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for IfThenFunction<V>
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
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange};

    fn token_index_map<V>(tokens: &[Token<V>]) -> HashMap<usize, usize>
    where
        V: Clone + Debug + Send + Sync + 'static,
    {
        tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
            .collect()
    }

    fn coefficient_for_index(constraint: &LinearConstraint<f64>, index: usize) -> f64 {
        *constraint
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == index)
            .expect("expected monomial should exist")
            .coefficient()
    }

    #[test]
    fn if_then_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            3.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            4.0,
        );
        let if_then = IfThenFunction::indicator(31001, "if_then", premise, consequence, 10.0);

        let mut tokens_true1 = VecTokenList::<f64>::new();
        let tx_true1 = Token::from_generic(x.clone(), 0);
        tx_true1.set_result(2.0);
        tokens_true1.add_token(tx_true1);
        assert_eq!(if_then.calculate_value(&tokens_true1, false), Some(1.0));

        let mut tokens_false = VecTokenList::<f64>::new();
        let tx_false = Token::from_generic(x.clone(), 0);
        tx_false.set_result(3.0);
        tokens_false.add_token(tx_false);
        assert_eq!(if_then.calculate_value(&tokens_false, false), Some(0.0));

        let mut tokens_true2 = VecTokenList::<f64>::new();
        let tx_true2 = Token::from_generic(x, 0);
        tx_true2.set_result(5.0);
        tokens_true2.add_token(tx_true2);
        assert_eq!(if_then.calculate_value(&tokens_true2, false), Some(1.0));
    }

    #[test]
    fn if_then_generates_expected_constraints() {
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            3.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            4.0,
        );

        let if_then_constraint = IfThenFunction::new(
            31011,
            "if_then_constraint",
            premise.clone(),
            consequence.clone(),
            10.0,
        );
        let if_then_indicator =
            IfThenFunction::indicator(31012, "if_then_indicator", premise, consequence, 10.0);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(
            if_then_constraint
                .premise_indicator_variable()
                .id()
                .unique_id() as usize,
            if_then_constraint.premise_indicator_variable().index(),
        );
        symbol_to_index.insert(
            if_then_constraint
                .consequence_indicator_variable()
                .id()
                .unique_id() as usize,
            if_then_constraint.consequence_indicator_variable().index(),
        );
        symbol_to_index.insert(
            if_then_constraint.result_variable().id().unique_id() as usize,
            if_then_constraint.result_variable().index(),
        );

        let constraints_constraint = if_then_constraint
            .mechanism_constraints(&symbol_to_index)
            .unwrap();
        assert_eq!(constraints_constraint.len(), 6);

        let mut symbol_to_index_indicator = HashMap::new();
        symbol_to_index_indicator.insert(
            if_then_indicator
                .premise_indicator_variable()
                .id()
                .unique_id() as usize,
            if_then_indicator.premise_indicator_variable().index(),
        );
        symbol_to_index_indicator.insert(
            if_then_indicator
                .consequence_indicator_variable()
                .id()
                .unique_id() as usize,
            if_then_indicator.consequence_indicator_variable().index(),
        );
        symbol_to_index_indicator.insert(
            if_then_indicator.result_variable().id().unique_id() as usize,
            if_then_indicator.result_variable().index(),
        );
        let constraints_indicator = if_then_indicator
            .mechanism_constraints(&symbol_to_index_indicator)
            .unwrap();
        assert_eq!(constraints_indicator.len(), 7);
    }

    #[test]
    fn if_then_infers_big_m_for_internal_inequality_indicators() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            ConstraintRelation::LessEqual,
            0.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        );
        let if_then: IfThenFunction<f64> =
            IfThenFunction::indicator(31013, "if_then_bound", premise, consequence, 100.0);

        let mut aux_tokens = Vec::new();
        if_then
            .register_tokens(&mut aux_tokens)
            .expect("if_then tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let premise_index = *symbol_to_index
            .get(&(if_then.premise_indicator_variable().id().unique_id() as usize))
            .expect("premise indicator index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = if_then
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_then constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "if_then_bound_premise_ineq_ub")
            .expect("premise upper inequality constraint should exist");

        assert!((upper.inequality.rhs - 5.0).abs() <= 1e-9);
        assert!((coefficient_for_index(upper, premise_index) - 5.0).abs() <= 1e-9);
    }
}
