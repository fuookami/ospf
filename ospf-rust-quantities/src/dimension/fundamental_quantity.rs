//! Fundamental dimension - 基础量纲
//! Fundamental dimension - Base physical dimensions
//!
//! 定义物理量的基础量纲，支持运行时和编译时两种表示方式。
//! Defines base dimensions of physical quantities, supporting both runtime and compile-time representations.
//!
//! # SI 基本量纲 / SI Base Dimensions
//! - Length (L) - 长度
//! - Mass (M) - 质量
//! - Time (T) - 时间
//! - Electric Current (I) - 电流
//! - Thermodynamic Temperature (Θ) - 热力学温度
//! - Amount of Substance (N) - 物质的量
//! - Luminous Intensity (J) - 发光强度
//!
//! # 非 SI 基本量纲 / Non-SI Base Dimensions
//! - Information (ℐ) - 信息量
//! - Plane Angle (φ) - 平面角
//! - Solid Angle (Ω) - 立体角
//!
//! # 编译时运算 / Compile-time Operations
//! 支持类型级别的量纲乘法、除法、幂次和倒数运算。
//! Supports type-level dimension multiplication, division, power, and reciprocal operations.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Add, Mul, Neg, Sub};
use std::sync::Arc;
use once_cell::sync::Lazy;
use typenum::{Integer, N2, P1, P2, P3, Z0};

// ============================================================================
// 基本量特征 / Fundamental dimension trait
// ============================================================================

/// FundamentalQuantity - 基础量纲特征
/// FundamentalQuantity - Base dimension trait
pub trait FundamentalDimension: fmt::Debug + Send + Sync + 'static {
    /// 量纲符号 (如 L, M, T)
    /// Dimension symbol (e.g., L, M, T)
    fn symbol(&self) -> &str;

    /// 转换为基础量纲枚举
    /// Convert to fundamental quantity enum
    fn runtime_value(&self) -> FundamentalQuantityEnum;
}

/// FundamentalQuantity - 基本量
/// FundamentalQuantity - Fundamental Quantity
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundamentalQuantity {
    /// 基础量纲 / Base dimension
    pub dimension: FundamentalQuantityEnum,
    /// 幂次 / Power
    pub power: i64,
}

impl Hash for FundamentalQuantity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dimension.hash(state);
        self.power.to_string().hash(state);
    }
}

impl FundamentalQuantity {
    /// 创建新的量纲幂次
    /// Create new dimension power
    pub fn new(dimension: FundamentalQuantityEnum, power: i64) -> Self {
        Self { dimension, power }
    }
}

// ============================================================================
// 编译时基本量特征 / Compile-time Fundamental dimension trait
// ============================================================================

/// CTFundamentalDimension - 编译时基础量纲特征
/// CTFundamentalDimension - Compile-time base dimension trait
pub trait CTFundamentalDimension: FundamentalDimension + Sized {
    /// 量纲符号
    /// Dimension symbol
    const SYMBOL: &str;

    /// 运行时量纲值
    /// Runtime dimension value
    const INSTANT: Lazy<Self>;
}

// ============================================================================
// 编译时基础量纲幂次 / Compile-time fundamental quantity with power
// ============================================================================

/// CTFundamentalQuantityTrait - 编译时基础量纲幂次特征
/// CTFundamentalQuantityTrait - Compile-time fundamental quantity trait
///
/// 使用 typenum 的类型级整数来表示幂次，支持编译期运算和比较
/// Uses typenum's type-level integers to represent powers, supporting compile-time operations and comparisons
pub trait CTFundamentalQuantityTrait {
    /// 基础量纲类型 / Base dimension type
    type D: CTFundamentalDimension;
    /// 幂次（类型级整数）/ Power (type-level integer)
    type P: Integer;
    /// 运行时量纲幂次值 / Runtime dimension power value
    const INSTANT: Lazy<FundamentalQuantity> = Lazy::new(|| {
        FundamentalQuantity::new(
            <Self::D as FundamentalDimension>::runtime_value(&*Self::D::INSTANT),
            Self::P::to_i64(),
        )
    });
}

