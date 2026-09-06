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
    DerivedQuantity, DerivedQuantityBuilder, QuantityDomain, SameDerivedDimension,
};
use crate::scale::Scale;
use crate::unit::concept::UnitTrait;
use crate::unit::conversion_value::{UnitConversionCalculation, UnitConversionValue};
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

/// UnitConversionRule - 单位转换规则
/// UnitConversionRule - Unit conversion rule
///
/// 约定标准值为 `value * scale + offset`。
/// Standard value is defined as `value * scale + offset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitConversionRule {
    /// 线性转换 / Linear conversion
    Linear { scale: Scale },
    /// 仿射转换 / Affine conversion
    Affine { scale: Scale, offset: BigDecimal },
}

impl UnitConversionRule {
    /// 创建线性转换规则 / Create a linear conversion rule
    pub fn linear(scale: Scale) -> Self {
        Self::Linear { scale }
    }

    /// 创建仿射转换规则 / Create an affine conversion rule
    pub fn affine(scale: Scale, offset: BigDecimal) -> Self {
        Self::Affine { scale, offset }
    }

    /// 获取线性比例部分 / Get the linear scale part
    pub fn scale(&self) -> &Scale {
        match self {
            Self::Linear { scale } | Self::Affine { scale, .. } => scale,
        }
    }

    /// 获取 offset 部分 / Get the offset part
    pub fn offset(&self) -> BigDecimal {
        match self {
            Self::Linear { .. } => BigDecimal::from(0),
            Self::Affine { offset, .. } => offset.clone(),
        }
    }

