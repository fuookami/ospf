//! 完整线性结构的不可变快照与会话版本。 / Immutable full linear snapshots and session revisions.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;
use std::ops::{Add, Mul, Neg, Sub};
use bigdecimal::BigDecimal;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use crate::error::{ModelError, Result};
use crate::model::flatten::{Linear, LinearMonomial};
use crate::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use crate::model::{ConstraintRelation, FunctionExpansionPolicy, LinearInequality, MetaModel, ObjectiveCategory};
use crate::token::{AnyVariable, Token, TokenVariableData};
use crate::variable::{VariableId, VariableType};

/// 线性结构数值域；有限性检查不经过 f64。 / Linear structural value domain, without f64 finite checks.
pub trait LinearModelValue:
    Clone + Debug + PartialOrd + Send + Sync + 'static + Zero
    + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Neg<Output = Self>
{
    /// 在原数值域中判断有限性。 / Check finiteness in the original value domain.
    fn is_finite_value(&self) -> bool;

    /// 在原数值域按绝对容差判断整数性。 / Check integrality with an absolute tolerance in the native domain.
    fn is_integral_value(&self, tolerance: &Self) -> bool;

    /// 浮点域的精确二进制值；其他数值域直接使用原域运算。 / Exact binary value for floating domains; other domains use native arithmetic.
    fn exact_binary_value(&self) -> Option<BigRational> { None }
}

impl LinearModelValue for f64 {
    fn is_finite_value(&self) -> bool { self.is_finite() }
    fn is_integral_value(&self, tolerance: &Self) -> bool { (self - self.round()).abs() <= *tolerance }
    fn exact_binary_value(&self) -> Option<BigRational> { BigRational::from_float(*self) }
}

impl LinearModelValue for f32 {
    fn is_finite_value(&self) -> bool { self.is_finite() }
    fn is_integral_value(&self, tolerance: &Self) -> bool { (self - self.round()).abs() <= *tolerance }
    fn exact_binary_value(&self) -> Option<BigRational> { BigRational::from_float(*self as f64) }
}

impl LinearModelValue for BigDecimal {
    fn is_finite_value(&self) -> bool { true }
    fn is_integral_value(&self, tolerance: &Self) -> bool {
        let fraction = (self - self.with_scale(0)).abs();
        &fraction <= tolerance || BigDecimal::one() - fraction <= *tolerance
    }
}

impl LinearModelValue for BigRational {
    fn is_finite_value(&self) -> bool { true }
    fn is_integral_value(&self, tolerance: &Self) -> bool {
        let fraction = (self - BigRational::from_integer(self.to_integer())).abs();
        &fraction <= tolerance || BigRational::one() - fraction <= *tolerance
    }
}

/// 模型会话及结构版本，不能跨模型使用。 / Model session and structural revision, never interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinearModelRevision {
    /// 模型生命周期身份。 / Model-lifecycle identity.
    pub owner: u64,
    /// 单调结构版本。 / Monotonic structural revision.
    pub sequence: u64,
}

/// 已展开线性行；系数按变量身份而非位置索引。 / Expanded row keyed by variable identity, not position.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearModelRow<V> {
    /// 稳定身份；自动身份仅在本模型会话有效。 / Stable identity; automatic identities are model-local.
    pub id: String,
    /// 原始显示名，不作为身份。 / Original display name, not an identity.
    pub source_name: String,
    /// 已合并并移除精确零的系数。 / Merged coefficients with exact zeros removed.
    pub coefficients: BTreeMap<VariableId, V>,
    /// 扣除多项式常数后的右端。 / Right-hand side after removing the polynomial constant.
    pub rhs: V,
    /// 比较方向。 / Comparison direction.
    pub relation: ConstraintRelation,
    /// 原始约束组身份。 / Original constraint-group identity.
    pub group_id: Option<u64>,
    /// 延迟约束标记。 / Lazy-constraint flag.
    pub lazy: bool,
    /// 约束优先级。 / Constraint priority.
    pub priority: u32,
    /// 原始附加参数。 / Original additional arguments.
    pub args: Option<String>,
    /// 来源函数符号身份。 / Originating function-symbol identity.
    pub source_symbol_id: Option<u64>,
}

/// 已复制的变量状态，不保留可变范围引用。 / Copied variable state without mutable bound references.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearVariableState<V> {
    /// 变量身份。 / Variable identity.
    pub variable: VariableId,
    /// 显示名称。 / Display name.
    pub name: String,
    /// 变量类型。 / Variable type.
    pub var_type: VariableType,
    /// 下界；None 表示无界。 / Lower bound; None denotes unboundedness.
    pub lower_bound: Option<V>,
    /// 上界；None 表示无界。 / Upper bound; None denotes unboundedness.
    pub upper_bound: Option<V>,
}

impl<V: LinearModelValue + One> LinearVariableState<V> {
    /// 松弛整数性时保留隐式类型边界与显式边界的交集。
    /// Relax integrality while preserving the intersection of implicit type bounds and explicit bounds.
    pub fn relaxed(&self) -> Result<Self> {
        if self.lower_bound.as_ref().is_some_and(|bound| !bound.is_finite_value())
            || self.upper_bound.as_ref().is_some_and(|bound| !bound.is_finite_value())
        { return Err(invalid("nonfinite explicit variable bound")); }
        let (lower, upper) = match self.var_type {
            VariableType::Binary | VariableType::Percentage => (Some(V::zero()), Some(V::one())),
            VariableType::Ternary => (Some(V::zero()), Some(V::one() + V::one())),
            VariableType::BalancedTernary => (Some(-V::one()), Some(V::one())),
            VariableType::UInteger | VariableType::UContinuous => (Some(V::zero()), None),
            _ => (None, None),
        };
        let mut state = self.clone();
        if let Some(lower) = lower {
            state.lower_bound = Some(state.lower_bound.filter(|bound| bound >= &lower).unwrap_or(lower));
        }
        if let Some(upper) = upper {
            state.upper_bound = Some(state.upper_bound.filter(|bound| bound <= &upper).unwrap_or(upper));
        }
        if matches!((&state.lower_bound, &state.upper_bound), (Some(lower), Some(upper)) if lower > upper) {
            return Err(invalid("empty variable domain after applying implicit type bounds"));
        }
        state.var_type = VariableType::Continuous;
        Ok(state)
    }
}

