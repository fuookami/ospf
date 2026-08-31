//! Derived dimension - 导出量纲
//! Derived dimension - Derived physical dimensions
//!
//! 导出量纲由基础量纲的幂次积组成，如 L²·M·T⁻²（能量）。
//! Derived dimensions are composed of products of powers of base dimensions, e.g., L²·M·T⁻² (energy).
//!
//! # 运行时表示 / Runtime Representation
//! - `DerivedQuantity`: 运行时导出量纲，支持动态创建和运算
//! - `DerivedQuantityBuilder`: 构建器模式，用于链式构建量纲
//!
//! # 编译时表示 / Compile-time Representation
//! - `CTDerivedQuantity`: 编译时导出量纲 trait
//! - `CTDerivedMul`, `CTDerivedDiv`, `CTDerivedPow`, `CTDerivedReciprocal`: 编译时运算类型
//!
//! # 编译时量纲检查 / Compile-time Dimension Checking
//! - `SameDerivedDimension`: 编译时量纲相等约束

use super::fundamental_quantity::{
    CTFundamentalQuantityTrait, FundamentalDimension, FundamentalQuantity, FundamentalQuantityEnum,
    I, Info, J, L, M, N, Omega, Phi, T, Theta,
};
use crate::dimension::{
    CTFundamentalDimension, CTFundamentalDiv, CTFundamentalMul, CTFundamentalPow,
    CTFundamentalReciprocal, SameFundamentalQuantity,
};
use once_cell::sync::Lazy;
use ospf_rust_math::operator::reciprocal::Reciprocal;
use std::clone::Clone;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::string::ToString;
use std::sync::{Arc, OnceLock};
use typenum::Integer;
// ============================================================================
// 运行时导出量纲 / Runtime derived dimension
// ============================================================================

/// QuantityDomain - 物理量取值域
/// QuantityDomain - Quantity value domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuantityDomain {
    /// 连续量 / Continuous quantity
    Continuous,
    /// 离散量 / Discrete quantity
    Discrete,
}

impl QuantityDomain {
    /// 取值域乘法合成规则
    /// Compose value domain for multiplication
    pub const fn mul_domain(self, rhs: Self) -> Self {
        if matches!(self, Self::Discrete) && matches!(rhs, Self::Discrete) {
            Self::Discrete
        } else {
            Self::Continuous
        }
    }

    /// 取值域除法合成规则
    /// Compose value domain for division
    pub const fn div_domain(self, _rhs: Self) -> Self {
        Self::Continuous
    }

    /// 取值域幂次合成规则
    /// Compose value domain for powers
    pub const fn pow(self, index: i64) -> Self {
        if index <= 0 {
            Self::Continuous
        } else if index == 1 {
            self
        } else {
            let mut i = 1;
            let mut domain = self;
            while i < index {
                domain = domain.mul_domain(self);
                i += 1;
            }
            domain
        }
    }
}

impl Mul for QuantityDomain {
    type Output = QuantityDomain;

    fn mul(self, rhs: Self) -> Self::Output {
        self.mul_domain(rhs)
    }
}

impl Div for QuantityDomain {
    type Output = QuantityDomain;

    fn div(self, _rhs: Self) -> Self::Output {
        self.div_domain(_rhs)
    }
}

fn add_power(
    quantities: &Vec<FundamentalQuantity>,
    dimension: FundamentalQuantityEnum,
    power: i64,
) -> Vec<FundamentalQuantity> {
    // 创建新的 quantities 副本并修改
    // Create new quantities copy and modify
    let mut new_quantities = Vec::with_capacity(quantities.len());
    new_quantities.extend(quantities.iter().cloned());

    // 查找是否有相同量纲
    // Look for same dimension
    if let Some(existing) = new_quantities
        .iter_mut()
        .find(|dp| dp.dimension == dimension)
    {
        existing.power += power;
        // 如果幂次为0，移除该项
        // Remove if power is zero
        if existing.power == 0 {
            new_quantities.retain(|dp| dp.dimension != dimension);
        }
    } else {
        new_quantities.push(FundamentalQuantity::new(dimension, power));
    }

    simplify(new_quantities)
}

fn add_powers(
    mut quantities: Vec<FundamentalQuantity>,
    rhs: &Vec<FundamentalQuantity>,
) -> Vec<FundamentalQuantity> {
    // 预分配容量，避免多次重新分配
    // Pre-allocate capacity to avoid multiple reallocations
    quantities.reserve(rhs.len());

    for dp in rhs {
        if let Some(existing) = quantities
            .iter_mut()
            .find(|odp| odp.dimension == dp.dimension)
        {
            existing.power += dp.power;
            // 如果幂次为0，移除该项
            // Remove if power is zero
            if existing.power == 0 {
                quantities.retain(|odp| odp.dimension != dp.dimension);
            }
        } else {
            quantities.push(FundamentalQuantity::new(dp.dimension.clone(), dp.power));
        }
    }

    simplify(quantities)
}

fn sub_powers(
    mut quantities: Vec<FundamentalQuantity>,
    rhs: &Vec<FundamentalQuantity>,
) -> Vec<FundamentalQuantity> {
    // 预分配容量，避免多次重新分配
    // Pre-allocate capacity to avoid multiple reallocations
    quantities.reserve(rhs.len());

    for dp in rhs {
        if let Some(existing) = quantities
            .iter_mut()
            .find(|odp| odp.dimension == dp.dimension)
        {
            existing.power -= dp.power;
            // 如果幂次为0，移除该项
            // Remove if power is zero
            if existing.power == 0 {
                quantities.retain(|odp| odp.dimension != dp.dimension);
            }
        } else {
            quantities.push(FundamentalQuantity::new(dp.dimension.clone(), -dp.power));
        }
    }

    simplify(quantities)
}

