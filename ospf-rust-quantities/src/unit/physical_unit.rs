//! Physical unit - 物理单位
//! Physical unit - Physical units
//!
//! 提供运行时和编译时的单位表示，实现零成本抽象。
//! Provides runtime and compile-time unit representations, achieving zero-cost abstraction.
//!
//! # 运行时单位 / Runtime Units
//! - `Unit`: 运行时单位，包含名称、符号、量纲和比例尺
//! - `UnitBuilder`: 构建器模式，用于链式构建单位
//!
//! # 编译时单位 / Compile-time Units
//! - `CTUnit`: 编译时单位 trait，所有编译时单位类型必须实现
//! - `CTUnitMul`, `CTUnitDiv`, `CTUnitPow`, `CTUnitReciprocal`: 编译时单位运算类型
//!
//! # 单位转换 / Unit Conversion
//! - `ct_conversion_factor`: 编译时单位转换系数计算，带量纲检查
//! - `SameUnit`: 编译时单位相等约束

use crate::dimension::derived_quantity::{
    CTDerivedDiv, CTDerivedMul, CTDerivedPow, CTDerivedQuantity, CTDerivedReciprocal,
    DerivedQuantity, DerivedQuantityBuilder, SameDerivedDimension,
};
use crate::scale::Scale;
use crate::unit::concept::UnitTrait;
use bigdecimal::BigDecimal;
use once_cell::sync::Lazy;
use ospf_rust_math::operator::reciprocal::Reciprocal;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Div, Mul};
use std::sync::Arc;
use typenum::Integer;
// ============================================================================
// 运行时单位数据结构 / Runtime unit data structure
// ============================================================================

/// UnitInner - 单位内部数据
/// UnitInner - Unit internal data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitInner {
    /// 单位名称 / Unit name
    name: String,
    /// 单位符号 / Unit symbol
    symbol: String,
    /// 单位量纲 / Unit dimension
    dimension: DerivedQuantity,
    /// 相对于基本单位的比例尺 / Scale relative to base unit
    scale: Scale,
}

/// Unit - 运行时单位
/// Unit - Runtime unit
///
/// 单位的运行时表示，包含名称、符号、量纲和比例尺
/// Runtime representation of a unit, containing name, symbol, dimension and scale
#[derive(Debug, Clone)]
pub struct Unit {
    inner: Arc<UnitInner>,
}

impl Unit {
    /// 创建单位
    /// Create unit
    pub fn new(name: String, symbol: String, dimension: DerivedQuantity, scale: Scale) -> Self {
        Self {
            inner: Arc::new(UnitInner {
                name,
                symbol,
                dimension,
                scale,
            }),
        }
    }

    /// 获取名称 / Get name
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// 获取符号 / Get symbol
    pub fn symbol(&self) -> &str {
        &self.inner.symbol
    }

    /// 获取量纲 / Get dimension
    pub fn dimension(&self) -> &DerivedQuantity {
        &self.inner.dimension
    }

    /// 获取比例尺 / Get scale
    pub fn scale(&self) -> &Scale {
        &self.inner.scale
    }

    /// 检查量纲是否相等
    /// Check if dimensions are equal
    pub fn same_dimension(&self, other: &Unit) -> bool {
        self.inner.dimension == other.inner.dimension
    }

    /// 计算到另一个单位的转换系数
    /// Calculate conversion factor to another unit
    ///
    /// 返回 None 如果量纲不匹配
    /// Returns None if dimensions don't match
    pub fn conversion_factor_to(&self, other: &Unit) -> Option<BigDecimal> {
        if !self.same_dimension(other) {
            return None;
        }
        Some(self.inner.scale.value() / other.inner.scale.value())
    }
}

pub struct UnitBuilder {
    name: Option<String>,
    symbol: Option<String>,
    dimension: DerivedQuantityBuilder,
    scale: Scale,
}

impl UnitBuilder {
    pub fn new(dimension: DerivedQuantityBuilder, scale: Scale) -> Self {
        Self {
            name: None,
            symbol: None,
            dimension,
            scale,
        }
    }

    pub fn new_by(dimension: &DerivedQuantity, scale: Scale) -> Self {
        Self {
            name: None,
            symbol: None,
            dimension: DerivedQuantityBuilder::new(dimension.powers().cloned().collect()),
            scale,
        }
    }

    pub fn build(self) -> Unit {
        let name = self.name.unwrap_or_else(|| "".to_string());
        let symbol = self.symbol.unwrap_or_else(|| "".to_string());
        Unit::new(name, symbol, self.dimension.into(), self.scale)
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn symbol(&mut self, symbol: &str) -> &mut Self {
        self.symbol = Some(symbol.to_string());
        self
    }
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.inner.dimension == other.inner.dimension && self.inner.scale == other.inner.scale
    }
}