/// 完整线性快照；目标统一为最小化，常数仅保留一次。 / Full snapshot with a minimizing objective and one constant.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearModelSnapshot<V> {
    /// 会话版本。 / Session revision.
    pub revision: LinearModelRevision,
    /// 包含函数辅助变量的完整变量表。 / Complete variables including function helpers.
    pub variables: Vec<LinearVariableState<V>>,
    /// 完整展开的约束行。 / Fully expanded constraint rows.
    pub rows: Vec<LinearModelRow<V>>,
    /// 最小化目标系数。 / Minimizing objective coefficients.
    pub objective: BTreeMap<VariableId, V>,
    /// 最小化目标常数。 / Minimizing objective constant.
    pub objective_constant: V,
    /// 原始模型目标方向。 / Original model objective direction.
    pub objective_category: ObjectiveCategory,
}

/// f64 边界的精确目标求值，独立保留变量项与常数。 / Exact objective evaluation at the f64 boundary, separating variable terms and constant.
#[derive(Debug, Clone)]
pub struct LinearObjectiveEvaluationFlt64 {
    variable: BigRational,
    constant: BigRational,
}

impl LinearObjectiveEvaluationFlt64 {
    /// 精确累加系数与解值的乘积；调用方须先核对完整向量维度。 / Exactly accumulate coefficient/value products; the caller must first check complete vector dimensions.
    pub fn new(terms: impl IntoIterator<Item = (f64, f64)>, constant: f64) -> Result<Self> {
        let exact = |value| BigRational::from_float(value).ok_or_else(|| invalid("nonfinite objective operand"));
        let variable = terms.into_iter().try_fold(BigRational::zero(), |total, (coefficient, value)|
            Ok::<_, crate::error::CoreError>(total + exact(coefficient)? * exact(value)?))?;
        Ok(Self { variable, constant: exact(constant)? })
    }

    fn upward(value: BigRational) -> Result<f64> {
        let mut rounded = value.to_f64().filter(|value| value.is_finite()).ok_or_else(|| invalid("objective cannot be represented as finite f64"))?;
        if BigRational::from_float(rounded).is_some_and(|rounded| rounded < value) { rounded = rounded.next_up(); }
        if rounded.is_finite() { Ok(rounded) } else { Err(invalid("objective upper bound is not finite")) }
    }

    /// 不含常数的目标，向上舍入为最小化候选解界。 / Variable-only objective rounded upward as a minimizing incumbent bound.
    pub fn variable_value(&self) -> Result<f64> { Self::upward(self.variable.clone()) }

    /// 常数只计一次的完整目标，向上舍入。 / Full objective counting the constant exactly once, rounded upward.
    pub fn total_value(&self) -> Result<f64> { Self::upward(&self.variable + &self.constant) }

    /// 最优 master 目标的向下舍入值；本函数本身不证明最优性。 / Downward-rounded optimal-master objective; this method does not establish optimality itself.
    pub fn lower_bound(&self) -> Result<f64> { Ok(-Self::upward(-(&self.variable + &self.constant))?) }

    /// 按相对容差精确比较报告中的无常数目标；拒绝缺失与非有限值。 / Exactly compare a reported variable-only objective with relative tolerance; missing/nonfinite values fail.
    pub fn validate_reported(&self, reported: Option<f64>, tolerance: f64) -> Result<()> {
        let reported = reported.and_then(BigRational::from_float).ok_or_else(|| invalid("missing or nonfinite reported objective"))?;
        Self::compare(&self.variable, &reported, tolerance)
    }

    /// 精确比较两种注册路径的完整目标。 / Exactly compare full objectives from two registration paths.
    pub fn validate_total(&self, other: &Self, tolerance: f64) -> Result<()> {
        Self::compare(&(&self.variable + &self.constant), &(&other.variable + &other.constant), tolerance)
    }

    fn compare(left: &BigRational, right: &BigRational, tolerance: f64) -> Result<()> {
        if !tolerance.is_finite() || !(0.0..0.5).contains(&tolerance) { return Err(invalid("invalid objective tolerance")); }
        let tolerance = BigRational::from_float(tolerance).ok_or_else(|| invalid("invalid objective tolerance"))?;
        if (left - right).abs() > tolerance * left.abs().max(right.abs()).max(BigRational::one()) {
            return Err(invalid("objective differs from exact model evaluation"));
        }
        Ok(())
    }
}

impl LinearModelSnapshot<f64> {
    /// 按源身份求值完整目标，不接受缺失或多余变量。 / Evaluate the full objective by source identity, rejecting missing or extra variables.
    pub fn evaluate_objective_flt64(&self, values: &BTreeMap<VariableId, f64>) -> Result<LinearObjectiveEvaluationFlt64> {
        if values.len() != self.variables.len() || self.variables.iter().any(|state| !values.contains_key(&state.variable)) {
            return Err(invalid("objective requires the complete source vector"));
        }
        LinearObjectiveEvaluationFlt64::new(self.objective.iter().map(|(variable, coefficient)| (*coefficient, values[variable])), self.objective_constant)
    }
}