/// CTFundamentalQuantity - 编译时基础量纲幂次的具体类型
/// CTFundamentalQuantity - Concrete compile-time fundamental quantity type
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::dimension::fundamental_quantity::{CTFundamentalQuantity, L};
/// use ospf_rust_quantities::dimension::CTFundamentalQuantityTrait;
/// use typenum::{P1, P2};
///
/// type Length = CTFundamentalQuantity<L, P1>;  // L^1
/// type Area = CTFundamentalQuantity<L, P2>;    // L^2
/// ```
pub struct CTFundamentalQuantity<D: CTFundamentalDimension, P: Integer> {
    _marker: PhantomData<(D, P)>,
}

impl<D: CTFundamentalDimension, P: Integer> CTFundamentalQuantityTrait
    for CTFundamentalQuantity<D, P>
{
    type D = D;
    type P = P;
}

// ============================================================================
// 编译时量纲相等判定 / Compile-time dimension equality
// ============================================================================

/// SameFundamentalDimension - 编译时基础量纲相等 marker trait
/// SameFundamentalDimension - Compile-time same fundamental dimension marker trait
///
/// 当两个量纲类型的 D 相同时，自动实现此 trait
/// This trait is automatically implemented when two types have the same D
pub trait SameFundamentalDimension<Q: CTFundamentalQuantityTrait>:
    CTFundamentalQuantityTrait
{
}

impl<Q1, Q2> SameFundamentalDimension<Q2> for Q1
where
    Q1: CTFundamentalQuantityTrait,
    Q2: CTFundamentalQuantityTrait<D = Q1::D>,
{
}

/// SameFundamentalPower - 编译时幂次相等 marker trait
/// SameFundamentalPower - Compile-time same power marker trait
///
/// 当两个量纲类型的 P 相同时，自动实现此 trait
/// This trait is automatically implemented when two types have the same P
pub trait SameFundamentalPower<Q: CTFundamentalQuantityTrait>: CTFundamentalQuantityTrait {}

impl<Q1, Q2> SameFundamentalPower<Q2> for Q1
where
    Q1: CTFundamentalQuantityTrait,
    Q2: CTFundamentalQuantityTrait<P = Q1::P>,
{
}

/// SameFundamentalQuantity - 编译时量纲完全相等 marker trait（D 和 P 都相同）
/// SameFundamentalQuantity - Compile-time same quantity marker trait (both D and P are equal)
pub trait SameFundamentalQuantity<Q: CTFundamentalQuantityTrait>:
    CTFundamentalQuantityTrait
{
}

impl<Q1, Q2> SameFundamentalQuantity<Q2> for Q1
where
    Q1: CTFundamentalQuantityTrait,
    Q2: CTFundamentalQuantityTrait<D = Q1::D, P = Q1::P>,
{
}

// ============================================================================
// 编译时量纲运算 / Compile-time dimension operations
// ============================================================================

/// CTFundamentalMul - 编译时量纲乘法
/// CTFundamentalMul - Compile-time dimension multiplication
///
/// 两个相同基础量纲的幂次相加：L^1 * L^2 = L^3
/// Powers of the same base dimension are added: L^1 * L^2 = L^3
pub struct CTFundamentalMul<Q1: CTFundamentalQuantityTrait, Q2: CTFundamentalQuantityTrait> {
    _marker: PhantomData<(Q1, Q2)>,
}

impl<Q1, Q2> CTFundamentalQuantityTrait for CTFundamentalMul<Q1, Q2>
where
    Q1: CTFundamentalQuantityTrait,
    Q2: CTFundamentalQuantityTrait<D = Q1::D>, // 要求 D 相同 / Require same D
    Q1::P: Add<Q2::P>,
    <Q1::P as Add<Q2::P>>::Output: Integer,
{
    type D = Q1::D;
    type P = <Q1::P as Add<Q2::P>>::Output; // P1 + P2
}

