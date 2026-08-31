//e! Quantity - 物理量
//! Quantity - Physical quantity
//!
//! 统一的物理量类型，支持编译时和运行时两种模式：
//! Unified quantity type, supporting both compile-time and runtime modes:
//!
//! - `Quantity<V, Unit>`: 运行时物理量，单位在运行时确定
//! - `Quantity<V, U: CTUnit>`: 编译时物理量，单位类型在编译时确定（零成本抽象）
//!
//! # 核心特性 / Key Features
//! - 泛型值类型支持
//! - 编译时量纲检查（CTUnit）
//! - 运行时单位转换（Unit）
//! - 类型安全的算术运算

use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Sub};
use bigdecimal::BigDecimal;
use ospf_rust_base::{ErrorPosition, Ret, error};
use ospf_rust_math::operator::abs::Abs;
use ospf_rust_math::operator::reciprocal::Reciprocal;
use ospf_rust_math::operator::tolerance::{Tolerance, TolerancedEq, TolerancedOrd};
use crate::dimension::DerivedQuantity;
use crate::dimension::derived_quantity::{CTDerivedQuantity, SameDerivedDimension};
use crate::error::{DimensionMismatchError, UnitConversionError};
use crate::unit::concept::UnitTrait;
use crate::unit::{
    CTUnit, CTUnitDiv, CTUnitMul, CTUnitReciprocal, Unit, UnitConversionRule, UnitSystem,
};
use crate::unit::conversion_value::{UnitConversionCalculation, UnitConversionValue};

// ============================================================================
// Quantity - 统一的物理量结构体 / Unified quantity struct
// ============================================================================

/// Quantity - 物理量
/// Quantity - Physical quantity
///
/// 统一的物理量类型，通过 `U` 类型参数区分编译时和运行时模式。
/// Unified quantity type, distinguished by `U` type parameter for compile-time and runtime modes.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型，通常是 `BigDecimal` 或 `f64`
/// - `U`: 单位类型，实现 `UnitTrait`
///   - `Unit`: 运行时单位
///   - `Meter` 等编译时单位类型（实现 `CTUnit`）
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_quantities::quantity::Quantity;
/// use ospf_rust_quantities::unit::derived::Meter;
/// use ospf_rust_quantities::unit::{Unit, CTUnit};
/// use bigdecimal::BigDecimal;
///
/// // 运行时物理量
/// let rt_q: Quantity<BigDecimal, Unit> = Quantity::new(
///     BigDecimal::from(10),
///     Meter::INSTANT.clone()
/// );
///
/// // 编译时物理量（零成本抽象，使用 new_ct）
/// let ct_q: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
/// ```
#[derive(Debug, Clone)]
pub struct Quantity<V, U: UnitTrait> {
    /// 物理量的值
    /// Value of the physical quantity
    pub value: V,
    /// 物理量的单位
    /// Unit of the physical quantity
    pub unit: U,
}

// ============================================================================
// 构造方法 / Construction methods
// ============================================================================

impl<V, U: UnitTrait> Quantity<V, U> {
    /// 创建新的物理量
    /// Create a new physical quantity
    ///
    /// # 参数 / Parameters
    /// - `value`: 物理量的值
    /// - `unit`: 物理量的单位
    pub fn new(value: V, unit: U) -> Self {
        Self { value, unit }
    }
}

// ============================================================================
// 编译时单位（CTUnit）的构造方法 / Construction methods for CTUnit
// ============================================================================

impl<V, U: CTUnit + Default> Quantity<V, U> {
    /// 创建编译时物理量（零参数构造）
    /// Create compile-time quantity (zero-argument construction)
    ///
    /// 编译时单位是零大小类型（ZST），不需要传入单位实例。
    /// Compile-time units are zero-sized types (ZST), no unit instance needed.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::Quantity;
    /// use ospf_rust_quantities::unit::derived::Meter;
    /// use bigdecimal::BigDecimal;
    ///
    /// let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
    /// ```
    pub fn new_ct(value: V) -> Self {
        Self {
            value,
            unit: U::default(),
        }
    }

    /// 获取单位符号（编译时）
    /// Get unit symbol (compile-time)
    pub fn unit_symbol() -> &'static str {
        U::SYMBOL
    }

    /// 获取单位名称（编译时）
    /// Get unit name (compile-time)
    pub fn unit_name() -> &'static str {
        U::NAME
    }
}

// ============================================================================
// 通用方法 / General methods
// ============================================================================

impl<V, U: UnitTrait> Quantity<V, U> {
    /// 获取值引用
    /// Get value reference
    pub fn value(&self) -> &V {
        &self.value
    }

    /// 获取单位引用
    /// Get unit reference
    pub fn unit(&self) -> &U {
        &self.unit
    }

    /// 转换值类型
    /// Convert value type
    ///
    /// 使用提供的函数将值从一种类型转换为另一种类型。
    /// Converts the value from one type to another using the provided function.
    pub fn map_value<T, F>(self, f: F) -> Quantity<T, U>
    where
        F: FnOnce(V) -> T,
    {
        Quantity::new(f(self.value), self.unit)
    }

    /// 尝试转换值类型
    /// Try to convert value type
    pub fn try_map_value<T, F, E>(self, f: F) -> Option<Quantity<T, U>>
    where
        F: FnOnce(V) -> Result<T, E>,
    {
        Some(Quantity::new(f(self.value).ok()?, self.unit))
    }
}

impl<V: Clone, U: UnitTrait + Clone> Quantity<V, U> {
    /// 转换值类型（引用版本）
    /// Convert value type (reference version)
    pub fn map_value_ref<T, F>(&self, f: F) -> Quantity<T, U>
    where
        F: FnOnce(&V) -> T,
    {
        Quantity::new(f(&self.value), self.unit.clone())
    }
}

impl<V, U: UnitTrait> AsRef<Quantity<V, U>> for Quantity<V, U> {
    fn as_ref(&self) -> &Quantity<V, U> {
        self
    }
}

// ============================================================================
// 运行时单位转换 / Runtime unit conversion
// ============================================================================

impl<V> Quantity<V, Unit>
where
    V: Clone + UnitConversionValue,
{
    /// 获取量纲
    /// Get dimension
    pub fn dimension(&self) -> DerivedQuantity {
        self.unit.dimension().clone()
    }

    /// 转换到另一个单位（运行时）
    /// Convert to another unit (runtime)
    ///
    /// 将物理量转换到另一个具有相同量纲的单位。
    /// Converts the quantity to another unit with the same dimension.
    ///
    /// # 错误 / Errors
    /// 如果目标单位的量纲与当前单位的量纲不匹配，返回 `DimensionMismatchError`。
    /// Returns `DimensionMismatchError` if the target unit's dimension doesn't match.
    pub fn to_unit(&self, target: &Unit) -> Ret<Quantity<V, Unit>> {
        if self.unit == *target {
            return Ok(self.clone());
        }

        let value = match self.unit.convert_value_to(self.value.clone(), target) {
            Some(value) => value,
            None => {
                return Err(Box::new(error!(DimensionMismatchError {
                    expected: target.dimension().symbol().to_string(),
                    actual: self.unit.dimension().symbol().to_string(),
                    operation: "unit conversion"
                })));
            }
        };

        Ok(Quantity::new(value, target.clone()))
    }

    /// 尝试转换到另一个单位（返回 Option）
    /// Try to convert to another unit (returns Option)
    pub fn try_to_unit(&self, target: &Unit) -> Option<Quantity<V, Unit>> {
        self.to_unit(target).ok()
    }

    /// 转换为指定单位制下的标准单位
    /// Convert to standard unit in the given unit system
    pub fn to_standard_unit(&self, system: &dyn UnitSystem) -> Option<Quantity<V, Unit>> {
        let standard_unit = system.standard_unit_for_dimension(&self.dimension())?;
        self.try_to_unit(&standard_unit)
    }
}