    /// 是否为线性转换 / Whether the rule is linear
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::Linear { .. })
    }
}

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
    /// 单位取值域 / Unit value domain
    domain: QuantityDomain,
    /// 到标准单位的转换规则 / Conversion rule to the standard unit
    conversion: UnitConversionRule,
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
    /// 创建单位 / Create unit
    pub fn new(name: String, symbol: String, dimension: DerivedQuantity, scale: Scale) -> Self {
        let domain = dimension.domain();
        Self::new_with_conversion_and_domain(
            name,
            symbol,
            dimension,
            UnitConversionRule::linear(scale),
            domain,
        )
    }

    /// 使用指定取值域创建单位 / Create unit with a specified value domain
    pub fn new_with_domain(
        name: String,
        symbol: String,
        dimension: DerivedQuantity,
        scale: Scale,
        domain: QuantityDomain,
    ) -> Self {
        Self::new_with_conversion_and_domain(
            name,
            symbol,
            dimension,
            UnitConversionRule::linear(scale),
            domain,
        )
    }

    /// 使用转换规则创建单位 / Create unit with conversion rule
    pub fn new_with_conversion(
        name: String,
        symbol: String,
        dimension: DerivedQuantity,
        conversion: UnitConversionRule,
    ) -> Self {
        let domain = dimension.domain();
        Self::new_with_conversion_and_domain(name, symbol, dimension, conversion, domain)
    }

    /// 使用转换规则和指定取值域创建单位 / Create unit with conversion rule and a specified value domain
    pub fn new_with_conversion_and_domain(
        name: String,
        symbol: String,
        dimension: DerivedQuantity,
        conversion: UnitConversionRule,
        domain: QuantityDomain,
    ) -> Self {
        Self {
            inner: Arc::new(UnitInner {
                name,
                symbol,
                dimension,
                domain,
                conversion,
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

    /// 获取取值域 / Get value domain
    pub fn domain(&self) -> QuantityDomain {
        self.inner.domain
    }

    /// 获取比例尺 / Get scale
    pub fn scale(&self) -> &Scale {
        self.inner.conversion.scale()
    }

    /// 获取转换规则 / Get conversion rule
    pub fn conversion(&self) -> &UnitConversionRule {
        &self.inner.conversion
    }

    /// 是否为线性单位 / Whether this unit is linear
    pub fn is_linear(&self) -> bool {
        self.inner.conversion.is_linear()
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
        if !self.is_linear() || !other.is_linear() {
            return None;
        }
        Some(self.scale().value() / other.scale().value())
    }

    /// 将值转换到另一个同量纲单位
    /// Convert a value to another unit with the same dimension
    pub fn convert_value_to<V>(&self, value: V, other: &Unit) -> Option<V>
    where
        V: UnitConversionValue,
    {
        if !self.same_dimension(other) {
            return None;
        }
        let standard = self.inner.conversion.to_standard_value_checked(value)?;
        other.inner.conversion.value_from_standard_checked(standard)
    }
}

pub struct UnitBuilder {
    name: Option<String>,
    symbol: Option<String>,
    dimension: DerivedQuantityBuilder,
    scale: Scale,
    domain: QuantityDomain,
}

impl UnitBuilder {
    pub fn new(dimension: DerivedQuantityBuilder, scale: Scale) -> Self {
        let domain = dimension.domain();
        Self {
            name: None,
            symbol: None,
            dimension,
            scale,
            domain,
        }
    }

    pub fn new_with_domain(
        dimension: DerivedQuantityBuilder,
        scale: Scale,
        domain: QuantityDomain,
    ) -> Self {
        Self {
            name: None,
            symbol: None,
            dimension,
            scale,
            domain,
        }
    }

    pub fn new_by(dimension: &DerivedQuantity, scale: Scale) -> Self {
        Self {
            name: None,
            symbol: None,
            dimension: DerivedQuantityBuilder::new(dimension.powers().cloned().collect()),
            scale,
            domain: dimension.domain(),
        }
    }

    pub fn build(self) -> Unit {
        let name = self.name.unwrap_or_else(|| "".to_string());
        let symbol = self.symbol.unwrap_or_else(|| "".to_string());
        Unit::new_with_domain(name, symbol, self.dimension.into(), self.scale, self.domain)
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
        self.inner.dimension == other.inner.dimension
            && self.domain() == other.domain()
            && self.inner.conversion == other.inner.conversion
    }
}

impl Eq for Unit {}

impl std::hash::Hash for Unit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.dimension.hash(state);
        self.domain().hash(state);
        self.inner
            .conversion
            .scale()
            .value()
            .to_string()
            .hash(state);
        self.inner.conversion.offset().to_string().hash(state);
        self.inner.conversion.is_linear().hash(state);
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
        assert!(
            self.is_linear() && rhs.is_linear(),
            "Cannot multiply affine units"
        );
        UnitBuilder::new_with_domain(
            &self.inner.dimension * &rhs.inner.dimension,
            self.scale() * rhs.scale(),
            self.domain() * rhs.domain(),
        )
    }
}

impl Mul for &Unit {
    type Output = UnitBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        assert!(
            self.is_linear() && rhs.is_linear(),
            "Cannot multiply affine units"
        );
        UnitBuilder::new_with_domain(
            &self.inner.dimension * &rhs.inner.dimension,
            self.scale() * rhs.scale(),
            self.domain() * rhs.domain(),
        )
    }
}

impl Mul for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        UnitBuilder::new_with_domain(
            self.dimension * rhs.dimension,
            self.scale * rhs.scale,
            self.domain * rhs.domain,
        )
    }
}

impl Mul<Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: Unit) -> Self::Output {
        assert!(rhs.is_linear(), "Cannot multiply affine units");
        UnitBuilder::new_with_domain(
            self.dimension * &rhs.inner.dimension,
            self.scale * rhs.scale(),
            self.domain * rhs.domain(),
        )
    }
}

impl Mul<&Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: &Unit) -> Self::Output {
        assert!(rhs.is_linear(), "Cannot multiply affine units");
        UnitBuilder::new_with_domain(
            self.dimension * &rhs.inner.dimension,
            self.scale * rhs.scale(),
            self.domain * rhs.domain(),
        )
    }
}

impl Div for Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        assert!(
            self.is_linear() && rhs.is_linear(),
            "Cannot divide affine units"
        );
        UnitBuilder::new_with_domain(
            &self.inner.dimension / &rhs.inner.dimension,
            self.scale() / rhs.scale(),
            self.domain() / rhs.domain(),
        )
    }
}

