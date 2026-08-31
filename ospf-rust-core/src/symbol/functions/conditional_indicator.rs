//! 可注册的范围驱动关系指示器 / Registerable range-driven relation indicator
//!
//! 本模块保留 `ConditionalIfFunction` 的纯条件描述职责，并提供可以注册到模型的
//! `ConditionalIndicatorFunction`。后者只接受显式有限范围，不会回退到默认 Big-M。
//! This module keeps `ConditionalIfFunction` as a descriptive condition and adds
//! `ConditionalIndicatorFunction`, which registers an indicator using explicit finite
//! bounds and never falls back to a default Big-M value.

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
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};
use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::conditional::{
    ConditionBounds, ConditionRelation, ConditionalIfFunction, TruthValue, branch_coverage,
    classify, relation_indicator_constraints, validate_relation_bounds,
};
use super::discrete_condition::{
    DiscreteConditionLinearization, to_strict_positive_condition_with_derived_lattice_proof,
};

fn evaluate_linear<V>(
    polynomial: &Linear<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = polynomial.constant_term().clone();
    for monomial in polynomial.monomials() {
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

fn finite_value<V>(value: &V, label: &str) -> Result<f64>
where
    V: ToPrimitive,
{
    let value = value.to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "conditional indicator `{label}` cannot be converted to f64"
        ))
    })?;
    if !value.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "conditional indicator `{label}` must be finite"
        ))
        .into());
    }
    Ok(value)
}

fn convert_f64<V>(value: f64, label: &str) -> Result<V>
where
    V: FromPrimitive + ToPrimitive,
{
    let converted = V::from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "conditional indicator `{label}` cannot represent value {value}"
        ))
    })?;
    finite_value(&converted, label)?;
    Ok(converted)
}

fn validate_inputs<V>(
    condition: &Linear<V>,
    relation: ConditionRelation,
    bounds: &ConditionBounds<V>,
    strict_boundary: &V,
) -> Result<()>
where
    V: Clone + Debug + FromPrimitive + ToPrimitive,
{
    validate_relation_bounds(bounds, relation, strict_boundary)?;
    convert_f64::<V>(0.0, "zero")?;
    convert_f64::<V>(1.0, "one")?;
    finite_value(condition.constant_term(), "condition constant")?;
    for monomial in condition.monomials() {
        finite_value(monomial.coefficient(), "condition coefficient")?;
    }
    Ok(())
}

fn validate_discrete_linearization(
    linearization: DiscreteConditionLinearization,
) -> Result<()> {
    if !linearization.sign.is_finite()
        || (linearization.sign != 1.0 && linearization.sign != -1.0)
    {
        return Err(ModelError::InvalidConstraint(
            "discrete condition linearization sign must be either 1 or -1".to_string(),
        )
        .into());
    }
    if !linearization.offset.is_finite() {
        return Err(ModelError::InvalidConstraint(
            "discrete condition linearization offset must be finite".to_string(),
        )
        .into());
    }
    Ok(())
}

fn apply_discrete_linearization(
    value: f64,
    linearization: DiscreteConditionLinearization,
    label: &str,
) -> Result<f64> {
    if !value.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "conditional indicator `{label}` must be finite"
        ))
        .into());
    }
    let transformed = value * linearization.sign + linearization.offset;
    if !transformed.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "conditional indicator transformed `{label}` must be finite"
        ))
        .into());
    }
    Ok(transformed)
}

