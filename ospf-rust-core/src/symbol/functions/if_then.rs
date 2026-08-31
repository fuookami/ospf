//! If-Then 蕴含函数符号 / If-Then implication function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::ConditionalIndicatorFunction;
use super::conditional::{ConditionBounds, ConditionRelation, ConditionalIfFunction, TruthValue};
use super::{InequalityFunction, InequalityKind};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange, new_group_id,
};
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

/// 旧版不等式蕴含函数的明确名称 / Explicit name for the legacy inequality-implication function
///
/// `IfThenFunction` 历史上建模两个不等式及其逻辑连接。条件值函数必须以独立名称
/// 追加，避免两种语义互相替换。
/// `IfThenFunction` historically models two inequalities and a logical link.
/// A conditional-value function must be added under a distinct name; this alias
/// keeps the old constraint mode available without conflating the two meanings.
pub type IfThenConstraintFunction<V = f64> = IfThenFunction<V>;

/// 条件值函数：条件成立时返回 `then_poly`，否则返回零。
/// Conditional-value function: return `then_poly` when the condition is true, otherwise zero.
///
/// 该类型只承载关系分类和纯求值语义，不注册 Big-M 约束；需要模型注册时，调用方应
/// 使用显式范围和条件指示器适配器。这样不会改变旧 `IfThenFunction` 的不等式蕴含行为。
/// This type intentionally carries only relation classification and pure evaluation. It does not
/// register Big-M constraints, so callers must use an explicit-range indicator adapter for model
/// registration. The legacy inequality-implication behavior of `IfThenFunction` is unchanged.
#[derive(Debug, Clone)]
pub struct ConditionalThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 条件关系描述 / Condition relation descriptor
    pub condition: ConditionalIfFunction<V>,
    /// 条件成立时的结果多项式 / Result polynomial used on the true branch
    pub then_poly: Linear<V>,
    /// 条件关系的可注册指示器 / Registerable condition-relation indicator
    condition_indicator: ConditionalIndicatorFunction<V>,
    /// 条件值结果变量 / Conditional-value result variable
    result_var: ContinuousVariableItem,
    /// then 多项式的显式有限范围 / Explicit finite range of the then polynomial
    then_bounds: ConditionBounds<V>,
    /// 中间符号标识符 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 声明的依赖 ID 列表 / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> ConditionalThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建条件值函数 / Create a conditional-value function
    pub fn new(condition: ConditionalIfFunction<V>, then_poly: Linear<V>) -> Result<Self> {
        let then_bounds = infer_constant_bounds(&then_poly)?;
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("conditional_then", id);
        Self::with_parts(id, name, condition, then_poly, then_bounds)
    }

    /// 使用显式 then 范围创建条件值函数 / Create with an explicit then range
    ///
    /// 非常量 then 多项式必须通过该入口提供可证明的有限范围。
    /// A non-constant then polynomial must provide a provable finite range here.
    pub fn new_with_bounds(
        condition: ConditionalIfFunction<V>,
        then_poly: Linear<V>,
        then_bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("conditional_then", id);
        Self::with_parts(id, name, condition, then_poly, then_bounds)
    }

    /// 使用调用方名称和显式范围创建条件值函数。
    /// Create a conditional-value function with a caller name and explicit range.
    pub fn named(
        name: impl AsRef<str>,
        condition: ConditionalIfFunction<V>,
        then_poly: Linear<V>,
        then_bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        Self::with_parts(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            condition,
            then_poly,
            then_bounds,
        )
    }

    fn with_parts(
        id: u64,
        name: impl AsRef<str>,
        condition: ConditionalIfFunction<V>,
        then_poly: Linear<V>,
        then_bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        let name = name.as_ref();
        then_bounds.validate()?;
        validate_finite_linear(&then_poly, "conditional then polynomial")?;
        let condition_indicator = ConditionalIndicatorFunction::new(
            auxiliary_symbol_id(id, 1),
            &format!("{name}_condition"),
            condition.condition.clone(),
            condition.relation,
            condition.strict_boundary.clone(),
            condition.bounds.clone(),
        )?;
        let (result_lower, result_upper) = result_range(&then_bounds)?;
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(new_group_id(), 0),
            &format!("{name}_result"),
            VariableRange::bounded(result_lower, result_upper),
        );
        let function = Self {
            condition,
            then_poly,
            condition_indicator,
            result_var,
            then_bounds,
            id: IntermediateSymbolId::new(id, name),
            declared_dependency_ids: Vec::new(),
        };
        function.validate()?;
        Ok(function)
    }

    /// 从关系、严格边界和条件范围创建条件值函数。
    /// Create a conditional-value function from a relation, strict boundary, and condition bounds.
    pub fn from_parts(
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        condition_bounds: ConditionBounds<V>,
        then_poly: Linear<V>,
    ) -> Result<Self> {
        Self::new(
            ConditionalIfFunction::new(condition, relation, strict_boundary, condition_bounds)?,
            then_poly,
        )
    }

    /// 从关系和两个显式范围创建条件值函数 / Create from relation and both explicit ranges
    pub fn from_parts_with_bounds(
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        condition_bounds: ConditionBounds<V>,
        then_poly: Linear<V>,
        then_bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        Self::new_with_bounds(
            ConditionalIfFunction::new(condition, relation, strict_boundary, condition_bounds)?,
            then_poly,
            then_bounds,
        )
    }

    /// 校验条件范围和结果多项式的有限性 / Validate condition bounds and result-polynomial finiteness
    pub fn validate(&self) -> Result<()> {
        self.condition.bounds.validate()?;
        self.condition
            .classify(&self.condition.bounds.lower)
            .map(|_| ())?;
        validate_finite_linear(&self.then_poly, "conditional then polynomial")?;
        self.then_bounds.validate()?;
        let (expected_lower, expected_upper) = result_range(&self.then_bounds)?;
        let actual_lower = self
            .result_var
            .range()
            .lower_bound
            .as_ref()
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "conditional then result requires a lower bound".to_string(),
                )
            })?;
        let actual_upper = self
            .result_var
            .range()
            .upper_bound
            .as_ref()
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "conditional then result requires an upper bound".to_string(),
                )
            })?;
        if *actual_lower > expected_lower || *actual_upper < expected_upper {
            return Err(ModelError::InvalidConstraint(
                "conditional then result range must cover zero and then range".to_string(),
            )
            .into());
        }
        Ok(())
    }

    /// 获取条件描述 / Get the condition descriptor
    pub fn condition_descriptor(&self) -> &ConditionalIfFunction<V> {
        &self.condition
    }

    /// 获取条件成立分支的多项式 / Get the true-branch polynomial
    pub fn then_polynomial(&self) -> &Linear<V> {
        &self.then_poly
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取条件指示器 / Get the condition indicator
    pub fn condition_indicator(&self) -> &ConditionalIndicatorFunction<V> {
        &self.condition_indicator
    }

    /// 获取 then 多项式的范围 / Get the then-polynomial range
    pub fn then_bounds(&self) -> &ConditionBounds<V> {
        &self.then_bounds
    }

    /// 获取结果范围 / Get the result range
    pub fn result_range(&self) -> VariableRange<f64> {
        self.result_var.range().clone()
    }

    /// 设置声明的依赖 ID 列表 / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 对条件差值分类 / Classify a condition difference
    pub fn classify(&self, difference: &V) -> Result<TruthValue> {
        self.validate()?;
        self.condition.classify(difference)
    }

    /// 使用已求得的 then 值进行纯求值 / Evaluate with an already evaluated then value
    ///
    /// Undefined 条件返回 `None`，不会伪造为零。
    /// An Undefined condition returns `None` and is never fabricated as zero.
    pub fn evaluate(&self, difference: &V, then_value: &V) -> Result<Option<V>> {
        match self.classify(difference)? {
            TruthValue::True => {
                if !then_value
                    .to_f64()
                    .map(|value| value.is_finite())
                    .unwrap_or(false)
                {
                    return Err(ModelError::InvalidConstraint(
                        "conditional then value must be finite and convertible to f64".to_string(),
                    )
                    .into());
                }
                Ok(Some(then_value.clone()))
            }
            TruthValue::False => Ok(Some(V::from_f64(0.0).ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "failed to convert conditional then zero value".to_string(),
                )
            })?)),
            TruthValue::Undefined => Ok(None),
        }
    }

    /// 从 token 求得 then 多项式后进行纯求值 / Evaluate after resolving the then polynomial from tokens
    pub fn evaluate_with_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Result<Option<V>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
    {
        let difference = match evaluate_linear(&self.condition.condition, token_table, zero_if_none)
        {
            Some(value) => value,
            None => return Ok(None),
        };
        match self.classify(&difference)? {
            TruthValue::False => {
                return Ok(Some(V::from_f64(0.0).ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert conditional then zero value".to_string(),
                    )
                })?));
            }
            TruthValue::Undefined => return Ok(None),
            TruthValue::True => {}
        }
        let then_value = match evaluate_linear(&self.then_poly, token_table, zero_if_none) {
            Some(value) => value,
            None => return Ok(None),
        };
        self.evaluate(&difference, &then_value)
    }

    fn variable_index(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &ContinuousVariableItem,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "conditional then result variable id {}",
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn indicator_index(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &BinaryVariableItem,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "conditional then indicator variable id {}",
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn append_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        let mut staged = Vec::new();
        self.condition_indicator.register_tokens(&mut staged)?;
        staged.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        let mut ids = HashSet::with_capacity(staged.len());
        for token in &staged {
            if !ids.insert(token.id()) || tokens.iter().any(|existing| existing.id() == token.id())
            {
                return Err(ModelError::ConstraintConflict(format!(
                    "conditional then `{}` helper token already exists",
                    self.id.name
                ))
                .into());
            }
        }
        tokens.extend(staged);
        Ok(())
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
        let mut constraints = self
            .condition_indicator
            .mechanism_constraints(symbol_to_index)?;
        let indicator_index =
            Self::indicator_index(symbol_to_index, self.condition_indicator.result_variable())?;
        let result_index = Self::variable_index(symbol_to_index, &self.result_var)?;
        let lower = to_f64(&self.then_bounds.lower).ok_or_else(|| {
            ModelError::InvalidConstraint("conditional then lower bound is not finite".to_string())
        })?;
        let upper = to_f64(&self.then_bounds.upper).ok_or_else(|| {
            ModelError::InvalidConstraint("conditional then upper bound is not finite".to_string())
        })?;
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err(ModelError::InvalidConstraint(
                "conditional then bounds must be finite and ordered".to_string(),
            )
            .into());
        }
        let lower_v = convert_f64_to_v::<V>(lower, "conditional then lower bound")?;
        let upper_v = convert_f64_to_v::<V>(upper, "conditional then upper bound")?;
        let zero = convert_f64_to_v::<V>(0.0, "conditional then zero")?;
        let one = convert_f64_to_v::<V>(1.0, "conditional then one")?;
        let neg_one = convert_f64_to_v::<V>(-1.0, "conditional then negative one")?;
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        let mut add_constraint =
            |name: String, polynomial: Linear<V>, relation: ConstraintRelation, rhs: V| {
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(polynomial, relation, rhs),
                    &name,
                    source.clone(),
                ));
            };

        // z = y * t 的四条 McCormick/Big-M 线性约束，适用于含负值范围。
        // Four linear product constraints for z = y * t, including negative ranges.
        let polynomial = Linear::new(
            self.then_poly
                .monomials()
                .iter()
                .map(|monomial| {
                    LinearMonomial::new(
                        monomial.coefficient().clone() * neg_one.clone(),
                        monomial.var_index(),
                    )
                })
                .chain([
                    LinearMonomial::new(lower_v.clone() * neg_one.clone(), indicator_index),
                    LinearMonomial::new(one.clone(), result_index),
                ])
                .collect(),
            self.then_poly.constant_term().clone() * neg_one.clone(),
        );
        add_constraint(
            format!("{}_then_true_ub", self.id.name),
            polynomial,
            ConstraintRelation::LessEqual,
            lower_v.clone() * neg_one.clone(),
        );

        let polynomial = Linear::new(
            self.then_poly
                .monomials()
                .iter()
                .map(|monomial| {
                    LinearMonomial::new(
                        monomial.coefficient().clone() * neg_one.clone(),
                        monomial.var_index(),
                    )
                })
                .chain([
                    LinearMonomial::new(upper_v.clone() * neg_one.clone(), indicator_index),
                    LinearMonomial::new(one.clone(), result_index),
                ])
                .collect(),
            self.then_poly.constant_term().clone() * neg_one.clone(),
        );
        add_constraint(
            format!("{}_then_true_lb", self.id.name),
            polynomial,
            ConstraintRelation::GreaterEqual,
            upper_v.clone() * neg_one.clone(),
        );

        add_constraint(
            format!("{}_then_zero_lb", self.id.name),
            Linear::new(
                vec![
                    LinearMonomial::new(one.clone(), result_index),
                    LinearMonomial::new(lower_v * neg_one.clone(), indicator_index),
                ],
                zero.clone(),
            ),
            ConstraintRelation::GreaterEqual,
            zero.clone(),
        );
        add_constraint(
            format!("{}_then_zero_ub", self.id.name),
            Linear::new(
                vec![
                    LinearMonomial::new(one, result_index),
                    LinearMonomial::new(upper_v * neg_one, indicator_index),
                ],
                zero,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(0.0, "conditional then zero")?,
        );
        Ok(constraints)
    }
}