// ============================================================================
// 编译时单位转换 / Compile-time unit conversion
// ============================================================================

impl<V, U: CTUnit> Quantity<V, U>
where
    V: Clone + UnitConversionValue,
{
    /// 转换到另一个单位类型（编译时量纲检查）
    /// Convert to another unit type (compile-time dimension check)
    ///
    /// 将物理量转换到另一个具有相同量纲的单位类型。
    /// 量纲匹配检查在编译时完成，如果量纲不匹配会导致编译错误。
    ///
    /// Converts the quantity to another unit type with the same dimension.
    /// Dimension matching is checked at compile time; mismatched dimensions cause compile errors.
    ///
    /// # 类型参数 / Type Parameters
    /// - `Target`: 目标单位类型，必须与当前单位具有相同量纲
    pub fn to<Target: CTUnit + Default>(self) -> Quantity<V, Target>
    where
        <U as CTUnit>::Dimension: SameDerivedDimension<<Target as CTUnit>::Dimension>,
    {
        let value = U::convert_value_to::<V, Target>(self.value)
            .expect("无法转换不同量纲的物理量 / Cannot convert quantities with different dimensions");
        Quantity::new_ct(value)
    }
}

// ============================================================================
// 编译时转运行时 / Compile-time to runtime conversion
// ============================================================================

impl<V, U: CTUnit> Quantity<V, U>
where
    V: Clone,
{
    /// 转换为运行时物理量
    /// Convert to runtime quantity
    ///
    /// 将编译时物理量转换为运行时物理量。
    /// Converts a compile-time quantity to a runtime quantity.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::Quantity;
    /// use ospf_rust_quantities::unit::derived::Meter;
    /// use bigdecimal::BigDecimal;
    ///
    /// let ct_q: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
    /// let rt_q: Quantity<BigDecimal, ospf_rust_quantities::unit::Unit> = ct_q.to_runtime();
    /// ```
    pub fn to_runtime(&self) -> Quantity<V, Unit> {
        Quantity::new(self.value.clone(), U::INSTANT.clone())
    }
}

// ============================================================================
// 比较操作（运行时）/ Comparison operations (runtime)
// ============================================================================

impl<V> PartialEq for Quantity<V, Unit>
where
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if !self.unit.same_dimension(&other.unit) {
            return false;
        }

        if self.unit == other.unit {
            self.value == other.value
        } else {
            false
        }
    }
}

impl<V> Eq for Quantity<V, Unit> where V: Eq {}

impl<V> PartialOrd for Quantity<V, Unit>
where
    V: PartialOrd + Clone + UnitConversionValue,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if !self.unit.same_dimension(&other.unit) {
            return None;
        }

        if self.unit == other.unit {
            self.value.partial_cmp(&other.value)
        } else {
            let converted = other.try_to_unit(&self.unit)?;
            self.value.partial_cmp(&converted.value)
        }
    }
}

// ============================================================================
// 比较操作（编译时）/ Comparison operations (compile-time)
// ============================================================================

impl<V: PartialEq, U: CTUnit> PartialEq for Quantity<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<V: Eq, U: CTUnit> Eq for Quantity<V, U> {}

impl<V: PartialOrd, U: CTUnit> PartialOrd for Quantity<V, U> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<V: Ord, U: CTUnit> Ord for Quantity<V, U> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

// ============================================================================
// 算术操作（运行时，返回 Result）/ Arithmetic operations (runtime, returning Result)
// ============================================================================

impl<V> Quantity<V, Unit>
where
    V: UnitConversionValue,
{
    fn linear_difference_unit(unit: &Unit) -> Unit {
        if unit.is_linear() {
            return unit.clone();
        }

        Unit::new_with_conversion(
            format!("{} difference", unit.name()),
            format!("delta({})", unit.symbol()),
            unit.dimension().clone(),
            UnitConversionRule::linear(unit.scale().clone()),
        )
    }

    fn convert_linear_difference_value(value: V, from: &Unit, to: &Unit) -> Option<V> {
        if !from.same_dimension(to) {
            return None;
        }
        let factor = from.scale().value() / to.scale().value();
        V::from_decimal(&factor).map(|f| value * f)
    }

    /// 加法（返回 Result）
    /// Addition (returning Result)
    ///
    /// 将两个相同量纲的物理量相加。如果单位不同，会自动转换单位。
    /// Adds two quantities with the same dimension. Automatically converts units if different.
    pub fn checked_add(&self, other: &Self) -> Ret<Quantity<V, Unit>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "addition"
            })));
        }
        if !self.unit.is_linear() && !other.unit.is_linear() {
            return Err(Box::new(error!(UnitConversionError {
                from_unit: other.unit.symbol().to_string(),
                to_unit: self.unit.symbol().to_string(),
                reason: "two affine temperature points cannot be added"
            })));
        }

        if !self.unit.is_linear() || !other.unit.is_linear() {
            if !self.unit.is_linear() {
                let delta = match Self::convert_linear_difference_value(
                    other.value.clone(),
                    &other.unit,
                    &self.unit,
                ) {
                    Some(value) => value,
                    None => {
                        return Err(Box::new(error!(UnitConversionError {
                            from_unit: other.unit.symbol().to_string(),
                            to_unit: self.unit.symbol().to_string(),
                            reason: "linear difference cannot be converted to affine unit"
                        })));
                    }
                };
                return Ok(Quantity::new(self.value.clone() + delta, self.unit.clone()));
            }

            let delta = match Self::convert_linear_difference_value(
                self.value.clone(),
                &self.unit,
                &other.unit,
            ) {
                Some(value) => value,
                None => {
                    return Err(Box::new(error!(UnitConversionError {
                        from_unit: self.unit.symbol().to_string(),
                        to_unit: other.unit.symbol().to_string(),
                        reason: "linear difference cannot be converted to affine unit"
                    })));
                }
            };
            return Ok(Quantity::new(
                other.value.clone() + delta,
                other.unit.clone(),
            ));
        } else if self.unit == other.unit {
            Ok(Quantity::new(
                self.value.clone() + other.value.clone(),
                self.unit.clone(),
            ))
        } else {
            let converted = other.to_unit(&self.unit)?;
            Ok(Quantity::new(
                self.value.clone() + converted.value,
                self.unit.clone(),
            ))
        }
    }

    /// 减法（返回 Result）
    /// Subtraction (returning Result)
    pub fn checked_sub(&self, other: &Self) -> Ret<Quantity<V, Unit>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "subtraction"
            })));
        }
        if !self.unit.is_linear() && !other.unit.is_linear() {
            let self_standard = match self
                .unit
                .conversion()
                .to_standard_value_checked(self.value.clone())
            {
                Some(v) => v,
                None => {
                    return Err(Box::new(error!(UnitConversionError {
                        from_unit: self.unit.symbol().to_string(),
                        to_unit: "standard".to_string(),
                        reason: "numeric conversion failed for affine subtraction"
                    })));
                }
            };
            let other_standard = match other
                .unit
                .conversion()
                .to_standard_value_checked(other.value.clone())
            {
                Some(v) => v,
                None => {
                    return Err(Box::new(error!(UnitConversionError {
                        from_unit: other.unit.symbol().to_string(),
                        to_unit: "standard".to_string(),
                        reason: "numeric conversion failed for affine subtraction"
                    })));
                }
            };
            let difference_unit = Self::linear_difference_unit(&self.unit);
            let scale = match V::from_decimal(difference_unit.scale().value()) {
                Some(v) => v,
                None => {
                    return Err(Box::new(error!(UnitConversionError {
                        from_unit: self.unit.symbol().to_string(),
                        to_unit: "standard".to_string(),
                        reason: "scale numeric conversion failed"
                    })));
                }
            };
            return Ok(Quantity::new(
                (self_standard - other_standard) / scale,
                difference_unit,
            ));
        }

        if self.unit.is_linear() && !other.unit.is_linear() {
            return Err(Box::new(error!(UnitConversionError {
                from_unit: other.unit.symbol().to_string(),
                to_unit: self.unit.symbol().to_string(),
                reason: "a temperature point cannot be subtracted from a linear difference"
            })));
        }

        if !self.unit.is_linear() {
            let delta = match Self::convert_linear_difference_value(
                other.value.clone(),
                &other.unit,
                &self.unit,
            ) {
                Some(value) => value,
                None => {
                    return Err(Box::new(error!(UnitConversionError {
                        from_unit: other.unit.symbol().to_string(),
                        to_unit: self.unit.symbol().to_string(),
                        reason: "linear difference cannot be converted to affine unit"
                    })));
                }
            };
            return Ok(Quantity::new(self.value.clone() - delta, self.unit.clone()));
        } else if self.unit == other.unit {
            Ok(Quantity::new(
                self.value.clone() - other.value.clone(),
                self.unit.clone(),
            ))
        } else {
            let converted = other.to_unit(&self.unit)?;
            Ok(Quantity::new(
                self.value.clone() - converted.value,
                self.unit.clone(),
            ))
        }
    }
}

