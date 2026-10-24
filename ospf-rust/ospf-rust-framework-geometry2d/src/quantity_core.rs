//! 二维装箱量纲运算核心 / Core quantity operations for 2D packing
//!
//! 二维排样的领域代码需要**非空、泛型、失败可传播**的量值运算：量纲不兼容必须以错误暴露，
//! 不能中止进程。本文件集中提供这套操作符，泛型于 `V` 且保持编译时单位 `U`。
//! 2D layout domain code needs non-null, generic and failure-propagating quantity operations: an
//! incompatible dimension must surface as an error rather than aborting the process. This file provides
//! that set, generic over `V` while keeping the compile-time unit `U`.

use std::cmp::Ordering;

use ospf_rust_base::error::{Error, Ret};
use ospf_rust_math::algebra::concept::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

// ============================================================================
// 量纲判断 / Dimension predicates
// ============================================================================

/// 判断两个量是否同量纲 / Whether two quantities share the same dimension
pub fn is_same_dimension<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> bool
where
    U: UnitTrait,
{
    lhs.unit.dimension_symbol() == rhs.unit.dimension_symbol()
}

// ============================================================================
// 基础算术 / Basic arithmetic
// ============================================================================

/// 量值乘法（产生新量纲）/ Multiply two quantities, producing a new dimension
///
/// 二维面积与体积由此得到，例如 `width × height` 直接得到面积量。量纲按基础幂次相加合成，
/// 因此不会出现「长度平方被算成立方」的问题。
/// 2D areas and volumes come from here, e.g. `width × height` yields an area quantity directly. Dimensions
/// compose by adding base powers, so a squared length is never computed as a cube.
pub fn quantity_multiply<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Quantity<V, Unit>
where
    V: Field + Clone,
    U: UnitTrait + CTUnit,
{
    lhs.to_runtime() * rhs.to_runtime()
}

/// 同量纲加法（失败可传播）/ Dimension-safe addition
///
/// 量纲不兼容时返回错误，不 panic。
/// Returns an error instead of panicking on incompatible dimensions.
pub fn quantity_plus_safe<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Ret<Quantity<V, U>>
where
    V: Field + Clone,
    U: UnitTrait + Clone,
{
    if !is_same_dimension(lhs, rhs) {
        return Err(Box::new(DimensionMismatch::new(
            lhs.unit.dimension_symbol(),
            rhs.unit.dimension_symbol(),
            "addition",
        )));
    }
    Ok(Quantity::new(
        lhs.value.clone() + rhs.value.clone(),
        lhs.unit.clone(),
    ))
}

/// 同量纲减法（失败可传播）/ Dimension-safe subtraction
pub fn quantity_minus_safe<V, U>(
    lhs: &Quantity<V, U>,
    rhs: &Quantity<V, U>,
) -> Ret<Quantity<V, U>>
where
    V: Field + Clone,
    U: UnitTrait + Clone,
{
    if !is_same_dimension(lhs, rhs) {
        return Err(Box::new(DimensionMismatch::new(
            lhs.unit.dimension_symbol(),
            rhs.unit.dimension_symbol(),
            "subtraction",
        )));
    }
    Ok(Quantity::new(
        lhs.value.clone() - rhs.value.clone(),
        lhs.unit.clone(),
    ))
}

/// 无量纲缩放 / Scale by a dimensionless factor
pub fn quantity_scaled_by<V, U>(value: &Quantity<V, U>, factor: &V) -> Quantity<V, U>
where
    V: Field + Clone,
    U: UnitTrait + Clone,
{
    Quantity::new(value.value.clone() * factor.clone(), value.unit.clone())
}

/// 以样本单位构造零值 / Build a zero with the sample's unit
pub fn quantity_zero_of<V, U>(sample: &Quantity<V, U>) -> Quantity<V, U>
where
    V: Field + Clone,
    U: UnitTrait + Clone,
{
    Quantity::new(V::zero(), sample.unit.clone())
}

/// 以样本单位构造标量量值 / Build a scaled quantity with the sample's unit
pub fn quantity_of<V, U>(sample: &Quantity<V, U>, scaler: V) -> Quantity<V, U>
where
    V: Clone,
    U: UnitTrait + Clone,
{
    Quantity::new(scaler, sample.unit.clone())
}

// ============================================================================
// 比较 / Comparison
// ============================================================================

/// 比较两个同量纲量，量纲不一致返回 `None` / Compare same-dimension quantities; `None` on mismatch
pub fn quantity_ord_or_null<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Option<Ordering>
where
    V: PartialOrd,
    U: UnitTrait,
{
    if !is_same_dimension(lhs, rhs) {
        return None;
    }
    lhs.value.partial_cmp(&rhs.value)
}

