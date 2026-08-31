//! Quantity - 物理量 (运行时)
//! Quantity - Physical quantity (runtime)
//!
//! 运行时物理量，包含值和单位两个字段，支持动态单位转换。
//! Runtime physical quantity with value and unit fields, supporting dynamic unit conversion.
//!
//! # 核心类型 / Core Types
//! - `Quantity<V>`: 运行时物理量，单位在运行时确定
//!
//! # 特性 / Features
//! - 泛型值类型支持
//! - 动态单位转换
//! - 量纲检查的算术运算
//! - 与单位制集成，支持标准单位转换
//!
//! # 示例 / Examples
//! ```
//! use ospf_rust_quantities::quantity::Quantity;
//! use ospf_rust_quantities::unit::derived::{Meter, Kilometer};
//! use ospf_rust_quantities::unit::CTUnit;
//! use bigdecimal::BigDecimal;
//!
//! // 创建物理量
//! let length = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());
//!
//! // 单位转换
//! let length_km = length.to_unit(&Kilometer::INSTANT.clone()).unwrap();
//! assert_eq!(length_km.value, BigDecimal::from(1));
//! ```

use crate::dimension::DerivedQuantity;
use crate::error::DimensionMismatchError;
use crate::unit::{Unit, UnitSystem};
use bigdecimal::BigDecimal;
use ospf_rust_base::{ErrorPosition, Ret, error};
use ospf_rust_math::algebra::concept::Epsilon;
use ospf_rust_math::operator::abs::Abs;
use ospf_rust_math::operator::reciprocal::Reciprocal;
use ospf_rust_math::operator::tolerance::{Tolerance, TolerancedEq, TolerancedOrd};
use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Sub};

// ============================================================================
// Quantity - 运行时物理量 / Runtime physical quantity
// ============================================================================

/// Quantity - 运行时物理量
/// Quantity - Runtime physical quantity
///
/// 包含值和单位的物理量，支持泛型值类型。
/// Physical quantity with value and unit, supporting generic value types.
///
/// 运行时物理量的单位在运行时确定，支持动态单位转换。
/// The unit of a runtime quantity is determined at runtime, supporting dynamic unit conversion.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型，通常是 `BigDecimal` 或 `f64`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_quantities::quantity::Quantity;
/// use ospf_rust_quantities::unit::derived::Meter;
/// use ospf_rust_quantities::unit::CTUnit;
/// use bigdecimal::BigDecimal;
///
/// let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
/// assert_eq!(length.value, BigDecimal::from(10));
/// assert_eq!(length.unit.symbol(), "m");
/// ```
#[derive(Debug, Clone)]
pub struct Quantity<V> {
    /// 物理量的值
    /// Value of the physical quantity
    pub value: V,
    /// 物理量的单位
    /// Unit of the physical quantity
    pub unit: Unit,
}

impl<V> Quantity<V> {
    /// 创建新的物理量
    /// Create a new physical quantity
    ///
    /// # 参数 / Parameters
    /// - `value`: 物理量的值
    /// - `unit`: 物理量的单位
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::Quantity;
    /// use ospf_rust_quantities::unit::derived::Meter;
    /// use ospf_rust_quantities::unit::CTUnit;
    /// use bigdecimal::BigDecimal;
    ///
    /// let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
    /// ```
    pub fn new(value: V, unit: Unit) -> Self {
        Self { value, unit }
    }

    /// 获取量纲
    /// Get dimension
    ///
    /// 返回该物理量的量纲。
    /// Returns the dimension of this physical quantity.
    pub fn dimension(&self) -> DerivedQuantity {
        self.unit.dimension().clone()
    }

    /// 转换值类型
    /// Convert value type
    ///
    /// 使用提供的函数将值从一种类型转换为另一种类型。
    /// Converts the value from one type to another using the provided function.
    ///
    /// # 参数 / Parameters
    /// - `f`: 值转换函数
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::Quantity;
    /// use ospf_rust_quantities::unit::derived::Meter;
    /// use ospf_rust_quantities::unit::CTUnit;
    /// use bigdecimal::BigDecimal;
    ///
    /// let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
    /// let length_f64: Quantity<f64> = length.map_value(|v| v.to_string().parse().unwrap());
    /// ```
    pub fn map_value<U, F>(&self, f: F) -> Quantity<U>
    where
        F: FnOnce(&V) -> U,
    {
        Quantity::new(f(&self.value), self.unit.clone())
    }