// ============================================================================
// 算术操作符（运行时）/ Arithmetic operators (runtime)
// ============================================================================

impl<V> Add for Quantity<V, Unit>
where
    V: UnitConversionValue,
{
    type Output = Quantity<V, Unit>;

    fn add(self, other: Self) -> Self::Output {
        self.checked_add(&other)
            .expect("无法对不同量纲的物理量进行加法运算 / Cannot add quantities with different dimensions")
    }
}

impl<V> Sub for Quantity<V, Unit>
where
    V: UnitConversionValue,
{
    type Output = Quantity<V, Unit>;

    fn sub(self, other: Self) -> Self::Output {
        self.checked_sub(&other)
            .expect("无法对不同量纲的物理量进行减法运算 / Cannot subtract quantities with different dimensions")
    }
}

impl<V> Mul<V> for Quantity<V, Unit>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn mul(self, rhs: V) -> Self::Output {
        assert!(
            self.unit.is_linear(),
            "Cannot multiply quantity with affine unit"
        );
        Quantity::new(self.value * rhs, self.unit)
    }
}

impl<V> Div<V> for Quantity<V, Unit>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn div(self, rhs: V) -> Self::Output {
        assert!(
            self.unit.is_linear(),
            "Cannot divide quantity with affine unit"
        );
        Quantity::new(self.value / rhs, self.unit)
    }
}

impl<V> Neg for Quantity<V, Unit>
where
    V: Neg<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn neg(self) -> Self::Output {
        assert!(
            self.unit.is_linear(),
            "Cannot negate quantity with affine unit"
        );
        Quantity::new(-self.value, self.unit)
    }
}

// ============================================================================
// 物理量乘除（运行时，产生新量纲）/ Quantity multiplication/division (runtime)
// ============================================================================

impl<V> Mul for Quantity<V, Unit>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn mul(self, other: Self) -> Self::Output {
        let new_unit = (&self.unit * &other.unit).build();
        Quantity::new(self.value * other.value, new_unit)
    }
}

impl<V> Div for Quantity<V, Unit>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn div(self, other: Self) -> Self::Output {
        let new_unit = (&self.unit / &other.unit).build();
        Quantity::new(self.value / other.value, new_unit)
    }
}

// ============================================================================
// 算术操作符（编译时）/ Arithmetic operators (compile-time)
// ============================================================================

impl<V, U: CTUnit + Default> Add for Quantity<V, U>
where
    V: Add<Output = V>,
{
    type Output = Quantity<V, U>;

    fn add(self, other: Self) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot add affine-unit quantities"
        );
        Quantity::new_ct(self.value + other.value)
    }
}

impl<V, U: CTUnit + Default> Sub for Quantity<V, U>
where
    V: Sub<Output = V>,
{
    type Output = Quantity<V, U>;

    fn sub(self, other: Self) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot subtract affine-unit quantities"
        );
        Quantity::new_ct(self.value - other.value)
    }
}

/// 标量乘法（值类型与标量类型相同）
/// Scalar multiplication (value type equals scalar type)
///
/// 支持 `Quantity<V, U> * V` 形式的标量乘法。
/// Supports scalar multiplication in the form `Quantity<V, U> * V`.
impl<V, U: CTUnit + Default> Mul<V> for Quantity<V, U>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V, U>;

    fn mul(self, rhs: V) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot multiply affine-unit quantity"
        );
        Quantity::new_ct(self.value * rhs)
    }
}

/// 不同类型标量乘法的辅助 trait
/// Helper trait for different-type scalar multiplication
///
/// 用于支持 `Quantity<Linear<T>, U> * T` 形式的标量乘法。
/// Used to support scalar multiplication in the form `Quantity<Linear<T>, U> * T`.
pub trait ScalarMul<S> {
    type Output;
    fn scalar_mul(self, rhs: S) -> Self::Output;
}

impl<V, S, U: CTUnit + Default> ScalarMul<S> for Quantity<V, U>
where
    V: Clone + Mul<S, Output = V>,
{
    type Output = Quantity<V, U>;

    fn scalar_mul(self, rhs: S) -> Self::Output {
        Quantity::new_ct(self.value * rhs)
    }
}

impl<V, U: CTUnit + Default> Div<V> for Quantity<V, U>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V, U>;

    fn div(self, rhs: V) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot divide affine-unit quantity"
        );
        Quantity::new_ct(self.value / rhs)
    }
}

impl<V, U: CTUnit + Default> Neg for Quantity<V, U>
where
    V: Neg<Output = V>,
{
    type Output = Quantity<V, U>;

    fn neg(self) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot negate affine-unit quantity"
        );
        Quantity::new_ct(-self.value)
    }
}

// ============================================================================
// 物理量乘除（编译时，产生新单位类型）/ Quantity multiplication/division (compile-time)
// ============================================================================