/// CTFundamentalDiv - 编译时量纲除法
/// CTFundamentalDiv - Compile-time dimension division
///
/// 两个相同基础量纲的幂次相减：L^3 / L^1 = L^2
/// Powers of the same base dimension are subtracted: L^3 / L^1 = L^2
pub struct CTFundamentalDiv<Q1: CTFundamentalQuantityTrait, Q2: CTFundamentalQuantityTrait> {
    _marker: PhantomData<(Q1, Q2)>,
}

impl<Q1, Q2> CTFundamentalQuantityTrait for CTFundamentalDiv<Q1, Q2>
where
    Q1: CTFundamentalQuantityTrait,
    Q2: CTFundamentalQuantityTrait<D = Q1::D>, // 要求 D 相同 / Require same D
    Q1::P: Sub<Q2::P>,
    <Q1::P as Sub<Q2::P>>::Output: Integer,
{
    type D = Q1::D;
    type P = <Q1::P as Sub<Q2::P>>::Output; // P1 - P2
}

/// CTFundamentalPow - 编译时量纲幂次
/// CTFundamentalPow - Compile-time dimension power
///
/// 量纲的幂次乘以常数：(L^2)^3 = L^6
/// The power of the dimension is multiplied by a constant: (L^2)^3 = L^6
pub struct CTFundamentalPow<Q: CTFundamentalQuantityTrait, N: Integer> {
    _marker: PhantomData<(Q, N)>,
}

impl<Q, N> CTFundamentalQuantityTrait for CTFundamentalPow<Q, N>
where
    Q: CTFundamentalQuantityTrait,
    N: Integer,
    Q::P: Mul<N>,
    <Q::P as Mul<N>>::Output: Integer,
{
    type D = Q::D;
    type P = <Q::P as Mul<N>>::Output; // P * N
}

/// CTFundamentalReciprocal - 编译时量纲倒数
/// CTFundamentalReciprocal - Compile-time dimension reciprocal
///
/// 量纲的倒数：L^2 -> L^-2
/// The reciprocal of the dimension: L^2 -> L^-2
pub struct CTFundamentalReciprocal<Q: CTFundamentalQuantityTrait> {
    _marker: PhantomData<Q>,
}

impl<Q> CTFundamentalQuantityTrait for CTFundamentalReciprocal<Q>
where
    Q: CTFundamentalQuantityTrait,
    Q::P: Neg,
    <Q::P as Neg>::Output: Integer,
{
    type D = Q::D;
    type P = <Q::P as Neg>::Output; // -P
}

// ============================================================================
// 基础量纲类型 / Base dimension types
// ============================================================================

/// 长度量纲: L / Length dimension: L
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct L;

impl FundamentalDimension for L {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::Length
    }
}

impl CTFundamentalDimension for L {
    const SYMBOL: &'static str = "L";
    const INSTANT: Lazy<Self> = Lazy::new(|| L);
}

/// 质量量纲: M / Mass dimension: M
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct M;

impl FundamentalDimension for M {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::Mass
    }
}

impl CTFundamentalDimension for M {
    const SYMBOL: &'static str = "M";
    const INSTANT: Lazy<Self> = Lazy::new(|| M);
}

/// 时间量纲: T / Time dimension: T
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct T;

impl FundamentalDimension for T {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::Time
    }
}

impl CTFundamentalDimension for T {
    const SYMBOL: &'static str = "T";
    const INSTANT: Lazy<Self> = Lazy::new(|| T);
}

/// 电流量纲: I / Electric Current dimension: I
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct I;

impl FundamentalDimension for I {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::ElectricCurrent
    }
}

impl CTFundamentalDimension for I {
    const SYMBOL: &'static str = "I";
    const INSTANT: Lazy<Self> = Lazy::new(|| I);
}

/// 热力学温度量纲: Θ / Thermodynamic Temperature dimension: Θ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Theta;

impl FundamentalDimension for Theta {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::ThermodynamicTemperature
    }
}

impl CTFundamentalDimension for Theta {
    const SYMBOL: &'static str = "Θ";
    const INSTANT: Lazy<Self> = Lazy::new(|| Theta);
}

