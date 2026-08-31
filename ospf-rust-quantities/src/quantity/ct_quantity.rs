//! CTQuantity - 编译时物理量
//! CTQuantity - Compile-time physical quantity
//!
//! 编译时物理量，单位作为类型参数，提供零成本抽象和编译时量纲检查。
//! Compile-time physical quantity with unit as type parameter, providing zero-cost abstraction and compile-time dimension checking.
//!
//! # 核心类型 / Core Types
//! - `CTQuantity<V, U>`: 编译时物理量，单位类型 `U` 在编译时确定
//!
//! # 特性 / Features
//! - 零成本抽象：单位类型在编译时完全确定
//! - 编译时量纲检查：不匹配的量纲操作会导致编译错误
//! - 泛型值类型支持
//! - 自动单位转换（带编译时量纲约束）
//!
//! # 示例 / Examples
//! ```
//! use ospf_rust_quantities::quantity::CTQuantity;
//! use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Second};
//! use ospf_rust_quantities::unit::CTUnit;
//! use bigdecimal::BigDecimal;
//!
//! // 创建编译时物理量
//! let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
//!
//! // 编译时单位转换（量纲检查在编译时完成）
//! let length_km: CTQuantity<BigDecimal, Kilometer> = length.to();
//! assert_eq!(length_km.value, BigDecimal::from(1) / BigDecimal::from(100));
//!
//! // 物理量除法产生新单位类型
//! let length2: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
//! let time: CTQuantity<BigDecimal, Second> = CTQuantity::new(BigDecimal::from(2));
//! let velocity = length2 / time; // 类型: CTQuantity<BigDecimal, CTUnitDiv<Meter, Second>>
//! ```

use std::ops::{Add, Sub, Mul, Div, Neg};
use std::marker::PhantomData;
use crate::unit::{CTUnit, ct_conversion_factor, CTUnitMul, CTUnitDiv, CTUnitReciprocal};
use crate::dimension::SameDerivedDimension;
use ospf_rust_math::operator::abs::Abs;
use ospf_rust_math::operator::reciprocal::Reciprocal;
use ospf_rust_math::operator::tolerance::{Tolerance, TolerancedEq, TolerancedOrd};
use bigdecimal::BigDecimal;
use std::cmp::Ordering;

// ============================================================================
// CTQuantity - 编译时物理量 / Compile-time physical quantity
// ============================================================================

/// CTQuantity - 编译时物理量
/// CTQuantity - Compile-time physical quantity
///
/// 包含值和编译时单位类型的物理量，提供零成本抽象。
/// Physical quantity with value and compile-time unit type, providing zero-cost abstraction.
///
/// 单位类型 `U` 在编译时确定，所有量纲检查都在编译时完成。
/// The unit type `U` is determined at compile time, and all dimension checks are done at compile time.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型，通常是 `BigDecimal` 或 `f64`
/// - `U`: 编译时单位类型，必须实现 `CTUnit` trait
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_quantities::quantity::CTQuantity;
/// use ospf_rust_quantities::unit::derived::Meter;
/// use ospf_rust_quantities::unit::CTUnit;
/// use bigdecimal::BigDecimal;
///
/// let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
/// assert_eq!(length.value, BigDecimal::from(10));
/// assert_eq!(CTQuantity::<BigDecimal, Meter>::unit_symbol(), "m");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct CTQuantity<V, U: CTUnit> {
    /// 物理量的值
    /// Value of the physical quantity
    pub value: V,
    /// 单位类型特征
    /// Unit type marker
    _unit: PhantomData<U>,
}

impl<V, U: CTUnit> CTQuantity<V, U> {
    /// 创建新的编译时物理量
    /// Create a new compile-time physical quantity
    ///
    /// # 参数 / Parameters
    /// - `value`: 物理量的值
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::CTQuantity;
    /// use ospf_rust_quantities::unit::derived::Meter;
    /// use bigdecimal::BigDecimal;
    ///
    /// let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
    /// ```
    pub fn new(value: V) -> Self {
        Self { 
            value, 
            _unit: PhantomData 
        }
    }
    