impl<V, U1: CTUnit + Default, U2: CTUnit + Default> Mul<Quantity<V, U2>> for Quantity<V, U1>
where
    V: Mul<Output = V>,
    CTUnitMul<U1, U2>: CTUnit + Default,
{
    type Output = Quantity<V, CTUnitMul<U1, U2>>;

    fn mul(self, other: Quantity<V, U2>) -> Self::Output {
        assert!(
            *U1::OFFSET == BigDecimal::from(0) && *U2::OFFSET == BigDecimal::from(0),
            "Cannot multiply affine-unit quantities"
        );
        Quantity::new_ct(self.value * other.value)
    }
}

impl<V, U1: CTUnit + Default, U2: CTUnit + Default> Div<Quantity<V, U2>> for Quantity<V, U1>
where
    V: Div<Output = V>,
    CTUnitDiv<U1, U2>: CTUnit + Default,
{
    type Output = Quantity<V, CTUnitDiv<U1, U2>>;

    fn div(self, other: Quantity<V, U2>) -> Self::Output {
        assert!(
            *U1::OFFSET == BigDecimal::from(0) && *U2::OFFSET == BigDecimal::from(0),
            "Cannot divide affine-unit quantities"
        );
        Quantity::new_ct(self.value / other.value)
    }
}

// ============================================================================
// 引用版本的算术操作符（运行时）/ Reference version arithmetic operators (runtime)
// ============================================================================

impl<V> Add<&Quantity<V, Unit>> for &Quantity<V, Unit>
where
    V: UnitConversionValue,
{
    type Output = Quantity<V, Unit>;

    fn add(self, other: &Quantity<V, Unit>) -> Self::Output {
        self.checked_add(other)
            .expect("无法对不同量纲的物理量进行加法运算 / Cannot add quantities with different dimensions")
    }
}

impl<V> Sub<&Quantity<V, Unit>> for &Quantity<V, Unit>
where
    V: UnitConversionValue,
{
    type Output = Quantity<V, Unit>;

    fn sub(self, other: &Quantity<V, Unit>) -> Self::Output {
        self.checked_sub(other)
            .expect("无法对不同量纲的物理量进行减法运算 / Cannot subtract quantities with different dimensions")
    }
}

impl<V> Mul<&V> for &Quantity<V, Unit>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn mul(self, rhs: &V) -> Self::Output {
        assert!(
            self.unit.is_linear(),
            "Cannot multiply quantity with affine unit"
        );
        Quantity::new(self.value.clone() * rhs.clone(), self.unit.clone())
    }
}

impl<V> Div<&V> for &Quantity<V, Unit>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn div(self, rhs: &V) -> Self::Output {
        assert!(
            self.unit.is_linear(),
            "Cannot divide quantity with affine unit"
        );
        Quantity::new(self.value.clone() / rhs.clone(), self.unit.clone())
    }
}

impl<V> Neg for &Quantity<V, Unit>
where
    V: Neg<Output = V> + Clone,
{
    type Output = Quantity<V, Unit>;

    fn neg(self) -> Self::Output {
        assert!(
            self.unit.is_linear(),
            "Cannot negate quantity with affine unit"
        );
        Quantity::new(-self.value.clone(), self.unit.clone())
    }
}

impl<V> Mul<&Quantity<V, Unit>> for &Quantity<V, Unit>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn mul(self, other: &Quantity<V, Unit>) -> Self::Output {
        let new_unit = (&self.unit * &other.unit).build();
        Quantity::new(self.value.clone() * other.value.clone(), new_unit)
    }
}

impl<V> Div<&Quantity<V, Unit>> for &Quantity<V, Unit>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn div(self, other: &Quantity<V, Unit>) -> Self::Output {
        let new_unit = (&self.unit / &other.unit).build();
        Quantity::new(self.value.clone() / other.value.clone(), new_unit)
    }
}

// ============================================================================
// 引用版本的算术操作符（编译时）/ Reference version arithmetic operators (compile-time)
// ============================================================================

impl<V, U: CTUnit + Default> Add<&Quantity<V, U>> for &Quantity<V, U>
where
    V: Add<Output = V> + Clone,
{
    type Output = Quantity<V, U>;

    fn add(self, other: &Quantity<V, U>) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot add affine-unit quantities"
        );
        Quantity::new_ct(self.value.clone() + other.value.clone())
    }
}

impl<V, U: CTUnit + Default> Sub<&Quantity<V, U>> for &Quantity<V, U>
where
    V: Sub<Output = V> + Clone,
{
    type Output = Quantity<V, U>;

    fn sub(self, other: &Quantity<V, U>) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot subtract affine-unit quantities"
        );
        Quantity::new_ct(self.value.clone() - other.value.clone())
    }
}

impl<V, U: CTUnit + Default> Mul<&V> for &Quantity<V, U>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V, U>;

    fn mul(self, rhs: &V) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot multiply affine-unit quantity"
        );
        Quantity::new_ct(self.value.clone() * rhs.clone())
    }
}

impl<V, U: CTUnit + Default> Div<&V> for &Quantity<V, U>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V, U>;

    fn div(self, rhs: &V) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot divide affine-unit quantity"
        );
        Quantity::new_ct(self.value.clone() / rhs.clone())
    }
}

impl<V, U: CTUnit + Default> Neg for &Quantity<V, U>
where
    V: Neg<Output = V> + Clone,
{
    type Output = Quantity<V, U>;

    fn neg(self) -> Self::Output {
        assert!(
            *U::OFFSET == BigDecimal::from(0),
            "Cannot negate affine-unit quantity"
        );
        Quantity::new_ct(-self.value.clone())
    }
}

impl<V, U1: CTUnit + Default, U2: CTUnit + Default> Mul<&Quantity<V, U2>> for &Quantity<V, U1>
where
    V: Clone + Mul<Output = V>,
    CTUnitMul<U1, U2>: CTUnit + Default,
{
    type Output = Quantity<V, CTUnitMul<U1, U2>>;

    fn mul(self, other: &Quantity<V, U2>) -> Self::Output {
        assert!(
            *U1::OFFSET == BigDecimal::from(0) && *U2::OFFSET == BigDecimal::from(0),
            "Cannot multiply affine-unit quantities"
        );
        Quantity::new_ct(self.value.clone() * other.value.clone())
    }
}

impl<V, U1: CTUnit + Default, U2: CTUnit + Default> Div<&Quantity<V, U2>> for &Quantity<V, U1>
where
    V: Clone + Div<Output = V>,
    CTUnitDiv<U1, U2>: CTUnit + Default,
{
    type Output = Quantity<V, CTUnitDiv<U1, U2>>;

    fn div(self, other: &Quantity<V, U2>) -> Self::Output {
        assert!(
            *U1::OFFSET == BigDecimal::from(0) && *U2::OFFSET == BigDecimal::from(0),
            "Cannot divide affine-unit quantities"
        );
        Quantity::new_ct(self.value.clone() / other.value.clone())
    }
}

// ============================================================================
// From 实现 / From implementations
// ============================================================================

impl<V> From<(V, Unit)> for Quantity<V, Unit> {
    fn from((value, unit): (V, Unit)) -> Self {
        Quantity::new(value, unit)
    }
}

// ============================================================================
// Default 实现 / Default implementation
// ============================================================================

impl<V: Default, U: UnitTrait + Default> Default for Quantity<V, U> {
    fn default() -> Self {
        Quantity::new(V::default(), U::default())
    }
}

