//! 原数值域与 solver 边界，业务比较不使用 solver 容差。
//! Native-domain and solver boundaries; business comparisons never use solver tolerances.

use ospf_rust_quantities::{Quantity, unit::{Unit, UnitConversionValue}};
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use crate::domain::task::SchedulingSolverValueAdapter;
use crate::{GanttError, GanttResult};

/// 同时保留原域精确值与归一化 solver 系数。 / Retains the native value and normalized solver coefficient together.
#[derive(Debug, Clone)]
pub struct ActivityValue<V> {
    /// 归一化原域值，业务复核使用它 / Normalized native value used by business validation
    pub value: V,
    /// 有限 solver 系数，不据此声称原域无损 / Finite solver coefficient, not a claim of native-domain losslessness
    pub solver: f64,
}

pub(crate) fn input(message: impl Into<String>) -> GanttError {
    GanttError::InvalidInput { message: message.into() }
}

pub(crate) fn conversion(message: impl Into<String>) -> GanttError {
    GanttError::Conversion { message: message.into() }
}

pub(crate) fn zero<V: SolveValue>() -> GanttResult<V> {
    V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| conversion(error.to_string()))
}

pub(crate) fn scalar<V: SolveValue>(value: f64) -> GanttResult<V> {
    V::from_f64_with_policy(value, SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| conversion(error.to_string()))
}

/// 规范排序后确定性求和；f64 使用补偿归约，不依赖事件输入排列。
/// Deterministic canonical summation; f64 uses compensated reduction independent of event input order.
pub fn sum_activity_values<V>(values: impl IntoIterator<Item = V>) -> GanttResult<V>
where
    V: SolveValue + UnitConversionValue,
{
    let mut values = values.into_iter().collect::<Vec<_>>();
    for value in &values {
        let finite = value.to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| conversion(error.to_string()))?;
        if !finite.is_finite() { return Err(conversion("non-finite activity summand")); }
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    if std::any::TypeId::of::<V>() == std::any::TypeId::of::<f64>() {
        let mut sum = 0.0_f64;
        let mut correction = 0.0_f64;
        for value in values {
            let value = value.to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| conversion(error.to_string()))?;
            let next = sum + value;
            let delta = if sum.abs() >= value.abs() { (sum - next) + value } else { (value - next) + sum };
            correction += delta;
            if !next.is_finite() || !delta.is_finite() || !correction.is_finite() {
                return Err(conversion("activity compensated summation overflow"));
            }
            sum = next;
        }
        return scalar(sum + correction);
    }
    let sum = values.into_iter().fold(zero::<V>()?, |sum, value| sum + value);
    let finite = sum.to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| conversion(error.to_string()))?;
    if !finite.is_finite() { return Err(conversion("activity native summation overflow")); }
    Ok(sum)
}

/// 拒绝仿射单位，允许合法线性温差。 / Reject affine units while allowing linear temperature differences.
pub fn validate_activity_unit(unit: &Unit) -> GanttResult<()> {
    if !unit.is_linear() {
        return Err(input("activity linear model does not support affine units"));
    }
    if unit.scale().value() <= 0_i32 {
        return Err(input("activity unit scale must be positive"));
    }
    Ok(())
}

/// 检查原值与自定义 adapter 输出，精确拒绝非零下溢。
/// Check native values and custom adapter output, rejecting nonzero underflow exactly.
pub fn activity_value<V: SolveValue>(
    value: V,
    adapter: &dyn SchedulingSolverValueAdapter<V>,
) -> GanttResult<ActivityValue<V>> {
    let native = value.to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| conversion(error.to_string()))?;
    let solver = adapter.try_from_value(&value)?;
    let is_zero = value == zero::<V>()?;
    if !native.is_finite() || !solver.is_finite() {
        return Err(conversion("activity values must be finite"));
    }
    if (!is_zero && (solver == 0.0 || native == 0.0)) || (is_zero && solver != 0.0) {
        return Err(conversion("activity conversion changed exact zero/nonzero status"));
    }
    Ok(ActivityValue { value, solver })
}

/// 先检查两端单位，再在原域换算；不丢弃原域精度。
/// Validate both units before native-domain conversion, preserving the native value.
pub fn normalize_activity_value<V>(
    quantity: &Quantity<V, Unit>,
    target: &Unit,
    adapter: &dyn SchedulingSolverValueAdapter<V>,
) -> GanttResult<ActivityValue<V>>
where
    V: SolveValue + UnitConversionValue,
{
    validate_activity_unit(&quantity.unit)?;
    validate_activity_unit(target)?;
    let normalized = if quantity.unit == *target {
        quantity.value.clone()
    } else {
        let factor = quantity.unit.conversion_factor_to(target)
            .ok_or_else(|| conversion("incompatible activity units"))?;
        let factor = V::from_decimal(&factor)
            .ok_or_else(|| conversion("activity unit factor cannot be represented"))?;
        if factor.partial_cmp(&zero::<V>()?) != Some(std::cmp::Ordering::Greater) {
            return Err(conversion("activity unit factor must remain positive and nonzero"));
        }
        // 原值或换算因子可超出 f64；只在归一化完成后进入 solver 边界。
        // Native values or factors may exceed f64; cross the solver boundary only after normalization.
        quantity.value.clone() * factor
    };
    if quantity.value != zero::<V>()? && normalized == zero::<V>()? {
        return Err(conversion("activity unit conversion underflowed to zero"));
    }
    activity_value(normalized, adapter)
}

/// 生产提取使用的可失败反向边界，同样检查非零下溢。
/// Fallible reverse boundary for production extraction, also checking nonzero underflow.
pub fn activity_value_from_solver<V: SolveValue>(
    value: f64,
    adapter: &dyn SchedulingSolverValueAdapter<V>,
) -> GanttResult<V> {
    if !value.is_finite() {
        return Err(conversion("activity solver value must be finite"));
    }
    let converted = adapter.try_into_value(value)?;
    let checked = activity_value(converted, adapter)?;
    if (value == 0.0) != (checked.value == zero::<V>()?) {
        return Err(conversion("activity reverse conversion changed exact zero/nonzero status"));
    }
    Ok(checked.value)
}