impl Eq for Unit {}

impl std::hash::Hash for Unit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.name.hash(state);
        self.inner.symbol.hash(state);
        self.inner.dimension.hash(state);
        // Scale doesn't implement Hash, use value string representation
        // Scale 没有实现 Hash，使用值的字符串表示
        self.inner.scale.value().to_string().hash(state);
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.symbol)
    }
}

impl From<UnitBuilder> for Unit {
    fn from(builder: UnitBuilder) -> Self {
        builder.build()
    }
}

impl Mul for Unit {
    type Output = UnitBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        UnitBuilder::new(
            &self.inner.dimension * &rhs.inner.dimension,
            &self.inner.scale * &rhs.inner.scale,
        )
    }
}

impl Mul for &Unit {
    type Output = UnitBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        UnitBuilder::new(
            &self.inner.dimension * &rhs.inner.dimension,
            &self.inner.scale * &rhs.inner.scale,
        )
    }
}

impl Mul for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        UnitBuilder::new(self.dimension * rhs.dimension, self.scale * rhs.scale)
    }
}

impl Mul<Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: Unit) -> Self::Output {
        UnitBuilder::new(
            self.dimension * &rhs.inner.dimension,
            self.scale * &rhs.inner.scale,
        )
    }
}

impl Mul<&Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: &Unit) -> Self::Output {
        UnitBuilder::new(
            self.dimension * &rhs.inner.dimension,
            self.scale * &rhs.inner.scale,
        )
    }
}

impl Div for Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        UnitBuilder::new(
            &self.inner.dimension / &rhs.inner.dimension,
            &self.inner.scale / &rhs.inner.scale,
        )
    }
}

impl Div for &Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        UnitBuilder::new(
            &self.inner.dimension / &rhs.inner.dimension,
            &self.inner.scale / &rhs.inner.scale,
        )
    }
}

impl Div for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        UnitBuilder::new(self.dimension / rhs.dimension, self.scale / rhs.scale)
    }
}

impl Div<Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: Unit) -> Self::Output {
        UnitBuilder::new(
            self.dimension / &rhs.inner.dimension,
            self.scale / &rhs.inner.scale,
        )
    }
}

impl Div<&Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: &Unit) -> Self::Output {
        UnitBuilder::new(
            self.dimension / &rhs.inner.dimension,
            self.scale / &rhs.inner.scale,
        )
    }
}

impl Mul<i64> for Unit {
    type Output = UnitBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        UnitBuilder::new_by(&self.inner.dimension, &self.inner.scale * rhs)
    }
}

impl<'a> Mul<i64> for &'a Unit {
    type Output = UnitBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        UnitBuilder::new_by(&self.inner.dimension, &self.inner.scale * rhs)
    }
}

impl Mul<i64> for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        UnitBuilder::new(self.dimension * rhs, self.scale * rhs)
    }
}

impl Div<i64> for Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        UnitBuilder::new_by(&self.inner.dimension, &self.inner.scale / rhs)
    }
}

impl<'a> Div<i64> for &'a Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        UnitBuilder::new_by(&self.inner.dimension.clone(), &self.inner.scale / rhs)
    }
}

impl Div<i64> for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        UnitBuilder::new(self.dimension / rhs, self.scale / rhs)
    }
}

impl Reciprocal for Unit {
    type Output = UnitBuilder;

    fn reciprocal(self) -> Self::Output {
        UnitBuilder::new(
            (&self.inner.dimension).reciprocal().into(),
            (&self.inner.scale).reciprocal(),
        )
    }
}

impl Reciprocal for &Unit {
    type Output = UnitBuilder;

    fn reciprocal(self) -> Self::Output {
        UnitBuilder::new(
            (&self.inner.dimension).reciprocal().into(),
            (&self.inner.scale).reciprocal(),
        )
    }
}

// ============================================================================
// UnitTrait 实现 / UnitTrait implementations
// ============================================================================

/// 为 Unit 实现 UnitTrait
/// Implement UnitTrait for Unit
impl UnitTrait for Unit {
    type Dimension = DerivedQuantity;

    fn symbol(&self) -> &str {
        &self.inner.symbol
    }

    fn name(&self) -> &str {
        &self.inner.name
    }

    fn dimension_symbol(&self) -> String {
        self.inner.dimension.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        self.inner.scale.value().clone()
    }
}

// ============================================================================
// 编译时单位 trait / Compile-time unit trait
// ============================================================================