/// 编译行到实际源行的符号映射，包含等式拆行。 / Compiled-row origin and sign, including equality splits.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearCompiledRow {
    /// 本次快照的实际源行身份。 / Actual source-row identity in this snapshot.
    pub source: String,
    /// 标准化行相对源行的乘数（1 或 -1）。 / Standard-form multiplier relative to the source row (1 or -1).
    pub multiplier: i8,
}

/// f64 线性快照的受控标准化结果。 / Controlled standard-form compilation of an f64 snapshot.
#[derive(Debug, Clone)]
pub struct LinearModelCompilationFlt64 {
    /// 完整标准模型，目标不含常数。 / Complete standard model, excluding the objective constant.
    pub model: LinearTriadModel,
    /// 按本次 solver 行顺序的真实源行映射。 / Actual origins in this compilation's solver-row order.
    pub rows: Vec<LinearCompiledRow>,
    /// 原快照会话版本。 / Originating snapshot revision.
    pub revision: LinearModelRevision,
    /// 独立保留、仅计一次的最小化目标常数。 / Separately retained minimizing objective constant, counted once.
    pub objective_constant: f64,
}

impl LinearModelSnapshot<f64> {
    /// 在同一 f64 数值域编译，并显式记录每个标准行的来源与符号。
    /// Compile within the same f64 value domain, recording each standard row's origin and sign.
    pub fn compile_flt64(&self) -> Result<LinearModelCompilationFlt64> {
        let mut basic = BasicLinearTriadModel::new("linear_snapshot");
        let mut indices = BTreeMap::new();
        for (index, state) in self.variables.iter().enumerate() {
            let token = Token::new(AnyVariable::new(TokenVariableData {
                id: state.variable, index, name: state.name.clone(), display_name: None, var_type: state.var_type,
                lower_bound: state.lower_bound, upper_bound: state.upper_bound,
            }), index);
            if indices.insert(state.variable, index).is_some() { return Err(invalid("duplicate compiled variable")); }
            basic.add_variable(token);
        }
        let mut rows = Vec::new();
        for row in &self.rows {
            let multipliers: &[i8] = match row.relation {
                ConstraintRelation::LessEqual => &[1],
                ConstraintRelation::GreaterEqual => &[-1],
                ConstraintRelation::Equal => &[1, -1],
            };
            for multiplier in multipliers {
                let mut terms = SparseVector::new();
                for (variable, coefficient) in &row.coefficients {
                    let index = *indices.get(variable).ok_or_else(|| invalid("unknown compiled row variable"))?;
                    terms.add(index, checked(*coefficient * f64::from(*multiplier))?);
                }
                basic.add_constraint_with_metadata(terms, checked(row.rhs * f64::from(*multiplier))?,
                    format!("{}_{}", row.id, if *multiplier > 0 { "le" } else { "ge" }),
                    row.group_id, row.lazy, row.priority, row.args.clone(), row.source_symbol_id);
                rows.push(LinearCompiledRow { source: row.id.clone(), multiplier: *multiplier });
            }
        }
        let mut model = LinearTriadModel::from_basic(basic);
        model.objective_category = ObjectiveCategory::Minimum;
        for (variable, coefficient) in &self.objective {
            let index = *indices.get(variable).ok_or_else(|| invalid("unknown compiled objective variable"))?;
            model.c[index] = checked(*coefficient)?;
        }
        crate::solver::audit::validate_linear_model_for_backend(&model)?;
        Ok(LinearModelCompilationFlt64 { model, rows, revision: self.revision,
            objective_constant: checked(self.objective_constant)? })
    }
}

/// 模型自有的结构观察器；只在显式刷新时检测变化。 / Model-owned observer detecting changes only on explicit refresh.
pub struct LinearModelState<V> {
    latest: Option<LinearModelSnapshot<V>>,
    pub(crate) registered_rows: BTreeMap<String, (u64, LinearModelRow<V>)>,
}

impl<V> LinearModelState<V> {
    pub(crate) fn new() -> Self {
        Self { latest: None, registered_rows: BTreeMap::new() }
    }
}

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::InvalidConstraint(message.into()).into()
}

fn checked<V: LinearModelValue>(value: V) -> Result<V> {
    if value.is_finite_value() {
        Ok(value)
    } else {
        Err(invalid("linear model state contains a nonfinite value"))
    }
}

fn coefficients<V: LinearModelValue>(
    polynomial: &Linear<V>,
    variables: &[VariableId],
) -> Result<BTreeMap<VariableId, V>> {
    let mut result = BTreeMap::new();
    for term in polynomial.monomials() {
        let variable = *variables.get(term.var_index())
            .ok_or_else(|| invalid("linear model state references an unknown variable"))?;
        let coefficient = checked(term.coefficient().clone())?;
        let previous = result.remove(&variable).unwrap_or_else(V::zero);
        result.insert(variable, checked(previous + coefficient)?);
    }
    result.retain(|_, coefficient| !coefficient.is_zero());
    Ok(result)
}