/// 物质的量量纲: N / Amount of Substance dimension: N
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct N;

impl FundamentalDimension for N {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::AmountOfSubstance
    }
}

impl CTFundamentalDimension for N {
    const SYMBOL: &'static str = "N";
    const INSTANT: Lazy<Self> = Lazy::new(|| N);
}

/// 发光强度量纲: J / Luminous Intensity dimension: J
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct J;

impl FundamentalDimension for J {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::LuminousIntensity
    }
}

impl CTFundamentalDimension for J {
    const SYMBOL: &'static str = "J";
    const INSTANT: Lazy<Self> = Lazy::new(|| J);
}

/// 信息量量纲: ℐ / Information dimension: ℐ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Info;

impl FundamentalDimension for Info {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::Information
    }
}

impl CTFundamentalDimension for Info {
    const SYMBOL: &'static str = "ℐ";
    const INSTANT: Lazy<Self> = Lazy::new(|| Info);
}

/// 平面角量纲: φ / Plane Angle dimension: φ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Phi;

impl FundamentalDimension for Phi {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::PlaneAngle
    }
}

impl CTFundamentalDimension for Phi {
    const SYMBOL: &'static str = "φ";
    const INSTANT: Lazy<Self> = Lazy::new(|| Phi);
}

/// 立体角量纲: Ω / Solid Angle dimension: Ω
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Omega;

impl FundamentalDimension for Omega {
    fn symbol(&self) -> &'static str {
        Self::SYMBOL
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::SolidAngle
    }
}

impl CTFundamentalDimension for Omega {
    const SYMBOL: &'static str = "Ω";
    const INSTANT: Lazy<Self> = Lazy::new(|| Omega);
}

// ============================================================================
// 类型别名：所有基础量纲的 0、1 幂次 / Type aliases: 0, 1 powers for all base dimensions
// ============================================================================

// ----------------------------------------------------------------------------
// 长度 L / Length L
// ----------------------------------------------------------------------------
/// L^0 = 无量纲 / dimensionless
pub type L0 = CTFundamentalQuantity<L, Z0>;
/// L^1 = 长度 / length
pub type L1 = CTFundamentalQuantity<L, P1>;
/// L^2 = 面积 / area
pub type L2 = CTFundamentalQuantity<L, P2>;
/// L^3 = 体积 / volume
pub type L3 = CTFundamentalQuantity<L, P3>;

// ----------------------------------------------------------------------------
// 质量 M / Mass M
// ----------------------------------------------------------------------------
/// M^0 = 无量纲 / dimensionless
pub type M0 = CTFundamentalQuantity<M, Z0>;
/// M^1 = 质量 / mass
pub type M1 = CTFundamentalQuantity<M, P1>;

// ----------------------------------------------------------------------------
// 时间 T / Time T
// ----------------------------------------------------------------------------
/// T^0 = 无量纲 / dimensionless
pub type T0 = CTFundamentalQuantity<T, Z0>;
/// T^1 = 时间 / time
pub type T1 = CTFundamentalQuantity<T, P1>;
/// T^-1 = 频率 / frequency
pub type TN1 = CTFundamentalQuantity<T, typenum::N1>;
/// T^-2 = 加速度相关 / acceleration related
pub type TN2 = CTFundamentalQuantity<T, N2>;

// ----------------------------------------------------------------------------
// 电流 I / Electric Current I
// ----------------------------------------------------------------------------
/// I^0 = 无量纲 / dimensionless
pub type I0 = CTFundamentalQuantity<I, Z0>;
/// I^1 = 电流 / electric current
pub type I1 = CTFundamentalQuantity<I, P1>;

// ----------------------------------------------------------------------------
// 热力学温度 Θ / Thermodynamic Temperature Θ
// ----------------------------------------------------------------------------
/// Θ^0 = 无量纲 / dimensionless
pub type Theta0 = CTFundamentalQuantity<Theta, Z0>;
/// Θ^1 = 热力学温度 / thermodynamic temperature
pub type Theta1 = CTFundamentalQuantity<Theta, P1>;