// ============================================================================
// Zero/One 实现 - 不适用 / Zero/One implementations - Not applicable
// ============================================================================
//
// 注意：`Quantity` 无法实现 `Zero` 和 `One` trait，原因如下：
// Note: `Quantity` cannot implement `Zero` and `One` traits for the following reasons:
//
// 1. **运行时单位 (`Unit`)**：没有默认值，且乘法产生新单位类型
//    **Runtime unit (`Unit`)**: No default value, and multiplication produces new unit type
//
// 2. **编译时单位 (`U: CTUnit`)**：乘法 `Mul<Self>` 会产生 `CTUnitMul<U, U>` 类型，而非 `Self`
//    **Compile-time unit (`U: CTUnit`)**: `Mul<Self>` produces `CTUnitMul<U, U>` type, not `Self`
//
// 对于需要零值的情况，请使用：
// For cases requiring zero values, please use:
// ```
// let zero: Quantity<f64, Meter> = Quantity::new_ct(0.0);
// ```

// ============================================================================
// Reciprocal 运算符实现（运行时）/ Reciprocal operator implementations (runtime)
// ============================================================================

impl<V> Reciprocal for Quantity<V, Unit>
where
    V: Reciprocal<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn reciprocal(self) -> Self::Output {
        let new_unit = self.unit.reciprocal().build();
        Quantity::new(self.value.reciprocal(), new_unit)
    }
}

impl<V> Reciprocal for &Quantity<V, Unit>
where
    V: Clone + Reciprocal<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn reciprocal(self) -> Self::Output {
        let new_unit = self.unit.clone().reciprocal().build();
        Quantity::new(self.value.clone().reciprocal(), new_unit)
    }
}

// ============================================================================
// Reciprocal 运算符实现（编译时）/ Reciprocal operator implementations (compile-time)
// ============================================================================

impl<V, U: CTUnit> Reciprocal for Quantity<V, U>
where
    V: Reciprocal<Output = V>,
    CTUnitReciprocal<U>: CTUnit + Default,
{
    type Output = Quantity<V, CTUnitReciprocal<U>>;

    fn reciprocal(self) -> Self::Output {
        Quantity::new_ct(self.value.reciprocal())
    }
}

impl<V, U: CTUnit> Reciprocal for &Quantity<V, U>
where
    for<'a> &'a V: Reciprocal<Output = V>,
    CTUnitReciprocal<U>: CTUnit + Default,
{
    type Output = Quantity<V, CTUnitReciprocal<U>>;

    fn reciprocal(self) -> Self::Output {
        Quantity::new_ct(self.value.reciprocal())
    }
}

// ============================================================================
// Abs 运算符实现（运行时）/ Abs operator implementations (runtime)
// ============================================================================

impl<V> Abs for Quantity<V, Unit>
where
    V: Abs<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn abs(self) -> Self::Output {
        Quantity::new(self.value.abs(), self.unit)
    }
}

impl<V> Abs for &Quantity<V, Unit>
where
    for<'a> &'a V: Abs<Output = V>,
{
    type Output = Quantity<V, Unit>;

    fn abs(self) -> Self::Output {
        Quantity::new(self.value.abs(), self.unit.clone())
    }
}

// ============================================================================
// Abs 运算符实现（编译时）/ Abs operator implementations (compile-time)
// ============================================================================

impl<V, U: CTUnit + Default> Abs for Quantity<V, U>
where
    V: Abs<Output = V>,
{
    type Output = Quantity<V, U>;

    fn abs(self) -> Self::Output {
        Quantity::new_ct(self.value.abs())
    }
}

impl<V, U: CTUnit + Default> Abs for &Quantity<V, U>
where
    for<'a> &'a V: Abs<Output = V>,
{
    type Output = Quantity<V, U>;

    fn abs(self) -> Self::Output {
        Quantity::new_ct(self.value.abs())
    }
}

// ============================================================================
// TolerancedEq 实现（运行时）/ TolerancedEq implementation (runtime)
// ============================================================================

impl<V> TolerancedEq for Quantity<V, Unit>
where
    V: TolerancedEq<Value = V> + UnitConversionValue,
    for<'a> &'a V: Mul<V, Output = V>,
{
    type Value = V;

    fn eq_within(&self, other: &Self, tolerance: &Tolerance<V>) -> bool {
        if !self.unit.same_dimension(&other.unit) {
            return false;
        }

        if self.unit == other.unit {
            self.value.eq_within(&other.value, &tolerance)
        } else {
            let factor = match self.unit.conversion_factor_to(&other.unit) {
                Some(f) => f,
                None => return false,
            };
            let factor_v = match V::from_decimal(&factor) {
                Some(v) => v,
                None => return false,
            };
            let converted_value = &other.value * factor_v;
            self.value.eq_within(&converted_value, &tolerance)
        }
    }
}

// ============================================================================
// TolerancedEq 实现（编译时）/ TolerancedEq implementation (compile-time)
// ============================================================================

impl<V, U: CTUnit> TolerancedEq for Quantity<V, U>
where
    V: TolerancedEq<Value = V>,
{
    type Value = V;

    fn eq_within(&self, other: &Self, tolerance: &Tolerance<V>) -> bool {
        self.value.eq_within(&other.value, &tolerance)
    }
}

// ============================================================================
// TolerancedOrd 实现（运行时）/ TolerancedOrd implementation (runtime)
// ============================================================================

impl<V> TolerancedOrd for Quantity<V, Unit>
where
    V: TolerancedOrd<Value = V> + UnitConversionValue,
    for<'a> &'a V: Mul<V, Output = V>,
{
    fn cmp_within(&self, other: &Self, tolerance: &Tolerance<V>) -> Ordering {
        if !self.unit.same_dimension(&other.unit) {
            return self
                .unit
                .dimension()
                .symbol()
                .cmp(&other.unit.dimension().symbol());
        }

        if self.unit == other.unit {
            self.value.cmp_within(&other.value, &tolerance)
        } else {
            let factor = match self.unit.conversion_factor_to(&other.unit) {
                Some(f) => f,
                None => return Ordering::Equal,
            };
            let factor_v = match V::from_decimal(&factor) {
                Some(v) => v,
                None => return Ordering::Equal,
            };
            let converted_value = &other.value * factor_v;
            self.value.cmp_within(&converted_value, &tolerance)
        }
    }
}

// ============================================================================
// TolerancedOrd 实现（编译时）/ TolerancedOrd implementation (compile-time)
// ============================================================================

impl<V, U: CTUnit> TolerancedOrd for Quantity<V, U>
where
    V: TolerancedOrd<Value = V>,
{
    fn cmp_within(&self, other: &Self, tolerance: &Tolerance<V>) -> Ordering {
        self.value.cmp_within(&other.value, &tolerance)
    }
}

// ============================================================================
// QuantityTrait - 物理量统一接口 / Unified quantity interface
// ============================================================================

/// QuantityTrait - 物理量统一接口
/// QuantityTrait - Unified quantity interface
///
/// 所有物理量类型必须实现此 trait。
/// All quantity types must implement this trait.
pub trait QuantityTrait {
    /// 值类型 / Value type
    type Value: Clone;

    /// 获取值 / Get value
    fn value(&self) -> &Self::Value;

    /// 获取单位符号 / Get unit symbol
    fn unit_symbol(&self) -> &str;

    /// 获取单位名称 / Get unit name
    fn unit_name(&self) -> &str;

    /// 获取量纲符号 / Get dimension symbol
    fn dimension_symbol(&self) -> String;