/// CTUnit - 编译时单位 trait
/// CTUnit - Compile-time unit trait
///
/// 所有编译时单位类型必须实现此 trait
/// All compile-time unit types must implement this trait
///
/// # 注意 / Note
/// 实现 `CTUnit` 的类型必须同时实现 `UnitTrait`
/// Types implementing `CTUnit` must also implement `UnitTrait`
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::unit::physical_unit::CTUnit;
/// use ospf_rust_quantities::unit::concept::UnitTrait;
/// use ospf_rust_quantities::dimension::derived::Length;
/// use ospf_rust_quantities::dimension::CTDerivedQuantity;
/// use ospf_rust_quantities::scale::Scale;
/// use once_cell::sync::Lazy;
/// use bigdecimal::BigDecimal;
///
/// struct Meter;
///
/// impl CTUnit for Meter {
///     const NAME: &'static str = "meter";
///     const SYMBOL: &'static str = "m";
///     type Dimension = Length;
/// }
///
/// impl UnitTrait for Meter {
///     type Dimension = Length;
///     fn symbol(&self) -> &str { "m" }
///     fn name(&self) -> &str { "meter" }
///     fn dimension_symbol(&self) -> String { Length::INSTANT.symbol().to_string() }
///     fn scale_value(&self) -> BigDecimal { BigDecimal::from(1) }
/// }
/// ```
pub trait CTUnit: UnitTrait {
    /// 单位名称 / Unit name
    const NAME: &'static str = "";

    /// 单位符号 / Unit symbol
    const SYMBOL: &'static str = "";

    /// 单位比例尺 / Unit scale
    const SCALE: Lazy<Scale> = Lazy::new(|| Scale::new());

    /// 单位量纲类型 / Unit dimension type
    type Dimension: CTDerivedQuantity;

    /// 运行时单位实例 / Runtime unit instance
    const INSTANT: Lazy<Unit> = Lazy::new(|| {
        Unit::new(
            Self::NAME.to_string(),
            Self::SYMBOL.to_string(),
            <Self as CTUnit>::Dimension::INSTANT.clone(),
            Self::SCALE.clone(),
        )
    });

    /// 检查是否与另一个单位类型量纲相等（运行时比较）
    /// Check if dimension equal to another unit type (runtime comparison)
    fn dim_eq<U: CTUnit>() -> bool {
        <Self as CTUnit>::Dimension::INSTANT.symbol() == <U as CTUnit>::Dimension::INSTANT.symbol()
    }

    /// 检查是否与另一个单位类型完全相等（量纲和比例尺，运行时比较）
    /// Check if fully equal to another unit type (dimension and scale, runtime comparison)
    fn unit_eq<U: CTUnit>() -> bool {
        Self::dim_eq::<U>() && *Self::SCALE == *U::SCALE
    }

    /// 计算到另一个单位的转换系数（运行时计算）
    /// Calculate conversion factor to another unit (runtime calculation)
    ///
    /// 返回 None 如果量纲不匹配
    /// Returns None if dimensions don't match
    fn conversion_factor_to<U: CTUnit>() -> Option<BigDecimal> {
        if !Self::dim_eq::<U>() {
            return None;
        }
        Some(Self::SCALE.value() / U::SCALE.value())
    }
}

// ============================================================================
// 编译时单位转换（编译时量纲检查）/ Compile-time unit conversion (compile-time dimension check)
// ============================================================================

/// 编译时单位转换系数计算
/// Compile-time unit conversion factor calculation
///
/// 使用 `SameDerivedDimension` trait 约束，只有量纲相同的单位才能转换
/// Uses `SameDerivedDimension` trait constraint, only units with same dimension can convert
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::unit::physical_unit::{ct_conversion_factor, CTUnit};
/// use ospf_rust_quantities::unit::derived::length::{Meter, Kilometer};
///
/// // 编译通过：米到千米可以转换
/// // Compiles: meter to kilometer can convert
/// let factor = ct_conversion_factor::<Meter, Kilometer>();
/// // factor = 0.001 (1 m = 0.001 km)
/// ```
pub fn ct_conversion_factor<From: CTUnit, To: CTUnit>() -> BigDecimal
where
    <From as CTUnit>::Dimension: SameDerivedDimension<<To as CTUnit>::Dimension>,
{
    let from_binding = &From::SCALE;
    let from_scale = from_binding.value();
    let to_binding = &To::SCALE;
    let to_scale = to_binding.value();
    from_scale / to_scale
}

/// SameUnit - 编译时单位相等 trait
/// SameUnit - Compile-time unit equality trait
///
/// 用于在编译时检查两个单位类型是否相同（量纲和比例尺都相同）
/// Used to check if two unit types are the same at compile-time (same dimension and scale)
pub trait SameUnit<U: CTUnit>: CTUnit {}