/// 条件值函数的兼容别名 / Compatibility alias for the conditional-value function
pub type ConditionalIfThenFunction<V> = ConditionalThenFunction<V>;

fn auxiliary_symbol_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

fn infer_constant_bounds<V>(polynomial: &Linear<V>) -> Result<ConditionBounds<V>>
where
    V: Clone + Debug + FromPrimitive + ToPrimitive,
{
    if !polynomial.monomials().is_empty() {
        return Err(ModelError::InvalidConstraint(
            "conditional then requires explicit finite then bounds for a non-constant polynomial"
                .to_string(),
        )
        .into());
    }
    let value = polynomial.constant_term().to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(
            "conditional then constant cannot be converted to a finite bound".to_string(),
        )
    })?;
    if !value.is_finite() {
        return Err(ModelError::InvalidConstraint(
            "conditional then constant must be finite".to_string(),
        )
        .into());
    }
    let bound = V::from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint("conditional then constant is not representable".to_string())
    })?;
    Ok(ConditionBounds {
        lower: bound.clone(),
        upper: bound,
    })
}

fn result_range<V>(then_bounds: &ConditionBounds<V>) -> Result<(f64, f64)>
where
    V: Clone + Debug + ToPrimitive,
{
    then_bounds.validate()?;
    let lower = then_bounds.lower.to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(
            "conditional then lower bound cannot be converted".to_string(),
        )
    })?;
    let upper = then_bounds.upper.to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(
            "conditional then upper bound cannot be converted".to_string(),
        )
    })?;
    Ok((lower.min(0.0), upper.max(0.0)))
}