    /// 获取单位符号
    /// Get unit symbol
    ///
    /// 返回编译时单位类型的符号。
    /// Returns the symbol of the compile-time unit type.
    pub fn unit_symbol() -> &'static str {
        U::SYMBOL
    }
    
    /// 获取单位名称
    /// Get unit name
    ///
    /// 返回编译时单位类型的名称。
    /// Returns the name of the compile-time unit type.
    pub fn unit_name() -> &'static str {
        U::NAME
    }
}

// ============================================================================
// 单位转换 / Unit conversion
// ============================================================================

impl<V, U: CTUnit> CTQuantity<V, U>
where
    V: Clone + Mul<V, Output = V> + Div<V, Output = V>,
    bigdecimal::BigDecimal: Into<V> + From<V>,
{
    /// 转换到另一个单位类型（编译时检查量纲）
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
    ///
    /// # 约束 / Constraints
    /// - `U::Dimension: SameDerivedDimension<Target::Dimension>`: 编译时量纲相等约束
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_quantities::quantity::CTQuantity;
    /// use ospf_rust_quantities::unit::derived::{Meter, Kilometer};
    /// use bigdecimal::BigDecimal;
    ///
    /// let length_m: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(1000));
    /// let length_km: CTQuantity<BigDecimal, Kilometer> = length_m.to();
    /// assert_eq!(length_km.value, BigDecimal::from(1));
    /// ```
    pub fn to<Target: CTUnit>(self) -> CTQuantity<V, Target>
    where
        U::Dimension: SameDerivedDimension<Target::Dimension>,
    {
        let factor = ct_conversion_factor::<U, Target>();
        CTQuantity::new(self.value * factor.into())
    }
}

// ============================================================================
// 算术操作 / Arithmetic operations
// ============================================================================

impl<V, U: CTUnit> Add for CTQuantity<V, U>
where
    V: Add<Output = V>,
{
    type Output = CTQuantity<V, U>;
    
    fn add(self, other: Self) -> Self::Output {
        CTQuantity::new(self.value + other.value)
    }
}

impl<V, U: CTUnit> Sub for CTQuantity<V, U>
where
    V: Sub<Output = V>,
{
    type Output = CTQuantity<V, U>;
    
    fn sub(self, other: Self) -> Self::Output {
        CTQuantity::new(self.value - other.value)
    }
}

impl<V, U: CTUnit> Mul<V> for CTQuantity<V, U>
where
    V: Clone + Mul<Output = V>,
{
    type Output = CTQuantity<V, U>;
    
    fn mul(self, rhs: V) -> Self::Output {
        CTQuantity::new(self.value * rhs)
    }
}

impl<V, U: CTUnit> Div<V> for CTQuantity<V, U>
where
    V: Clone + Div<Output = V>,
{
    type Output = CTQuantity<V, U>;
    
    fn div(self, rhs: V) -> Self::Output {
        CTQuantity::new(self.value / rhs)
    }
}

impl<V, U: CTUnit> Neg for CTQuantity<V, U>
where
    V: Neg<Output = V>,
{
    type Output = CTQuantity<V, U>;
    
    fn neg(self) -> Self::Output {
        CTQuantity::new(-self.value)
    }
}

// ============================================================================
// 物理量乘除（产生新单位类型）/ Quantity multiplication/division (producing new unit types)
// ============================================================================

impl<V, U1: CTUnit, U2: CTUnit> Mul<CTQuantity<V, U2>> for CTQuantity<V, U1>
where
    V: Mul<Output = V>,
    CTUnitMul<U1, U2>: CTUnit,
{
    type Output = CTQuantity<V, CTUnitMul<U1, U2>>;
    
    fn mul(self, other: CTQuantity<V, U2>) -> Self::Output {
        CTQuantity::new(self.value * other.value)
    }
}