/// 简化量纲 / Simplify dimension
fn simplify(quantities: Vec<FundamentalQuantity>) -> Vec<FundamentalQuantity> {
    // 预分配容量，最多与输入相同
    // Pre-allocate capacity, at most the same as input
    let mut result = Vec::with_capacity(quantities.len());
    for dp in quantities {
        if dp.power != 0 {
            result.push(dp);
        }
    }
    // 收缩容量到实际大小，释放多余内存
    // Shrink capacity to actual size, release excess memory
    result.shrink_to_fit();
    result
}

/// DerivedQuantityInner - 运行时导出量纲
/// DerivedQuantityInner - Runtime derived physical dimension
///
/// 由基础量纲的幂次积组成，如 L²·M·T⁻²
/// Composed of products of powers of base dimensions, e.g., L²·M·T⁻²
#[derive(Debug, Eq)]
pub struct DerivedQuantityInner {
    /// 量纲名称 / Dimension name
    name: String,
    /// 量纲幂次列表 / Dimension power list
    quantities: Vec<FundamentalQuantity>,
    /// 取值域 / Value domain
    domain: QuantityDomain,
    /// 量纲符号 / Dimension symbol
    symbol: OnceLock<String>,
}

/// DerivedQuantity - 导出量纲
/// DerivedQuantity - Derived physical dimension
///
/// 由基础量纲的幂次积组成，如 L²·M·T⁻²
/// Composed of products of powers of base dimensions, e.g., L²·M·T⁻²
#[derive(Debug, Clone)]
pub struct DerivedQuantity {
    inner: Arc<DerivedQuantityInner>,
}

impl DerivedQuantity {
    /// 创建无量纲
    /// Create dimensionless
    pub fn none(name: String) -> Self {
        Self {
            inner: Arc::new(DerivedQuantityInner {
                name,
                quantities: Vec::new(),
                domain: QuantityDomain::Continuous,
                symbol: OnceLock::new(),
            }),
        }
    }

    /// 从单个基础量纲创建
    /// Create from single base dimension
    pub fn from_base(name: String, dimension: FundamentalQuantityEnum) -> Self {
        Self::from_base_with_power(name, dimension, 1)
    }

    /// 从单个基础量纲和幂次创建
    /// Create from single base dimension with power
    pub fn from_base_with_power(
        name: String,
        dimension: FundamentalQuantityEnum,
        power: i64,
    ) -> Self {
        if power == 0 {
            Self::none(name)
        } else {
            Self::from_base_with_power_and_domain(
                name,
                dimension,
                power,
                QuantityDomain::Continuous,
            )
        }
    }

    /// 从单个基础量纲、幂次和取值域创建
    /// Create from single base dimension, power, and value domain
    pub fn from_base_with_power_and_domain(
        name: String,
        dimension: FundamentalQuantityEnum,
        power: i64,
        domain: QuantityDomain,
    ) -> Self {
        if power == 0 {
            Self::none(name)
        } else {
            Self {
                inner: Arc::new(DerivedQuantityInner {
                    name,
                    quantities: vec![FundamentalQuantity::new(dimension, power)],
                    domain,
                    symbol: OnceLock::new(),
                }),
            }
        }
    }

    /// 从量纲幂次列表创建
    /// Create from dimension power list
    pub fn from_quantities(name: String, quantities: Vec<FundamentalQuantity>) -> Self {
        Self::from_quantities_with_domain(name, quantities, QuantityDomain::Continuous)
    }

    /// 从量纲幂次列表和取值域创建
    /// Create from dimension powers and value domain
    pub fn from_quantities_with_domain(
        name: String,
        quantities: Vec<FundamentalQuantity>,
        domain: QuantityDomain,
    ) -> Self {
        Self {
            inner: Arc::new(DerivedQuantityInner {
                name,
                quantities: simplify(quantities),
                domain,
                symbol: OnceLock::new(),
            }),
        }
    }

    /// 添加量纲幂次，返回新实例
    /// Add dimension power, returns new instance
    pub fn add_power(&self, dimension: FundamentalQuantityEnum, power: i64) -> Self {
        if power == 0 {
            return self.clone();
        }

        Self {
            inner: Arc::new(DerivedQuantityInner {
                name: self.inner.name.clone(),
                quantities: add_power(&self.inner.quantities, dimension, power),
                domain: QuantityDomain::Continuous,
                symbol: OnceLock::new(),
            }),
        }
    }

    /// 获取量纲幂次
    /// Get dimension power
    pub fn get_power(&self, dimension: &FundamentalQuantityEnum) -> i64 {
        self.inner
            .quantities
            .iter()
            .find(|dp| &dp.dimension == dimension)
            .map(|dp| dp.power)
            .unwrap_or(0)
    }

    /// 是否为无量纲
    /// Check if dimensionless
    pub fn is_none(&self) -> bool {
        self.inner.quantities.is_empty()
    }

    /// 获取量纲幂次迭代器
    /// Get dimension powers iterator
    pub fn powers(&self) -> impl Iterator<Item = &FundamentalQuantity> {
        self.inner.quantities.iter()
    }

    /// 获取取值域 / Get value domain
    pub fn domain(&self) -> QuantityDomain {
        self.inner.domain
    }

    /// 获取量纲名称
    /// Get dimension name
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// 获取量纲符号表示
    /// Get dimension symbol representation
    pub fn symbol(&self) -> &str {
        self.inner.symbol.get_or_init(|| {
            if self.is_none() {
                return "1".to_string(); // 无量纲用 1 表示
            }

            // 预分配容量：每个量纲大约需要 5 个字符（符号 + 幂次）
            // Pre-allocate capacity: each dimension needs about 5 chars (symbol + power)
            let mut result = String::with_capacity(self.inner.quantities.len() * 5);
            let mut first = true;

            for dp in &self.inner.quantities {
                if first {
                    first = false;
                } else {
                    result.push('·');
                }

                result.push_str(dp.dimension.symbol());

                if dp.power != 1 {
                    result.push_str(&format!("^{}", dp.power));
                }
            }

            result
        })
    }
}