// ----------------------------------------------------------------------------
// 物质的量 N / Amount of Substance N
// ----------------------------------------------------------------------------
/// N^0 = 无量纲 / dimensionless
pub type N0 = CTFundamentalQuantity<N, Z0>;
/// N^1 = 物质的量 / amount of substance
pub type N1 = CTFundamentalQuantity<N, P1>;

// ----------------------------------------------------------------------------
// 发光强度 J / Luminous Intensity J
// ----------------------------------------------------------------------------
/// J^0 = 无量纲 / dimensionless
pub type J0 = CTFundamentalQuantity<J, Z0>;
/// J^1 = 发光强度 / luminous intensity
pub type J1 = CTFundamentalQuantity<J, P1>;

// ----------------------------------------------------------------------------
// 信息量 ℐ / Information ℐ
// ----------------------------------------------------------------------------
/// ℐ^0 = 无量纲 / dimensionless
pub type Info0 = CTFundamentalQuantity<Info, Z0>;
/// ℐ^1 = 信息量 / information
pub type Info1 = CTFundamentalQuantity<Info, P1>;

// ----------------------------------------------------------------------------
// 平面角 φ / Plane Angle φ
// ----------------------------------------------------------------------------
/// φ^0 = 无量纲 / dimensionless
pub type Phi0 = CTFundamentalQuantity<Phi, Z0>;
/// φ^1 = 平面角 / plane angle
pub type Phi1 = CTFundamentalQuantity<Phi, P1>;

// ----------------------------------------------------------------------------
// 立体角 Ω / Solid Angle Ω
// ----------------------------------------------------------------------------
/// Ω^0 = 无量纲 / dimensionless
pub type Omega0 = CTFundamentalQuantity<Omega, Z0>;
/// Ω^1 = 立体角 / solid angle
pub type Omega1 = CTFundamentalQuantity<Omega, P1>;

/// FundamentalQuantityEnum - 基础量纲枚举
/// FundamentalQuantityEnum - Base dimension enum
#[derive(Debug, Clone)]
pub enum FundamentalQuantityEnum {
    /// 长度 / Length
    Length,
    /// 质量 / Mass
    Mass,
    /// 时间 / Time
    Time,
    /// 电流 / Electric Current
    ElectricCurrent,
    /// 热力学温度 / Thermodynamic Temperature
    ThermodynamicTemperature,
    /// 物质的量 / Amount of Substance
    AmountOfSubstance,
    /// 发光强度 / Luminous Intensity
    LuminousIntensity,
    /// 信息量 / Information
    Information,
    /// 平面角 / Plane Angle
    PlaneAngle,
    /// 立体角 / Solid Angle
    SolidAngle,
    /// 自定义基础量纲 / Custom base dimension
    Custom(Arc<dyn FundamentalDimension>),
}

impl PartialEq for FundamentalQuantityEnum {
    fn eq(&self, other: &Self) -> bool {
        self.symbol() == other.symbol()
    }
}

impl Eq for FundamentalQuantityEnum {}

impl Hash for FundamentalQuantityEnum {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.symbol().hash(state);
    }
}

impl FundamentalDimension for FundamentalQuantityEnum {
    fn symbol(&self) -> &str {
        match self {
            FundamentalQuantityEnum::Length => L::SYMBOL,
            FundamentalQuantityEnum::Mass => M::SYMBOL,
            FundamentalQuantityEnum::Time => T::SYMBOL,
            FundamentalQuantityEnum::ElectricCurrent => I::SYMBOL,
            FundamentalQuantityEnum::ThermodynamicTemperature => Theta::SYMBOL,
            FundamentalQuantityEnum::AmountOfSubstance => N::SYMBOL,
            FundamentalQuantityEnum::LuminousIntensity => J::SYMBOL,
            FundamentalQuantityEnum::Information => Info::SYMBOL,
            FundamentalQuantityEnum::PlaneAngle => Phi::SYMBOL,
            FundamentalQuantityEnum::SolidAngle => Omega::SYMBOL,
            FundamentalQuantityEnum::Custom(d) => d.symbol(),
        }
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        self.clone()
    }
}