fn transform_discrete_condition<V>(
    condition: &Linear<V>,
    linearization: DiscreteConditionLinearization,
) -> Result<Linear<V>>
where
    V: Clone + Debug + FromPrimitive + ToPrimitive,
{
    validate_discrete_linearization(linearization)?;
    let monomials = condition
        .monomials()
        .iter()
        .enumerate()
        .map(|(index, monomial)| {
            let coefficient = finite_value(monomial.coefficient(), "condition coefficient")?;
            let coefficient = apply_discrete_linearization(
                coefficient,
                DiscreteConditionLinearization {
                    sign: linearization.sign,
                    offset: 0.0,
                },
                &format!("condition coefficient {index}"),
            )?;
            Ok(LinearMonomial::new(
                convert_f64(coefficient, "transformed condition coefficient")?,
                monomial.var_index(),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let constant = finite_value(condition.constant_term(), "condition constant")?;
    let constant = apply_discrete_linearization(constant, linearization, "condition constant")?;
    Ok(Linear::new(
        monomials,
        convert_f64(constant, "transformed condition constant")?,
    ))
}

fn transform_discrete_bounds<V>(
    bounds: &ConditionBounds<V>,
    linearization: DiscreteConditionLinearization,
) -> Result<ConditionBounds<V>>
where
    V: Clone + Debug + FromPrimitive + ToPrimitive,
{
    validate_discrete_linearization(linearization)?;
    let lower = finite_value(&bounds.lower, "condition lower bound")?;
    let upper = finite_value(&bounds.upper, "condition upper bound")?;
    let (transformed_lower, transformed_upper) = if linearization.sign > 0.0 {
        (
            apply_discrete_linearization(lower, linearization, "lower bound")?,
            apply_discrete_linearization(upper, linearization, "upper bound")?,
        )
    } else {
        (
            apply_discrete_linearization(upper, linearization, "lower bound")?,
            apply_discrete_linearization(lower, linearization, "upper bound")?,
        )
    };
    Ok(ConditionBounds {
        lower: convert_f64(transformed_lower, "transformed condition lower bound")?,
        upper: convert_f64(transformed_upper, "transformed condition upper bound")?,
    })
}

/// 可注册的范围驱动关系指示器 / Registerable range-driven relation indicator.
///
/// 该函数将 `condition relation 0` 编码为结果二值变量。`strict_boundary` 定义严格
/// 关系真、假分支之间的间隔，`bounds` 必须覆盖条件多项式的实际取值范围。
/// The function encodes `condition relation 0` as a binary result variable.
/// `strict_boundary` separates strict true and false branches, while `bounds` must
/// cover the actual range of the condition polynomial.
#[derive(Debug, Clone)]
pub struct ConditionalIndicatorFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 中间符号标识符 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 条件差值多项式 / Condition-difference polynomial
    condition: Linear<V>,
    /// 条件关系 / Condition relation
    relation: ConditionRelation,
    /// 真、假分支之间的正间隔 / Positive gap between true and false branches
    strict_boundary: V,
    /// 条件多项式的显式有限范围 / Explicit finite condition bounds
    bounds: ConditionBounds<V>,
    /// 结果二值变量 / Binary result variable
    result_var: BinaryVariableItem,
    /// 条件指示二值变量 / Binary condition-indicator variable
    indicator_var: BinaryVariableItem,
    /// 声明的依赖符号 ID 列表 / Declared dependency symbol IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> ConditionalIndicatorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建范围驱动关系指示器 / Create a range-driven relation indicator.
    ///
    /// 构造阶段即执行范围、边界和多项式有限性预检，使失败不会产生可注册的半成品。
    /// Bounds, boundary, and polynomial finiteness are checked during construction so
    /// a failed preflight cannot produce a partially registerable object.
    pub fn new(
        id: u64,
        name: &str,
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        validate_inputs(&condition, relation, &bounds, &strict_boundary)?;

        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_if", name));
        let indicator_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_if_nz", name));

        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            condition,
            relation,
            strict_boundary,
            bounds,
            result_var,
            indicator_var,
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 使用自动 ID 与调用方名称创建指示器 / Create an indicator with an auto ID and a caller name.
    pub fn named(
        name: impl AsRef<str>,
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            condition,
            relation,
            strict_boundary,
            bounds,
        )
    }

    /// 使用自动 ID 与自动名称创建指示器 / Create an indicator with an auto ID and name.
    pub fn auto(
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("conditional_indicator", id);
        Self::new(id, &name, condition, relation, strict_boundary, bounds)
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取条件多项式 / Get the condition polynomial.
    pub fn condition_polynomial(&self) -> &Linear<V> {
        &self.condition
    }

    /// 获取条件关系 / Get the condition relation.
    pub fn relation(&self) -> ConditionRelation {
        self.relation
    }

    /// 获取严格边界 / Get the strict boundary.
    pub fn strict_boundary(&self) -> &V {
        &self.strict_boundary
    }

    /// 获取条件范围 / Get the explicit condition bounds.
    pub fn condition_bounds(&self) -> &ConditionBounds<V> {
        &self.bounds
    }

    /// 获取条件范围（兼容短名称）/ Get condition bounds (short compatibility name).
    pub fn bounds(&self) -> &ConditionBounds<V> {
        &self.bounds
    }

    /// 获取结果二值变量 / Get the binary result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取条件指示变量 / Get the condition indicator variable.
    pub fn condition_indicator_variable(&self) -> &BinaryVariableItem {
        &self.indicator_var
    }

    /// 获取条件指示变量（兼容短名称）/ Get the condition indicator (short compatibility name).
    pub fn indicator_variable(&self) -> &BinaryVariableItem {
        &self.indicator_var
    }

    /// 获取稳定的结果多项式 / Get the stable result polynomial.
    pub fn result_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                convert_f64(1.0, "result polynomial coefficient")
                    .expect("one is representable for a registered value type"),
                self.result_var.index(),
            )],
            convert_f64(0.0, "result polynomial constant")
                .expect("zero is representable for a registered value type"),
        )
    }

    /// 获取两个辅助变量 / Get the two helper variables.
    pub fn helper_variables(&self) -> [&BinaryVariableItem; 2] {
        [&self.result_var, &self.indicator_var]
    }

    /// 分类给定的条件差值 / Classify a condition difference.
    pub fn classify(&self, difference: &V) -> Result<TruthValue> {
        classify(difference, self.relation, &self.strict_boundary)
    }

    /// 将分类结果转换为二值值，Undefined 返回 None / Convert the classification to a binary value; Undefined returns None.
    pub fn evaluate_difference(&self, difference: &V) -> Result<Option<V>> {
        match self.classify(difference)? {
            TruthValue::True => convert_f64(1.0, "true result").map(Some),
            TruthValue::False => convert_f64(0.0, "false result").map(Some),
            TruthValue::Undefined => Ok(None),
        }
    }

    fn validate(&self) -> Result<()> {
        validate_inputs(
            &self.condition,
            self.relation,
            &self.bounds,
            &self.strict_boundary,
        )
    }

    fn index_of(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &BinaryVariableItem,
        role: &str,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "conditional indicator {role} variable id {}",
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn fixed_constraint(
        variable_index: usize,
        value: f64,
        name: &str,
        source: &Arc<dyn IntermediateSymbol<V>>,
    ) -> Result<LinearConstraint<V>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        Ok(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![LinearMonomial::new(
                        convert_f64(1.0, "fixed variable coefficient")?,
                        variable_index,
                    )],
                    convert_f64(0.0, "fixed variable constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64(value, "fixed variable value")?,
            ),
            name,
            source.clone(),
        ))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        self.validate()?;

        let result_index = Self::index_of(symbol_to_index, &self.result_var, "result")?;
        let indicator_index = Self::index_of(symbol_to_index, &self.indicator_var, "condition")?;
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());

        if let Some(coverage) = branch_coverage(&self.bounds, self.relation, &self.strict_boundary)?
        {
            let value = match coverage {
                TruthValue::True => 1.0,
                TruthValue::False => 0.0,
                TruthValue::Undefined => unreachable!("branch coverage cannot return Undefined"),
            };
            return Ok(vec![
                Self::fixed_constraint(
                    indicator_index,
                    value,
                    &format!("{}_if_fold_indicator", self.id.name),
                    &source,
                )?,
                Self::fixed_constraint(
                    result_index,
                    value,
                    &format!("{}_if_fold_result", self.id.name),
                    &source,
                )?,
            ]);
        }

        let relation_rows = relation_indicator_constraints(
            &self.condition,
            indicator_index,
            self.relation,
            &self.bounds,
            &self.strict_boundary,
        )?;
        let mut constraints = relation_rows
            .into_iter()
            .enumerate()
            .map(|(index, inequality)| {
                LinearConstraint::from_symbol(
                    inequality,
                    &format!(
                        "{}_if_{}",
                        self.id.name,
                        if index == 0 { "lower" } else { "upper" }
                    ),
                    source.clone(),
                )
            })
            .collect::<Vec<_>>();

        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64(1.0, "result-link coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64(-1.0, "indicator-link coefficient")?,
                            indicator_index,
                        ),
                    ],
                    convert_f64(0.0, "result-link constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64(0.0, "result-link rhs")?,
            ),
            &format!("{}_if_eq", self.id.name),
            source,
        ));
        Ok(constraints)
    }
}