impl<V: LinearModelValue + One> LinearModelSnapshot<V> {
    /// 独立复核完整变量向量的边界、整数性和全部展开行。
    /// Independently validate bounds, integrality and every expanded row of a complete variable vector.
    ///
    /// 容差必须有限、非负且小于 0.5；不接受缺失或多余变量，不评价 solver 最优性。
    /// Tolerance must be finite, nonnegative and below 0.5; missing/extra variables fail, solver optimality is not assessed.
    pub fn validate_solution(&self, values: &BTreeMap<VariableId, V>, tolerance: &V) -> Result<()> {
        if !tolerance.is_finite_value() || tolerance < &V::zero()
            || tolerance.clone() + tolerance.clone() >= V::one()
        {
            return Err(invalid("invalid linear solution tolerance"));
        }
        if values.len() != self.variables.len() || values.values().any(|value| !value.is_finite_value()) {
            return Err(invalid("linear solution must contain a complete finite variable vector"));
        }
        for state in &self.variables {
            let value = values.get(&state.variable).ok_or_else(|| invalid("missing source variable"))?;
            if let (Some(exact_value), Some(exact_tolerance)) = (value.exact_binary_value(), tolerance.exact_binary_value()) {
                let bounds = state.relaxed()?;
                let below = bounds.lower_bound.as_ref().and_then(LinearModelValue::exact_binary_value)
                    .is_some_and(|bound| exact_value < bound - &exact_tolerance);
                let above = bounds.upper_bound.as_ref().and_then(LinearModelValue::exact_binary_value)
                    .is_some_and(|bound| exact_value > bound + &exact_tolerance);
                if below || above || (state.var_type.is_integer() && !value.is_integral_value(tolerance)) {
                    return Err(invalid(format!("linear solution violates exact variable domain: {}", state.name)));
                }
                continue;
            }
            if state.lower_bound.as_ref().is_some_and(|lower| *value < lower.clone() - tolerance.clone())
                || state.upper_bound.as_ref().is_some_and(|upper| *value > upper.clone() + tolerance.clone())
                || (state.var_type.is_integer() && !value.is_integral_value(tolerance))
            {
                return Err(invalid(format!("linear solution violates variable domain: {}", state.name)));
            }
            let zero = V::zero();
            let one = V::one();
            let (lower, upper) = match state.var_type {
                VariableType::Binary | VariableType::Percentage => (Some(zero), Some(one)),
                VariableType::Ternary => (Some(zero), Some(one.clone() + one)),
                VariableType::BalancedTernary => (Some(-one.clone()), Some(one)),
                VariableType::UInteger | VariableType::UContinuous => (Some(zero), None),
                _ => (None, None),
            };
            if lower.is_some_and(|lower| *value < lower - tolerance.clone())
                || upper.is_some_and(|upper| *value > upper + tolerance.clone())
            {
                return Err(invalid("linear solution violates implicit variable-type bounds"));
            }
        }
        for row in &self.rows {
            if let (Some(rhs), Some(tolerance)) = (row.rhs.exact_binary_value(), tolerance.exact_binary_value()) {
                let mut lhs = BigRational::zero();
                for (variable, coefficient) in &row.coefficients {
                    let value = values.get(variable).ok_or_else(|| invalid("unknown row variable"))?;
                    lhs += coefficient.exact_binary_value().ok_or_else(|| invalid("nonfinite row coefficient"))?
                        * value.exact_binary_value().ok_or_else(|| invalid("nonfinite row value"))?;
                }
                let feasible = match row.relation {
                    ConstraintRelation::LessEqual => lhs <= rhs + tolerance,
                    ConstraintRelation::Equal => (lhs - rhs).abs() <= tolerance,
                    ConstraintRelation::GreaterEqual => lhs >= rhs - tolerance,
                };
                if !feasible { return Err(invalid(format!("linear solution violates exact row: {}", row.id))); }
                continue;
            }
            let mut lhs = V::zero();
            for (variable, coefficient) in &row.coefficients {
                let value = values.get(variable).ok_or_else(|| invalid("unknown row variable"))?;
                lhs = checked(lhs + checked(coefficient.clone() * value.clone())?)?;
            }
            let lower_ok = lhs >= row.rhs.clone() - tolerance.clone();
            let upper_ok = lhs <= row.rhs.clone() + tolerance.clone();
            let feasible = match row.relation {
                ConstraintRelation::LessEqual => upper_ok,
                ConstraintRelation::Equal => lower_ok && upper_ok,
                ConstraintRelation::GreaterEqual => lower_ok,
            };
            if !feasible {
                return Err(invalid(format!("linear solution violates row: {}", row.id)));
            }
        }
        Ok(())
    }
}

impl<V: LinearModelValue> MetaModel<V> {
    /// 注册不可变动态行；同 ID 同内容幂等，删除后不得复用身份。
    /// Register an immutable dynamic row; identical IDs/contents are idempotent, removed identities cannot be reused.
    ///
    /// 仅接受已注册变量的列序号，不接受未展开符号；失败不修改模型。
    /// Accepts indices of registered variables only, not unexpanded symbols; failures leave the model unchanged.
    pub fn register_linear_row(&mut self, id: &str, inequality: LinearInequality<V>) -> Result<()> {
        if id.trim().is_empty() || id.starts_with("model-local-") {
            return Err(invalid("dynamic row identity is empty or uses a reserved prefix"));
        }
        let variables = self.tokens().iter().map(|token| token.id()).collect::<Vec<_>>();
        let row = LinearModelRow {
            id: id.to_string(),
            source_name: id.to_string(),
            coefficients: coefficients(&inequality.polynomial, &variables)?,
            rhs: checked(checked(inequality.rhs)? - checked(inequality.polynomial.constant_term().clone())?)?,
            relation: inequality.relation,
            group_id: None,
            lazy: false,
            priority: 0,
            args: None,
            source_symbol_id: None,
        };
        {
            let state = self.linear_state.lock();
            if let Some((origin, previous)) = state.registered_rows.get(id) {
                return if self.constraint_ids().contains(origin) && previous == &row {
                    Ok(())
                } else {
                    Err(invalid(format!("dynamic row identity already occupied: {id}")))
                };
            }
        }
        let monomials = variables.iter().enumerate().filter_map(|(index, variable)| {
            row.coefficients.get(variable).map(|value| LinearMonomial::new(value.clone(), index))
        }).collect();
        self.add_linear_polynomial_constraint(
            Linear::new(monomials, V::zero()), row.relation, row.rhs.clone(), id,
        )?;
        let origin = *self.constraint_ids().last().ok_or_else(|| invalid("dynamic row registration lost its origin"))?;
        self.linear_state.lock().registered_rows.insert(id.to_string(), (origin, row));
        Ok(())
    }