impl Div for &Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        assert!(
            self.is_linear() && rhs.is_linear(),
            "Cannot divide affine units"
        );
        UnitBuilder::new_with_domain(
            &self.inner.dimension / &rhs.inner.dimension,
            self.scale() / rhs.scale(),
            self.domain() / rhs.domain(),
        )
    }
}

impl Div for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        UnitBuilder::new_with_domain(
            self.dimension / rhs.dimension,
            self.scale / rhs.scale,
            self.domain / rhs.domain,
        )
    }
}

impl Div<Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: Unit) -> Self::Output {
        assert!(rhs.is_linear(), "Cannot divide affine units");
        UnitBuilder::new_with_domain(
            self.dimension / &rhs.inner.dimension,
            self.scale / rhs.scale(),
            self.domain / rhs.domain(),
        )
    }
}

impl Div<&Unit> for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: &Unit) -> Self::Output {
        assert!(rhs.is_linear(), "Cannot divide affine units");
        UnitBuilder::new_with_domain(
            self.dimension / &rhs.inner.dimension,
            self.scale / rhs.scale(),
            self.domain / rhs.domain(),
        )
    }
}

impl Mul<i64> for Unit {
    type Output = UnitBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        assert!(self.is_linear(), "Cannot scale affine units");
        UnitBuilder::new_with_domain(
            DerivedQuantityBuilder::new(self.inner.dimension.powers().cloned().collect()),
            self.scale() * rhs,
            self.domain(),
        )
    }
}

impl<'a> Mul<i64> for &'a Unit {
    type Output = UnitBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        assert!(self.is_linear(), "Cannot scale affine units");
        UnitBuilder::new_with_domain(
            DerivedQuantityBuilder::new(self.inner.dimension.powers().cloned().collect()),
            self.scale() * rhs,
            self.domain(),
        )
    }
}

impl Mul<i64> for UnitBuilder {
    type Output = UnitBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        UnitBuilder::new_with_domain(self.dimension * rhs, self.scale * rhs, self.domain)
    }
}

impl Div<i64> for Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        assert!(self.is_linear(), "Cannot scale affine units");
        UnitBuilder::new_with_domain(
            DerivedQuantityBuilder::new(self.inner.dimension.powers().cloned().collect()),
            self.scale() / rhs,
            self.domain(),
        )
    }
}

impl<'a> Div<i64> for &'a Unit {
    type Output = UnitBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        assert!(self.is_linear(), "Cannot scale affine units");
        UnitBuilder::new_with_domain(
            DerivedQuantityBuilder::new(self.inner.dimension.powers().cloned().collect()),
            self.scale() / rhs,
            self.domain(),
        )
    }
}

impl Div<i64> for UnitBuilder {
    type Output = UnitBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        UnitBuilder::new_with_domain(self.dimension / rhs, self.scale / rhs, self.domain)
    }
}

impl Reciprocal for Unit {
    type Output = UnitBuilder;

    fn reciprocal(self) -> Self::Output {
        assert!(self.is_linear(), "Cannot take reciprocal of affine units");
        UnitBuilder::new_with_domain(
            (&self.inner.dimension).reciprocal().into(),
            self.scale().reciprocal(),
            QuantityDomain::Continuous,
        )
    }
}

impl Reciprocal for &Unit {
    type Output = UnitBuilder;

