//! If-Then 蕴含函数符号 / If-Then implication function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::infer_linear_shifted_abs_bound_from_tokens;
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

/// 内部关系指示器允许的最小 Big-M / Minimum Big-M allowed for an internal relation indicator
///
/// 与 `InequalityFunction` 的推断下限保持同一个数值，保证延迟与即时两条路径解析出相同的 M。
/// Mirrors the inference floor of `InequalityFunction` so the deferred and eager paths resolve the
/// same M.
const MIN_BIG_M: f64 = 1.0;

/// 解析内部关系指示器的 Big-M。
///
/// 取法与 `InequalityFunction` 的 `mechanism_constraints_with_tokens` 完全一致：先按令牌边界推断
/// （推断值不低于 [`MIN_BIG_M`]），取不到时回退到构造时写入的配置值；配置值不可用（无法转换、
/// 非有限或不大于 0）时返回 `None`，由即时展开路径报出配置错误。
///
/// Resolve the Big-M of an internal relation indicator.
///
/// The resolution is identical to `mechanism_constraints_with_tokens` of `InequalityFunction`:
/// infer from token bounds first (floored by [`MIN_BIG_M`]), fall back to the value configured at
/// construction, and return `None` when the configured value is unusable (not convertible,
/// non-finite or not positive) so eager expansion surfaces the configuration error.
fn resolve_indicator_big_m<V>(
    indicator: &InequalityFunction<V>,
    tokens: &[Token<V>],
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    if let Some(inferred) = infer_linear_shifted_abs_bound_from_tokens(
        indicator.left_polynomial(),
        indicator.right_value(),
        tokens,
    ) {
        return Some(inferred.max(MIN_BIG_M));
    }
    let configured = to_f64(indicator.big_m())?;
    if !configured.is_finite() || configured <= 0.0 {
        return None;
    }
    Some(configured)
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

    /// 替换条件与 then 多项式，保留全部变量与符号标识 / Replace the condition and then polynomials while keeping every variable and symbol identity.
    ///
    /// 供二次包装器在机制展开时把桥接列重映射到模型索引使用；范围边界、
    /// 关系与三值语义不变。
    /// Used by quadratic wrappers to remap bridge columns to model indices during
    /// mechanism expansion; bounds, relations, and three-valued semantics stay unchanged.
    pub(crate) fn with_condition_and_then_polynomials(
        &self,
        condition: Linear<V>,
        then_poly: Linear<V>,
    ) -> Self {
        let mut cloned = self.clone();
        cloned.condition.condition = condition.clone();
        cloned.then_poly = then_poly;
        cloned.condition_indicator = cloned
            .condition_indicator
            .with_condition_polynomial(condition);
        cloned
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

/// 条件值原生写入里条件关系的核心单边映射 / Core one-sided mapping of the condition relation in a
/// conditional-value native write
///
/// 即时展开的条件块由 [`relation_indicator_constraints`] 生成两条行，把条件差式 `s`（条件多项式本身）
/// 与条件指示列 `ind` 线性化：一条在 `ind = 1` 时收紧、一条在 `ind = 0` 时收紧，另一侧恰好松弛成符号
/// 声明的条件范围 `[bounds.lower, bounds.upper]`。因此把 `ind` 固定为 1 / 0 后，每行对 `s` 的投影里各
/// 有一条**不含范围系数**的核心关系，本结构就是这两条核心关系（`strict_boundary` 是严格关系与包含关系
/// 之间的正间隔）。
///
/// Eager expansion's condition block emits two rows through [`relation_indicator_constraints`] linearising the
/// condition difference `s` (the condition polynomial itself) against the condition indicator column `ind`:
/// one tightens at `ind = 1` and one at `ind = 0`, while the opposite side relaxes to exactly the declared
/// condition range `[bounds.lower, bounds.upper]`. Fixing `ind` to 1 / 0 therefore leaves exactly one core
/// relation per projection that carries no range coefficient, and those two core relations are what this
/// structure holds (`strict_boundary` is the positive gap between the strict and the inclusive relation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionalValueRelationCore {
    /// 条件指示列取真时的核心关系 `s REL rhs` / Core relation `s REL rhs` for `indicator = 1`
    pub when_true: (ConstraintRelation, f64),
    /// 条件指示列取假时的核心关系 `s REL rhs` / Core relation `s REL rhs` for `indicator = 0`
    pub when_false: (ConstraintRelation, f64),
}

/// 返回条件关系在给定严格边界下的两条核心单边关系 / The two core one-sided relations of a condition
/// relation for a given strict boundary
///
/// 逐关系由 [`relation_linearization_values`] 的四个系数推出（`lower` / `upper` 是条件范围，
/// `boundary` 是严格边界）：
///
/// | 关系 | `ind = 1` | `ind = 0` | 被松弛的两侧 |
/// |---|---|---|---|
/// | `Greater`（`s > 0`） | `s >= boundary` | `s <= 0` | `s >= lower`、`s <= upper` |
/// | `GreaterEqual`（`s >= 0`） | `s >= 0` | `s <= -boundary` | `s >= lower`、`s <= upper` |
/// | `Less`（`s < 0`） | `s <= -boundary` | `s >= 0` | `s >= lower`、`s <= upper` |
/// | `LessEqual`（`s <= 0`） | `s <= 0` | `s >= boundary` | `s >= lower`、`s <= upper` |
///
/// 四种关系被松弛掉的两侧**都是**符号声明的条件范围本身，因此原生写入的冗余证明只需要该区间是有限有序
/// 的（见 `native.rs` 的对照说明），不需要从 SDK 读列界。
///
/// Derived per relation from the four coefficients of [`relation_linearization_values`] (`lower` / `upper`
/// are the condition range and `boundary` the strict boundary):
///
/// | relation | `ind = 1` | `ind = 0` | relaxed sides |
/// |---|---|---|---|
/// | `Greater` (`s > 0`) | `s >= boundary` | `s <= 0` | `s >= lower`, `s <= upper` |
/// | `GreaterEqual` (`s >= 0`) | `s >= 0` | `s <= -boundary` | `s >= lower`, `s <= upper` |
/// | `Less` (`s < 0`) | `s <= -boundary` | `s >= 0` | `s >= lower`, `s <= upper` |
/// | `LessEqual` (`s <= 0`) | `s <= 0` | `s >= boundary` | `s >= lower`, `s <= upper` |
///
/// In all four relations the relaxed sides are **exactly** the symbol's declared condition range, so the
/// native write's redundancy proof only needs that interval to be finite and ordered (see the comparison in
/// `native.rs`) and never reads SDK column bounds.
pub fn conditional_value_relation_core(
    relation: ConditionRelation,
    strict_boundary: f64,
) -> ConditionalValueRelationCore {
    match relation {
        ConditionRelation::Greater => ConditionalValueRelationCore {
            when_true: (ConstraintRelation::GreaterEqual, strict_boundary),
            when_false: (ConstraintRelation::LessEqual, 0.0),
        },
        ConditionRelation::GreaterEqual => ConditionalValueRelationCore {
            when_true: (ConstraintRelation::GreaterEqual, 0.0),
            when_false: (ConstraintRelation::LessEqual, -strict_boundary),
        },
        ConditionRelation::Less => ConditionalValueRelationCore {
            when_true: (ConstraintRelation::LessEqual, -strict_boundary),
            when_false: (ConstraintRelation::GreaterEqual, 0.0),
        },
        ConditionRelation::LessEqual => ConditionalValueRelationCore {
            when_true: (ConstraintRelation::LessEqual, 0.0),
            when_false: (ConstraintRelation::GreaterEqual, strict_boundary),
        },
    }
}

/// 条件值函数的延迟结构 / Deferred structure of a conditional-value function
///
/// 与 IF、蕴含、极值同一模式：结构持有产生它的符号（`Arc`）与创建时冻结的参数，并通过**同一个**即时
/// 生成器物化，使延迟物化与即时展开逐行一致。本函数的即时展开没有可配置或可推断的 Big-M（条件块用符号
/// 声明的有限条件范围，分支块用 then 多项式的显式有限范围），因此冻结参数就是 `then_bounds` 本身——
/// 它已随符号克隆一起被冻结。
///
/// 结果列是条件值结果列；条件指示器的两个辅助列（结果列与条件指示列）都上报为辅助列，全部参与「是否被
/// 外部引用 / 是否可省略」的判定。
///
/// Same pattern as IF, implication and the extrema: the structure holds the symbol that produced it (an
/// `Arc`) together with the parameters frozen at creation time and materializes through the **same** eager
/// generator, so deferred materialization matches eager expansion row by row. This function's eager expansion
/// has no configurable or inferable Big-M (the condition block uses the symbol's declared finite condition
/// range and the branch block the then polynomial's explicit finite range), so the frozen parameter is
/// `then_bounds` itself, which is already frozen by cloning the symbol.
///
/// The result column is the conditional-value result, while the condition indicator's two helper columns (its
/// result and its indicator column) are both reported as helpers and take part in the
/// externally-referenced / omittable analysis.
#[derive(Debug)]
pub struct ConditionalThenStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<ConditionalThenFunction<V>>,
    /// 结果列 / Result column
    result: VariableId,
    /// 辅助列 / Helper columns
    helpers: Vec<VariableId>,
    /// 冻结的 then 多项式范围 / Frozen then-polynomial range
    then_bounds: ConditionBounds<V>,
}