impl<V> ConditionalIndicatorFunction<V>
where
    V: Clone
        + Debug
        + PartialEq
        + Send
        + Sync
        + 'static
        + ToPrimitive
        + FromPrimitive,
{
    /// 从离散关系创建受检指示器 / Create a checked indicator from a discrete relation.
    ///
    /// 格点证明从原始条件和实际 token 元数据推导，不接受调用方提供的伪造步长。
    /// The lattice proof is derived from the original condition and actual token metadata;
    /// callers cannot provide an unverified step.
    pub fn from_discrete_condition(
        id: u64,
        name: &str,
        descriptor: ConditionalIfFunction<V>,
        delta: V,
        tokens: &[Token<V>],
    ) -> Result<Self> {
        let ConditionalIfFunction {
            condition,
            relation,
            strict_boundary,
            bounds,
        } = descriptor;
        validate_relation_bounds(&bounds, relation, &strict_boundary)?;
        let linearization = to_strict_positive_condition_with_derived_lattice_proof(
            relation,
            &strict_boundary,
            &delta,
            &condition,
            tokens,
        )?;
        let transformed_condition = transform_discrete_condition(&condition, linearization)?;
        let transformed_bounds = transform_discrete_bounds(&bounds, linearization)?;
        Self::new(
            id,
            name,
            transformed_condition,
            ConditionRelation::Greater,
            strict_boundary,
            transformed_bounds,
        )
    }
}