pub struct DerivedQuantityBuilder {
    name: Option<String>,
    quantities: Vec<FundamentalQuantity>,
    domain: QuantityDomain,
}

impl DerivedQuantityBuilder {
    pub fn new(quantities: Vec<FundamentalQuantity>) -> Self {
        Self::new_with_domain(quantities, QuantityDomain::Continuous)
    }

    pub fn new_with_domain(quantities: Vec<FundamentalQuantity>, domain: QuantityDomain) -> Self {
        Self {
            name: None,
            quantities,
            domain,
        }
    }

    pub fn build(self) -> DerivedQuantity {
        DerivedQuantity::from_quantities_with_domain(
            self.name.unwrap_or_else(|| "".to_string()),
            self.quantities.clone(),
            self.domain,
        )
    }

    pub fn domain(&self) -> QuantityDomain {
        self.domain
    }

    pub fn name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }
}

impl Clone for DerivedQuantityInner {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            quantities: self.quantities.clone(),
            domain: self.domain,
            symbol: OnceLock::new(),
        }
    }
}

impl PartialEq for DerivedQuantityInner {
    fn eq(&self, other: &Self) -> bool {
        // 只比较量纲内容，忽略 name
        // Only compare dimension content, ignore name
        self.quantities == other.quantities
    }
}

impl PartialEq for DerivedQuantity {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for DerivedQuantity {}

impl Default for DerivedQuantity {
    fn default() -> Self {
        Self::none("".to_string())
    }
}

impl fmt::Display for DerivedQuantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

impl Hash for DerivedQuantity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.symbol().hash(state);
    }
}

impl From<DerivedQuantityBuilder> for DerivedQuantity {
    fn from(builder: DerivedQuantityBuilder) -> Self {
        builder.build()
    }
}

// ============================================================================
// 运行时量纲运算 / Runtime dimension operations
// ============================================================================

impl Mul for DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        let new_quantities = add_powers(self.inner.quantities.clone(), &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain() * rhs.domain())
    }
}

impl Mul for &DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        let new_quantities = add_powers(self.inner.quantities.clone(), &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain() * rhs.domain())
    }
}

impl Mul for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn mul(self, rhs: Self) -> Self::Output {
        let new_quantities = add_powers(self.quantities, &rhs.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain * rhs.domain)
    }
}

impl Mul<DerivedQuantity> for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn mul(self, rhs: DerivedQuantity) -> Self::Output {
        let new_quantities = add_powers(self.quantities, &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain * rhs.domain())
    }
}

impl Mul<&DerivedQuantity> for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn mul(self, rhs: &DerivedQuantity) -> Self::Output {
        let new_quantities = add_powers(self.quantities, &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain * rhs.domain())
    }
}

impl Mul<i64> for DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        // 预分配容量
        // Pre-allocate capacity
        let mut new_quantities = Vec::with_capacity(self.inner.quantities.len());
        for dp in &self.inner.quantities {
            new_quantities.push(FundamentalQuantity::new(
                dp.dimension.clone(),
                dp.power * rhs,
            ));
        }
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain().pow(rhs))
    }
}

impl Mul<i64> for &DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn mul(self, rhs: i64) -> Self::Output {
        // 预分配容量
        // Pre-allocate capacity
        let mut new_quantities = Vec::with_capacity(self.inner.quantities.len());
        for dp in &self.inner.quantities {
            new_quantities.push(FundamentalQuantity::new(
                dp.dimension.clone(),
                dp.power * rhs,
            ));
        }
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain().pow(rhs))
    }
}

impl Mul<i64> for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn mul(mut self, rhs: i64) -> Self::Output {
        self.quantities.iter_mut().for_each(|dp| dp.power *= rhs);
        self.domain = self.domain.pow(rhs);
        self
    }
}

impl Div for DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        let new_quantities = sub_powers(self.inner.quantities.clone(), &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain() / rhs.domain())
    }
}

impl Div for &DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        let new_quantities = sub_powers(self.inner.quantities.clone(), &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain() / rhs.domain())
    }
}

impl Div for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn div(self, rhs: Self) -> Self::Output {
        let new_quantities = sub_powers(self.quantities, &rhs.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain / rhs.domain)
    }
}

impl Div<DerivedQuantity> for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn div(self, rhs: DerivedQuantity) -> Self::Output {
        let new_quantities = sub_powers(self.quantities, &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain / rhs.domain())
    }
}

impl Div<&DerivedQuantity> for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn div(self, rhs: &DerivedQuantity) -> Self::Output {
        let new_quantities = sub_powers(self.quantities, &rhs.inner.quantities);
        DerivedQuantityBuilder::new_with_domain(new_quantities, self.domain / rhs.domain())
    }
}

impl Div<i64> for DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        // 预分配容量
        // Pre-allocate capacity
        let mut new_quantities = Vec::with_capacity(self.inner.quantities.len());
        for dp in &self.inner.quantities {
            new_quantities.push(FundamentalQuantity::new(
                dp.dimension.clone(),
                dp.power / rhs,
            ));
        }
        DerivedQuantityBuilder::new_with_domain(new_quantities, QuantityDomain::Continuous)
    }
}