    /// 完整展开并复制当前结构；变化推进版本，结果值和缓存不影响版本。
    /// Fully expand and copy the current structure; changes advance the revision, solution values and caches do not.
    ///
    /// 使用即时通用展开语义覆盖 deferred 策略，不修改源模型；不支持调用期间并发修改。
    /// Uses full eager fallback semantics regardless of deferred policy, without mutating the source; no concurrent mutation.
    pub fn linear_model_snapshot(&self) -> Result<LinearModelSnapshot<V>> {
        let mechanism = self.try_to_mechanism_model_with_expansion_policy(
            None, Some(FunctionExpansionPolicy::Eager),
        )?;
        if !mechanism.as_basic().quadratic_constraints().is_empty() {
            return Err(invalid("linear model state cannot project quadratic constraints"));
        }
        let declared_objective = mechanism.objective().clone();
        let mut basic = mechanism.into_basic();
        basic.materialize_deferred_functions()?;
        let variables = basic.tokens().iter().map(|token| {
            let lower_bound = token.variable.lower_bound().map(checked).transpose()?;
            let upper_bound = token.variable.upper_bound().map(checked).transpose()?;
            if matches!((&lower_bound, &upper_bound), (Some(lower), Some(upper)) if lower > upper) {
                return Err(invalid("linear model state contains reversed variable bounds"));
            }
            Ok(LinearVariableState {
                variable: token.id(), name: token.variable.name().to_string(),
                var_type: token.var_type(), lower_bound, upper_bound,
            })
        }).collect::<Result<Vec<_>>>()?;
        let variable_ids = variables.iter().map(|variable| variable.variable).collect::<Vec<_>>();
        if variable_ids.iter().collect::<HashSet<_>>().len() != variables.len() {
            return Err(invalid("linear model state contains duplicate variable identities"));
        }
        let mut state = self.linear_state.lock();
        let owner = self.model_identity();
        let explicit = state.registered_rows.iter().map(|(name, (origin, _))| (*origin, name.clone()))
            .collect::<BTreeMap<_, _>>();
        let rows = basic.constraints().iter().enumerate().map(|(index, constraint)| {
            let id = if let Some(origin) = self.constraint_ids().get(index) {
                explicit.get(origin).cloned().unwrap_or_else(|| format!("model-local-row:{owner}:{origin}"))
            } else {
                format!("model-local-derived:{owner}:{}", index - self.constraint_ids().len())
            };
            Ok(LinearModelRow {
                id, source_name: constraint.name.clone(),
                coefficients: coefficients(&constraint.inequality.polynomial, &variable_ids)?,
                rhs: checked(checked(constraint.inequality.rhs.clone())?
                    - checked(constraint.inequality.polynomial.constant_term().clone())?)?,
                relation: constraint.inequality.relation,
                group_id: constraint.group.as_ref().map(|group| group.id),
                lazy: constraint.lazy, priority: constraint.priority, args: constraint.args.clone(),
                source_symbol_id: constraint.from.as_ref().map(|symbol| symbol.id().id),
            })
        }).collect::<Result<Vec<_>>>()?;
        let mut objective = BTreeMap::new();
        let mut objective_constant = V::zero();
        for part in &declared_objective.sub_objectives {
            let weight = checked(part.weight.clone())?;
            let weight = if part.category.is_minimum() { weight } else { -weight };
            objective_constant = checked(objective_constant
                + checked(weight.clone() * checked(part.polynomial.constant_term().clone())?)?)?;
            for (variable, coefficient) in coefficients(&part.polynomial, &variable_ids)? {
                let previous = objective.remove(&variable).unwrap_or_else(V::zero);
                objective.insert(variable, checked(previous + checked(weight.clone() * coefficient)?)?);
            }
        }
        objective.retain(|_, coefficient| !coefficient.is_zero());
        let mut snapshot = LinearModelSnapshot {
            revision: state.latest.as_ref().map_or(LinearModelRevision { owner, sequence: 0 }, |old| old.revision),
            variables, rows, objective, objective_constant,
            objective_category: self.objective().category,
        };
        if let Some(previous) = &state.latest
            && previous != &snapshot
        {
            snapshot.revision.sequence = previous.revision.sequence.checked_add(1)
                .ok_or_else(|| invalid("linear model revision exhausted"))?;
        }
        state.latest = Some(snapshot.clone());
        Ok(snapshot)
    }