impl<V> ConditionalThenStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建结构描述 / Create a structure description.
    ///
    /// 冻结参数只有 `then_bounds`：即时展开的分支块依赖它，条件块依赖符号声明的条件范围与严格边界（都在
    /// 符号内部，随 `Arc` 一起冻结）。
    ///
    /// The only frozen parameter is `then_bounds`: the eager branch block depends on it while the condition
    /// block depends on the symbol's declared condition range and strict boundary, both inside the symbol and
    /// therefore frozen with the `Arc`.
    pub fn new(name: impl Into<String>, symbol: Arc<ConditionalThenFunction<V>>) -> Self {
        let result = symbol.result_variable().id();
        let helpers = symbol
            .condition_indicator()
            .helper_variables()
            .iter()
            .map(|variable| variable.id())
            .collect();
        let then_bounds = symbol.then_bounds().clone();
        Self {
            name: name.into(),
            symbol,
            result,
            helpers,
            then_bounds,
        }
    }
}

impl<V> ConditionalThenStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 获取函数名称 / Get the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取结果列 / Get the result column.
    pub fn result(&self) -> &VariableId {
        &self.result
    }

    /// 获取辅助列 / Get the helper columns.
    pub fn helpers(&self) -> &[VariableId] {
        &self.helpers
    }

    /// 获取冻结的 then 多项式范围 / Get the frozen then-polynomial range.
    pub fn then_bounds(&self) -> &ConditionBounds<V> {
        &self.then_bounds
    }

    /// 获取产生本结构的符号（只读）/ Read-only access to the symbol that produced this structure.
    ///
    /// 用途：原生 writer 必须把**同一份**条件关系、条件范围、严格边界与 then 多项式写成 SDK 的一般约束，
    /// 而不是在别处重新推导；本访问器只转发不可变引用，不复制公式，也不暴露可变状态。
    ///
    /// Purpose: a native writer must write this **very** condition relation, condition range, strict boundary
    /// and then polynomial as SDK general constraints instead of re-deriving them; this only forwards an
    /// immutable reference, copies no formula and exposes no mutable state.
    pub fn symbol(&self) -> &Arc<ConditionalThenFunction<V>> {
        &self.symbol
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for ConditionalThenStructure<V>
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
    fn function_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn usage_binding(&self) -> Option<crate::model::intermediate::StructureUsageBinding> {
        // 条件指示器的两个辅助列都是本结构的辅助列，参与「是否被外部引用 / 是否可省略」的判定。
        // Both helper columns of the condition indicator are helpers of this structure and take part in the
        // externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            self.helpers.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        // 冻结的 then 范围与全部辅助列都进入指纹：任一语义字段变化都必须让旧记录失效。
        // The frozen then range and every helper column are part of the fingerprint: any semantic change must
        // invalidate old records.
        let helpers = self
            .helpers
            .iter()
            .map(|helper| helper.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "conditional_then|{}|{}|{}|{}|{:.17e}|{:.17e}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            helpers,
            self.then_bounds
                .lower
                .to_f64()
                .expect("conditional then lower bound is representable"),
            self.then_bounds
                .upper
                .to_f64()
                .expect("conditional then upper bound is representable"),
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器，保证两条路径逐行一致。
        // Reuse the eager path's generator so both paths stay row-identical.
        self.symbol.build_mechanism_constraints(symbol_to_index)
    }
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

    fn deferred_structure_with_tokens(
        &self,
        _tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // 本函数的即时展开不需要推断或配置 Big-M（条件块用声明的条件范围，分支块用 then 范围的显式值），
        // 因此结构总能给出；折叠情形与其它语义性拒绝留给 SDK 无关的 planner，让拒绝理由可被单独测试。
        // This function's eager expansion needs no inferred or configured Big-M (the condition block uses the
        // declared condition range and the branch block the explicit then range), so the structure is always
        // offered; the folded case and the other semantic rejections are left to the SDK-free planner so their
        // reasons can be tested on their own.
        Some(Arc::new(ConditionalThenStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
        )))
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

    /// 收集两个内部关系指示器注册的全部辅助列。
    ///
    /// 直接复用两个指示器自己的令牌注册，因此等号形态下的 side 列也会被完整收集；漏报辅助列会
    /// 让原生路径误判它可以被省略。
    ///
    /// Collect every helper column registered by the two internal relation indicators.
    ///
    /// The indicators' own token registration is reused, so the side column of the equality form is
    /// collected as well; omitting a helper would let a native path wrongly drop it.
    fn indicator_helper_columns(&self) -> Result<Vec<crate::variable::VariableId>> {
        let mut tokens = Vec::new();
        self.premise_indicator.register_tokens(&mut tokens)?;
        self.consequence_indicator.register_tokens(&mut tokens)?;
        Ok(tokens.iter().map(|token| token.id()).collect())
    }

    /// 使用冻结的 Big-M 生成机制约束 / Build mechanism constraints with frozen Big-M values.
    ///
    /// 与即时路径的 `mechanism_constraints_with_tokens` 结构一致：先前提指示器、再结论指示器、
    /// 最后逻辑行；区别只是两个指示器的 M 来自调用方冻结的值，而不是当前令牌重新推断。
    ///
    /// Mirrors the eager `mechanism_constraints_with_tokens` layout: premise indicator rows,
    /// consequence indicator rows, then the logical rows. The only difference is that both
    /// indicators take the M frozen by the caller instead of re-inferring it from tokens.
    fn build_mechanism_constraints_with_big_ms(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        premise_big_m: f64,
        consequence_big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let premise_indicator = self
            .premise_indicator
            .with_big_m_value(convert_f64_to_v::<V>(premise_big_m, "if_then premise big-M")?);
        let consequence_indicator = self.consequence_indicator.with_big_m_value(
            convert_f64_to_v::<V>(consequence_big_m, "if_then consequence big-M")?,
        );
        let mut constraints =
            <InequalityFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
                &premise_indicator,
                symbol_to_index,
            )?;
        constraints.extend(
            <InequalityFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
                &consequence_indicator,
                symbol_to_index,
            )?,
        );
        constraints.extend(self.logical_constraints(symbol_to_index)?);
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

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 必须在结构创建时固定：两个内部关系指示器各自按令牌边界推断、再回退到构造配置值，
        // 与即时路径逐个指示器的取法完全一致；任一指示器都取不到可用 M 时不提供结构，让即时展开
        // 报出配置错误，而不是把错误推迟到物化阶段。
        // The Big-M values must be fixed when the structure is created: each internal relation
        // indicator infers from token bounds and falls back to its configured value exactly like the
        // eager path. When neither indicator yields a usable M, no structure is offered so eager
        // expansion surfaces the configuration error instead of deferring it to materialization.
        let premise_big_m = resolve_indicator_big_m(&self.premise_indicator, tokens)?;
        let consequence_big_m = resolve_indicator_big_m(&self.consequence_indicator, tokens)?;
        let helpers = self.indicator_helper_columns().ok()?;
        Some(Arc::new(IfThenStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            helpers,
            premise_big_m,
            consequence_big_m,
        )))
    }
}