impl<V, U1: CTUnit, U2: CTUnit> Div<CTQuantity<V, U2>> for CTQuantity<V, U1>
where
    V: Div<Output = V>,
    CTUnitDiv<U1, U2>: CTUnit,
{
    type Output = CTQuantity<V, CTUnitDiv<U1, U2>>;
    
    fn div(self, other: CTQuantity<V, U2>) -> Self::Output {
        CTQuantity::new(self.value / other.value)
    }
}

// ============================================================================
// 引用版本的算术操作符 / Reference version arithmetic operators
// ============================================================================

impl<V, U: CTUnit> Add<&CTQuantity<V, U>> for &CTQuantity<V, U>
where
    V: Add<Output = V> + Clone,
{
    type Output = CTQuantity<V, U>;

    fn add(self, other: &CTQuantity<V, U>) -> Self::Output {
        CTQuantity::new(self.value.clone() + other.value.clone())
    }
}

impl<V, U: CTUnit> Sub<&CTQuantity<V, U>> for &CTQuantity<V, U>
where
    V: Sub<Output = V> + Clone,
{
    type Output = CTQuantity<V, U>;

    fn sub(self, other: &CTQuantity<V, U>) -> Self::Output {
        CTQuantity::new(self.value.clone() - other.value.clone())
    }
}

impl<V, U: CTUnit> Mul<&V> for &CTQuantity<V, U>
where
    V: Clone + Mul<Output = V>,
{
    type Output = CTQuantity<V, U>;

    fn mul(self, rhs: &V) -> Self::Output {
        CTQuantity::new(self.value.clone() * rhs.clone())
    }
}

impl<V, U: CTUnit> Div<&V> for &CTQuantity<V, U>
where
    V: Clone + Div<Output = V>,
{
    type Output = CTQuantity<V, U>;

    fn div(self, rhs: &V) -> Self::Output {
        CTQuantity::new(self.value.clone() / rhs.clone())
    }
}

impl<V, U: CTUnit> Neg for &CTQuantity<V, U>
where
    V: Neg<Output = V> + Clone,
{
    type Output = CTQuantity<V, U>;

    fn neg(self) -> Self::Output {
        CTQuantity::new(-self.value.clone())
    }
}

impl<V, U1: CTUnit, U2: CTUnit> Mul<&CTQuantity<V, U2>> for &CTQuantity<V, U1>
where
    V: Clone + Mul<Output = V>,
    CTUnitMul<U1, U2>: CTUnit,
{
    type Output = CTQuantity<V, CTUnitMul<U1, U2>>;

    fn mul(self, other: &CTQuantity<V, U2>) -> Self::Output {
        CTQuantity::new(self.value.clone() * other.value.clone())
    }
}

impl<V, U1: CTUnit, U2: CTUnit> Div<&CTQuantity<V, U2>> for &CTQuantity<V, U1>
where
    V: Clone + Div<Output = V>,
    CTUnitDiv<U1, U2>: CTUnit,
{
    type Output = CTQuantity<V, CTUnitDiv<U1, U2>>;

    fn div(self, other: &CTQuantity<V, U2>) -> Self::Output {
        CTQuantity::new(self.value.clone() / other.value.clone())
    }
}

// ============================================================================
// 值类型转换 / Value type conversion
// ============================================================================