/// 比较两个同量纲量，量纲不一致返回错误 / Compare same-dimension quantities, failing on mismatch
pub fn quantity_ord_safe<V, U>(
    lhs: &Quantity<V, U>,
    rhs: &Quantity<V, U>,
    axis: &str,
) -> Ret<Ordering>
where
    V: PartialOrd,
    U: UnitTrait,
{
    quantity_ord_or_null(lhs, rhs).ok_or_else(|| {
        Box::new(DimensionMismatch::new(
            lhs.unit.dimension_symbol(),
            rhs.unit.dimension_symbol(),
            axis,
        )) as Box<dyn Error>
    })
}

/// 同量纲取较大者 / Take the larger of two same-dimension quantities
pub fn quantity_max_safe<V, U>(
    lhs: &Quantity<V, U>,
    rhs: &Quantity<V, U>,
    axis: &str,
) -> Ret<Quantity<V, U>>
where
    V: PartialOrd + Clone,
    U: UnitTrait + Clone,
{
    match quantity_ord_safe(lhs, rhs, axis)? {
        Ordering::Less => Ok(rhs.clone()),
        _ => Ok(lhs.clone()),
    }
}

/// 同量纲取较小者 / Take the smaller of two same-dimension quantities
pub fn quantity_min_safe<V, U>(
    lhs: &Quantity<V, U>,
    rhs: &Quantity<V, U>,
    axis: &str,
) -> Ret<Quantity<V, U>>
where
    V: PartialOrd + Clone,
    U: UnitTrait + Clone,
{
    match quantity_ord_safe(lhs, rhs, axis)? {
        Ordering::Greater => Ok(rhs.clone()),
        _ => Ok(lhs.clone()),
    }
}

/// 宽松取较大者（量纲不一致时取左值）/ Lenient max, taking the left value on dimension mismatch
pub fn quantity_max_of<V, U>(lhs: Quantity<V, U>, rhs: Quantity<V, U>) -> Quantity<V, U>
where
    V: PartialOrd,
    U: UnitTrait,
{
    match quantity_ord_or_null(&lhs, &rhs) {
        Some(Ordering::Less) => rhs,
        _ => lhs,
    }
}

/// 严格大于 / Strictly greater
pub fn quantity_greater<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> bool
where
    V: PartialOrd,
    U: UnitTrait,
{
    quantity_ord_or_null(lhs, rhs) == Some(Ordering::Greater)
}

/// 严格小于 / Strictly less
pub fn quantity_less<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> bool
where
    V: PartialOrd,
    U: UnitTrait,
{
    quantity_ord_or_null(lhs, rhs) == Some(Ordering::Less)
}

/// 大于或等于 / Greater or equal
pub fn quantity_greater_or_equal<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> bool
where
    V: PartialOrd,
    U: UnitTrait,
{
    matches!(
        quantity_ord_or_null(lhs, rhs),
        Some(Ordering::Greater) | Some(Ordering::Equal)
    )
}

/// 小于或等于 / Less or equal
pub fn quantity_less_or_equal<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> bool
where
    V: PartialOrd,
    U: UnitTrait,
{
    matches!(
        quantity_ord_or_null(lhs, rhs),
        Some(Ordering::Less) | Some(Ordering::Equal)
    )
}

/// 量值相等（同量纲且数值相等）/ Quantity equality (same dimension and equal value)
pub fn quantity_equal<V1, V2, U1, U2>(
    lhs: &Quantity<V1, U1>,
    rhs: &Quantity<V2, U2>,
    compare: impl Fn(&V1, &V2) -> bool,
) -> bool
where
    U1: UnitTrait,
    U2: UnitTrait,
{
    lhs.unit.dimension_symbol() == rhs.unit.dimension_symbol() && compare(&lhs.value, &rhs.value)
}

// ============================================================================
// 正负判断 / Sign predicates
// ============================================================================

/// 是否为正 / Whether strictly positive
pub fn quantity_is_positive<V, U>(value: &Quantity<V, U>) -> bool
where
    V: PartialOrd + num_traits::Zero,
    U: UnitTrait,
{
    value.value > V::zero()
}

/// 是否为非正 / Whether non-positive
pub fn quantity_is_non_positive<V, U>(value: &Quantity<V, U>) -> bool
where
    V: PartialOrd + num_traits::Zero,
    U: UnitTrait,
{
    value.value <= V::zero()
}

// ============================================================================
// 聚合与排序 / Aggregation and ordering
// ============================================================================

/// 求和；空序列或量纲不一致返回 `None` / Sum; `None` on an empty sequence or dimension mismatch
pub fn quantity_sum<V, U>(values: impl IntoIterator<Item = Quantity<V, U>>) -> Option<Quantity<V, U>>
where
    V: Field + Clone,
    U: UnitTrait + Clone,
{
    let mut iter = values.into_iter();
    let first = iter.next()?;
    let mut total = first.value;
    let unit = first.unit;
    for value in iter {
        if value.unit.dimension_symbol() != unit.dimension_symbol() {
            return None;
        }
        total = total + value.value;
    }
    Some(Quantity::new(total, unit))
}