// ============================================================================
// 自定义基础量纲 / Custom fundamental dimension
// ============================================================================

/// CustomFundamentalDimension - 自定义基础量纲
/// CustomFundamentalDimension - Custom fundamental dimension
///
/// 用于运行时动态创建自定义量纲，无需预定义类型。
/// Used for runtime dynamic creation of custom dimensions without predefined types.
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::dimension::fundamental_quantity::{CustomFundamentalDimension, FundamentalDimension};
///
/// let tome = CustomFundamentalDimension::new("T");
/// assert_eq!(tome.symbol(), "T");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomFundamentalDimension {
    symbol: String,
}

impl CustomFundamentalDimension {
    /// 创建新的自定义量纲
    /// Create new custom dimension
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
        }
    }

    /// 转换为 FundamentalQuantityEnum
    /// Convert to FundamentalQuantityEnum
    pub fn to_enum(&self) -> FundamentalQuantityEnum {
        FundamentalQuantityEnum::Custom(Arc::new(self.clone()))
    }
}

impl FundamentalDimension for CustomFundamentalDimension {
    fn symbol(&self) -> &str {
        &self.symbol
    }

    fn runtime_value(&self) -> FundamentalQuantityEnum {
        self.to_enum()
    }
}

impl fmt::Display for FundamentalQuantityEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use typenum::{N3, N4, N5, P4, P5};

    // ========================================================================
    // 基础量纲符号测试 / Base dimension symbol tests
    // ========================================================================

    #[test]
    fn test_dimension_symbols() {
        assert_eq!(FundamentalDimension::symbol(&*L::INSTANT), "L");
        assert_eq!(FundamentalDimension::symbol(&*M::INSTANT), "M");
        assert_eq!(FundamentalDimension::symbol(&*T::INSTANT), "T");
        assert_eq!(FundamentalDimension::symbol(&*I::INSTANT), "I");
        assert_eq!(FundamentalDimension::symbol(&*Theta::INSTANT), "Θ");
        assert_eq!(FundamentalDimension::symbol(&*N::INSTANT), "N");
        assert_eq!(FundamentalDimension::symbol(&*J::INSTANT), "J");
    }

    #[test]
    fn test_base_dimensions() {
        assert_eq!(L::SYMBOL, "L");
        assert_eq!(M::SYMBOL, "M");
        assert_eq!(T::SYMBOL, "T");
        assert_eq!(I::SYMBOL, "I");
    }

    #[test]
    fn test_dimension_conversion() {
        let rt_length = *L::INSTANT;
        assert_eq!(FundamentalDimension::symbol(&rt_length), "L");

        let rt_mass = *M::INSTANT;
        assert_eq!(FundamentalDimension::symbol(&rt_mass), "M");
    }

    // ========================================================================
    // 编译时量纲幂次测试 / Compile-time dimension power tests
    // ========================================================================

    #[test]
    fn test_ct_fundamental_quantity_trait() {
        // 验证类型别名的幂次值 / Verify power values of type aliases
        assert_eq!(<L1 as CTFundamentalQuantityTrait>::P::I64, 1);
        assert_eq!(<L2 as CTFundamentalQuantityTrait>::P::I64, 2);
        assert_eq!(<L3 as CTFundamentalQuantityTrait>::P::I64, 3);
        assert_eq!(<TN1 as CTFundamentalQuantityTrait>::P::I64, -1);
        assert_eq!(<TN2 as CTFundamentalQuantityTrait>::P::I64, -2);
        assert_eq!(<M0 as CTFundamentalQuantityTrait>::P::I64, 0);
    }

    // ========================================================================
    // 编译时乘法测试 / Compile-time multiplication tests
    // ========================================================================

    #[test]
    fn test_ct_fundamental_mul() {
        // L^1 * L^1 = L^2
        type Mul1 = CTFundamentalMul<L1, L1>;
        assert_eq!(<Mul1 as CTFundamentalQuantityTrait>::P::I64, 2);

        // L^1 * L^2 = L^3
        type Mul2 = CTFundamentalMul<L1, L2>;
        assert_eq!(<Mul2 as CTFundamentalQuantityTrait>::P::I64, 3);

        // L^2 * L^1 = L^3
        type Mul3 = CTFundamentalMul<L2, L1>;
        assert_eq!(<Mul3 as CTFundamentalQuantityTrait>::P::I64, 3);

        // T^-1 * T^-1 = T^-2
        type Mul4 = CTFundamentalMul<TN1, TN1>;
        assert_eq!(<Mul4 as CTFundamentalQuantityTrait>::P::I64, -2);

        // T^-1 * T^-2 = T^-3
        type Mul5 = CTFundamentalMul<TN1, TN2>;
        assert_eq!(<Mul5 as CTFundamentalQuantityTrait>::P::I64, -3);
    }

    // ========================================================================
    // 编译时除法测试 / Compile-time division tests
    // ========================================================================

    #[test]
    fn test_ct_fundamental_div() {
        // L^3 / L^1 = L^2
        type Div1 = CTFundamentalDiv<L3, L1>;
        assert_eq!(<Div1 as CTFundamentalQuantityTrait>::P::I64, 2);

        // L^2 / L^1 = L^1
        type Div2 = CTFundamentalDiv<L2, L1>;
        assert_eq!(<Div2 as CTFundamentalQuantityTrait>::P::I64, 1);

        // L^1 / L^1 = L^0
        type Div3 = CTFundamentalDiv<L1, L1>;
        assert_eq!(<Div3 as CTFundamentalQuantityTrait>::P::I64, 0);

        // T^-1 / T^-1 = T^0
        type Div4 = CTFundamentalDiv<TN1, TN1>;
        assert_eq!(<Div4 as CTFundamentalQuantityTrait>::P::I64, 0);

        // T^-1 / T^-2 = T^1
        type Div5 = CTFundamentalDiv<TN1, TN2>;
        assert_eq!(<Div5 as CTFundamentalQuantityTrait>::P::I64, 1);
    }

    // ========================================================================
    // 编译时幂次测试 / Compile-time power tests
    // ========================================================================

    #[test]
    fn test_ct_fundamental_pow() {
        // (L^1)^2 = L^2
        type Pow1 = CTFundamentalPow<L1, P2>;
        assert_eq!(<Pow1 as CTFundamentalQuantityTrait>::P::I64, 2);

        // (L^2)^2 = L^4
        type Pow2 = CTFundamentalPow<L2, P2>;
        assert_eq!(<Pow2 as CTFundamentalQuantityTrait>::P::I64, 4);

        // (L^2)^3 = L^6
        type Pow3 = CTFundamentalPow<L2, P3>;
        assert_eq!(<Pow3 as CTFundamentalQuantityTrait>::P::I64, 6);

        // (T^-1)^2 = T^-2
        type Pow4 = CTFundamentalPow<TN1, P2>;
        assert_eq!(<Pow4 as CTFundamentalQuantityTrait>::P::I64, -2);

        // (T^-2)^3 = T^-6
        type Pow5 = CTFundamentalPow<TN2, P3>;
        assert_eq!(<Pow5 as CTFundamentalQuantityTrait>::P::I64, -6);
    }

    // ========================================================================
    // 编译时量纲相等判定测试 / Compile-time dimension equality tests
    // ========================================================================

    #[test]
    fn test_same_fundamental_dimension() {
        // 编译期断言：相同的 D / Compile-time assertion: same D
        fn assert_same_dimension<Q1, Q2>()
        where
            Q1: CTFundamentalQuantityTrait + SameFundamentalDimension<Q2>,
            Q2: CTFundamentalQuantityTrait,
        {
        }

        // L^1 和 L^2 有相同的 D (都是 L)
        assert_same_dimension::<L1, L2>();
        // L^1 和 L^3 有相同的 D
        assert_same_dimension::<L1, L3>();
        // T^-1 和 T^-2 有相同的 D (都是 T)
        assert_same_dimension::<TN1, TN2>();

        // 注意：以下代码如果取消注释会导致编译错误，因为 D 不同
        // assert_same_dimension::<CTDimLength1, CTDimMass1>();  // 编译失败！
    }

    #[test]
    fn test_same_fundamental_power() {
        // 编译期断言：相同的 P / Compile-time assertion: same P
        fn assert_same_power<Q1, Q2>()
        where
            Q1: CTFundamentalQuantityTrait + SameFundamentalPower<Q2>,
            Q2: CTFundamentalQuantityTrait,
        {
        }

        // L^1 和 M^1 有相同的 P (都是 1)
        assert_same_power::<L1, M1>();
        // L^2 和 T^2 有相同的 P (都是 2，如果定义了 CTDimTime2)
        // 注意：我们没有定义 CTDimTime2，所以这里用其他例子
        // L^0 和 M^0 有相同的 P (都是 0)
        assert_same_power::<L0, M0>();

        // 注意：以下代码如果取消注释会导致编译错误，因为 P 不同
        // assert_same_power::<CTDimLength1, CTDimLength2>();  // 编译失败！
    }

    #[test]
    fn test_same_fundamental_quantity() {
        // 编译期断言：完全相同（D 和 P 都相同）
        // Compile-time assertion: exactly same (both D and P are equal)
        fn assert_same_quantity<Q1, Q2>()
        where
            Q1: CTFundamentalQuantityTrait + SameFundamentalQuantity<Q2>,
            Q2: CTFundamentalQuantityTrait,
        {
        }

        // L^1 和 L^1 完全相同
        assert_same_quantity::<L1, L1>();
        // L^2 和 L^2 完全相同
        assert_same_quantity::<L2, L2>();
        // T^-1 和 T^-1 完全相同
        assert_same_quantity::<TN1, TN1>();

        // 注意：以下代码如果取消注释会导致编译错误
        // assert_same_quantity::<CTDimLength1, CTDimLength2>();  // 编译失败！P 不同
        // assert_same_quantity::<CTDimLength1, CTDimMass1>();    // 编译失败！D 不同
    }

    // ========================================================================
    // 复合运算测试 / Compound operation tests
    // ========================================================================

    #[test]
    fn test_compound_operations() {
        // (L^1 * L^1) / L^1 = L^1
        type Compound1 = CTFundamentalDiv<CTFundamentalMul<L1, L1>, L1>;
        assert_eq!(<Compound1 as CTFundamentalQuantityTrait>::P::I64, 1);

        // ((L^1 * L^2) / L^1) = L^2
        type Compound2 = CTFundamentalDiv<CTFundamentalMul<L1, L2>, L1>;
        assert_eq!(<Compound2 as CTFundamentalQuantityTrait>::P::I64, 2);

        // (L^2)^2 / L^2 = L^2
        type Compound3 = CTFundamentalDiv<CTFundamentalPow<L2, P2>, L2>;
        assert_eq!(<Compound3 as CTFundamentalQuantityTrait>::P::I64, 2);

        // 验证 D 类型保持不变 / Verify D type remains unchanged
        fn check_is_length<Q>()
        where
            Q: CTFundamentalQuantityTrait<D = L>,
        {
        }

        check_is_length::<Compound1>();
        check_is_length::<Compound2>();
        check_is_length::<Compound3>();
    }

    #[test]
    fn test_power_to_integer() {
        // 验证 typenum 的类型级整数可以正确转换为运行时值
        // Verify typenum's type-level integers can be correctly converted to runtime values
        assert_eq!(Z0::I64, 0);
        assert_eq!(P1::I64, 1);
        assert_eq!(typenum::N1::I64, -1);
        assert_eq!(P2::I64, 2);
        assert_eq!(N2::I64, -2);
        assert_eq!(P3::I64, 3);
        assert_eq!(N3::I64, -3);
        assert_eq!(P4::I64, 4);
        assert_eq!(N4::I64, -4);
        assert_eq!(P5::I64, 5);
        assert_eq!(N5::I64, -5);
    }
}