    /// 尝试转换值类型
    /// Try to convert value type
    ///
    /// 使用提供的函数尝试将值从一种类型转换为另一种类型。
    /// Attempts to convert the value from one type to another using the provided function.
    ///
    /// # 参数 / Parameters
    /// - `f`: 值转换函数
    ///
    /// # 返回 / Returns
    /// - `Some(Quantity<U>)`: 转换成功
    /// - `None`: 转换失败
    pub fn try_map_value<U, F, E>(&self, f: F) -> Option<Quantity<U>>
    where
        F: FnOnce(&V) -> Result<U, E>,
    {
        Some(Quantity::new(f(&self.value).ok()?, self.unit.clone()))
    }
}

impl<V: Clone> Quantity<V> {
    /// 转换值类型（引用版本）
    /// Convert value type (reference version)
    ///
    /// 使用提供的函数将值从一种类型转换为另一种类型。
    /// Converts the value from one type to another using the provided function.
    ///
    /// # 参数 / Parameters
    /// - `f`: 值转换函数
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::Quantity;
    /// use ospf_rust_quantities::unit::derived::Meter;
    /// use ospf_rust_quantities::unit::CTUnit;
    /// use bigdecimal::BigDecimal;
    ///
    /// let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
    /// let length_f64: Quantity<f64> = length.as_ref().map_value(|v| v.to_string().parse().unwrap());
    /// ```
    pub fn map_value_ref<U, F>(&self, f: F) -> Quantity<U>
    where
        F: FnOnce(&V) -> U,
    {
        Quantity::new(f(&self.value), self.unit.clone())
    }
}

impl<V> AsRef<Quantity<V>> for Quantity<V> {
    fn as_ref(&self) -> &Quantity<V> {
        self
    }
}

// ============================================================================
// 单位转换 / Unit conversion
// ============================================================================

impl<V> Quantity<V>
where
    V: Clone + Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    /// 转换到另一个单位
    /// Convert to another unit
    ///
    /// 将物理量转换到另一个具有相同量纲的单位。
    /// Converts the quantity to another unit with the same dimension.
    ///
    /// # 错误 / Errors
    /// 如果目标单位的量纲与当前单位的量纲不匹配，返回 `DimensionMismatchError`。
    /// Returns `DimensionMismatchError` if the target unit's dimension doesn't match.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::Quantity;
    /// use ospf_rust_quantities::unit::derived::{Meter, Kilometer};
    /// use ospf_rust_quantities::unit::CTUnit;
    /// use bigdecimal::BigDecimal;
    ///
    /// let length_m = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());
    /// let length_km = length_m.to_unit(&Kilometer::INSTANT.clone()).unwrap();
    /// assert_eq!(length_km.value, BigDecimal::from(1));
    /// ```
    pub fn to_unit(&self, target: &Unit) -> Ret<Quantity<V>> {
        let factor = match self.unit.conversion_factor_to(target) {
            Some(f) => f,
            None => {
                return Err(Box::new(error!(DimensionMismatchError {
                    expected: target.dimension().symbol().to_string(),
                    actual: self.unit.dimension().symbol().to_string(),
                    operation: "unit conversion"
                })));
            }
        };

        if factor == BigDecimal::from(1) {
            return Ok(self.clone());
        }

        Ok(Quantity::new(
            self.value.clone() * factor.into(),
            target.clone(),
        ))
    }

    /// 尝试转换到另一个单位（返回 Option）
    /// Try to convert to another unit (returns Option)
    ///
    /// 与 `to_unit` 相同，但返回 `Option` 而不是 `Result`。
    /// Same as `to_unit`, but returns `Option` instead of `Result`.
    pub fn try_to_unit(&self, target: &Unit) -> Option<Quantity<V>> {
        self.to_unit(target).ok()
    }

    /// 转换为指定单位制下的标准单位
    /// Convert to standard unit in the given unit system
    ///
    /// 如果用户为该量纲指定了标准单位，则转换为用户指定的标准单位；
    /// 否则转换为从基本单位推导的默认标准单位。
    ///
    /// If a user-specified standard unit exists for this dimension, converts to that;
    /// otherwise converts to the default unit derived from base units.
    ///
    /// # 参数 / Parameters
    /// - `system`: 单位制引用
    ///
    /// # 返回 / Returns
    /// - `Some(Quantity)`: 转换后的物理量
    /// - `None`: 如果无法找到标准单位
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::Quantity;
    /// use ospf_rust_quantities::unit::derived::Meter;
    /// use ospf_rust_quantities::unit::{UnitSystem, CTUnit};
    /// use ospf_rust_quantities::unit::system::SI_SYSTEM;
    /// use bigdecimal::BigDecimal;
    ///
    /// let length = Quantity::new(BigDecimal::from(100), Meter::INSTANT.clone());
    /// let standard = length.to_standard_unit(SI_SYSTEM.as_ref()).unwrap();
    /// assert_eq!(standard.unit.symbol(), "m"); // SI 中长度的标准单位是米
    /// ```
    pub fn to_standard_unit(&self, system: &dyn UnitSystem) -> Option<Quantity<V>> {
        let standard_unit = system.standard_unit_for_dimension(&self.dimension())?;
        self.try_to_unit(&standard_unit)
    }
}