// 自反实现 / Reflexive implementation
impl<U: CTUnit> SameUnit<U> for U {}

// ============================================================================
// 编译时单位类型运算 / Compile-time unit type operations
// ============================================================================

pub struct CTUnitMul<U1: CTUnit, U2: CTUnit> {
    _marker: PhantomData<(U1, U2)>,
}

impl<U1: CTUnit, U2: CTUnit> Default for CTUnitMul<U1, U2> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<U1: CTUnit, U2: CTUnit> UnitTrait for CTUnitMul<U1, U2>
where
    CTDerivedMul<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>: CTDerivedQuantity,
{
    type Dimension = CTDerivedMul<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>;

    fn symbol(&self) -> &'static str {
        "" // 复合单位无简短符号 / Compound unit has no short symbol
    }

    fn name(&self) -> &'static str {
        "" // 复合单位无简短名称 / Compound unit has no short name
    }

    fn dimension_symbol(&self) -> String {
        <CTDerivedMul<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension> as CTDerivedQuantity>::INSTANT.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        Self::SCALE.value().clone()
    }
}

impl<U1: CTUnit, U2: CTUnit> CTUnit for CTUnitMul<U1, U2>
where
    CTDerivedMul<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>: CTDerivedQuantity,
{
    const SCALE: Lazy<Scale> = Lazy::new(|| &*U1::SCALE * &*U2::SCALE);
    type Dimension = CTDerivedMul<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>;
}

pub struct CTUnitDiv<U1: CTUnit, U2: CTUnit> {
    _marker: PhantomData<(U1, U2)>,
}

impl<U1: CTUnit, U2: CTUnit> Default for CTUnitDiv<U1, U2> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<U1: CTUnit, U2: CTUnit> UnitTrait for CTUnitDiv<U1, U2>
where
    CTDerivedDiv<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>: CTDerivedQuantity,
{
    type Dimension = CTDerivedDiv<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>;

    fn symbol(&self) -> &'static str {
        ""
    }

    fn name(&self) -> &'static str {
        ""
    }

    fn dimension_symbol(&self) -> String {
        <CTDerivedDiv<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension> as CTDerivedQuantity>::INSTANT.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        Self::SCALE.value().clone()
    }
}

impl<U1: CTUnit, U2: CTUnit> CTUnit for CTUnitDiv<U1, U2>
where
    CTDerivedDiv<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>: CTDerivedQuantity,
{
    const SCALE: Lazy<Scale> = Lazy::new(|| &*U1::SCALE / &*U2::SCALE);
    type Dimension = CTDerivedDiv<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>;
}

pub struct CTUnitPow<U: CTUnit, N: Integer> {
    _marker: PhantomData<(U, N)>,
}

impl<U: CTUnit, N: Integer> Default for CTUnitPow<U, N> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<U: CTUnit, N: Integer> UnitTrait for CTUnitPow<U, N>
where
    CTDerivedPow<<U as CTUnit>::Dimension, N>: CTDerivedQuantity,
{
    type Dimension = CTDerivedPow<<U as CTUnit>::Dimension, N>;

    fn symbol(&self) -> &'static str {
        ""
    }

    fn name(&self) -> &'static str {
        ""
    }

    fn dimension_symbol(&self) -> String {
        <CTDerivedPow<<U as CTUnit>::Dimension, N> as CTDerivedQuantity>::INSTANT.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        Self::SCALE.value().clone()
    }
}

impl<U: CTUnit, N: Integer> CTUnit for CTUnitPow<U, N>
where
    CTDerivedPow<<U as CTUnit>::Dimension, N>: CTDerivedQuantity,
{
    const SCALE: Lazy<Scale> = Lazy::new(|| U::SCALE.clone().pow(&BigDecimal::from(N::I64)));
    type Dimension = CTDerivedPow<<U as CTUnit>::Dimension, N>;
}

pub struct CTUnitReciprocal<U: CTUnit> {
    _marker: PhantomData<U>,
}

impl<U: CTUnit> Default for CTUnitReciprocal<U> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<U: CTUnit> UnitTrait for CTUnitReciprocal<U>
where
    CTDerivedReciprocal<<U as CTUnit>::Dimension>: CTDerivedQuantity,
{
    type Dimension = CTDerivedReciprocal<<U as CTUnit>::Dimension>;

    fn symbol(&self) -> &'static str {
        ""
    }

    fn name(&self) -> &'static str {
        ""
    }

    fn dimension_symbol(&self) -> String {
        <CTDerivedReciprocal<<U as CTUnit>::Dimension> as CTDerivedQuantity>::INSTANT.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        Self::SCALE.value().clone()
    }
}