    /// 获取比例尺值 / Get scale value
    fn scale_value(&self) -> BigDecimal;
}

// ============================================================================
// 为运行时 Quantity 实现 QuantityTrait
// Implement QuantityTrait for runtime Quantity
// ============================================================================

impl<V: Clone> QuantityTrait for Quantity<V, Unit> {
    type Value = V;

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn unit_symbol(&self) -> &str {
        self.unit.symbol()
    }

    fn unit_name(&self) -> &str {
        self.unit.name()
    }

    fn dimension_symbol(&self) -> String {
        self.unit.dimension_symbol()
    }

    fn scale_value(&self) -> BigDecimal {
        self.unit.scale_value()
    }
}

// ============================================================================
// 为编译时 Quantity 实现 QuantityTrait
// Implement QuantityTrait for compile-time Quantity
// ============================================================================

impl<V: Clone, U: CTUnit> QuantityTrait for Quantity<V, U> {
    type Value = V;

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn unit_symbol(&self) -> &str {
        U::SYMBOL
    }

    fn unit_name(&self) -> &str {
        U::NAME
    }

    fn dimension_symbol(&self) -> String {
        <U as CTUnit>::Dimension::INSTANT.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        U::SCALE.value().clone()
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::derived_quantity::QuantityDomain;
    use crate::unit::CTUnit;
    use crate::unit::derived::{
        Bit, Byte, Celsius, Fahrenheit, Kelvin, Kilobit, Kilobyte, Kilogram, Kilometer, Meter,
        NoneUnit, Second,
    };
    use bigdecimal::BigDecimal;
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use ospf_rust_math::symbol::{DynSymbol, Linear, LinearMonomial, OwnedSymbol, SymbolDynId};
    use std::any::Any;
    use std::fmt::{Display, Formatter};
    use std::str::FromStr;
    // ========================================================================
    // 测试用简单符号 / Simple symbol for testing
    // ========================================================================

    #[derive(Debug, Clone)]
    struct SimpleSymbol {
        id: usize,
        name: String,
    }

    impl Display for SimpleSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for SimpleSymbol {
        fn name(&self) -> &str {
            &self.name
        }
        fn display_name(&self) -> &str {
            &self.name
        }
        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn make_symbol(name: &str, id: usize) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol {
            id,
            name: name.to_string(),
        })
    }

    fn assert_big_decimal_close(actual: &BigDecimal, expected: &str) {
        let expected = BigDecimal::from_str(expected).unwrap();
        let tolerance = BigDecimal::from_str("0.0000000001").unwrap();
        let diff = if actual >= &expected {
            actual - &expected
        } else {
            &expected - actual
        };
        assert!(
            diff <= tolerance,
            "expected approximately {}, got {}",
            expected,
            actual
        );
    }

    // ========================================================================
    // 运行时物理量测试 / Runtime quantity tests
    // ========================================================================

    #[test]
    fn test_rt_quantity_creation() {
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        assert_eq!(length.value, BigDecimal::from(10));
        assert_eq!(length.unit.symbol(), "m");
    }