// ============================================================================
// 比较操作 / Comparison operations
// ============================================================================

impl<V> PartialEq for Quantity<V>
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

impl<V> Eq for Quantity<V> where V: Eq {}

impl<V> PartialOrd for Quantity<V>
where
    V: PartialOrd + Clone + Mul<V, Output = V>,
    BigDecimal: Into<V>,
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
// 算术操作 (返回 Result) / Arithmetic operations (returning Result)
// ============================================================================

impl<V> Quantity<V>
where
    V: Add<Output = V> + Sub<Output = V> + Clone + Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    /// 加法（返回 Result）
    /// Addition (returning Result)
    ///
    /// 将两个相同量纲的物理量相加。如果单位不同，会自动转换单位。
    /// Adds two quantities with the same dimension. Automatically converts units if different.
    ///
    /// # 错误 / Errors
    /// 如果两个物理量的量纲不匹配，返回 `DimensionMismatchError`。
    /// Returns `DimensionMismatchError` if dimensions don't match.
    pub fn checked_add(&self, other: &Self) -> Ret<Quantity<V>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "addition"
            })));
        }

        if self.unit == other.unit {
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
    ///
    /// 将两个相同量纲的物理量相减。如果单位不同，会自动转换单位。
    /// Subtracts two quantities with the same dimension. Automatically converts units if different.
    ///
    /// # 错误 / Errors
    /// 如果两个物理量的量纲不匹配，返回 `DimensionMismatchError`。
    /// Returns `DimensionMismatchError` if dimensions don't match.
    pub fn checked_sub(&self, other: &Self) -> Ret<Quantity<V>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "subtraction"
            })));
        }

        if self.unit == other.unit {
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
// 算术操作符（panic 版本）/ Arithmetic operators (panic version)
// ============================================================================

impl<V> Add for Quantity<V>
where
    V: Add<Output = V> + Sub<Output = V> + Clone + Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    type Output = Quantity<V>;

    fn add(self, other: Self) -> Self::Output {
        self.checked_add(&other)
            .expect("Cannot add quantities with different dimensions")
    }
}

impl<V> Sub for Quantity<V>
where
    V: Add<Output = V> + Sub<Output = V> + Clone + Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    type Output = Quantity<V>;

    fn sub(self, other: Self) -> Self::Output {
        self.checked_sub(&other)
            .expect("Cannot subtract quantities with different dimensions")
    }
}

impl<V> Mul<V> for Quantity<V>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V>;

    fn mul(self, rhs: V) -> Self::Output {
        Quantity::new(self.value * rhs, self.unit)
    }
}