fn validate_finite_linear<V>(polynomial: &Linear<V>, name: &str) -> Result<()>
where
    V: ToPrimitive,
{
    let is_finite = |value: &V| {
        value
            .to_f64()
            .map(|converted| converted.is_finite())
            .unwrap_or(false)
    };
    if !is_finite(polynomial.constant_term())
        || polynomial
            .monomials()
            .iter()
            .any(|monomial| !is_finite(monomial.coefficient()))
    {
        return Err(ModelError::InvalidConstraint(format!(
            "{name} must contain only finite solver-convertible values"
        ))
        .into());
    }
    Ok(())
}

impl<V> Display for ConditionalThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "conditional_then({})", self.id.name)
    }
}

impl<V> DynSymbol for ConditionalThenFunction<V>
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

impl<V> Symbol for ConditionalThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ConditionalThenFunction<V>
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
        self.append_tokens(tokens)
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

    fn range(&self) -> Option<VariableRange<V>> {
        let range = self.result_var.range();
        Some(VariableRange::new(
            range
                .lower_bound
                .map(|value| V::from_f64(value).expect("convert conditional then lower range")),
            range
                .upper_bound
                .map(|value| V::from_f64(value).expect("convert conditional then upper range")),
        ))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("conditional_then({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for ConditionalThenFunction<V>
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
        self.append_tokens(tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.evaluate_with_tokens(token_table, zero_if_none)
            .ok()
            .flatten()
    }
}

impl<V> LinearIntermediateSymbol<V> for ConditionalThenFunction<V>
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
    use crate::symbol::functions::conditional::ConditionBounds;
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

    #[test]
    fn conditional_then_supports_signed_values_and_undefined_conditions() {
        let condition = ConditionalIfFunction::new(
            Linear::constant(0.0),
            ConditionRelation::Greater,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .unwrap();
        let function = ConditionalThenFunction::new(condition, Linear::constant(-2.0)).unwrap();

        assert_eq!(function.classify(&0.1).unwrap(), TruthValue::True);
        assert_eq!(function.evaluate(&0.1, &-2.0).unwrap(), Some(-2.0));
        assert_eq!(function.evaluate(&-0.1, &-2.0).unwrap(), Some(0.0));
        assert_eq!(function.evaluate(&-0.1, &f64::NAN).unwrap(), Some(0.0));
        assert_eq!(function.evaluate(&0.05, &-2.0).unwrap(), None);
    }

    #[test]
    fn conditional_then_rejects_non_finite_then_values() {
        let function = ConditionalThenFunction::from_parts(
            Linear::constant(0.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
            Linear::constant(1.0),
        )
        .unwrap();
        assert!(function.evaluate(&0.0, &f64::NAN).is_err());
    }

    #[test]
    fn conditional_then_registers_indicator_result_and_signed_product_rows() {
        let function = ConditionalThenFunction::from_parts_with_bounds(
            Linear::constant(0.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
            Linear::new(vec![LinearMonomial::new(2.0, 17)], -1.0),
            ConditionBounds {
                lower: -1.0,
                upper: 3.0,
            },
        )
        .unwrap();

        assert_eq!(function.result_range(), VariableRange::bounded(-1.0, 3.0));
        let mut tokens = Vec::new();
        function.register_tokens(&mut tokens).unwrap();
        assert_eq!(tokens.len(), 3);

        let symbol_to_index = token_index_map(&tokens);
        let constraints = function.mechanism_constraints(&symbol_to_index).unwrap();
        assert_eq!(constraints.len(), 7);
        assert!(
            constraints
                .iter()
                .any(|constraint| constraint.name.ends_with("_then_true_ub"))
        );
        assert!(
            constraints
                .iter()
                .any(|constraint| constraint.name.ends_with("_then_zero_ub"))
        );

        let x = ContinuousVariableItem::create(VariableId::standalone(31_001), "x");
        let tx = Token::from_generic(x, 17);
        tx.set_result(2.0);
        let mut value_tokens = VecTokenList::new();
        value_tokens.add_token(tx);
        assert_eq!(function.calculate_value(&value_tokens, false), Some(3.0));
    }

    #[test]
    fn conditional_then_requires_explicit_bounds_for_non_constant_then() {
        let condition = ConditionalIfFunction::new(
            Linear::constant(0.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .unwrap();
        assert!(
            ConditionalThenFunction::new(
                condition,
                Linear::new(vec![LinearMonomial::new(1.0, 17)], 0.0),
            )
            .is_err()
        );
    }
}