    /// 显式刷新并拒绝过期或跨模型版本。 / Explicitly refresh and reject stale or foreign-model revisions.
    pub fn require_linear_model_revision(&self, revision: LinearModelRevision) -> Result<()> {
        if self.linear_model_snapshot()?.revision == revision {
            Ok(())
        } else {
            Err(invalid("stale or foreign linear model revision"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;
    use super::*;
    use crate::model::SubObjective;
    use crate::model::LinearExpressionBuilder;
    use crate::symbol::function::AbsFunction;
    use crate::variable::{Continuous, VariableRange};

    fn model() -> (MetaModel, usize) {
        let mut model = MetaModel::new("state");
        let x = model.register_auto_variable_with_range::<Continuous>(
            "x", VariableRange::bounded(-3.0, 7.0),
        ).unwrap();
        (model, x)
    }

    fn row(index: usize, coefficient: f64) -> LinearInequality {
        LinearInequality::less_equal(Linear::new(vec![LinearMonomial::new(coefficient, index)], 2.0), 5.0)
    }

    #[test]
    fn revisions_track_structure_not_solutions_or_returned_snapshot_edits() {
        let (mut model, x) = model();
        let mut first = model.linear_model_snapshot().unwrap();
        assert_eq!(first.revision.sequence, 0);
        model.set_solution(&[2.0]);
        model.require_linear_model_revision(first.revision).unwrap();
        first.variables[0].upper_bound = Some(999.0);
        assert_eq!(model.linear_model_snapshot().unwrap().variables[0].upper_bound, Some(7.0));
        model.set_variable_range_by_index(x, VariableRange::bounded(-3.0, 4.0)).unwrap();
        assert!(model.require_linear_model_revision(first.revision).is_err());
        let bounded = model.linear_model_snapshot().unwrap();
        assert_eq!(bounded.revision.sequence, 1);
        model.objective_mut().sub_objectives.push(SubObjective::minimize(Linear::new(vec![], 6.0), "constant"));
        assert_eq!(model.linear_model_snapshot().unwrap().revision.sequence, 2);
        model.add_le_constraint(&[(x, 2.0)], 3.0, "extra").unwrap();
        assert_eq!(model.linear_model_snapshot().unwrap().revision.sequence, 3);
    }

    #[test]
    fn foreign_revision_is_rejected_even_for_identical_empty_models() {
        let first = MetaModel::<f64>::new("same");
        let second = MetaModel::<f64>::new("same");
        assert!(second.require_linear_model_revision(first.linear_model_snapshot().unwrap().revision).is_err());
    }

    #[test]
    fn dynamic_rows_are_canonical_idempotent_and_normalize_constants() {
        let (mut model, x) = model();
        let variable = model.tokens()[x].id();
        let inequality = LinearInequality::less_equal(Linear::new(vec![
            LinearMonomial::new(2.0, x), LinearMonomial::new(-1.0, x),
            LinearMonomial::new(0.0, x),
        ], 2.0), 5.0);
        model.register_linear_row("capacity", inequality).unwrap();
        let first = model.linear_model_snapshot().unwrap();
        model.register_linear_row("capacity", row(x, 1.0)).unwrap();
        assert_eq!(model.linear_model_snapshot().unwrap(), first);
        assert_eq!(first.rows[0].id, "capacity");
        assert_eq!(first.rows[0].coefficients, BTreeMap::from([(variable, 1.0)]));
        assert_eq!(first.rows[0].rhs, 3.0);
        assert!(model.register_linear_row("capacity", row(x, 2.0)).is_err());
        assert_eq!(model.num_constraints(), 1);
    }

    #[test]
    fn removed_row_identity_cannot_be_rebound_to_an_identical_same_name_row() {
        let (mut model, x) = model();
        model.register_linear_row("capacity", row(x, 1.0)).unwrap();
        let first = model.linear_model_snapshot().unwrap();
        model.retain_constraints(|_| false);
        model.add_inequality(row(x, 1.0), "capacity").unwrap();
        let next = model.linear_model_snapshot().unwrap();
        assert_ne!(first.rows[0].id, next.rows[0].id);
        assert!(model.register_linear_row("capacity", row(x, 1.0)).is_err());
    }

    #[test]
    fn ordinary_row_identity_survives_other_row_removal_but_not_recreation() {
        let (mut model, x) = model();
        model.add_le_constraint(&[(x, 1.0)], 1.0, "a").unwrap();
        model.add_le_constraint(&[(x, 1.0)], 2.0, "b").unwrap();
        let first = model.linear_model_snapshot().unwrap();
        model.retain_constraints(|row| row.name == "b");
        assert_eq!(model.linear_model_snapshot().unwrap().rows[0].id, first.rows[1].id);
        model.retain_constraints(|_| false);
        model.add_le_constraint(&[(x, 1.0)], 2.0, "b").unwrap();
        assert_ne!(model.linear_model_snapshot().unwrap().rows[0].id, first.rows[1].id);
    }

    #[test]
    fn invalid_dynamic_rows_fail_atomically_including_zero_unknown_terms() {
        let (mut model, x) = model();
        for (id, inequality) in [
            ("", row(x, 1.0)), ("model-local-row:reserved", row(x, 1.0)),
            ("unknown", row(999, 0.0)), ("nan", row(x, f64::NAN)),
            ("infinite", row(x, f64::INFINITY)),
            ("rhs", LinearInequality::equal(Linear::new(vec![], 0.0), f64::INFINITY)),
            ("overflow", LinearInequality::equal(Linear::new(vec![
                LinearMonomial::new(f64::MAX, x), LinearMonomial::new(f64::MAX, x),
            ], 0.0), 0.0)),
        ] {
            assert!(model.register_linear_row(id, inequality).is_err(), "{id}");
        }
        assert_eq!(model.num_constraints(), 0);
    }

    #[test]
    fn snapshot_rejects_nonfinite_source_rows_objectives_and_bounds() {
        let (mut model, x) = model();
        model.add_le_constraint(&[(x, f64::NAN)], 1.0, "bad").unwrap();
        assert!(model.linear_model_snapshot().is_err());
        model.retain_constraints(|_| false);
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![], f64::INFINITY), "bad"));
        assert!(model.linear_model_snapshot().is_err());
        model.objective_mut().sub_objectives.clear();
        model.set_variable_range_by_index(x, VariableRange::bounded(f64::NAN, 7.0)).unwrap();
        assert!(model.linear_model_snapshot().is_err());
    }

    #[test]
    fn weighted_mixed_objectives_and_constant_are_normalized_once() {
        let (mut model, x) = model();
        model.set_objective_category(ObjectiveCategory::Maximum);
        model.add_sub_objective(SubObjective::new_with_weight(ObjectiveCategory::Minimum,
            Linear::new(vec![LinearMonomial::new(2.0, x)], 7.0), "cost", 3.0));
        model.add_sub_objective(SubObjective::new_with_weight(ObjectiveCategory::Maximum,
            Linear::new(vec![LinearMonomial::new(4.0, x)], 5.0), "reward", 2.0));
        let snapshot = model.linear_model_snapshot().unwrap();
        assert_eq!(snapshot.objective[&model.tokens()[x].id()], -2.0);
        assert_eq!(snapshot.objective_constant, 11.0);
        assert_eq!(snapshot.objective_category, ObjectiveCategory::Maximum);
    }

    #[test]
    fn deferred_and_eager_snapshots_have_identical_full_helper_semantics() {
        let (mut model, x) = model();
        model.add_symbol(Arc::new(AbsFunction::new(991_001, "absolute",
            Linear::new(vec![LinearMonomial::new(1.0, x)], 0.0)))).unwrap();
        let first = model.linear_model_snapshot().unwrap();
        assert!(first.variables.len() > 1);
        assert!(!first.rows.is_empty());
        model.set_function_expansion_policy(FunctionExpansionPolicy::DeferredNativeFirst);
        assert!(!model.try_to_mechanism_model().unwrap().as_basic().deferred_functions().is_empty());
        assert_eq!(model.linear_model_snapshot().unwrap(), first);
        model.set_function_expansion_policy(FunctionExpansionPolicy::Auto);
        assert_eq!(model.linear_model_snapshot().unwrap(), first);
    }

    #[test]
    fn failed_transaction_restores_registration_but_never_reuses_a_published_revision() {
        let (mut model, x) = model();
        let first = model.linear_model_snapshot().unwrap();
        let mut transient = first.revision;
        let result: std::result::Result<(), ()> = model.transaction(|model| {
            model.register_linear_row("temporary", row(x, 1.0)).unwrap();
            transient = model.linear_model_snapshot().unwrap().revision;
            Err(())
        });
        assert!(result.is_err());
        assert!(model.require_linear_model_revision(transient).is_err());
        let restored = model.linear_model_snapshot().unwrap();
        assert!(restored.revision.sequence > transient.sequence);
        assert!(restored.rows.is_empty());
        model.register_linear_row("temporary", row(x, 2.0)).unwrap();
        assert_eq!(model.linear_model_snapshot().unwrap().rows[0].id, "temporary");
    }

    #[test]
    fn decimal_snapshot_keeps_digits_beyond_f64_and_detects_sub_f64_edits() {
        let mut model = MetaModel::<BigDecimal>::new("decimal");
        let x = model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(0.0, 1.0)).unwrap();
        let precise = BigDecimal::from_str("9007199254740993.00000000000000000001").unwrap();
        model.set_variable_range_by_index(x, VariableRange::bounded(BigDecimal::zero(), precise.clone())).unwrap();
        model.register_linear_row("precise", LinearInequality::equal(Linear::new(vec![
            LinearMonomial::new(precise.clone(), x),
        ], BigDecimal::zero()), precise.clone())).unwrap();
        let first = model.linear_model_snapshot().unwrap();
        assert_eq!(first.rows[0].coefficients[&model.tokens()[x].id()], precise);
        assert_eq!(first.variables[0].upper_bound, Some(precise.clone()));
        let changed = precise + BigDecimal::from_str("0.00000000000000000001").unwrap();
        model.set_variable_range_by_index(x, VariableRange::bounded(BigDecimal::zero(), changed)).unwrap();
        assert!(model.require_linear_model_revision(first.revision).is_err());
    }