/// IF-THEN 蕴含的求解器无关结构描述
/// Solver-neutral structure description of the IF-THEN implication
///
/// 与 IF/极值采用同一模式：持有产生它的符号（`Arc`）与创建时固定的一组 Big-M，物化时回调手写
/// 路径的同一个公式生成器并传入同一组 M，因此延迟物化与 EAGER 展开逐行一致（含 M 取值）。蕴含
/// 结果二值列是结果列；两个内部关系指示器注册的结果列（以及等号形态下的 side 列）是本结构的辅助
/// 列，全部上报以免原生路径误判可以省略。
///
/// Follows the same pattern as IF and the extrema: the structure holds the symbol that produced it
/// (an `Arc`) together with the Big-M values fixed at creation time and materializes through the
/// very same formula generator as the handwritten eager path with those same values, so deferred
/// materialization matches eager expansion row by row, including the M values. The implication
/// result column is the result column, while the columns registered by the two internal relation
/// indicators (plus their side columns for the equality form) are helpers and are all reported so a
/// native path cannot wrongly omit them.
#[derive(Debug)]
pub struct IfThenStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<IfThenFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 内部关系指示器的辅助列 / Helper columns of the internal relation indicators
    helpers: Vec<crate::variable::VariableId>,
    /// 前提指示器冻结的 Big-M / Big-M frozen for the premise indicator
    premise_big_m: f64,
    /// 结论指示器冻结的 Big-M / Big-M frozen for the consequence indicator
    consequence_big_m: f64,
}