impl<V> Div<V> for Quantity<V>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V>;

    fn div(self, rhs: V) -> Self::Output {
        Quantity::new(self.value / rhs, self.unit)
    }
}

impl<V> Neg for Quantity<V>
where
    V: Neg<Output = V>,
{
    type Output = Quantity<V>;

    fn neg(self) -> Self::Output {
        Quantity::new(-self.value, self.unit)
    }
}

// ============================================================================
// 物理量乘除（产生新量纲）/ Quantity multiplication/division (producing new dimensions)
// ============================================================================

impl<V> Mul for Quantity<V>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V>;

    fn mul(self, other: Self) -> Self::Output {
        // 单位相乘产生新单位
        // Unit multiplication produces new unit
        let new_unit = (&self.unit * &other.unit).build();
        Quantity::new(self.value * other.value, new_unit)
    }
}

impl<V> Div for Quantity<V>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V>;

    fn div(self, other: Self) -> Self::Output {
        // 单位相除产生新单位
        // Unit division produces new unit
        let new_unit = (&self.unit / &other.unit).build();
        Quantity::new(self.value / other.value, new_unit)
    }
}

// ============================================================================
// 引用版本的算术操作符 / Reference version arithmetic operators
// ============================================================================

impl<V> Add<&Quantity<V>> for &Quantity<V>
where
    V: Add<Output = V> + Sub<Output = V> + Clone + Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    type Output = Quantity<V>;

    fn add(self, other: &Quantity<V>) -> Self::Output {
        self.checked_add(other)
            .expect("Cannot add quantities with different dimensions")
    }
}

impl<V> Sub<&Quantity<V>> for &Quantity<V>
where
    V: Add<Output = V> + Sub<Output = V> + Clone + Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    type Output = Quantity<V>;

    fn sub(self, other: &Quantity<V>) -> Self::Output {
        self.checked_sub(other)
            .expect("Cannot subtract quantities with different dimensions")
    }
}

impl<V> Mul<&V> for &Quantity<V>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V>;

    fn mul(self, rhs: &V) -> Self::Output {
        Quantity::new(self.value.clone() * rhs.clone(), self.unit.clone())
    }
}

impl<V> Div<&V> for &Quantity<V>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V>;

    fn div(self, rhs: &V) -> Self::Output {
        Quantity::new(self.value.clone() / rhs.clone(), self.unit.clone())
    }
}

impl<V> Neg for &Quantity<V>
where
    V: Neg<Output = V> + Clone,
{
    type Output = Quantity<V>;

    fn neg(self) -> Self::Output {
        Quantity::new(-self.value.clone(), self.unit.clone())
    }
}

impl<V> Mul<&Quantity<V>> for &Quantity<V>
where
    V: Clone + Mul<Output = V>,
{
    type Output = Quantity<V>;

    fn mul(self, other: &Quantity<V>) -> Self::Output {
        // 单位相乘产生新单位
        // Unit multiplication produces new unit
        let new_unit = (&self.unit * &other.unit).build();
        Quantity::new(self.value.clone() * other.value.clone(), new_unit)
    }
}

impl<V> Div<&Quantity<V>> for &Quantity<V>
where
    V: Clone + Div<Output = V>,
{
    type Output = Quantity<V>;

    fn div(self, other: &Quantity<V>) -> Self::Output {
        // 单位相除产生新单位
        // Unit division produces new unit
        let new_unit = (&self.unit / &other.unit).build();
        Quantity::new(self.value.clone() / other.value.clone(), new_unit)
    }
}

// ============================================================================
// From 实现 / From implementations
// ============================================================================

impl<V> From<(V, Unit)> for Quantity<V> {
    fn from((value, unit): (V, Unit)) -> Self {
        Quantity::new(value, unit)
    }
}

// ============================================================================
// Reciprocal 运算符实现 / Reciprocal operator implementations
// ============================================================================

impl<V> Reciprocal for Quantity<V>
where
    V: Reciprocal<Output = V>,
{
    type Output = Quantity<V>;

    fn reciprocal(self) -> Self::Output {
        // 单位取倒数产生新单位
        // Unit reciprocal produces new unit
        let new_unit = self.unit.reciprocal().build();
        Quantity::new(self.value.reciprocal(), new_unit)
    }
}