    #[test]
    fn rational_snapshot_keeps_exact_thirds_without_solver_conversion() {
        let mut model = MetaModel::<BigRational>::new("rational");
        let x = model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(0.0, 1.0)).unwrap();
        let third = BigRational::new(1.into(), 3.into());
        model.register_linear_row("third", LinearInequality::equal(Linear::new(vec![
            LinearMonomial::new(third.clone(), x),
        ], BigRational::zero()), third.clone())).unwrap();
        let snapshot = model.linear_model_snapshot().unwrap();
        assert_eq!(snapshot.rows[0].coefficients[&model.tokens()[x].id()], third);
        assert_eq!(snapshot.rows[0].rhs, third);
    }

    #[test]
    fn objective_builder_and_native_inputs_preserve_weighted_constants() {
        let mut model = MetaModel::<BigDecimal>::new("builder");
        let constant = BigDecimal::from_str("7.00000000000000000001").unwrap();
        model.set_linear_objective_input(LinearExpressionBuilder::with_constant(constant.clone())
            .maximize("reward").weight(BigDecimal::from(3)));
        let snapshot = model.linear_model_snapshot().unwrap();
        assert_eq!(snapshot.objective_constant, -constant * BigDecimal::from(3));
        let mut model = MetaModel::<f64>::new("minimum");
        model.add_linear_objective_input(LinearExpressionBuilder::with_constant(5.0).minimize("constant"));
        assert_eq!(model.linear_model_snapshot().unwrap().objective_constant, 5.0);
    }

    #[test]
    fn full_solution_validation_checks_implicit_domains_rows_and_native_integrality() {
        let mut model = MetaModel::<f64>::new("validate");
        let x = model.register_auto_variable::<crate::variable::Binary>("x").unwrap();
        let id = model.tokens()[x].id();
        model.add_ge_constraint(&[(x, 1.0)], 1.0, "required").unwrap();
        let snapshot = model.linear_model_snapshot().unwrap();
        snapshot.validate_solution(&BTreeMap::from([(id, 1.0)]), &1e-8).unwrap();
        for values in [BTreeMap::new(), BTreeMap::from([(id, 0.0)]),
            BTreeMap::from([(id, 0.5)]), BTreeMap::from([(id, 2.0)]),
            BTreeMap::from([(id, f64::INFINITY)])]
        {
            assert!(snapshot.validate_solution(&values, &1e-8).is_err());
        }
        for tolerance in [-1.0, 0.5, f64::NAN, f64::INFINITY] {
            assert!(snapshot.validate_solution(&BTreeMap::from([(id, 1.0)]), &tolerance).is_err());
        }
        assert!(BigDecimal::from_str("-2.00000000000000000001").unwrap()
            .is_integral_value(&BigDecimal::from_str("0.00000000000000000002").unwrap()));
        assert!(!BigRational::new(1.into(), 3.into()).is_integral_value(&BigRational::zero()));
    }

    #[test]
    fn float_solution_validation_keeps_cancelling_row_residuals_in_both_float_domains() {
        macro_rules! verify {
            ($value:ty, $large:expr) => {{
                let mut model = MetaModel::<$value>::new("exact_float_row");
                let mut terms = Vec::new();
                for (index, coefficient) in [$large, 1.0, -$large].into_iter().enumerate() {
                    let variable = model.register_auto_variable_with_range::<Continuous>(
                        &format!("x_{index}"), VariableRange::bounded(1.0, 1.0)).unwrap();
                    terms.push(LinearMonomial::new(coefficient, variable));
                }
                model.register_linear_row("cancelled", LinearInequality::less_equal(Linear::new(terms, 0.0), 0.0)).unwrap();
                let mut snapshot = model.linear_model_snapshot().unwrap();
                let values = snapshot.variables.iter().map(|state| (state.variable, 1.0)).collect();
                assert!(snapshot.validate_solution(&values, &1e-7).is_err());
                snapshot.rows[0].rhs = 1.0;
                snapshot.validate_solution(&values, &1e-7).unwrap();
            }};
        }
        verify!(f64, 1e16_f64);
        verify!(f32, 1e8_f32);
    }

    #[test]
    fn objective_boundary_keeps_exact_cancellation_constants_and_directed_bounds() {
        let objective = LinearObjectiveEvaluationFlt64::new([(1e16, 1.0), (-1.0, 1.0), (-1e16, 1.0)], 7.0).unwrap();
        objective.validate_reported(Some(-1.0), 1e-7).unwrap();
        assert!(objective.validate_reported(Some(0.0), 1e-7).is_err());
        assert_eq!(objective.variable_value().unwrap(), -1.0);
        assert_eq!(objective.total_value().unwrap(), 6.0);
        assert_eq!(objective.lower_bound().unwrap(), 6.0);
        let between = LinearObjectiveEvaluationFlt64::new([(1e16, 1.0)], -1.0).unwrap();
        assert_eq!(between.total_value().unwrap(), 1e16);
        assert_eq!(between.lower_bound().unwrap(), 1e16_f64.next_down());
        let same = LinearObjectiveEvaluationFlt64::new([(6.0, 1.0)], 0.0).unwrap();
        objective.validate_total(&same, 0.0).unwrap();
    }

    #[test]
    fn compilation_records_real_row_origins_signs_and_separates_objective_constant() {
        let (mut model, x) = model();
        for (id, relation) in [("less", ConstraintRelation::LessEqual), ("equal", ConstraintRelation::Equal), ("greater", ConstraintRelation::GreaterEqual)] {
            model.register_linear_row(id, LinearInequality::new(Linear::new(vec![LinearMonomial::new(2.0, x)], 3.0), relation, 5.0)).unwrap();
        }
        model.set_linear_objective_input(LinearExpressionBuilder::with_constant(7.0).term(x, 11.0).minimize("cost"));
        let compilation = model.linear_model_snapshot().unwrap().compile_flt64().unwrap();
        assert_eq!(compilation.rows.iter().map(|row| (row.source.as_str(), row.multiplier)).collect::<Vec<_>>(),
            vec![("less", 1), ("equal", 1), ("equal", -1), ("greater", -1)]);
        assert_eq!(compilation.model.b, vec![2.0, 2.0, -2.0, -2.0]);
        assert_eq!(compilation.model.c, vec![11.0]);
        assert_eq!(compilation.objective_constant, 7.0);
        assert_eq!(compilation.model.A.rows[2].entries, vec![(0, -2.0)]);
    }

    #[test]
    fn relaxation_intersects_implicit_and_explicit_domains_instead_of_dropping_type_bounds() {
        let mut state = LinearVariableState { variable: crate::variable::new_standalone_id(), name: "implicit_binary".into(),
            var_type: VariableType::Binary, lower_bound: Some(-5.0), upper_bound: Some(7.0) };
        let relaxed = state.relaxed().unwrap();
        assert_eq!(relaxed.var_type, VariableType::Continuous);
        assert_eq!((relaxed.lower_bound, relaxed.upper_bound), (Some(0.0), Some(1.0)));
        state.lower_bound = Some(0.25);
        state.upper_bound = Some(0.75);
        let relaxed = state.relaxed().unwrap();
        assert_eq!((relaxed.lower_bound, relaxed.upper_bound), (Some(0.25), Some(0.75)));
        state.lower_bound = Some(2.0);
        assert!(state.relaxed().is_err());
        state.lower_bound = Some(f64::NAN);
        assert!(state.relaxed().is_err());
        state.lower_bound = None;
        state.upper_bound = None;
        state.var_type = VariableType::UInteger;
        assert_eq!(state.relaxed().unwrap().lower_bound, Some(0.0));
    }
}