impl<V, U: CTUnit> CTQuantity<V, U> {
    /// 转换值类型
    /// Convert value type
    ///
    /// 使用提供的函数将值从一种类型转换为另一种类型。
    /// Converts the value from one type to another using the provided function.
    ///
    /// # 参数 / Parameters
    /// - `f`: 值转换函数
    pub fn map_value<T, F>(self, f: F) -> CTQuantity<T, U>
    where
        F: FnOnce(V) -> T,
    {
        CTQuantity::new(f(self.value))
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
    /// - `Some(CTQuantity<T, U>)`: 转换成功
    /// - `None`: 转换失败
    pub fn try_map_value<T, F, E>(self, f: F) -> Option<CTQuantity<T, U>>
    where
        F: FnOnce(V) -> Result<T, E>,
    {
        Some(CTQuantity::new(f(self.value).ok()?))
    }
}

impl<V: Clone, U: CTUnit> CTQuantity<V, U> {
    /// 转换值类型（引用版本）
    /// Convert value type (reference version)
    ///
    /// 使用提供的函数将值从一种类型转换为另一种类型。
    /// Converts the value from one type to another using the provided function.
    ///
    /// # 参数 / Parameters
    /// - `f`: 值转换函数
    pub fn map_value_ref<T, F>(&self, f: F) -> CTQuantity<T, U>
    where
        F: FnOnce(&V) -> T,
    {
        CTQuantity::new(f(&self.value))
    }
}

impl<V, U: CTUnit> AsRef<CTQuantity<V, U>> for CTQuantity<V, U> {
    fn as_ref(&self) -> &CTQuantity<V, U> {
        self
    }
}

// ============================================================================
// Reciprocal 运算符实现 / Reciprocal operator implementations
// ============================================================================

impl<V, U: CTUnit> Reciprocal for CTQuantity<V, U>
where
    V: Reciprocal<Output = V>,
    CTUnitReciprocal<U>: CTUnit,
{
    type Output = CTQuantity<V, CTUnitReciprocal<U>>;

    fn reciprocal(self) -> Self::Output {
        CTQuantity::new(self.value.reciprocal())
    }
}

impl<V, U: CTUnit> Reciprocal for &CTQuantity<V, U>
where
    for<'a> &'a V: Reciprocal<Output = V>,
    CTUnitReciprocal<U>: CTUnit,
{
    type Output = CTQuantity<V, CTUnitReciprocal<U>>;

    fn reciprocal(self) -> Self::Output {
        CTQuantity::new(self.value.reciprocal())
    }
}

// ============================================================================
// Abs 运算符实现 / Abs operator implementations
// ============================================================================

impl<V, U: CTUnit> Abs for CTQuantity<V, U>
where
    V: Abs<Output = V>,
{
    type Output = CTQuantity<V, U>;

    fn abs(self) -> Self::Output {
        // 单位不变，只取值的绝对值
        // Unit remains the same, only take absolute value of the value
        CTQuantity::new(self.value.abs())
    }
}

impl<V, U: CTUnit> Abs for &CTQuantity<V, U>
where
    for<'a> &'a V: Abs<Output = V>,
{
    type Output = CTQuantity<V, U>;

    fn abs(self) -> Self::Output {
        // 单位不变，只取值的绝对值
        // Unit remains the same, only take absolute value of the value
        CTQuantity::new(self.value.abs())
    }
}

// ============================================================================
// TolerancedEq 实现 / TolerancedEq implementation
// ============================================================================

impl<V, U: CTUnit> TolerancedEq for CTQuantity<V, U>
where
    V: TolerancedEq<Value = V>,
{
    type Value = V;

    fn eq_within(&self, other: &Self, tolerance: &Tolerance<V>) -> bool {
        // 编译时单位类型相同，直接比较值
        // Compile-time unit type is the same, directly compare values
        self.value.eq_within(&other.value, &tolerance)
    }
}

// ============================================================================
// TolerancedOrd 实现 / TolerancedOrd implementation
// ============================================================================

impl<V, U: CTUnit> TolerancedOrd for CTQuantity<V, U>
where
    V: TolerancedOrd<Value = V>,
{
    fn cmp_within(&self, other: &Self, tolerance: &Tolerance<V>) -> Ordering {
        // 编译时单位类型相同，直接比较值
        // Compile-time unit type is the same, directly compare values
        self.value.cmp_within(&other.value, &tolerance)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::{Meter, Kilometer, Second, Kilogram};
    use bigdecimal::BigDecimal;

    // ========================================================================
    // 创建测试 / Creation tests
    // ========================================================================

    #[test]
    fn test_ct_quantity_creation() {
        // 测试基本创建
        // Test basic creation
        let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        assert_eq!(length.value, BigDecimal::from(10));
        assert_eq!(CTQuantity::<BigDecimal, Meter>::unit_symbol(), "m");
        assert_eq!(CTQuantity::<BigDecimal, Meter>::unit_name(), "meter");
    }

    // ========================================================================
    // 单位转换测试 / Unit conversion tests
    // ========================================================================

    #[test]
    fn test_ct_quantity_unit_conversion() {
        // 测试编译时单位转换
        // Test compile-time unit conversion
        let length_m: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(1000));
        let length_km: CTQuantity<BigDecimal, Kilometer> = length_m.to();
        assert_eq!(length_km.value, BigDecimal::from(1));
    }

    #[test]
    fn test_ct_quantity_unit_conversion_reverse() {
        // 测试反向单位转换
        // Test reverse unit conversion
        let length_km: CTQuantity<BigDecimal, Kilometer> = CTQuantity::new(BigDecimal::from(1));
        let length_m: CTQuantity<BigDecimal, Meter> = length_km.to();
        assert_eq!(length_m.value, BigDecimal::from(1000));
    }

    // ========================================================================
    // 算术操作测试 / Arithmetic operation tests
    // ========================================================================

    #[test]
    fn test_ct_quantity_add() {
        // 测试加法
        // Test addition
        let q1: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let q2: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(5));
        let sum = q1 + q2;
        assert_eq!(sum.value, BigDecimal::from(15));
    }