    #[test]
    fn test_rt_quantity_unit_conversion() {
        let length_m = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());
        let length_km = length_m.to_unit(&Kilometer::INSTANT.clone()).unwrap();
        assert_eq!(length_km.value, BigDecimal::from(1));
        assert_eq!(length_km.unit.symbol(), "km");
    }

    #[test]
    fn test_rt_temperature_affine_conversion() {
        assert_eq!(Celsius::SYMBOL, "°C");
        assert_eq!(Fahrenheit::SYMBOL, "°F");

        let zero_c = Quantity::new(BigDecimal::from(0), Celsius::INSTANT.clone());
        let kelvin = zero_c.to_unit(&Kelvin::INSTANT.clone()).unwrap();
        assert_big_decimal_close(&kelvin.value, "273.15");

        let thirty_two_f = Quantity::new(BigDecimal::from(32), Fahrenheit::INSTANT.clone());
        let kelvin = thirty_two_f.to_unit(&Kelvin::INSTANT.clone()).unwrap();
        assert_big_decimal_close(&kelvin.value, "273.15");

        let boiling_c = Quantity::new(BigDecimal::from(100), Celsius::INSTANT.clone());
        let fahrenheit = boiling_c.to_unit(&Fahrenheit::INSTANT.clone()).unwrap();
        assert_big_decimal_close(&fahrenheit.value, "212");
    }

    #[test]
    fn test_information_unit_domain_matches_kotlin() {
        assert_eq!(Bit::DOMAIN, QuantityDomain::Discrete);
        assert_eq!(Bit::INSTANT.domain(), QuantityDomain::Discrete);
        assert_eq!(Byte::DOMAIN, QuantityDomain::Discrete);
        assert_eq!(Byte::INSTANT.domain(), QuantityDomain::Discrete);
        assert_eq!(Kilobit::DOMAIN, QuantityDomain::Continuous);
        assert_eq!(Kilobit::INSTANT.domain(), QuantityDomain::Continuous);
        assert_eq!(Kilobyte::DOMAIN, QuantityDomain::Continuous);
        assert_eq!(Kilobyte::INSTANT.domain(), QuantityDomain::Continuous);
    }

    #[test]
    fn test_none_unit_alias() {
        assert_eq!(NoneUnit::SYMBOL, "1");
    }

    #[test]
    fn test_rt_quantity_add() {
        let q1 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(5), Meter::INSTANT.clone());
        let sum = q1 + q2;
        assert_eq!(sum.value, BigDecimal::from(15));
    }

    #[test]
    fn test_rt_affine_quantity_add_is_rejected() {
        let q1 = Quantity::new(BigDecimal::from(10), Celsius::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(5), Celsius::INSTANT.clone());
        assert!(q1.checked_add(&q2).is_err());
    }

    #[test]
    fn test_rt_affine_quantity_sub_returns_linear_difference() {
        let boiling = Quantity::new(BigDecimal::from(100), Celsius::INSTANT.clone());
        let freezing = Quantity::new(BigDecimal::from(0), Celsius::INSTANT.clone());
        let difference = boiling.checked_sub(&freezing).unwrap();
        assert_big_decimal_close(&difference.value, "100");
        assert!(difference.unit.is_linear());
        assert_eq!(difference.unit.dimension(), Celsius::INSTANT.dimension());

        let shifted = freezing.checked_add(&difference).unwrap();
        assert_eq!(shifted.unit.symbol(), "°C");
        assert_big_decimal_close(&shifted.value, "100");

        let back = shifted.checked_sub(&difference).unwrap();
        assert_eq!(back.unit.symbol(), "°C");
        assert_big_decimal_close(&back.value, "0");
    }

    #[test]
    #[should_panic(expected = "Cannot multiply quantity with affine unit")]
    fn test_rt_affine_scalar_multiply_is_rejected() {
        let q = Quantity::new(BigDecimal::from(10), Celsius::INSTANT.clone());
        let _ = q * BigDecimal::from(2);
    }

    #[test]
    fn test_rt_quantity_mul() {
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());
        let product = length * mass;
        assert_eq!(product.value, BigDecimal::from(50));
    }

    // ========================================================================
    // 编译时物理量测试 / Compile-time quantity tests
    // ========================================================================

    #[test]
    fn test_ct_quantity_creation() {
        let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        assert_eq!(length.value, BigDecimal::from(10));
        assert_eq!(Quantity::<BigDecimal, Meter>::unit_symbol(), "m");
    }

    #[test]
    fn test_ct_quantity_unit_conversion() {
        let length_m: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(1000));
        let length_km: Quantity<BigDecimal, Kilometer> = length_m.to();
        assert_eq!(length_km.value, BigDecimal::from(1));
    }

    #[test]
    fn test_ct_temperature_affine_conversion() {
        let zero_c: Quantity<BigDecimal, Celsius> = Quantity::new_ct(BigDecimal::from(0));
        let kelvin: Quantity<BigDecimal, Kelvin> = zero_c.to();
        assert_big_decimal_close(&kelvin.value, "273.15");

        let boiling_c: Quantity<BigDecimal, Celsius> = Quantity::new_ct(BigDecimal::from(100));
        let fahrenheit: Quantity<BigDecimal, Fahrenheit> = boiling_c.to();
        assert_big_decimal_close(&fahrenheit.value, "212");
    }

    #[test]
    fn test_ct_quantity_add() {
        let q1: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let q2: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(5));
        let sum = q1 + q2;
        assert_eq!(sum.value, BigDecimal::from(15));
    }

    #[test]
    fn test_ct_quantity_mul_different_units() {
        let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let mass: Quantity<BigDecimal, Kilogram> = Quantity::new_ct(BigDecimal::from(5));
        let product = length * mass;
        assert_eq!(product.value, BigDecimal::from(50));
    }

    #[test]
    fn test_ct_quantity_div() {
        let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(100));
        let time: Quantity<BigDecimal, Second> = Quantity::new_ct(BigDecimal::from(10));
        let velocity = length / time;
        assert_eq!(velocity.value, BigDecimal::from(10));
    }

    // ========================================================================
    // 编译时转运行时测试 / Compile-time to runtime tests
    // ========================================================================

    #[test]
    fn test_ct_to_runtime() {
        let ct_q: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let rt_q = ct_q.to_runtime();
        assert_eq!(rt_q.value, BigDecimal::from(10));
        assert_eq!(rt_q.unit.symbol(), "m");
    }

    // ========================================================================
    // QuantityTrait 测试 / QuantityTrait tests
    // ========================================================================

    #[test]
    fn test_quantity_trait_rt() {
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        assert_eq!(length.unit_symbol(), "m");
        assert_eq!(length.unit_name(), "meter");
    }

    #[test]
    fn test_quantity_trait_ct() {
        let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        assert_eq!(length.unit_symbol(), "m");
        assert_eq!(length.unit_name(), "meter");
    }

    #[test]
    fn test_geometry_accepts_quantity_scalars() {
        use ospf_rust_math::geometry::{Box2, Point2, Rectangle2};

        type Length = Quantity<BigDecimal, Meter>;

        fn meters(value: i32) -> Length {
            Quantity::new_ct(BigDecimal::from(value))
        }

        let point: Point2<Length> = Point2::new(meters(1), meters(2));
        assert_eq!(point.x_ref().value, BigDecimal::from(1));
        assert_eq!(point.y_ref().value, BigDecimal::from(2));

        let rectangle = Rectangle2::new(meters(3), meters(4));
        assert_eq!(rectangle.width_ref().value, BigDecimal::from(3));
        assert_eq!(rectangle.height_ref().value, BigDecimal::from(4));

        let bbox: Box2<Length> = Box2::new(meters(0), meters(0), rectangle);
        assert_eq!(bbox.rectangle_width().unwrap().value, BigDecimal::from(3));
        assert_eq!(bbox.rectangle_height().unwrap().value, BigDecimal::from(4));
        assert!(bbox.contains_rectangle_point(&point));
        assert!(bbox.contains_rectangle(meters(3), meters(4), true, true, true));
        assert!(!bbox.contains_rectangle(meters(4), meters(2), true, true, true));
    }

    // ========================================================================
    // 物理量多项式测试 / Physical quantity polynomial tests
    // ========================================================================

    /// E92a: 验证 `Quantity<Linear<f64>, Meter>` 编译通过
    /// E92a: Verify `Quantity<Linear<f64>, Meter>` compiles
    #[test]
    fn test_quantity_linear_f64() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 创建线性多项式：2x + 3y + 1.0
        // Create linear polynomial: 2x + 3y + 1.0
        let linear = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            1.0,
        );

        // 包装为物理量
        // Wrap as physical quantity
        let distance: Quantity<Linear<f64>, Meter> = Quantity::new_ct(linear);

        // 验证基本属性
        // Verify basic properties
        assert_eq!(distance.value.len(), 2);
        assert_eq!(distance.value.constant, 1.0);

        // 测试标量乘法（使用 ScalarMul trait）
        // Test scalar multiplication (using ScalarMul trait)
        let scaled = distance.clone().scalar_mul(2.0);
        assert_eq!(scaled.value.monomials[0].coefficient, 4.0);

        // 测试加法
        // Test addition
        let linear2 = Linear::new(vec![LinearMonomial::new(1.0, x.clone())], 2.0);
        let distance2: Quantity<Linear<f64>, Meter> = Quantity::new_ct(linear2);

        let sum = distance + distance2;
        // 注意：Linear 的加法可能保留所有项，具体行为取决于 Linear 的实现
        // Note: Linear addition may retain all terms, behavior depends on Linear implementation
        assert!(sum.value.len() >= 2);
        assert_eq!(sum.value.constant, 3.0);
    }

    /// E92b: 验证 `Quantity<Linear<BigDecimal>, Meter>` 编译通过
    /// E92b: Verify `Quantity<Linear<BigDecimal>, Meter>` compiles
    #[test]
    fn test_quantity_linear_bigdecimal() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 创建线性多项式：2x + 3y + 1.0
        // Create linear polynomial: 2x + 3y + 1.0
        let linear = Linear::new(
            vec![
                LinearMonomial::new(BigDecimal::from(2), x.clone()),
                LinearMonomial::new(BigDecimal::from(3), y.clone()),
            ],
            BigDecimal::from(1),
        );

        // 包装为物理量
        // Wrap as physical quantity
        let distance: Quantity<Linear<BigDecimal>, Meter> = Quantity::new_ct(linear);

        // 验证基本属性
        // Verify basic properties
        assert_eq!(distance.value.len(), 2);
        assert_eq!(distance.value.constant, BigDecimal::from(1));

        // 测试标量乘法（使用 ScalarMul trait）
        // Test scalar multiplication (using ScalarMul trait)
        let scaled = distance.clone().scalar_mul(BigDecimal::from(2));
        assert_eq!(scaled.value.monomials[0].coefficient, BigDecimal::from(4));

        // 测试加法
        // Test addition
        let linear2 = Linear::new(
            vec![LinearMonomial::new(BigDecimal::from(1), x.clone())],
            BigDecimal::from(2),
        );
        let distance2: Quantity<Linear<BigDecimal>, Meter> = Quantity::new_ct(linear2);

        let sum = distance + distance2;
        // 注意：Linear 的加法可能保留所有项，具体行为取决于 Linear 的实现
        // Note: Linear addition may retain all terms, behavior depends on Linear implementation
        assert!(sum.value.len() >= 2);
        assert_eq!(sum.value.constant, BigDecimal::from(3));
    }

    /// E92c: 验证 `Quantity<Linear<BigRational>, Meter>` 编译通过
    /// E92c: Verify `Quantity<Linear<BigRational>, Meter>` compiles
    #[test]
    fn test_quantity_linear_bigrational() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 创建线性多项式：3/2 x + 5/2 y + 1/1
        // Create linear polynomial: 3/2 x + 5/2 y + 1/1
        let coef_x = BigRational::new(BigInt::from(3), BigInt::from(2));
        let coef_y = BigRational::new(BigInt::from(5), BigInt::from(2));
        let constant = BigRational::new(BigInt::from(1), BigInt::from(1));

        let linear = Linear::new(
            vec![
                LinearMonomial::new(coef_x.clone(), x.clone()),
                LinearMonomial::new(coef_y.clone(), y.clone()),
            ],
            constant.clone(),
        );

        // 包装为物理量
        // Wrap as physical quantity
        let distance: Quantity<Linear<BigRational>, Meter> = Quantity::new_ct(linear);

        // 验证基本属性
        // Verify basic properties
        assert_eq!(distance.value.len(), 2);
        assert_eq!(distance.value.constant, constant);

        // 测试标量乘法（使用 ScalarMul trait）
        // Test scalar multiplication (using ScalarMul trait)
        let scalar = BigRational::new(BigInt::from(2), BigInt::from(1));
        let scaled = distance.clone().scalar_mul(scalar);
        // 3/2 * 2 = 3
        let expected = BigRational::new(BigInt::from(3), BigInt::from(1));
        assert_eq!(scaled.value.monomials[0].coefficient, expected);

        // 测试加法
        // Test addition
        let coef2 = BigRational::new(BigInt::from(1), BigInt::from(1));
        let linear2 = Linear::new(
            vec![LinearMonomial::new(coef2.clone(), x.clone())],
            BigRational::new(BigInt::from(2), BigInt::from(1)),
        );
        let distance2: Quantity<Linear<BigRational>, Meter> = Quantity::new_ct(linear2);

        let sum = distance + distance2;
        // 注意：Linear 的加法可能保留所有项，具体行为取决于 Linear 的实现
        // Note: Linear addition may retain all terms, behavior depends on Linear implementation
        assert!(sum.value.len() >= 2);
    }

    // ========================================================================
    // f64 运行时单位转换测试 / f64 runtime unit conversion tests
    // ========================================================================

    #[test]
    fn test_f64_runtime_unit_conversion_m_to_km() {
        // 1000 m -> 1 km
        let q = Quantity::new(1000.0_f64, Meter::INSTANT.clone());
        let converted = q.to_unit(&Kilometer::INSTANT.clone()).unwrap();
        assert!((converted.value - 1.0).abs() < 1e-10);
        assert_eq!(converted.unit.symbol(), "km");
    }

    #[test]
    fn test_f64_runtime_unit_conversion_km_to_m() {
        // 1 km -> 1000 m
        let q = Quantity::new(1.0_f64, Kilometer::INSTANT.clone());
        let converted = q.to_unit(&Meter::INSTANT.clone()).unwrap();
        assert!((converted.value - 1000.0).abs() < 1e-10);
        assert_eq!(converted.unit.symbol(), "m");
    }

    #[test]
    fn test_f64_runtime_add_different_units() {
        // 1 km + 500 m = 1.5 km
        let q1 = Quantity::new(1.0_f64, Kilometer::INSTANT.clone());
        let q2 = Quantity::new(500.0_f64, Meter::INSTANT.clone());
        let sum = q1.checked_add(&q2).unwrap();
        assert!((sum.value - 1.5).abs() < 1e-10);
        assert_eq!(sum.unit.symbol(), "km");
    }

    #[test]
    fn test_f64_runtime_sub_different_units() {
        // 1500 m - 1 km = 500 m
        let q1 = Quantity::new(1500.0_f64, Meter::INSTANT.clone());
        let q2 = Quantity::new(1.0_f64, Kilometer::INSTANT.clone());
        let diff = q1.checked_sub(&q2).unwrap();
        assert!((diff.value - 500.0).abs() < 1e-10);
        assert_eq!(diff.unit.symbol(), "m");
    }

    #[test]
    fn test_f64_runtime_add_operators() {
        // 验证 + 和 - 操作符可用于 f64
        // Verify + and - operators work for f64
        let q1 = Quantity::new(1.0_f64, Kilometer::INSTANT.clone());
        let q2 = Quantity::new(500.0_f64, Meter::INSTANT.clone());
        let sum = q1 + q2;
        assert!((sum.value - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_f64_runtime_partial_ord() {
        // 验证 f64 Quantity 可比较
        // Verify f64 quantities can be compared
        let q1 = Quantity::new(1.0_f64, Kilometer::INSTANT.clone());
        let q2 = Quantity::new(500.0_f64, Meter::INSTANT.clone());
        assert!(q1 > q2);
    }

    // ========================================================================
    // BigRational 运行时单位转换测试 / BigRational runtime unit conversion tests
    // ========================================================================

    #[test]
    fn test_bigrational_runtime_unit_conversion() {
        // 1500 m -> 3/2 km
        let q = Quantity::new(
            BigRational::from_integer(BigInt::from(1500)),
            Meter::INSTANT.clone(),
        );
        let converted = q.to_unit(&Kilometer::INSTANT.clone()).unwrap();
        let expected = BigRational::new(BigInt::from(3), BigInt::from(2));
        assert_eq!(converted.value, expected);
        assert_eq!(converted.unit.symbol(), "km");
    }

    #[test]
    fn test_bigrational_runtime_exact_conversion() {
        // 验证 BigRational 精确性：1/3 km -> 1000/3 m
        let q = Quantity::new(
            BigRational::new(BigInt::from(1), BigInt::from(3)),
            Kilometer::INSTANT.clone(),
        );
        let converted = q.to_unit(&Meter::INSTANT.clone()).unwrap();
        let expected = BigRational::new(BigInt::from(1000), BigInt::from(3));
        assert_eq!(converted.value, expected);
    }

    // ========================================================================
    // f64 仿射单位测试 / f64 affine unit tests
    // ========================================================================

    #[test]
    fn test_f64_affine_temperature_conversion() {
        // 0°C -> 273.15 K (f64 近似)
        let zero_c = Quantity::new(0.0_f64, Celsius::INSTANT.clone());
        let kelvin = zero_c.to_unit(&Kelvin::INSTANT.clone()).unwrap();
        assert!((kelvin.value - 273.15).abs() < 1e-6);

        // 32°F -> 273.15 K (f64 近似)
        let thirty_two_f = Quantity::new(32.0_f64, Fahrenheit::INSTANT.clone());
        let kelvin = thirty_two_f.to_unit(&Kelvin::INSTANT.clone()).unwrap();
        assert!((kelvin.value - 273.15).abs() < 1e-6);
    }

    #[test]
    fn test_f64_affine_sub_returns_linear_difference() {
        // 100°C - 0°C = 100 delta(°C) (f64 近似)
        let boiling = Quantity::new(100.0_f64, Celsius::INSTANT.clone());
        let freezing = Quantity::new(0.0_f64, Celsius::INSTANT.clone());
        let diff = boiling.checked_sub(&freezing).unwrap();
        assert!((diff.value - 100.0).abs() < 1e-6);
        assert!(diff.unit.is_linear());
    }
}