    fn reciprocal(self) -> Self::Output {
        assert!(self.is_linear(), "Cannot take reciprocal of affine units");
        UnitBuilder::new_with_domain(
            (&self.inner.dimension).reciprocal().into(),
            self.scale().reciprocal(),
            QuantityDomain::Continuous,
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
        self.scale().value().clone()
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

    /// 单位 offset / Unit offset
    const OFFSET: Lazy<BigDecimal> = Lazy::new(|| BigDecimal::from(0));

    /// 单位取值域 / Unit value domain
    const DOMAIN: QuantityDomain = <<Self as CTUnit>::Dimension as CTDerivedQuantity>::DOMAIN;

    /// 单位量纲类型 / Unit dimension type
    type Dimension: CTDerivedQuantity;

    /// 运行时单位实例 / Runtime unit instance
    const INSTANT: Lazy<Unit> = Lazy::new(|| {
        let conversion = if *Self::OFFSET == BigDecimal::from(0) {
            UnitConversionRule::linear(Self::SCALE.clone())
        } else {
            UnitConversionRule::affine(Self::SCALE.clone(), Self::OFFSET.clone())
        };

        Unit::new_with_conversion_and_domain(
            Self::NAME.to_string(),
            Self::SYMBOL.to_string(),
            <Self as CTUnit>::Dimension::INSTANT.clone(),
            conversion,
            Self::DOMAIN,
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
        Self::dim_eq::<U>() && *Self::SCALE == *U::SCALE && *Self::OFFSET == *U::OFFSET
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
        if *Self::OFFSET != BigDecimal::from(0) || *U::OFFSET != BigDecimal::from(0) {
            return None;
        }
        Some(Self::SCALE.value() / U::SCALE.value())
    }

    /// 将值转换到另一个编译时单位
    /// Convert a value to another compile-time unit
    fn convert_value_to<V, U: CTUnit>(value: V) -> Option<V>
    where
        V: UnitConversionValue,
    {
        if !Self::dim_eq::<U>() {
            return None;
        }
        // 通过 UnitConversionRule 执行 checked 转换
        // Perform checked conversion via UnitConversionRule
        let from_rule = &*Self::INSTANT;
        let to_rule = &*U::INSTANT;
        let standard = from_rule.conversion().to_standard_value_checked(value)?;
        to_rule.conversion().value_from_standard_checked(standard)
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
        Self {
            _marker: PhantomData,
        }
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
    const DOMAIN: QuantityDomain = U1::DOMAIN.mul_domain(U2::DOMAIN);
    type Dimension = CTDerivedMul<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>;
}

pub struct CTUnitDiv<U1: CTUnit, U2: CTUnit> {
    _marker: PhantomData<(U1, U2)>,
}

impl<U1: CTUnit, U2: CTUnit> Default for CTUnitDiv<U1, U2> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
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
    const DOMAIN: QuantityDomain = U1::DOMAIN.div_domain(U2::DOMAIN);
    type Dimension = CTDerivedDiv<<U1 as CTUnit>::Dimension, <U2 as CTUnit>::Dimension>;
}

pub struct CTUnitPow<U: CTUnit, N: Integer> {
    _marker: PhantomData<(U, N)>,
}

impl<U: CTUnit, N: Integer> Default for CTUnitPow<U, N> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
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
        <CTDerivedPow<<U as CTUnit>::Dimension, N> as CTDerivedQuantity>::INSTANT
            .symbol()
            .to_string()
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
    const DOMAIN: QuantityDomain = U::DOMAIN.pow(N::I64);
    type Dimension = CTDerivedPow<<U as CTUnit>::Dimension, N>;
}

pub struct CTUnitReciprocal<U: CTUnit> {
    _marker: PhantomData<U>,
}

impl<U: CTUnit> Default for CTUnitReciprocal<U> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
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
        <CTDerivedReciprocal<<U as CTUnit>::Dimension> as CTDerivedQuantity>::INSTANT
            .symbol()
            .to_string()
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
    const DOMAIN: QuantityDomain = QuantityDomain::Continuous;
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

        fn symbol(&self) -> &'static str {
            "m"
        }
        fn name(&self) -> &'static str {
            "meter"
        }
        fn dimension_symbol(&self) -> String {
            Length::INSTANT.symbol().to_string()
        }
        fn scale_value(&self) -> BigDecimal {
            BigDecimal::from(1)
        }
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