/// 按选择器求和 / Sum with a selector
pub fn sum_of_quantity<T, V, U>(
    values: impl IntoIterator<Item = T>,
    selector: impl Fn(&T) -> Quantity<V, U>,
) -> Option<Quantity<V, U>>
where
    V: Field + Clone,
    U: UnitTrait + Clone,
{
    quantity_sum(values.into_iter().map(|value| selector(&value)))
}

/// 按量值升序排序 / Sort ascending by quantity
pub fn sorted_by_quantity<T, V, U>(
    values: impl IntoIterator<Item = T>,
    selector: impl Fn(&T) -> Quantity<V, U>,
) -> Vec<T>
where
    V: PartialOrd,
    U: UnitTrait,
{
    let mut items: Vec<T> = values.into_iter().collect();
    items.sort_by(|lhs, rhs| {
        quantity_ord_or_null(&selector(lhs), &selector(rhs)).unwrap_or(Ordering::Equal)
    });
    items
}

/// 按量值降序排序 / Sort descending by quantity
pub fn sorted_by_descending_quantity<T, V, U>(
    values: impl IntoIterator<Item = T>,
    selector: impl Fn(&T) -> Quantity<V, U>,
) -> Vec<T>
where
    V: PartialOrd,
    U: UnitTrait,
{
    let mut items: Vec<T> = values.into_iter().collect();
    items.sort_by(|lhs, rhs| {
        quantity_ord_or_null(&selector(rhs), &selector(lhs)).unwrap_or(Ordering::Equal)
    });
    items
}

// ============================================================================
// 计数与整数缩放 / Fitting counts and integer scaling
// ============================================================================

/// 计算容器内可容纳多少个单元 / Count how many cells fit inside a container
///
/// 单元尺寸非正或量纲不一致时返回 0，避免除零与跨量纲比较。
/// Returns 0 when the cell size is non-positive or the dimensions differ, avoiding division by zero and
/// cross-dimension comparison.
pub fn count_fitting<V, U>(cell: &Quantity<V, U>, container: &Quantity<V, U>) -> usize
where
    V: Field + Clone + PartialOrd + num_traits::Zero + num_traits::ToPrimitive,
    U: UnitTrait,
{
    if !is_same_dimension(cell, container) {
        return 0;
    }
    if !quantity_is_positive(cell) {
        return 0;
    }
    let ratio = container.value.clone() / cell.value.clone();
    match ratio.to_f64() {
        Some(ratio) if ratio.is_finite() && ratio > 0.0 => ratio.floor() as usize,
        _ => 0,
    }
}

/// 按整数倍缩放 / Scale by an integer multiple
pub fn scaled_quantity<V, U>(cell: &Quantity<V, U>, times: usize) -> Quantity<V, U>
where
    V: Field + Clone,
    U: UnitTrait + Clone,
{
    let factor = if times == 0 {
        V::zero()
    } else {
        let mut accumulated = V::one();
        for _ in 1..times {
            accumulated = accumulated + V::one();
        }
        accumulated
    };
    Quantity::new(cell.value.clone() * factor, cell.unit.clone())
}

// ============================================================================
// 常量构造 / Constant construction
// ============================================================================

/// 构造数值 `1/2` / Build the value `1/2`
///
/// 通过 `Field` 的加减运算构造，避免依赖 `From<f64>` 的额外约束。
/// Built with `Field` addition so no extra `From<f64>` bound is required.
pub fn half_value<V>() -> V
where
    V: Field + Clone,
{
    let one = V::one();
    let two = one.clone() + one;
    V::one() / two
}

/// 构造数值 `2` / Build the value `2`
pub fn two_value<V>() -> V
where
    V: Field + Clone,
{
    V::one() + V::one()
}

/// 构造数值 `π` / Build the value `π`
///
/// 供圆形面积使用，泛型于任何满足 `Float` 的数值类型。
/// Used by circle areas, generic over any numeric type implementing `Float`.
pub fn pi_value<V>() -> V
where
    V: num_traits::Float,
{
    V::from(std::f64::consts::PI).unwrap_or_else(V::one)
}

// ============================================================================
// 量纲不匹配错误 / Dimension mismatch error
// ============================================================================

/// 量纲不匹配错误 / Dimension mismatch error
#[derive(Debug, Clone)]
pub struct DimensionMismatch {
    expected: String,
    actual: String,
    operation: String,
}

impl DimensionMismatch {
    /// 创建量纲不匹配错误 / Create a dimension mismatch error
    pub fn new(expected: impl Into<String>, actual: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            expected: expected.into(),
            actual: actual.into(),
            operation: operation.into(),
        }
    }
}

impl std::fmt::Display for DimensionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "量纲不匹配: {} 与 {}（{}）/ Dimension mismatch: {} and {} ({})",
            self.expected, self.actual, self.operation, self.expected, self.actual, self.operation
        )
    }
}

impl std::error::Error for DimensionMismatch {}

impl Error for DimensionMismatch {
    fn code(&self) -> ospf_rust_base::error::ErrorCode {
        ospf_rust_base::error::ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}