impl<V> Reciprocal for &Quantity<V>
where
    V: Clone + Reciprocal<Output = V>,
{
    type Output = Quantity<V>;

    fn reciprocal(self) -> Self::Output {
        // 单位取倒数产生新单位
        // Unit reciprocal produces new unit
        let new_unit = self.unit.clone().reciprocal().build();
        Quantity::new(self.value.clone().reciprocal(), new_unit)
    }
}

// ============================================================================
// Abs 运算符实现 / Abs operator implementations
// ============================================================================

impl<V> Abs for Quantity<V>
where
    V: Abs<Output = V>,
{
    type Output = Quantity<V>;

    fn abs(self) -> Self::Output {
        // 单位不变，只取值的绝对值
        // Unit remains the same, only take absolute value of the value
        Quantity::new(self.value.abs(), self.unit)
    }
}

impl<V> Abs for &Quantity<V>
where
    for<'a> &'a V: Abs<Output = V>,
{
    type Output = Quantity<V>;

    fn abs(self) -> Self::Output {
        // 单位不变，只取值的绝对值
        // Unit remains the same, only take absolute value of the value
        Quantity::new(self.value.abs(), self.unit.clone())
    }
}

// ============================================================================
// TolerancedEq 实现 / TolerancedEq implementation
// ============================================================================

impl<V> TolerancedEq for Quantity<V>
where
    V: TolerancedEq<Value = V>,
    for<'a> &'a V: Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    type Value = V;

    fn eq_within(&self, other: &Self, tolerance: &Tolerance<V>) -> bool {
        // 不同量纲不相等
        // Different dimensions are not equal
        if !self.unit.same_dimension(&other.unit) {
            return false;
        }

        // 转换到相同单位后比较值
        // Convert to same unit then compare values
        if self.unit == other.unit {
            // 使用 V 的 TolerancedEq 实现
            // Use V's TolerancedEq implementation
            self.value.eq_within(&other.value, &tolerance)
        } else {
            // 获取转换因子
            // Get conversion factor
            let factor = match self.unit.conversion_factor_to(&other.unit) {
                Some(f) => f,
                None => return false,
            };
            
            // BigDecimal 转换为 V 后进行乘法和比较
            // Convert BigDecimal to V, then multiply and compare
            let converted_value = &other.value * factor.into();
            self.value.eq_within(&converted_value, &tolerance)
        }
    }
}

// ============================================================================
// TolerancedOrd 实现 / TolerancedOrd implementation
// ============================================================================