impl<V> IfThenStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(
        name: impl Into<String>,
        symbol: Arc<IfThenFunction<V>>,
        helpers: Vec<crate::variable::VariableId>,
        premise_big_m: f64,
        consequence_big_m: f64,
    ) -> Self {
        let result = symbol.result_variable().id();
        Self {
            name: name.into(),
            symbol,
            result,
            helpers,
            premise_big_m,
            consequence_big_m,
        }
    }

    /// 获取函数名称 / Get the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取结果列 / Get the result column.
    pub fn result(&self) -> &crate::variable::VariableId {
        &self.result
    }

    /// 获取辅助列 / Get the helper columns.
    pub fn helpers(&self) -> &[crate::variable::VariableId] {
        &self.helpers
    }

    /// 获取前提指示器冻结的 Big-M / Get the Big-M frozen for the premise indicator.
    pub fn premise_big_m(&self) -> f64 {
        self.premise_big_m
    }

    /// 获取结论指示器冻结的 Big-M / Get the Big-M frozen for the consequence indicator.
    pub fn consequence_big_m(&self) -> f64 {
        self.consequence_big_m
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for IfThenStructure<V>
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
    fn function_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn usage_binding(&self) -> Option<crate::model::intermediate::StructureUsageBinding> {
        // 两个内部关系指示器的结果列与 side 列都是本结构的辅助列，参与「是否被外部引用 /
        // 是否可省略」的判定。
        // The result and side columns of both internal relation indicators are helpers of this
        // structure and take part in the externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            self.helpers.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        // 两组 Big-M 与全部辅助列都进入指纹：任一语义字段变化都必须让旧记录失效。
        // Both Big-M values and every helper column are part of the fingerprint: any semantic
        // change must invalidate old records.
        let helpers = self
            .helpers
            .iter()
            .map(|helper| helper.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "if_then|{}|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            helpers,
            crate::model::intermediate::fingerprint_float(self.premise_big_m),
            crate::model::intermediate::fingerprint_float(self.consequence_big_m)
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一组 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M values so both paths stay
        // row-identical.
        self.symbol.build_mechanism_constraints_with_big_ms(
            symbol_to_index,
            self.premise_big_m,
            self.consequence_big_m,
        )
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
    use crate::model::{FunctionExpansionPolicy, MetaModel};
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

    /// 逐行比较延迟物化与即时展开 / Compare deferred materialization with eager expansion row by row
    fn assert_rows_match(eager: &[LinearConstraint<f64>], deferred: &[LinearConstraint<f64>]) {
        assert_eq!(eager.len(), deferred.len());
        for (eager_row, deferred_row) in eager.iter().zip(deferred.iter()) {
            assert_eq!(eager_row.name, deferred_row.name);
            assert_eq!(eager_row.inequality.relation, deferred_row.inequality.relation);
            assert_eq!(eager_row.inequality.rhs, deferred_row.inequality.rhs);
            assert_eq!(
                eager_row.inequality.polynomial.constant_term(),
                deferred_row.inequality.polynomial.constant_term()
            );
        }
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

    #[test]
    fn if_then_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_100),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        // premise = 2x + 1 <= 0：x ∈ [-2, 3] 上取值范围 [-3, 7]，推断 M = 7。
        // premise = 2x + 1 <= 0 over x ∈ [-2, 3] spans [-3, 7], so the inferred M is 7.
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            ConstraintRelation::LessEqual,
            0.0,
        );
        // consequence = x >= 1：取值范围 [-2, 3] 减去右侧值后绝对界为 3，推断 M = 3。
        // consequence = x >= 1 spans [-2, 3]; shifted by the rhs its absolute bound is 3, so the
        // inferred M is 3.
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        );
        let function: IfThenFunction<f64> = IfThenFunction::new(
            96_101,
            "if_then_deferred",
            premise,
            consequence,
            100.0,
        );

        let mut auxiliary_tokens = Vec::new();
        function
            .register_tokens(&mut auxiliary_tokens)
            .expect("if_then tokens should be registered");
        let symbol_to_index = token_index_map(&auxiliary_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(auxiliary_tokens);

        let structure = function
            .deferred_structure_with_tokens(&tokens)
            .expect("if_then should always expose a deferred structure");
        assert_eq!(structure.function_name(), "if_then_deferred");
        let binding = structure
            .usage_binding()
            .expect("if_then structure should expose a usage binding");
        assert_eq!(binding.result, function.result_variable().id());
        assert_eq!(
            binding.helpers,
            vec![
                function.premise_indicator_variable().id(),
                function.consequence_indicator_variable().id(),
            ]
        );
        assert!(structure.fingerprint().is_some());

        let eager = function
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager if_then constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("if_then structure should materialize");
        assert!(!eager.is_empty());
        assert_rows_match(&eager, &deferred);

        // 结构必须冻结即时路径推断出的每个指示器 M：前提 7、结论 3。
        // The structure must freeze the per-indicator M inferred by the eager path: 7 and 3.
        let premise_upper = deferred
            .iter()
            .find(|constraint| constraint.name == "if_then_deferred_premise_ineq_ub")
            .expect("premise upper inequality row should exist");
        assert!((premise_upper.inequality.rhs - 7.0).abs() <= 1e-9);
        let consequence_lower = deferred
            .iter()
            .find(|constraint| constraint.name == "if_then_deferred_consequence_ineq_lb")
            .expect("consequence lower inequality row should exist");
        assert!((consequence_lower.inequality.rhs + 3.0).abs() <= 1e-9);

        // 没有令牌边界时回退到构造配置的 M，两条路径仍然逐行一致。
        // Without token bounds the configured M is used and both paths still agree row by row.
        let configured_structure = function
            .deferred_structure_with_tokens(&[])
            .expect("if_then should fall back to the configured big-M");
        let eager_default = function
            .mechanism_constraints_with_tokens(&symbol_to_index, &[])
            .expect("eager if_then constraints should be generated");
        let deferred_default = configured_structure
            .materialize(&symbol_to_index)
            .expect("if_then structure should materialize");
        assert_rows_match(&eager_default, &deferred_default);
    }

    #[test]
    fn if_then_structure_is_withheld_when_no_usable_big_m_exists() {
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::LessEqual,
            0.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        );
        // 配置 M 非正且没有令牌边界：两条取法都不可用，必须留给 EAGER 路径报错。
        // The configured M is not positive and no token bounds exist: neither source is usable, so
        // the error must be left to the eager path.
        let function: IfThenFunction<f64> = IfThenFunction::new(
            96_102,
            "if_then_without_big_m",
            premise,
            consequence,
            -1.0,
        );

        assert!(function.deferred_structure_with_tokens(&[]).is_none());
        assert!(
            <IfThenFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
                &function,
                &HashMap::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn if_then_defers_through_the_model_pipeline() {
        fn rows(policy: FunctionExpansionPolicy) -> Vec<String> {
            let mut model = MetaModel::<f64>::new("if_then_deferred_pipeline");
            model.set_function_expansion_policy(policy);
            let x = ContinuousVariableItem::with_range(
                VariableId::standalone(96_200),
                "x",
                VariableRange::bounded(-2.0, 3.0),
            );
            let x_index = model.register_variable(x).expect("x should register");
            let premise = LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(2.0, x_index)], 1.0),
                ConstraintRelation::LessEqual,
                0.0,
            );
            let consequence = LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
                ConstraintRelation::GreaterEqual,
                1.0,
            );
            let function: IfThenFunction<f64> = IfThenFunction::new(
                96_201,
                "if_then_pipeline",
                premise,
                consequence,
                100.0,
            );
            model
                .add_symbol(Arc::new(function))
                .expect("if_then symbol should register");

            let mechanism = model
                .try_into_mechanism_model()
                .expect("mechanism model should build");
            if policy.is_deferred() {
                // 延迟策略下不写即时行，但保留结构描述。
                // A deferred policy writes no eager row while keeping the structure description.
                assert!(mechanism.as_basic().constraints().is_empty());
                assert_eq!(mechanism.as_basic().deferred_functions().len(), 1);
                assert_eq!(
                    mechanism.as_basic().deferred_functions()[0].function_name(),
                    "if_then_pipeline"
                );
            }

            let linear = mechanism.into_linear_triad_model();
            let mut names = linear.basic.constraint_names.clone();
            names.sort();
            names
        }

        let eager = rows(FunctionExpansionPolicy::Eager);
        assert!(!eager.is_empty(), "eager rows: {eager:?}");
        // 延迟路径经物化后必须与 EAGER 得到同一批行，并使用同一组推断 Big-M。
        // The deferred path must produce the same rows as eager expansion once materialized, using
        // the same inferred Big-M values.
        assert_eq!(eager, rows(FunctionExpansionPolicy::DeferredNativeFirst));
    }

    /// 条件值原生写入的 6 条指示必须在**二元域**上与即时展开的全部行实现「公开列投影点集等价」
    /// The conditional-value native write's six indicators must be **point-set equivalent over the public
    /// projection on the binary domain** to every eager row.
    ///
    /// 先把即时行数清点清楚（这是本批最容易漏的地方）：条件块 3 条（`_if_lower`、`_if_upper`、
    /// **`_if_eq` 等式链接行**）+ 分支块 4 条 McCormick 行 = **7 条**。原生 6 条指示 =
    /// 条件核心关系 2 条 + 分支等式对**两个内部二值列各一遍** 4 条。
    ///
    /// **这不是逐行对应**：即时那条 `result_ind = ind` 的等式链接行在原生写入里被**推导**出来——两列取值
    /// 不同时原生分支等式会强制 `t = 0` 且 `result = 0`，而该公开列点恰好也是即时展开在「两列同取假」时的
    /// 可行点。因此等价成立的前提是 `ind`、`result_ind` 都是**二元列**；本测试同时给出反例：`ind = 0.5` 时
    /// 即时因等式链接行不可行，而原生（该列两条指示都不激活）可行。
    ///
    /// The test first counts the eager rows (the easiest thing to miss in this batch): three condition rows
    /// (`_if_lower`, `_if_upper` and the **`_if_eq` equality link**) plus four branch McCormick rows = **7**.
    /// The six native indicators are two condition core relations plus the branch equalities written **once per
    /// internal binary column**.
    ///
    /// **This is not a row-by-row correspondence**: the eager `result_ind = ind` link row is **derived** in the
    /// native write — when the two columns disagree the native branch equalities force `t = 0` and `result = 0`,
    /// and that public-column point is exactly an eager-feasible point with both columns false. The equivalence
    /// therefore holds while `ind` and `result_ind` are **binary**; the test also gives the counterexample:
    /// at `ind = 0.5` eager is infeasible because of the link row while native is feasible (neither indicator
    /// keyed on that column is active).
    #[test]
    fn conditional_value_native_projection_matches_eager_rows_in_the_binary_domain() {
        // 列号口径：条件变量 0、then 变量 1、条件指示列 2、条件指示器结果列 3、结果列 4。
        // Column numbering: condition variable 0, then variable 1, condition indicator 2, condition indicator
        // result 3, result column 4.
        const CONDITION_COLUMN: usize = 0;
        const THEN_COLUMN: usize = 1;
        const INDICATOR_COLUMN: usize = 2;
        const CONDITION_RESULT_COLUMN: usize = 3;
        const RESULT_COLUMN: usize = 4;

        // 条件 `x - 1 >= 0`（声明范围 [-2, 2]，严格边界 0.1）；then 多项式 `2y + 1`。
        // Condition `x - 1 >= 0` (declared range [-2, 2], strict boundary 0.1); then polynomial `2y + 1`.
        let function = ConditionalThenFunction::from_parts_with_bounds(
            Linear::new(vec![LinearMonomial::new(1.0, CONDITION_COLUMN)], -1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -2.0,
                upper: 2.0,
            },
            Linear::new(vec![LinearMonomial::new(2.0, THEN_COLUMN)], 1.0),
            ConditionBounds {
                lower: -1.0,
                upper: 3.0,
            },
        )
        .expect("the conditional-value function should build");

        let symbol_to_index = HashMap::from([
            (
                function
                    .condition_indicator()
                    .indicator_variable()
                    .id()
                    .unique_id() as usize,
                INDICATOR_COLUMN,
            ),
            (
                function
                    .condition_indicator()
                    .result_variable()
                    .id()
                    .unique_id() as usize,
                CONDITION_RESULT_COLUMN,
            ),
            (
                function.result_variable().id().unique_id() as usize,
                RESULT_COLUMN,
            ),
        ]);
        let eager = function
            .mechanism_constraints(&symbol_to_index)
            .expect("the eager rows should be generated");
        let names = eager
            .iter()
            .map(|row| row.name.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            eager.len(),
            7,
            "three condition rows plus four branch rows are expected, got {names:?}"
        );
        assert!(
            names.iter().any(|name| name.ends_with("_if_eq")),
            "the condition indicator's equality link row is part of the eager set: {names:?}"
        );

        let core = conditional_value_relation_core(ConditionRelation::GreaterEqual, 0.1);
        let satisfies = |lhs: f64, relation: ConstraintRelation, rhs: f64| match relation {
            ConstraintRelation::LessEqual => lhs <= rhs + 1e-12,
            ConstraintRelation::GreaterEqual => lhs + 1e-12 >= rhs,
            ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-12,
        };
        let evaluate = |row: &LinearConstraint<f64>, values: &HashMap<usize, f64>| {
            let mut lhs = *row.inequality.polynomial.constant_term();
            for monomial in row.inequality.polynomial.monomials() {
                lhs += *monomial.coefficient()
                    * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
            }
            satisfies(lhs, row.inequality.relation, row.inequality.rhs)
        };

        // 原生侧：条件核心关系（ind）+ 分支等式（result_ind 与 ind 各一遍）。
        // Native side: the condition core relation (ind) plus the branch equalities (once for result_ind and
        // once for ind).
        let native_feasible = |x: f64, y: f64, indicator: f64, condition_result: f64, result: f64| {
            let difference = x - 1.0;
            let keyed = |column: f64, when_true: bool| -> bool {
                if (column - 1.0).abs() <= 1e-12 {
                    when_true
                } else if column.abs() <= 1e-12 {
                    !when_true
                } else {
                    false
                }
            };
            let branch = |when_true: bool| -> bool {
                if when_true {
                    (result - (2.0 * y + 1.0)).abs() <= 1e-12
                } else {
                    result.abs() <= 1e-12
                }
            };
            let mut satisfied = true;
            if indicator.abs() <= 1e-12 {
                satisfied &= satisfies(difference, core.when_false.0, core.when_false.1);
            }
            if (indicator - 1.0).abs() <= 1e-12 {
                satisfied &= satisfies(difference, core.when_true.0, core.when_true.1);
            }
            if keyed(indicator, true) {
                satisfied &= branch(true);
            }
            if keyed(indicator, false) {
                satisfied &= branch(false);
            }
            if keyed(condition_result, true) {
                satisfied &= branch(true);
            }
            if keyed(condition_result, false) {
                satisfied &= branch(false);
            }
            satisfied
        };

        // 二元域逐点比较：ind、result_ind 取 0/1，x 取两侧，result 取若干值（t = 2y + 1 = 2）。
        // Pointwise comparison on the binary domain: ind and result_ind take 0/1, x takes both sides and result
        // takes several values (t = 2y + 1 = 2).
        for indicator in [0.0f64, 1.0] {
            for condition_result in [0.0f64, 1.0] {
                for x in [0.5f64, 1.5] {
                    for result in [0.0f64, 1.0, 2.0, 3.0] {
                        let values = HashMap::from([
                            (CONDITION_COLUMN, x),
                            (THEN_COLUMN, 0.5),
                            (INDICATOR_COLUMN, indicator),
                            (CONDITION_RESULT_COLUMN, condition_result),
                            (RESULT_COLUMN, result),
                        ]);
                        let eager_feasible = eager.iter().all(|row| evaluate(row, &values));
                        let native = native_feasible(x, 0.5, indicator, condition_result, result);
                        assert_eq!(
                            eager_feasible, native,
                            "mismatch at (x, ind, result_ind, result) = ({x}, {indicator}, {condition_result}, {result})"
                        );
                    }
                }
            }
        }

        // 非二元内部列上两者必须分歧：即时等式链接行直接不可行，而原生该列的两条指示都不激活。
        // The two must disagree on a non-binary internal column: eager's equality link row is outright
        // infeasible while neither native indicator keyed on that column is active.
        let values = HashMap::from([
            (CONDITION_COLUMN, 1.5),
            (THEN_COLUMN, 0.5),
            (INDICATOR_COLUMN, 0.5),
            (CONDITION_RESULT_COLUMN, 0.0),
            (RESULT_COLUMN, 0.0),
        ]);
        assert!(
            !eager.iter().all(|row| evaluate(row, &values)),
            "eager must reject a non-binary condition indicator column"
        );
        assert!(
            native_feasible(1.5, 0.5, 0.5, 0.0, 0.0),
            "the native rows must accept it, which is why the writer verifies binaryness"
        );
    }
}