impl Div<i64> for &DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn div(self, rhs: i64) -> Self::Output {
        // 预分配容量
        // Pre-allocate capacity
        let mut new_quantities = Vec::with_capacity(self.inner.quantities.len());
        for dp in &self.inner.quantities {
            new_quantities.push(FundamentalQuantity::new(
                dp.dimension.clone(),
                dp.power / rhs,
            ));
        }
        DerivedQuantityBuilder::new_with_domain(new_quantities, QuantityDomain::Continuous)
    }
}

impl Div<i64> for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn div(mut self, rhs: i64) -> Self::Output {
        self.quantities.iter_mut().for_each(|dp| dp.power /= rhs);
        self.domain = QuantityDomain::Continuous;
        self
    }
}

impl Reciprocal for DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn reciprocal(self) -> DerivedQuantityBuilder {
        // 预分配容量
        // Pre-allocate capacity
        let mut new_quantities = Vec::with_capacity(self.inner.quantities.len());
        for dp in &self.inner.quantities {
            new_quantities.push(FundamentalQuantity {
                dimension: dp.dimension.clone(),
                power: -dp.power,
            });
        }
        DerivedQuantityBuilder::new_with_domain(new_quantities, QuantityDomain::Continuous)
    }
}

impl Reciprocal for &DerivedQuantity {
    type Output = DerivedQuantityBuilder;

    fn reciprocal(self) -> DerivedQuantityBuilder {
        // 预分配容量
        // Pre-allocate capacity
        let mut new_quantities = Vec::with_capacity(self.inner.quantities.len());
        for dp in &self.inner.quantities {
            new_quantities.push(FundamentalQuantity {
                dimension: dp.dimension.clone(),
                power: -dp.power,
            });
        }
        DerivedQuantityBuilder::new_with_domain(new_quantities, QuantityDomain::Continuous)
    }
}

impl Reciprocal for DerivedQuantityBuilder {
    type Output = DerivedQuantityBuilder;

    fn reciprocal(mut self) -> DerivedQuantityBuilder {
        self.quantities.iter_mut().for_each(|dp| {
            dp.power = -dp.power;
        });
        self.domain = QuantityDomain::Continuous;
        self
    }
}

// ============================================================================
// 编译时导出量纲特征 / Compile-time derived dimension trait
// ============================================================================

/// 合并编译时量纲符号
/// Merge compile-time dimension symbol
fn merge_ct_symbol<Q: CTFundamentalQuantityTrait>(mut first: bool, result: &mut String) -> bool {
    if first {
        first = false;
    } else {
        result.push('·');
    }

    result.push_str(<Q as CTFundamentalQuantityTrait>::D::SYMBOL);

    if Q::P::I64 != 1 {
        result.push_str(&format!("^{}", Q::P::I64));
    }

    first
}

/// CTDerivedQuantity - 编译时导出量纲特征
/// CTDerivedQuantity - Compile-time derived dimension trait
///
/// 每个导出量纲类型需要指定所有基础量纲的幂次
/// Each derived dimension type needs to specify powers for all base dimensions
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::dimension::derived_quantity::CTDerivedQuantity;
/// use ospf_rust_quantities::dimension::{L2, M0, T0, I0, Theta0, N0, J0, Info0, Phi0, Omega0};
///
/// struct Area;
/// impl CTDerivedQuantity for Area {
///     type L = L2;  // L²
///     type M = M0;  // M⁰
///     type T = T0;  // T⁰
///     type I = I0;
///     type Theta = Theta0;
///     type N = N0;
///     type J = J0;
///     type Info = Info0;
///     type Phi = Phi0;
///     type Omega = Omega0;
///     const NAME: &'static str = "Area";
/// }
/// ```
pub trait CTDerivedQuantity {
    /// L - 长度幂次 / Length power
    type L: CTFundamentalQuantityTrait<D = L>;
    /// M - 质量幂次 / Mass power
    type M: CTFundamentalQuantityTrait<D = M>;
    /// T - 时间幂次 / Time power
    type T: CTFundamentalQuantityTrait<D = T>;
    /// I - 电流幂次 / Electric Current power
    type I: CTFundamentalQuantityTrait<D = I>;
    /// Θ - 热力学温度幂次 / Thermodynamic Temperature power
    type Theta: CTFundamentalQuantityTrait<D = Theta>;
    /// N - 物质的量幂次 / Amount of Substance power
    type N: CTFundamentalQuantityTrait<D = N>;
    /// J - 发光强度幂次 / Luminous Intensity power
    type J: CTFundamentalQuantityTrait<D = J>;
    /// ℐ - 信息量幂次 / Information power
    type Info: CTFundamentalQuantityTrait<D = Info>;
    /// φ - 平面角幂次 / Plane Angle power
    type Phi: CTFundamentalQuantityTrait<D = Phi>;
    /// Ω - 立体角幂次 / Solid Angle power
    type Omega: CTFundamentalQuantityTrait<D = Omega>;

    /// 量纲名称 / Dimension name
    const NAME: &'static str = "";

    /// 取值域 / Value domain
    const DOMAIN: QuantityDomain = QuantityDomain::Continuous;