impl<V> TolerancedOrd for Quantity<V>
where
    V: TolerancedOrd<Value = V>,
    for<'a> &'a V: Mul<V, Output = V>,
    BigDecimal: Into<V>,
{
    fn cmp_within(&self, other: &Self, tolerance: &Tolerance<V>) -> Ordering {
        // 不同量纲无法比较
        // Cannot compare different dimensions
        if !self.unit.same_dimension(&other.unit) {
            // 回退到默认行为：比较量纲符号
            // Fallback: compare dimension symbols
            return self.dimension().symbol().cmp(&other.dimension().symbol());
        }

        // 转换到相同单位后比较值
        // Convert to same unit then compare values
        if self.unit == other.unit {
            // 使用 V 的 TolerancedOrd 实现
            // Use V's TolerancedOrd implementation
            self.value.cmp_within(&other.value, &tolerance)
        } else {
            // 获取转换因子
            // Get conversion factor
            let factor = match self.unit.conversion_factor_to(&other.unit) {
                Some(f) => f,
                None => return Ordering::Equal,
            };
            
            // BigDecimal 转换为 V 后进行乘法和比较
            // Convert BigDecimal to V, then multiply and compare
            let converted_value = &other.value * factor.into();
            self.value.cmp_within(&converted_value, &tolerance)
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::{Kilogram, Kilometer, Meter, Second};
    use crate::unit::system::SI_SYSTEM;
    use crate::unit::{CTUnit, UnitSystemBuilder};
    use BigDecimal;

    // ========================================================================
    // 创建测试 / Creation tests
    // ========================================================================

    #[test]
    fn test_quantity_creation() {
        // 测试基本创建
        // Test basic creation
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        assert_eq!(length.value, BigDecimal::from(10));
        assert_eq!(length.unit.symbol(), "m");
        assert_eq!(length.dimension().symbol(), "L");
    }

    #[test]
    fn test_quantity_from_tuple() {
        // 测试从元组创建
        // Test creation from tuple
        let length: Quantity<BigDecimal> = (BigDecimal::from(10), Meter::INSTANT.clone()).into();
        assert_eq!(length.value, BigDecimal::from(10));
    }

    // ========================================================================
    // 单位转换测试 / Unit conversion tests
    // ========================================================================

    #[test]
    fn test_unit_conversion() {
        // 测试米到千米的转换
        // Test meter to kilometer conversion
        let length_m = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());
        let length_km = length_m.to_unit(&Kilometer::INSTANT.clone()).unwrap();
        assert_eq!(length_km.value, BigDecimal::from(1));
        assert_eq!(length_km.unit.symbol(), "km");
    }

    #[test]
    fn test_unit_conversion_same_unit() {
        // 测试相同单位的转换（应返回相同值）
        // Test conversion to same unit (should return same value)
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let converted = length.to_unit(&Meter::INSTANT.clone()).unwrap();
        assert_eq!(converted.value, BigDecimal::from(10));
    }

    #[test]
    fn test_unit_conversion_error() {
        // 测试不同量纲单位转换的错误
        // Test error when converting between different dimensions
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let kg_unit = Kilogram::INSTANT.clone();
        let result = length.to_unit(&kg_unit);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.msg().contains("Dimension mismatch"));
    }

    #[test]
    fn test_try_to_unit() {
        // 测试 try_to_unit 方法
        // Test try_to_unit method
        let length = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());
        let converted = length.try_to_unit(&Kilometer::INSTANT.clone());
        assert!(converted.is_some());
        assert_eq!(converted.unwrap().value, BigDecimal::from(1));

        // 不同量纲应返回 None
        // Different dimension should return None
        let failed = length.try_to_unit(&Kilogram::INSTANT.clone());
        assert!(failed.is_none());
    }

    // ========================================================================
    // 标准单位转换测试 / Standard unit conversion tests
    // ========================================================================

    #[test]
    fn test_to_standard_unit_si() {
        // 测试转换为 SI 单位制的标准单位
        // Test conversion to SI standard unit
        let length = Quantity::new(BigDecimal::from(100), Meter::INSTANT.clone());
        let standard = length.to_standard_unit(SI_SYSTEM.as_ref()).unwrap();
        assert_eq!(standard.unit.symbol(), "m"); // SI 中长度的标准单位是米
    }

    #[test]
    fn test_to_standard_unit_custom() {
        // 测试转换为自定义单位制的标准单位
        // Test conversion to custom unit system's standard unit
        let custom_system = UnitSystemBuilder::new("Custom")
            .with_base_unit(
                crate::dimension::FundamentalQuantityEnum::Length,
                Meter::INSTANT.clone(),
            )
            .with_standard_unit(
                Meter::INSTANT.dimension().clone(),
                Kilometer::INSTANT.clone(),
            )
            .build();

        let length = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());
        let standard = length.to_standard_unit(custom_system.as_ref()).unwrap();
        assert_eq!(standard.unit.symbol(), "km");
        assert_eq!(standard.value, BigDecimal::from(1));
    }

    // ========================================================================
    // 算术操作测试 / Arithmetic operation tests
    // ========================================================================

    #[test]
    fn test_quantity_add() {
        // 测试相同单位的加法
        // Test addition with same units
        let q1 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(5), Meter::INSTANT.clone());
        let sum = q1 + q2;
        assert_eq!(sum.value, BigDecimal::from(15));
        assert_eq!(sum.unit.symbol(), "m");
    }

    #[test]
    fn test_quantity_add_different_units() {
        // 测试不同单位（相同量纲）的加法
        // Test addition with different units (same dimension)
        let q1 = Quantity::new(BigDecimal::from(1), Kilometer::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(500), Meter::INSTANT.clone());
        let sum = q1 + q2;
        // 1 km + 500 m = 1.5 km
        assert_eq!(sum.unit.symbol(), "km");
        let expected: BigDecimal = "1.5".parse().unwrap();
        let diff = &sum.value - &expected;
        assert!(diff.abs() < BigDecimal::from(1) / BigDecimal::from(1000));
    }

    #[test]
    fn test_quantity_add_error() {
        // 测试不同量纲加法的错误
        // Test error for addition with different dimensions
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());
        let result = length.checked_add(&mass);
        assert!(result.is_err());
    }

    #[test]
    fn test_quantity_sub() {
        // 测试减法
        // Test subtraction
        let q1 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(3), Meter::INSTANT.clone());
        let diff = q1 - q2;
        assert_eq!(diff.value, BigDecimal::from(7));
    }

    #[test]
    fn test_quantity_neg() {
        // 测试取负
        // Test negation
        let q = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let negated = -q;
        assert_eq!(negated.value, BigDecimal::from(-10));
    }

    // ========================================================================
    // 标量运算测试 / Scalar operation tests
    // ========================================================================

    #[test]
    fn test_quantity_scalar_mul() {
        // 测试标量乘法
        // Test scalar multiplication
        let q = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let doubled = q * BigDecimal::from(2);
        assert_eq!(doubled.value, BigDecimal::from(20));
        assert_eq!(doubled.unit.symbol(), "m");
    }

    #[test]
    fn test_quantity_scalar_div() {
        // 测试标量除法
        // Test scalar division
        let q = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let halved = q / BigDecimal::from(2);
        assert_eq!(halved.value, BigDecimal::from(5));
        assert_eq!(halved.unit.symbol(), "m");
    }

    // ========================================================================
    // 物理量乘除测试 / Quantity multiplication/division tests
    // ========================================================================

    #[test]
    fn test_quantity_mul_quantity() {
        // 测试物理量相乘（产生新量纲）
        // Test quantity multiplication (produces new dimension)
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());

        let product = length * mass;
        assert_eq!(product.value, BigDecimal::from(50));
        // 量纲应该是 L·M
        // Dimension should be L·M
        assert!(product.dimension().symbol().contains("L"));
        assert!(product.dimension().symbol().contains("M"));
    }

    #[test]
    fn test_quantity_div_quantity() {
        // 测试物理量相除（产生新量纲）
        // Test quantity division (produces new dimension)
        let length = Quantity::new(BigDecimal::from(100), Meter::INSTANT.clone());
        let time = Quantity::new(BigDecimal::from(10), Second::INSTANT.clone());

        let velocity = length / time;
        assert_eq!(velocity.value, BigDecimal::from(10));
        // 量纲应该是 L·T^-1（速度）
        // Dimension should be L·T^-1 (velocity)
        let dim = velocity.dimension();
        let dim_symbol = dim.symbol();
        assert!(dim_symbol.contains("L"));
        assert!(dim_symbol.contains("T"));
    }

    // ========================================================================
    // 比较操作测试 / Comparison operation tests
    // ========================================================================

    #[test]
    fn test_quantity_eq() {
        // 测试相等比较
        // Test equality comparison
        let q1 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q3 = Quantity::new(BigDecimal::from(5), Meter::INSTANT.clone());

        assert_eq!(q1, q2);
        assert_ne!(q1, q3);
    }

    #[test]
    fn test_quantity_ord() {
        // 测试大小比较
        // Test ordering comparison
        let q1 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(5), Meter::INSTANT.clone());

        assert!(q1 > q2);
        assert!(q2 < q1);
    }

    #[test]
    fn test_quantity_ord_different_units() {
        // 测试不同单位的大小比较
        // Test ordering comparison with different units
        let q1 = Quantity::new(BigDecimal::from(1), Kilometer::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(500), Meter::INSTANT.clone());

        assert!(q1 > q2); // 1 km > 500 m
    }
}