impl<V> Display for ConditionalIndicatorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "conditional_indicator({})", self.id.name)
    }
}

impl<V> DynSymbol for ConditionalIndicatorFunction<V>
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

impl<V> Symbol for ConditionalIndicatorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ConditionalIndicatorFunction<V>
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
        self.validate()?;
        let helper_tokens = vec![
            Token::from_generic(self.result_var.clone(), self.result_var.index()),
            Token::from_generic(self.indicator_var.clone(), self.indicator_var.index()),
        ];
        if helper_tokens.iter().any(|candidate| {
            tokens
                .iter()
                .any(|existing| existing.id() == candidate.id())
        }) {
            return Err(ModelError::ConstraintConflict(format!(
                "conditional indicator `{}` helper token already exists",
                self.id.name
            ))
            .into());
        }
        tokens.extend(helper_tokens);
        Ok(())
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        _tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index)
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
        format!("conditional_indicator({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for ConditionalIndicatorFunction<V>
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
        <Self as IntermediateSymbol<V>>::register_auxiliary_tokens(self, tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let difference = evaluate_linear(&self.condition, token_table, zero_if_none)?;
        self.evaluate_difference(&difference).ok().flatten()
    }
}

impl<V> LinearIntermediateSymbol<V> for ConditionalIndicatorFunction<V>
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
                convert_f64(1.0, "result polynomial coefficient")
                    .expect("one is representable for a registered value type"),
                self.result_var.index(),
            )],
            convert_f64(0.0, "result polynomial constant")
                .expect("zero is representable for a registered value type"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{Continuous, Integer, VariableItem};

    fn condition() -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)
    }

    fn bounds() -> ConditionBounds<f64> {
        ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        }
    }

    fn function(relation: ConditionRelation) -> ConditionalIndicatorFunction<f64> {
        ConditionalIndicatorFunction::new(900, "ci", condition(), relation, 0.1, bounds())
            .expect("test condition should pass preflight")
    }

    #[test]
    fn registers_result_and_indicator_helpers() {
        let indicator = function(ConditionRelation::Greater);
        let mut tokens = VecTokenList::new();
        let mut helpers = Vec::new();
        indicator.register_auxiliary_tokens(&mut helpers).unwrap();
        assert_eq!(helpers.len(), 2);
        tokens.try_add_tokens(helpers).unwrap();
        assert!(tokens
            .find_by_id(indicator.result_variable().id())
            .is_some());
        assert!(tokens
            .find_by_id(indicator.condition_indicator_variable().id())
            .is_some());
    }

    #[test]
    fn generates_expected_relation_rows_for_all_relations() {
        let cases = [
            (ConditionRelation::Greater, -2.0, -2.1, 0.0, -2.0),
            (ConditionRelation::GreaterEqual, -2.0, -2.0, -0.1, -2.1),
            (ConditionRelation::Less, 0.0, 2.0, 2.0, 2.1),
            (ConditionRelation::LessEqual, 0.1, 2.1, 2.0, 2.0),
        ];
        for (relation, lower_rhs, lower_indicator, upper_rhs, upper_indicator) in cases {
            let indicator = function(relation);
            let ids = HashMap::from([
                (indicator.result_variable().id().unique_id() as usize, 4),
                (
                    indicator.condition_indicator_variable().id().unique_id() as usize,
                    5,
                ),
            ]);
            let constraints = indicator.mechanism_constraints(&ids).unwrap();
            assert_eq!(constraints.len(), 3);
            assert_eq!(
                constraints[0].inequality.relation,
                ConstraintRelation::GreaterEqual
            );
            assert_eq!(
                constraints[1].inequality.relation,
                ConstraintRelation::LessEqual
            );
            let lower_term = constraints[0]
                .inequality
                .polynomial
                .monomials()
                .iter()
                .find(|monomial| monomial.var_index() == 5)
                .expect("lower row should contain the indicator");
            let upper_term = constraints[1]
                .inequality
                .polynomial
                .monomials()
                .iter()
                .find(|monomial| monomial.var_index() == 5)
                .expect("upper row should contain the indicator");
            assert!((constraints[0].inequality.rhs - lower_rhs).abs() <= 1e-9);
            assert!((*lower_term.coefficient() - lower_indicator).abs() <= 1e-9);
            assert!((constraints[1].inequality.rhs - upper_rhs).abs() <= 1e-9);
            assert!((*upper_term.coefficient() - upper_indicator).abs() <= 1e-9);
            assert_eq!(
                constraints[2].inequality.relation,
                ConstraintRelation::Equal
            );
            assert_eq!(constraints[2].inequality.polynomial.monomials().len(), 2);
        }
    }

    #[test]
    fn folds_single_branch_ranges_and_fixes_both_helpers() {
        let indicator = ConditionalIndicatorFunction::new(
            901,
            "always_true",
            condition(),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: 0.0,
                upper: 2.0,
            },
        )
        .unwrap();
        let ids = HashMap::from([
            (indicator.result_variable().id().unique_id() as usize, 7),
            (
                indicator.condition_indicator_variable().id().unique_id() as usize,
                8,
            ),
        ]);
        let constraints = indicator.mechanism_constraints(&ids).unwrap();
        assert_eq!(constraints.len(), 2);
        assert!(constraints.iter().all(|constraint| {
            constraint.inequality.relation == ConstraintRelation::Equal
                && constraint.inequality.rhs == 1.0
        }));
    }

    #[test]
    fn rejects_invalid_inputs_before_token_registration_or_constraints() {
        assert!(ConditionalIndicatorFunction::new(
            902,
            "bad_bounds",
            condition(),
            ConditionRelation::Greater,
            0.1,
            ConditionBounds {
                lower: 1.0,
                upper: -1.0,
            },
        )
        .is_err());

        assert!(ConditionalIndicatorFunction::new(
            904,
            "undefined_bounds",
            condition(),
            ConditionRelation::LessEqual,
            0.1,
            ConditionBounds {
                lower: 0.01,
                upper: 0.09,
            },
        )
        .is_err());

        let mut tokens = Vec::new();
        let invalid = ConditionalIndicatorFunction {
            id: IntermediateSymbolId::new(903, "bad_boundary"),
            condition: condition(),
            relation: ConditionRelation::Greater,
            strict_boundary: 0.0,
            bounds: bounds(),
            result_var: BinaryVariableItem::auto("bad_result"),
            indicator_var: BinaryVariableItem::auto("bad_indicator"),
            declared_dependency_ids: Vec::new(),
        };
        assert!(invalid.register_auxiliary_tokens(&mut tokens).is_err());
        assert!(tokens.is_empty());

        let invalid_undefined = ConditionalIndicatorFunction {
            id: IntermediateSymbolId::new(905, "undefined_registration"),
            condition: condition(),
            relation: ConditionRelation::Greater,
            strict_boundary: 0.1,
            bounds: ConditionBounds {
                lower: 0.01,
                upper: 0.09,
            },
            result_var: BinaryVariableItem::auto("undefined_result"),
            indicator_var: BinaryVariableItem::auto("undefined_indicator"),
            declared_dependency_ids: Vec::new(),
        };
        assert!(invalid_undefined
            .register_auxiliary_tokens(&mut tokens)
            .is_err());
        assert!(tokens.is_empty());
    }

    #[test]
    fn evaluates_using_shared_three_valued_relation_matrix() {
        let indicator = function(ConditionRelation::LessEqual);
        let mut tokens = VecTokenList::new();
        let x = crate::variable::ContinuousVariableItem::auto("x");
        let token = Token::from_generic(x, 0);
        token.set_result(0.0);
        tokens.add_token(token);
        assert_eq!(
            <ConditionalIndicatorFunction as FunctionSymbol>::calculate_value(
                &indicator, &tokens, false,
            ),
            Some(1.0)
        );
    }

    fn discrete_tokens() -> Vec<Token<f64>> {
        vec![Token::from_generic(
            VariableItem::<Integer>::auto("p"),
            0,
        )]
    }

    fn discrete_indicator(
        relation: ConditionRelation,
    ) -> ConditionalIndicatorFunction<f64> {
        let tokens = discrete_tokens();
        ConditionalIndicatorFunction::from_discrete_condition(
            910,
            "discrete_threshold",
            ConditionalIfFunction::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], -10.0),
                relation,
                0.1,
                ConditionBounds {
                    lower: -10.0,
                    upper: 10.0,
                },
            )
            .expect("integer condition descriptor should be valid"),
            1.0,
            &tokens,
        )
        .expect("integer condition should derive a lattice proof")
    }

    #[test]
    fn discrete_factory_evaluates_ge_and_le_at_integer_threshold() {
        for relation in [
            ConditionRelation::GreaterEqual,
            ConditionRelation::LessEqual,
        ] {
            let indicator = discrete_indicator(relation);
            assert_eq!(indicator.relation(), ConditionRelation::Greater);
            let expected_constant = if relation == ConditionRelation::GreaterEqual {
                -9.0
            } else {
                11.0
            };
            let expected_coefficient = if relation == ConditionRelation::GreaterEqual {
                1.0
            } else {
                -1.0
            };
            assert_eq!(*indicator.condition_polynomial().constant_term(), expected_constant);
            assert_eq!(
                *indicator.condition_polynomial().monomials()[0].coefficient(),
                expected_coefficient
            );

            for value in 0..=20 {
                let token = Token::from_generic(
                    VariableItem::<Integer>::auto("p_value"),
                    0,
                );
                token.set_result(f64::from(value));
                let mut values = VecTokenList::new();
                values.add_token(token);
                let expected = match relation {
                    ConditionRelation::GreaterEqual => value >= 10,
                    ConditionRelation::LessEqual => value <= 10,
                    _ => unreachable!(),
                };
                assert_eq!(
                    indicator.evaluate_from_tokens(&values, false),
                    Some(if expected { 1.0 } else { 0.0 }),
                    "relation {relation:?}, p={value}"
                );
            }
        }
    }

    #[test]
    fn discrete_factory_registers_transformed_greater_constraints() {
        for relation in [
            ConditionRelation::GreaterEqual,
            ConditionRelation::LessEqual,
        ] {
            let indicator = discrete_indicator(relation);
            let indexes = HashMap::from([
                (indicator.result_variable().id().unique_id() as usize, 4),
                (
                    indicator.condition_indicator_variable().id().unique_id() as usize,
                    5,
                ),
            ]);
            let constraints = indicator.mechanism_constraints(&indexes).unwrap();
            assert_eq!(constraints.len(), 3);
            assert_eq!(
                constraints[0].inequality.relation,
                ConstraintRelation::GreaterEqual
            );
            assert_eq!(
                constraints[1].inequality.relation,
                ConstraintRelation::LessEqual
            );
            let indicator_coefficient = if relation == ConditionRelation::GreaterEqual {
                -9.1
            } else {
                -9.1
            };
            let lower_indicator = constraints[0]
                .inequality
                .polynomial
                .monomials()
                .iter()
                .find(|monomial| monomial.var_index() == 5)
                .unwrap();
            assert!((*lower_indicator.coefficient() - indicator_coefficient).abs() <= 1e-9);
            assert_eq!(constraints[0].inequality.rhs, -9.0);
            assert_eq!(constraints[1].inequality.rhs, 0.0);
        }
    }

    #[test]
    fn discrete_factory_rejects_continuous_tokens_and_non_integer_coefficients() {
        let continuous_tokens = vec![Token::from_generic(
            VariableItem::<Continuous>::auto("continuous"),
            0,
        )];
        let make = |condition: Linear<f64>, tokens: &[Token<f64>]| {
            ConditionalIndicatorFunction::from_discrete_condition(
                911,
                "invalid_discrete",
                ConditionalIfFunction::new(
                    condition,
                    ConditionRelation::GreaterEqual,
                    0.1,
                    ConditionBounds {
                        lower: -10.0,
                        upper: 10.0,
                    },
                )
                .expect("test descriptor should have valid continuous bounds"),
                1.0,
                tokens,
            )
        };
        assert!(make(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], -10.0),
            &continuous_tokens,
        )
        .is_err());

        let integer_tokens = discrete_tokens();
        assert!(make(
            Linear::new(vec![LinearMonomial::new(0.5, 0)], -10.0),
            &integer_tokens,
        )
        .is_err());
    }
}