impl<U: CTUnit> CTUnit for CTUnitReciprocal<U>
where
    CTDerivedReciprocal<<U as CTUnit>::Dimension>: CTDerivedQuantity,
{
    const SCALE: Lazy<Scale> = Lazy::new(|| U::SCALE.clone().reciprocal());
    type Dimension = CTDerivedReciprocal<<U as CTUnit>::Dimension>;
}

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::derived::{Length, Time};
    use crate::scale::Scale;
    use once_cell::sync::Lazy;

    /// 测试用长度单位 / Test length unit
    struct TestMeter;

    impl UnitTrait for TestMeter {
        type Dimension = Length;

        fn symbol(&self) -> &'static str { "m" }
        fn name(&self) -> &'static str { "meter" }
        fn dimension_symbol(&self) -> String { Length::INSTANT.symbol().to_string() }
        fn scale_value(&self) -> BigDecimal { BigDecimal::from(1) }
    }

    impl CTUnit for TestMeter {
        const NAME: &'static str = "meter";
        const SYMBOL: &'static str = "m";
        const SCALE: Lazy<Scale> = Lazy::new(|| Scale::new());
        type Dimension = Length;
    }

    /// 测试用千米单位 / Test kilometer unit
    struct TestKilometer;

    impl UnitTrait for TestKilometer {
        type Dimension = Length;

        fn symbol(&self) -> &'static str { "km" }
        fn name(&self) -> &'static str { "kilometer" }
        fn dimension_symbol(&self) -> String { Length::INSTANT.symbol().to_string() }
        fn scale_value(&self) -> BigDecimal { BigDecimal::from(1000) }
    }

    impl CTUnit for TestKilometer {
        const NAME: &'static str = "kilometer";
        const SYMBOL: &'static str = "km";
        const SCALE: Lazy<Scale> = Lazy::new(|| Scale::from_int(1000));
        type Dimension = Length;
    }

    /// 测试用秒单位 / Test second unit
    struct TestSecond;

    impl UnitTrait for TestSecond {
        type Dimension = Time;

        fn symbol(&self) -> &'static str { "s" }
        fn name(&self) -> &'static str { "second" }
        fn dimension_symbol(&self) -> String { Time::INSTANT.symbol().to_string() }
        fn scale_value(&self) -> BigDecimal { BigDecimal::from(1) }
    }

    impl CTUnit for TestSecond {
        const NAME: &'static str = "second";
        const SYMBOL: &'static str = "s";
        const SCALE: Lazy<Scale> = Lazy::new(|| Scale::new());
        type Dimension = Time;
    }

    #[test]
    fn test_unit_creation() {
        let meter = TestMeter::INSTANT.clone();
        assert_eq!(meter.name(), "meter");
        assert_eq!(meter.symbol(), "m");
    }

    #[test]
    fn test_unit_scale() {
        let km = TestKilometer::INSTANT.clone();
        assert_eq!(km.scale().value(), &BigDecimal::from(1000));
    }

    #[test]
    fn test_dimension_equality() {
        // 米和千米有相同量纲
        assert!(TestMeter::dim_eq::<TestKilometer>());
        // 米和秒有不同量纲
        assert!(!TestMeter::dim_eq::<TestSecond>());
    }

    #[test]
    fn test_unit_equality() {
        // 米和千米量纲相同但比例尺不同
        assert!(!TestMeter::unit_eq::<TestKilometer>());
        // 米和米完全相同
        assert!(TestMeter::unit_eq::<TestMeter>());
    }

    #[test]
    fn test_conversion_factor() {
        let factor = TestMeter::conversion_factor_to::<TestKilometer>();
        assert!(factor.is_some());
        // 1 m = 0.001 km
        assert_eq!(
            factor.unwrap(),
            BigDecimal::from(BigDecimal::new(1.into(), 3))
        );

        // 米到秒无法转换
        let factor = TestMeter::conversion_factor_to::<TestSecond>();
        assert!(factor.is_none());
    }

    #[test]
    fn test_unit_display() {
        let meter = TestMeter::INSTANT.clone();
        let display = format!("{}", meter);
        assert_eq!(display, "m");
    }

    #[test]
    fn test_same_unit_trait() {
        fn assert_same_unit<U1: CTUnit + SameUnit<U2>, U2: CTUnit>() {}

        // TestMeter 和 TestMeter 相同
        assert_same_unit::<TestMeter, TestMeter>();
    }
}