    /// 量纲符号 / Dimension symbol
    const SYMBOL: Lazy<String> = Lazy::new(|| {
        let mut result = String::new();
        let mut first = true;

        if <Self::L as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::L>(first, &mut result);
        }
        if <Self::M as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::M>(first, &mut result);
        }
        if <Self::T as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::T>(first, &mut result);
        }
        if <Self::I as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::I>(first, &mut result);
        }
        if <Self::Theta as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::Theta>(first, &mut result);
        }
        if <Self::N as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::N>(first, &mut result);
        }
        if <Self::J as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::J>(first, &mut result);
        }
        if <Self::Info as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::Info>(first, &mut result);
        }
        if <Self::Phi as CTFundamentalQuantityTrait>::P::I64 != 0 {
            first = merge_ct_symbol::<Self::Phi>(first, &mut result);
        }
        if <Self::Omega as CTFundamentalQuantityTrait>::P::I64 != 0 {
            merge_ct_symbol::<Self::Omega>(first, &mut result);
        }

        if result.is_empty() {
            "1".to_string()
        } else {
            result
        }
    });

    /// 运行时量纲值 / Runtime dimension value
    const INSTANT: Lazy<DerivedQuantity> = Lazy::new(|| {
        DerivedQuantity::from_quantities_with_domain(
            Self::NAME.to_string(),
            vec![
                Self::L::INSTANT.clone(),
                Self::M::INSTANT.clone(),
                Self::T::INSTANT.clone(),
                Self::I::INSTANT.clone(),
                Self::Theta::INSTANT.clone(),
                Self::N::INSTANT.clone(),
                Self::J::INSTANT.clone(),
                Self::Info::INSTANT.clone(),
                Self::Phi::INSTANT.clone(),
                Self::Omega::INSTANT.clone(),
            ],
            Self::DOMAIN,
        )
    });
}

// ============================================================================
// 编译时量纲相等判定 / Compile-time dimension equality
// ============================================================================

/// SameDerivedDimension - 编译时导出量纲相等 marker trait
/// SameDerivedDimension - Compile-time same derived dimension marker trait
///
/// 当两个导出量纲的所有基础量纲幂次都相同时，自动实现此 trait
/// This trait is automatically implemented when all base dimension powers are equal
pub trait SameDerivedDimension<Q: CTDerivedQuantity>: CTDerivedQuantity {}

impl<Q1, Q2> SameDerivedDimension<Q2> for Q1
where
    Q1: CTDerivedQuantity,
    Q2: CTDerivedQuantity,
    Q1::L: SameFundamentalQuantity<Q2::L>,
    Q1::M: SameFundamentalQuantity<Q2::M>,
    Q1::T: SameFundamentalQuantity<Q2::T>,
    Q1::I: SameFundamentalQuantity<Q2::I>,
    Q1::Theta: SameFundamentalQuantity<Q2::Theta>,
    Q1::N: SameFundamentalQuantity<Q2::N>,
    Q1::J: SameFundamentalQuantity<Q2::J>,
    Q1::Info: SameFundamentalQuantity<Q2::Info>,
    Q1::Phi: SameFundamentalQuantity<Q2::Phi>,
    Q1::Omega: SameFundamentalQuantity<Q2::Omega>,
{
}

// ============================================================================
// 编译时导出量纲运算 / Compile-time derived dimension operations
// ============================================================================

/// CTDerivedMul - 编译时导出量纲乘法
/// CTDerivedMul - Compile-time derived dimension multiplication
///
/// 两个导出量纲相乘，对应的基础量纲幂次相加
/// Multiply two derived dimensions, add corresponding base dimension powers
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::dimension::derived_quantity::{CTDerivedMul, CTDerivedQuantity};
/// use ospf_rust_quantities::dimension::derived::Length;
///
/// // Area = Length * Length = L²
/// type Area = CTDerivedMul<Length, Length>;
/// ```
pub struct CTDerivedMul<D1: CTDerivedQuantity, D2: CTDerivedQuantity> {
    _marker: PhantomData<(D1, D2)>,
}

impl<D1: CTDerivedQuantity, D2: CTDerivedQuantity> CTDerivedQuantity for CTDerivedMul<D1, D2>
where
    <<D1 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P:
        Add<<<D2 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P as Add<
        <<D2 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
{
    type L = CTFundamentalMul<D1::L, D2::L>;
    type M = CTFundamentalMul<D1::M, D2::M>;
    type T = CTFundamentalMul<D1::T, D2::T>;
    type I = CTFundamentalMul<D1::I, D2::I>;
    type Theta = CTFundamentalMul<D1::Theta, D2::Theta>;
    type N = CTFundamentalMul<D1::N, D2::N>;
    type J = CTFundamentalMul<D1::J, D2::J>;
    type Info = CTFundamentalMul<D1::Info, D2::Info>;
    type Phi = CTFundamentalMul<D1::Phi, D2::Phi>;
    type Omega = CTFundamentalMul<D1::Omega, D2::Omega>;

    const DOMAIN: QuantityDomain = D1::DOMAIN.mul_domain(D2::DOMAIN);
}

/// CTDerivedDiv - 编译时导出量纲除法
/// CTDerivedDiv - Compile-time derived dimension division
///
/// 两个导出量纲相除，对应的基础量纲幂次相减
/// Divide two derived dimensions, subtract corresponding base dimension powers
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::dimension::derived_quantity::{CTDerivedDiv, CTDerivedQuantity};
/// use ospf_rust_quantities::dimension::derived::{Length, Time};
///
/// // Velocity = Length / Time = L·T⁻¹
/// type Velocity = CTDerivedDiv<Length, Time>;
/// ```
pub struct CTDerivedDiv<D1: CTDerivedQuantity, D2: CTDerivedQuantity> {
    _marker: PhantomData<(D1, D2)>,
}

impl<D1: CTDerivedQuantity, D2: CTDerivedQuantity> CTDerivedQuantity for CTDerivedDiv<D1, D2>
where
    <<D1 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
    <<D1 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P:
        Sub<<<D2 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P>,
    <<<D1 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P as Sub<
        <<D2 as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P,
    >>::Output: Integer,
{
    type L = CTFundamentalDiv<D1::L, D2::L>;
    type M = CTFundamentalDiv<D1::M, D2::M>;
    type T = CTFundamentalDiv<D1::T, D2::T>;
    type I = CTFundamentalDiv<D1::I, D2::I>;
    type Theta = CTFundamentalDiv<D1::Theta, D2::Theta>;
    type N = CTFundamentalDiv<D1::N, D2::N>;
    type J = CTFundamentalDiv<D1::J, D2::J>;
    type Info = CTFundamentalDiv<D1::Info, D2::Info>;
    type Phi = CTFundamentalDiv<D1::Phi, D2::Phi>;
    type Omega = CTFundamentalDiv<D1::Omega, D2::Omega>;

    const DOMAIN: QuantityDomain = D1::DOMAIN.div_domain(D2::DOMAIN);
}