        fn symbol(&self) -> &'static str {
            "km"
        }
        fn name(&self) -> &'static str {
            "kilometer"
        }
        fn dimension_symbol(&self) -> String {
            Length::INSTANT.symbol().to_string()
        }
        fn scale_value(&self) -> BigDecimal {
            BigDecimal::from(1000)
        }
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

        fn symbol(&self) -> &'static str {
            "s"
        }
        fn name(&self) -> &'static str {
            "second"
        }
        fn dimension_symbol(&self) -> String {
            Time::INSTANT.symbol().to_string()
        }
        fn scale_value(&self) -> BigDecimal {
            BigDecimal::from(1)
        }
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

    // ========================================================================
    // Phase 1.3: 纯单位运算测试 / Pure unit arithmetic tests
    // ========================================================================

    #[test]
    fn test_unit_mul_km_times_h() {
        // km * h -> UnitBuilder -> Unit (量纲: L*T)
        let km = TestKilometer::INSTANT.clone();
        let h = crate::unit::derived::Hour::INSTANT.clone();
        let composite = (&km * &h).build();
        // 量纲应包含 L 和 T
        let dim_symbol = composite.dimension().symbol();
        assert!(
            dim_symbol.contains("L") && dim_symbol.contains("T"),
            "Expected L*T dimension, got: {}",
            dim_symbol
        );
    }

    #[test]
    fn test_unit_div_mb_per_s() {
        // MB / s -> UnitBuilder -> Unit (量纲: ℐ/T)
        let mb = crate::unit::derived::Megabyte::INSTANT.clone();
        let s = TestSecond::INSTANT.clone();
        let composite = (&mb / &s).build();
        let dim_symbol = composite.dimension().symbol();
        assert!(
            dim_symbol.contains('T'),
            "Expected ℐ/T dimension, got: {}",
            dim_symbol
        );
    }

    #[test]
    fn test_unit_mul_div_chain() {
        // km * h / s -> 量纲: L*T/T = L
        let km = TestKilometer::INSTANT.clone();
        let h = crate::unit::derived::Hour::INSTANT.clone();
        let s = TestSecond::INSTANT.clone();
        let composite = ((&km * &h) / &s).build();
        let dim_symbol = composite.dimension().symbol();
        // km*h/s 的量纲是 L*T/T = L
        assert!(
            dim_symbol.contains("L"),
            "Expected L dimension (km*h/s), got: {}",
            dim_symbol
        );
    }

    #[test]
    fn test_quantity_with_composite_unit() {
        // Quantity<f64, Unit> = 100.0 km/h
        use crate::quantity::Quantity;
        let km = TestKilometer::INSTANT.clone();
        let h = crate::unit::derived::Hour::INSTANT.clone();
        let km_per_h = (&km / &h).build();
        let speed = Quantity::new(100.0_f64, km_per_h);
        assert_eq!(speed.value, 100.0);
        // 量纲应为速度量纲 (L/T)
        let dim_symbol = speed.unit.dimension().symbol();
        assert!(
            dim_symbol.contains("L") && dim_symbol.contains("T"),
            "Expected velocity dimension, got: {}",
            dim_symbol
        );
    }

    #[test]
    fn test_quantity_mul_produces_composite_unit() {
        // 10 km * 5 h = 50 km*h
        use crate::quantity::Quantity;
        let km = TestKilometer::INSTANT.clone();
        let h = crate::unit::derived::Hour::INSTANT.clone();
        let q_km = Quantity::new(10.0_f64, km);
        let q_h = Quantity::new(5.0_f64, h);
        let product = &q_km * &q_h;
        assert_eq!(product.value, 50.0);
        let dim_symbol = product.unit.dimension().symbol();
        assert!(
            dim_symbol.contains("L") && dim_symbol.contains("T"),
            "Expected L*T dimension, got: {}",
            dim_symbol
        );
    }

    #[test]
    fn test_quantity_div_produces_composite_unit() {
        // 100 MB / 2 s = 50 MB/s
        use crate::quantity::Quantity;
        let mb = crate::unit::derived::Megabyte::INSTANT.clone();
        let s = TestSecond::INSTANT.clone();
        let q_mb = Quantity::new(100.0_f64, mb);
        let q_s = Quantity::new(2.0_f64, s);
        let quotient = &q_mb / &q_s;
        assert_eq!(quotient.value, 50.0);
        let dim_symbol = quotient.unit.dimension().symbol();
        assert!(
            dim_symbol.contains('T'),
            "Expected ℐ/T dimension, got: {}",
            dim_symbol
        );
    }
}