    #[test]
    fn test_ct_quantity_sub() {
        // 测试减法
        // Test subtraction
        let q1: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let q2: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(3));
        let diff = q1 - q2;
        assert_eq!(diff.value, BigDecimal::from(7));
    }

    #[test]
    fn test_ct_quantity_neg() {
        // 测试取负
        // Test negation
        let q: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let negated = -q;
        assert_eq!(negated.value, BigDecimal::from(-10));
    }

    // ========================================================================
    // 标量运算测试 / Scalar operation tests
    // ========================================================================

    #[test]
    fn test_ct_quantity_scalar_mul() {
        // 测试标量乘法
        // Test scalar multiplication
        let q: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let doubled = q * BigDecimal::from(2);
        assert_eq!(doubled.value, BigDecimal::from(20));
    }

    #[test]
    fn test_ct_quantity_scalar_div() {
        // 测试标量除法
        // Test scalar division
        let q: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let halved = q / BigDecimal::from(2);
        assert_eq!(halved.value, BigDecimal::from(5));
    }

    // ========================================================================
    // 物理量乘除测试 / Quantity multiplication/division tests
    // ========================================================================

    #[test]
    fn test_ct_quantity_mul_different_units() {
        // 测试不同单位物理量相乘（产生新单位类型）
        // Test multiplication of quantities with different units (produces new unit type)
        let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let mass: CTQuantity<BigDecimal, Kilogram> = CTQuantity::new(BigDecimal::from(5));
        
        // 乘法产生新单位类型: m * kg
        // Multiplication produces new unit type: m * kg
        let product = length * mass;
        assert_eq!(product.value, BigDecimal::from(50));
    }

    #[test]
    fn test_ct_quantity_div() {
        // 测试物理量相除（产生新单位类型）
        // Test division of quantities (produces new unit type)
        let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(100));
        let time: CTQuantity<BigDecimal, Second> = CTQuantity::new(BigDecimal::from(10));
        
        // 除法产生速度单位: m / s
        // Division produces velocity unit: m / s
        let velocity = length / time;
        assert_eq!(velocity.value, BigDecimal::from(10));
    }

    #[test]
    fn test_ct_quantity_mul_same_unit() {
        // 测试相同单位物理量相乘（如面积）
        // Test multiplication of quantities with same unit (e.g., area)
        let length1: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(5));
        let length2: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(4));
        
        // 面积 = 长 * 宽
        // Area = length * width
        let area = length1 * length2;
        assert_eq!(area.value, BigDecimal::from(20));
    }

    // ========================================================================
    // 比较操作测试 / Comparison operation tests
    // ========================================================================

    #[test]
    fn test_ct_quantity_eq() {
        // 测试相等比较
        // Test equality comparison
        let q1: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let q2: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
        let q3: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(5));
        
        assert_eq!(q1, q2);
        assert_ne!(q1, q3);
    }
}