/// CTDerivedPow - 编译时导出量纲幂次
/// CTDerivedPow - Compile-time derived dimension power
///
/// 将导出量纲的所有基础量纲幂次乘以一个常数
/// Multiply all base dimension powers by a constant
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::dimension::derived_quantity::{CTDerivedPow, CTDerivedQuantity};
/// use ospf_rust_quantities::dimension::derived::Length;
/// use typenum::P3;
///
/// // Volume = Length³ = (Length)³
/// type Volume = CTDerivedPow<Length, P3>;
/// ```
pub struct CTDerivedPow<D: CTDerivedQuantity, N: Integer> {
    _marker: PhantomData<(D, N)>,
}

impl<D: CTDerivedQuantity, N: Integer> CTDerivedQuantity for CTDerivedPow<D, N>
where
    <<D as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
    <<D as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P: Mul<N>,
    <<<D as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P as Mul<N>>::Output: Integer,
{
    type L = CTFundamentalPow<D::L, N>;
    type M = CTFundamentalPow<D::M, N>;
    type T = CTFundamentalPow<D::T, N>;
    type I = CTFundamentalPow<D::I, N>;
    type Theta = CTFundamentalPow<D::Theta, N>;
    type N = CTFundamentalPow<D::N, N>;
    type J = CTFundamentalPow<D::J, N>;
    type Info = CTFundamentalPow<D::Info, N>;
    type Phi = CTFundamentalPow<D::Phi, N>;
    type Omega = CTFundamentalPow<D::Omega, N>;

    const DOMAIN: QuantityDomain = D::DOMAIN.pow(N::I64);
}

/// CTDerivedReciprocal - 编译时导出量纲倒数
/// CTDerivedReciprocal - Compile-time derived dimension reciprocal
///
/// 将导出量纲的所有基础量纲幂次取负
/// Negate all base dimension powers
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::dimension::derived_quantity::{CTDerivedReciprocal, CTDerivedQuantity};
/// use ospf_rust_quantities::dimension::derived::Time;
///
/// // Frequency = Time⁻¹ = 1/Time
/// type Frequency = CTDerivedReciprocal<Time>;
/// ```
pub struct CTDerivedReciprocal<D: CTDerivedQuantity> {
    _marker: PhantomData<D>,
}

impl<D: CTDerivedQuantity> CTDerivedQuantity for CTDerivedReciprocal<D>
where
    <<D as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::L as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::M as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::T as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::I as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::Theta as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::N as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::J as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::Info as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::Phi as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
    <<D as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P: Neg,
    <<<D as CTDerivedQuantity>::Omega as CTFundamentalQuantityTrait>::P as Neg>::Output: Integer,
{
    type L = CTFundamentalReciprocal<D::L>;
    type M = CTFundamentalReciprocal<D::M>;
    type T = CTFundamentalReciprocal<D::T>;
    type I = CTFundamentalReciprocal<D::I>;
    type Theta = CTFundamentalReciprocal<D::Theta>;
    type N = CTFundamentalReciprocal<D::N>;
    type J = CTFundamentalReciprocal<D::J>;
    type Info = CTFundamentalReciprocal<D::Info>;
    type Phi = CTFundamentalReciprocal<D::Phi>;
    type Omega = CTFundamentalReciprocal<D::Omega>;

    const DOMAIN: QuantityDomain = QuantityDomain::Continuous;
}

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::{
        I0, I1, Info0, J0, L0, L1, L2, M0, M1, N0, Omega0, Phi0, T0, T1, Theta0,
    };

    // ========================================================================
    // 运行时量纲测试 / Runtime dimension tests
    // ========================================================================

    #[test]
    fn test_dimensionless() {
        // 测试无量纲的创建和符号 / Test dimensionless creation and symbol
        let dim = DerivedQuantity::none("".to_string());
        assert!(dim.is_none());
        assert_eq!(dim.symbol(), "1");
    }

    #[test]
    fn test_dimension_multiplication() {
        // 测试量纲乘法：L * T^-1 = L·T^-1（速度）
        // Test dimension multiplication: L * T^-1 = L·T^-1 (velocity)
        let length = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let time = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Time);
        let velocity = (length / time).build();

        assert_eq!(velocity.symbol(), "L·T^-1");
    }

    #[test]
    fn test_dimension_division() {
        // 测试量纲除法：L / T = L·T^-1（速度）
        // Test dimension division: L / T = L·T^-1 (velocity)
        let length = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let time = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Time);
        let velocity = (length.clone() / time.clone()).build();

        assert_eq!(velocity.symbol(), "L·T^-1");

        // 测试 L / L = 1（无量纲）
        // Test L / L = 1 (dimensionless)
        let dimensionless = (length.clone() / length).build();
        assert!(dimensionless.is_none());
    }

    #[test]
    fn test_dimension_power() {
        // 测试量纲幂次：L * L = L^2（面积）
        // Test dimension power: L * L = L^2 (area)
        let length = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let area = (length.clone() * length.clone()).build();
        // 实际符号格式是 L^2
        // Actual symbol format is L^2
        assert_eq!(area.symbol(), "L^2");

        // 测试 L * L * L = L^3（体积）- 使用新的长度实例
        // Test L * L * L = L^3 (volume) - use new length instance
        let length3 = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let length4 = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let length5 = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let volume = (length3 * length4 * length5).build();
        assert_eq!(volume.symbol(), "L^3");
    }

    #[test]
    fn test_dimension_reciprocal() {
        // 测试量纲倒数：L -> L^-1
        // Test dimension reciprocal: L -> L^-1
        let length = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let reciprocal = length.reciprocal().build();
        assert_eq!(reciprocal.symbol(), "L^-1");

        // 测试 T -> T^-1（频率）
        // Test T -> T^-1 (frequency)
        let time = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Time);
        let frequency = time.reciprocal().build();
        assert_eq!(frequency.symbol(), "T^-1");
    }

    #[test]
    fn test_add_power() {
        // 测试添加量纲幂次（不可变方式）
        // Test adding dimension power (immutable style)
        let dim = DerivedQuantity::none("".to_string());
        let dim = dim.add_power(FundamentalQuantityEnum::Length, 2);
        // 验证幂次而不是符号（符号有缓存）
        // Verify power instead of symbol (symbol is cached)
        assert_eq!(dim.get_power(&FundamentalQuantityEnum::Length), 2);

        // 添加相同量纲，幂次相加
        // Add same dimension, powers add up
        let dim = dim.add_power(FundamentalQuantityEnum::Length, 1);
        assert_eq!(dim.get_power(&FundamentalQuantityEnum::Length), 3);

        // 添加负幂次
        // Add negative power
        let dim = dim.add_power(FundamentalQuantityEnum::Length, -1);
        assert_eq!(dim.get_power(&FundamentalQuantityEnum::Length), 2);

        // 添加零幂次，无变化
        // Add zero power, no change
        let dim = dim.add_power(FundamentalQuantityEnum::Length, 0);
        assert_eq!(dim.get_power(&FundamentalQuantityEnum::Length), 2);
    }

    #[test]
    fn test_get_power() {
        // 测试获取量纲幂次（不可变方式）
        // Test getting dimension power (immutable style)
        let dim = DerivedQuantity::none("".to_string());
        let dim = dim.add_power(FundamentalQuantityEnum::Length, 2);
        let dim = dim.add_power(FundamentalQuantityEnum::Mass, 1);

        assert_eq!(dim.get_power(&FundamentalQuantityEnum::Length), 2);
        assert_eq!(dim.get_power(&FundamentalQuantityEnum::Mass), 1);
        assert_eq!(dim.get_power(&FundamentalQuantityEnum::Time), 0);
    }

    #[test]
    fn test_from_base_with_power() {
        // 测试从基础量纲和幂次创建
        // Test creating from base dimension with power
        let dim = DerivedQuantity::from_base_with_power(
            "".to_string(),
            FundamentalQuantityEnum::Length,
            3,
        );
        assert_eq!(dim.symbol(), "L^3");

        // 幂次为 0 时创建无量纲
        // Create dimensionless when power is 0
        let dim_zero = DerivedQuantity::from_base_with_power(
            "".to_string(),
            FundamentalQuantityEnum::Length,
            0,
        );
        assert!(dim_zero.is_none());
    }

    #[test]
    fn test_complex_dimension() {
        // 测试复杂量纲：力 = M·L·T^-2
        // Test complex dimension: force = M·L·T^-2
        let mass = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Mass);
        let length = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let time = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Time);

        let time_squared = (time.clone() * time).build();
        let velocity = (length.clone() / time_squared).build();
        let force = (mass * velocity).build();

        // 力的符号应该是 M·L·T^-2 或类似
        // Force symbol should be M·L·T^-2 or similar
        let symbol = force.symbol();
        assert!(symbol.contains("M"));
        assert!(symbol.contains("L"));
        assert!(symbol.contains("T"));
    }

    #[test]
    fn test_quantity_domain_composition() {
        // 只有离散量相乘仍为离散 / Only discrete times discrete remains discrete
        assert_eq!(
            QuantityDomain::Discrete * QuantityDomain::Discrete,
            QuantityDomain::Discrete
        );
        assert_eq!(
            QuantityDomain::Discrete * QuantityDomain::Continuous,
            QuantityDomain::Continuous
        );
        assert_eq!(
            QuantityDomain::Discrete / QuantityDomain::Discrete,
            QuantityDomain::Continuous
        );
        assert_eq!(QuantityDomain::Discrete.pow(0), QuantityDomain::Continuous);
        assert_eq!(QuantityDomain::Discrete.pow(-1), QuantityDomain::Continuous);
        assert_eq!(QuantityDomain::Discrete.pow(2), QuantityDomain::Discrete);
    }

    #[test]
    fn test_runtime_derived_quantity_domain() {
        let information = DerivedQuantity::from_base_with_power_and_domain(
            "".to_string(),
            FundamentalQuantityEnum::Information,
            1,
            QuantityDomain::Discrete,
        );
        let length = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);

        assert_eq!(information.domain(), QuantityDomain::Discrete);
        assert_eq!(
            (&information * &information).build().domain(),
            QuantityDomain::Discrete
        );
        assert_eq!(
            (&information * &length).build().domain(),
            QuantityDomain::Continuous
        );
        assert_eq!(
            (&information / &information).build().domain(),
            QuantityDomain::Continuous
        );
        assert_eq!(
            (&information).reciprocal().build().domain(),
            QuantityDomain::Continuous
        );
    }

    // ========================================================================
    // 编译时量纲测试 / Compile-time dimension tests
    // ========================================================================

    #[test]
    fn test_compile_time_dimensionless() {
        // 验证无量纲的幂次都是 0 / Verify dimensionless powers are all 0
        assert_eq!(<L0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<M0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<T0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<I0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<Theta0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<N0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<J0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<Info0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<Phi0 as CTFundamentalQuantityTrait>::P::I64, 0);
        assert_eq!(<Omega0 as CTFundamentalQuantityTrait>::P::I64, 0);
    }

    #[test]
    fn test_compile_time_single_power() {
        // 验证单个基础量纲的幂次 / Verify single base dimension powers
        assert_eq!(<L1 as CTFundamentalQuantityTrait>::P::I64, 1);
        assert_eq!(<M1 as CTFundamentalQuantityTrait>::P::I64, 1);
        assert_eq!(<T1 as CTFundamentalQuantityTrait>::P::I64, 1);
        assert_eq!(<I1 as CTFundamentalQuantityTrait>::P::I64, 1);
    }

    #[test]
    fn test_compile_time_derived_mul() {
        // 测试编译时量纲乘法：L^1 * L^1 = L^2
        // Test compile-time dimension multiplication: L^1 * L^1 = L^2
        type TestL2 = CTFundamentalMul<L1, L1>;
        assert_eq!(<TestL2 as CTFundamentalQuantityTrait>::P::I64, 2);

        // L^1 * L^2 = L^3
        type TestL3 = CTFundamentalMul<L1, TestL2>;
        assert_eq!(<TestL3 as CTFundamentalQuantityTrait>::P::I64, 3);
    }

    #[test]
    fn test_compile_time_derived_div() {
        // 测试编译时量纲除法：L^2 / L^1 = L^1
        // Test compile-time dimension division: L^2 / L^1 = L^1
        type L1FromDiv = CTFundamentalDiv<L2, L1>;
        assert_eq!(<L1FromDiv as CTFundamentalQuantityTrait>::P::I64, 1);

        // L^1 / L^1 = L^0
        type L0FromDiv = CTFundamentalDiv<L1, L1>;
        assert_eq!(<L0FromDiv as CTFundamentalQuantityTrait>::P::I64, 0);
    }

    #[test]
    fn test_compile_time_derived_pow() {
        // 测试编译时量纲幂次：(L^1)^2 = L^2
        // Test compile-time dimension power: (L^1)^2 = L^2
        use typenum::P2;
        type L2FromPow = CTFundamentalPow<L1, P2>;
        assert_eq!(<L2FromPow as CTFundamentalQuantityTrait>::P::I64, 2);

        // (L^2)^2 = L^4
        type L4FromPow = CTFundamentalPow<L2, P2>;
        assert_eq!(<L4FromPow as CTFundamentalQuantityTrait>::P::I64, 4);
    }

    #[test]
    fn test_compile_time_derived_reciprocal() {
        // 测试编译时量纲倒数：L^1 -> L^-1
        // Test compile-time dimension reciprocal: L^1 -> L^-1
        type LNeg1 = CTFundamentalReciprocal<L1>;
        assert_eq!(<LNeg1 as CTFundamentalQuantityTrait>::P::I64, -1);

        // T^1 -> T^-1（频率）
        // T^1 -> T^-1 (frequency)
        type TNeg1 = CTFundamentalReciprocal<T1>;
        assert_eq!(<TNeg1 as CTFundamentalQuantityTrait>::P::I64, -1);
    }

    #[test]
    fn test_compile_time_quantity_domain() {
        use crate::dimension::derived::{Bandwidth, Information};
        use typenum::P2;

        type InformationSquared = CTDerivedPow<Information, P2>;
        type InformationRatio = CTDerivedDiv<Information, Information>;

        assert_eq!(Information::DOMAIN, QuantityDomain::Discrete);
        assert_eq!(
            <InformationSquared as CTDerivedQuantity>::DOMAIN,
            QuantityDomain::Discrete
        );
        assert_eq!(
            <InformationRatio as CTDerivedQuantity>::DOMAIN,
            QuantityDomain::Continuous
        );
        assert_eq!(Bandwidth::DOMAIN, QuantityDomain::Continuous);
    }

    #[test]
    fn test_same_derived_dimension() {
        // 测试编译时量纲相等判定
        // Test compile-time dimension equality
        use crate::dimension::derived::{Area, Length};

        fn assert_same_dim<
            D1: CTDerivedQuantity + SameDerivedDimension<D2>,
            D2: CTDerivedQuantity,
        >() {
        }

        // Length 和 Length 相同
        // Length and Length are the same
        assert_same_dim::<Length, Length>();

        // Area 和 Area 相同
        // Area and Area are the same
        assert_same_dim::<Area, Area>();
    }

    #[test]
    fn test_derived_quantity_trait() {
        // 测试导出量纲特征
        // Test derived quantity trait

        // 定义一个简单的导出量纲：面积 L^2
        // Define a simple derived dimension: area L^2
        struct TestArea;
        impl CTDerivedQuantity for TestArea {
            type L = L2;
            type M = M0;
            type T = T0;
            type I = I0;
            type Theta = Theta0;
            type N = N0;
            type J = J0;
            type Info = Info0;
            type Phi = Phi0;
            type Omega = Omega0;
            const NAME: &'static str = "TestArea";
        }

        // 验证符号
        // Verify symbol
        assert_eq!(*TestArea::SYMBOL, "L^2");
        assert_eq!(TestArea::NAME, "TestArea");
    }

    #[test]
    fn test_dimension_display() {
        // 测试量纲显示
        // Test dimension display
        let dim = DerivedQuantity::from_base("Test".to_string(), FundamentalQuantityEnum::Length);
        let display = format!("{}", dim);
        assert_eq!(display, "L");
    }

    #[test]
    fn test_dimension_hash() {
        // 测试量纲哈希
        // Test dimension hash
        use std::collections::HashSet;

        let mut set = HashSet::new();
        let dim1 = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let dim2 = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let dim3 = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Mass);

        set.insert(dim1.clone());
        assert!(set.contains(&dim2)); // 相同量纲
        assert!(!set.contains(&dim3)); // 不同量纲
    }
}
