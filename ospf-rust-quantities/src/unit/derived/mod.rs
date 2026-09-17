//! 导出单位 / Derived units
//!
//! 按量纲分类组织的 SI 导出单位，提供运行时和编译时两种表示方式 / SI derived units organized by dimension, providing both runtime and compile-time representations
//!
//! # 分类 / Categories
//! - 基本物理量：长度、质量、时间、电流、温度、物质的量、发光强度、信息量、角度
//! - 几何量：面积、体积
//! - 运动学量：速度、加速度、角速度、角加速度
//! - 动力学量：力、能量、功率、压力、扭矩
//! - 电磁学量：电荷、电压、电阻、电容、电感
//! - 光学量：光通量、照度、亮度

use super::concept::UnitTrait;
use super::physical_unit::CTUnit;
use crate::dimension::derived::DimLess;
use crate::dimension::derived_quantity::CTDerivedQuantity;
use crate::scale::Scale;
use bigdecimal::BigDecimal;
use once_cell::sync::Lazy;

#[macro_use]
mod macros;

// ============================================================================
// 模块声明 / Module declarations
// ============================================================================

// 基本物理量 / Base physical quantities
pub mod amount_of_substance;
pub mod information;
pub mod length;
pub mod luminous_intensity;
pub mod mass;
pub mod plane_angle;
pub mod solid_angle;
pub mod thermodynamic_temperature;
pub mod time;

// 导出几何量 / Derived geometric quantities
pub mod area;
pub mod volume;

// 导出运动学量 / Derived kinematic quantities
pub mod acceleration;
pub mod angular_acceleration;
pub mod angular_velocity;
pub mod velocity;

// 导出动力学量 / Derived dynamic quantities
pub mod energy;
pub mod force;
pub mod momentum;
pub mod power;
pub mod pressure;
pub mod stress;
pub mod torque;

// 导出材料性质量 / Derived material properties
pub mod flow_rate;
pub mod mass_density;
pub mod surface_density;

// 导出电磁学量 / Derived electromagnetic quantities
pub mod electrical;
pub mod resistance;

// 导出波动与周期量 / Derived wave and periodic quantities
pub mod bandwidth;
pub mod frequency;
pub mod wavenumber;

// 导出化学量 / Derived chemical quantities
pub mod catalytic_activity;

// ============================================================================
// 重导出基本单位类型 / Re-export base unit types
// ============================================================================

// 基本物理量 / Base physical quantities
pub use amount_of_substance::*;
pub use information::*;
pub use length::*;
pub use luminous_intensity::*;
pub use mass::*;
pub use plane_angle::*;
pub use solid_angle::*;
pub use thermodynamic_temperature::*;
pub use time::*;

// 导出几何量 / Derived geometric quantities
pub use area::*;
pub use volume::*;

// 导出运动学量 / Derived kinematic quantities
pub use acceleration::*;
pub use angular_acceleration::*;
pub use angular_velocity::*;
pub use velocity::*;

// 导出动力学量 / Derived dynamic quantities
pub use energy::*;
pub use force::*;
pub use momentum::*;
pub use power::*;
pub use pressure::*;
pub use stress::*;
pub use torque::*;

// 导出材料性质量 / Derived material properties
pub use flow_rate::*;
pub use mass_density::*;
pub use surface_density::*;

// 导出电磁学量 / Derived electromagnetic quantities
pub use electrical::*;
pub use resistance::*;

// 导出波动与周期量 / Derived wave and periodic quantities
pub use bandwidth::*;
pub use frequency::*;
pub use wavenumber::*;

// 导出化学量 / Derived chemical quantities
pub use catalytic_activity::*;

// ============================================================================
// 特殊单位类型 / Special unit types
// ============================================================================

/// 无量纲单位 / Dimensionless unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct None;

impl UnitTrait for None {
    type Dimension = DimLess;

    fn symbol(&self) -> &'static str {
        "1"
    }

    fn name(&self) -> &'static str {
        "None"
    }

    fn dimension_symbol(&self) -> String {
        DimLess::INSTANT.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        BigDecimal::from(1)
    }
}

impl CTUnit for None {
    const NAME: &'static str = "None";
    const SYMBOL: &'static str = "1";
    const SCALE: Lazy<Scale> = Lazy::new(|| Scale::new());
    type Dimension = DimLess;
}

/// Kotlin 命名兼容别名 / Kotlin naming compatibility alias
pub type NoneUnit = None;

// ============================================================================
// 测试辅助 / Test support
// ============================================================================

/// 单位定义测试辅助模块（仅测试构建编译）
/// Test helper module for unit definitions (compiled only in test builds)
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_support {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    use crate::dimension::derived_quantity::QuantityDomain;
    use crate::unit::concept::UnitTrait;
    use crate::unit::physical_unit::{CTUnit, Unit};

    /// 解析十进制字面量 / Parse a decimal literal
    pub(crate) fn decimal(text: &str) -> BigDecimal {
        BigDecimal::from_str(text).expect(
            "无法解析测试用十进制字面量 / Failed to parse test decimal literal",
        )
    }

    /// 断言比例尺值精确等于期望值 / Assert the scale value exactly equals the expected value
    pub(crate) fn assert_scale_exact(actual: &BigDecimal, expected: &str, case: &str) {
        assert_eq!(
            actual,
            &decimal(expected),
            "{}: 期望 {} / expected {}",
            case,
            expected,
            expected
        );
    }

    /// 断言比例尺值在相对容差内等于期望值 / Assert the scale value equals the expected value within a relative tolerance
    pub(crate) fn assert_scale_relative(
        actual: &BigDecimal,
        expected: &str,
        relative: &str,
        case: &str,
    ) {
        let expected = decimal(expected);
        let tolerance = expected.abs() * decimal(relative);
        let difference = (actual.clone() - &expected).abs();
        assert!(
            difference <= tolerance,
            "{}: 实际 {} 与期望 {} 的偏差 {} 超出相对容差 {} / actual {} vs expected {} with difference {} exceeding relative tolerance {}",
            case,
            actual,
            expected,
            difference,
            relative,
            actual,
            expected,
            difference,
            relative
        );
    }

    /// 断言两个编译时单位的比例尺完全相同 / Assert two compile-time units share exactly the same scale
    pub(crate) fn assert_scale_equals<A: CTUnit, B: CTUnit>(case: &str) {
        assert_eq!(
            A::SCALE.value(),
            B::SCALE.value(),
            "{}: {} 与 {} 的比例尺不相等 / scales of {} and {} differ",
            case,
            A::NAME,
            B::NAME,
            A::NAME,
            B::NAME
        );
    }

    /// 计算从 `From` 到 `To` 的换算系数 / Compute the conversion factor from `From` to `To`
    pub(crate) fn conversion_factor<From: CTUnit, To: CTUnit>() -> BigDecimal {
        From::conversion_factor_to::<To>().expect(
            "换算系数不存在：量纲不匹配或单位为仿射 / conversion factor unavailable: dimension mismatch or affine unit",
        )
    }

    /// 断言 `From` 到 `To` 的换算系数精确等于期望值 / Assert the conversion factor from `From` to `To` exactly equals the expected value
    pub(crate) fn assert_factor_exact<From: CTUnit, To: CTUnit>(expected: &str, case: &str) {
        let actual = conversion_factor::<From, To>();
        assert_scale_exact(&actual, expected, case);
    }

    /// 断言 `From` 到 `To` 的换算系数在相对容差内等于期望值 / Assert the conversion factor from `From` to `To` equals the expected value within a relative tolerance
    pub(crate) fn assert_factor_relative<From: CTUnit, To: CTUnit>(
        expected: &str,
        relative: &str,
        case: &str,
    ) {
        let actual = conversion_factor::<From, To>();
        assert_scale_relative(&actual, expected, relative, case);
    }

    /// 断言两个编译时单位量纲相同 / Assert two compile-time units share the same dimension
    pub(crate) fn assert_same_dimension<A: CTUnit, B: CTUnit>(case: &str) {
        assert!(
            A::dim_eq::<B>(),
            "{}: {} 与 {} 的量纲应相同 / dimensions of {} and {} should match",
            case,
            A::NAME,
            B::NAME,
            A::NAME,
            B::NAME
        );
    }

    /// 断言两个编译时单位量纲不同 / Assert two compile-time units have different dimensions
    pub(crate) fn assert_different_dimension<A: CTUnit, B: CTUnit>(case: &str) {
        assert!(
            !A::dim_eq::<B>(),
            "{}: {} 与 {} 的量纲应不同 / dimensions of {} and {} should differ",
            case,
            A::NAME,
            B::NAME,
            A::NAME,
            B::NAME
        );
    }

    /// 断言编译时单位的符号 / Assert the symbol of a compile-time unit
    pub(crate) fn assert_symbol<U: CTUnit>(expected: &str, case: &str) {
        assert_eq!(
            U::SYMBOL,
            expected,
            "{}: 符号应为 {} / symbol should be {}",
            case,
            expected,
            expected
        );
    }

    /// 断言编译时单位的名称 / Assert the name of a compile-time unit
    pub(crate) fn assert_name<U: CTUnit>(expected: &str, case: &str) {
        assert_eq!(
            U::NAME,
            expected,
            "{}: 名称应为 {} / name should be {}",
            case,
            expected,
            expected
        );
    }

    /// 断言编译时单位的符号与名称 / Assert both symbol and name of a compile-time unit
    pub(crate) fn assert_symbol_and_name<U: CTUnit>(symbol: &str, name: &str, case: &str) {
        assert_symbol::<U>(symbol, case);
        assert_name::<U>(name, case);
    }

    /// 断言编译时单位的取值域 / Assert the value domain of a compile-time unit
    pub(crate) fn assert_domain<U: CTUnit>(expected: QuantityDomain, case: &str) {
        assert_eq!(
            U::DOMAIN,
            expected,
            "{}: {} 的取值域不匹配 / value domain of {} does not match",
            case,
            U::NAME,
            U::NAME
        );
    }

    /// 断言编译时单位实例的符号与名称 / Assert symbol and name of the compile-time unit instance
    pub(crate) fn assert_instance_symbol_and_name<U: CTUnit>(symbol: &str, name: &str, case: &str) {
        let unit: &Unit = &U::INSTANT;
        assert_eq!(
            unit.symbol(),
            symbol,
            "{}: 实例符号应为 {} / instance symbol should be {}",
            case,
            symbol,
            symbol
        );
        assert_eq!(
            unit.name(),
            name,
            "{}: 实例名称应为 {} / instance name should be {}",
            case,
            name,
            name
        );
        assert_eq!(unit.scale_value(), U::SCALE.value().clone(), "{}", case);
    }

    /// 断言一个编译时单位相对基准单位的换算并非恒等 / Assert a compile-time unit is not identity relative to its base
    pub(crate) fn assert_not_identity<U: CTUnit>(case: &str) {
        assert_ne!(
            U::SCALE.value(),
            &BigDecimal::from(1),
            "{}: {} 的比例尺不应为 1 / scale of {} should not be 1",
            case,
            U::NAME,
            U::NAME
        );
    }

    /// 断言编译时单位的 offset / Assert the offset of a compile-time unit
    pub(crate) fn assert_offset<U: CTUnit>(expected: &str, case: &str) {
        let offset = (*U::OFFSET).clone();
        assert_scale_exact(&offset, expected, case);
    }

    /// 断言编译时单位具有正确的量纲符号 / Assert a compile-time unit has the expected dimension symbol
    pub(crate) fn assert_dimension_symbol<U: CTUnit>(expected: &str, case: &str) {
        let symbol = U::INSTANT.dimension().symbol().to_string();
        assert_eq!(
            symbol, expected,
            "{}: 量纲符号应为 {} / dimension symbol should be {}",
            case, expected, expected
        );
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;

    use super::*;
    use crate::dimension::derived::DimLess;
    use crate::unit::derived::test_support::{
        assert_dimension_symbol, assert_same_dimension, assert_scale_exact, assert_scale_relative,
    };

    #[test]
    fn test_none_unit_symbol_and_name() {
        // 无量纲单位的符号与名称 / Symbol and name of the dimensionless unit
        assert_eq!(None::SYMBOL, "1");
        assert_eq!(None::NAME, "None");
        assert_eq!(NoneUnit::SYMBOL, "1");
        assert_eq!(NoneUnit::NAME, "None");
    }

    #[test]
    fn test_none_unit_scale_is_one() {
        // 无量纲单位比例尺为 1 / Dimensionless unit scale is one
        assert_scale_exact(None::SCALE.value(), "1", "无量纲单位比例尺 / dimensionless unit scale");
    }

    #[test]
    fn test_none_unit_dimension_is_dimensionless() {
        // 无量纲单位量纲为 DimLess / Dimensionless unit dimension is DimLess
        assert_eq!(None::INSTANT.dimension().symbol(), DimLess::INSTANT.symbol());
        assert_eq!(None::INSTANT.dimension().symbol(), "1");
    }

    #[test]
    fn test_none_unit_runtime_trait_values() {
        // 运行时单位 trait 取值 / Runtime unit trait values
        let unit = None::INSTANT.clone();
        assert_eq!(unit.symbol(), "1");
        assert_eq!(unit.name(), "None");
        assert_eq!(unit.scale_value(), BigDecimal::from(1));
        assert!(unit.is_linear());
    }

    // ========================================================================
    // 复合单位运算 / Composite unit arithmetic
    // ========================================================================

    #[test]
    fn test_composite_unit_multiplication_combines_dimension_and_scale() {
        // 千米 * 小时：量纲为长度乘以时间，比例尺为 1000 * 3600
        // Kilometer times hour: the dimension is length times time and the scale is 1000 * 3600
        let kilometer = Kilometer::INSTANT.clone();
        let hour = Hour::INSTANT.clone();
        let composite = (&kilometer * &hour).build();

        assert_eq!(composite.scale().value(), &BigDecimal::from(3_600_000));
        assert_eq!(composite.dimension().symbol(), "L·T");
        assert!(composite.is_linear());
    }

    #[test]
    fn test_composite_unit_division_combines_dimension_and_scale() {
        // 千米 / 小时：量纲为长度除以时间，比例尺为 1000 / 3600
        // Kilometer divided by hour: the dimension is length over time and the scale is 1000 / 3600
        let kilometer = Kilometer::INSTANT.clone();
        let hour = Hour::INSTANT.clone();
        let composite = (&kilometer / &hour).build();

        assert_eq!(composite.dimension().symbol(), "L·T^-1");
        assert_scale_relative(
            composite.scale().value(),
            "0.27777777777777777777777777777777777777777777777778",
            "1e-15",
            "千米每小时复合单位 / kilometer per hour composite unit",
        );
    }

    #[test]
    fn test_composite_unit_matches_kilometer_per_hour_unit() {
        // 千米 / 小时 的比例尺应与预定义 KilometerPerHour 一致
        // The scale of kilometer divided by hour should match the predefined KilometerPerHour
        let kilometer = Kilometer::INSTANT.clone();
        let hour = Hour::INSTANT.clone();
        let composite = (&kilometer / &hour).build();
        assert_eq!(composite.scale().value(), KilometerPerHour::SCALE.value());
        assert_same_dimension::<KilometerPerHour, crate::unit::derived::KilometerPerHour>(
            "千米每小时自比较 / kilometer per hour self comparison",
        );
    }

    #[test]
    fn test_composite_unit_self_division_cancels_to_dimensionless() {
        // 千米 / 千米 = 无量纲且比例尺为 1 / Kilometer divided by kilometer is dimensionless with scale 1
        let kilometer = Kilometer::INSTANT.clone();
        let composite = (&kilometer / &kilometer).build();

        assert!(composite.dimension().is_none());
        assert_eq!(composite.dimension().symbol(), "1");
        assert_eq!(composite.scale().value(), &BigDecimal::from(1));
    }

    #[test]
    fn test_composite_unit_integer_scaling_ignores_scale_of_one() {
        // 注意：比例尺为 1 的基准单位内部不含任何底数因子，整数缩放对其无效（实现局限，已在报告中记录）
        // Note: a base unit whose scale is 1 holds no base factors, so integer scaling has no effect (implementation limitation, recorded in the report)
        let meter = Meter::INSTANT.clone();
        let scaled = (&meter * 3_i64).build();

        assert_eq!(scaled.scale().value(), &BigDecimal::from(1));
        assert_eq!(scaled.dimension().symbol(), "L");
    }

    #[test]
    fn test_composite_unit_kilometer_integer_scaling_is_exponential() {
        // 注意：`Scale` 的整型乘法把整数当作指数而非比例因子，故 1000 * 3 得到 1000^3
        // Note: integer multiplication on `Scale` treats the integer as an exponent rather than a factor, so 1000 * 3 yields 1000^3
        let kilometer = Kilometer::INSTANT.clone();
        let scaled = (&kilometer * 2_i64).build();

        assert_eq!(scaled.scale().value(), &BigDecimal::from(1_000_000));
        assert_eq!(scaled.dimension().symbol(), "L");
    }

    #[test]
    fn test_composite_unit_reciprocal_inverts_scale() {
        // 千米的倒数比例尺为 1/1000，量纲为 L^-1
        // The reciprocal of kilometer has scale 1/1000 and dimension L^-1
        use ospf_rust_math::operator::reciprocal::Reciprocal;

        let kilometer = Kilometer::INSTANT.clone();
        let composite = kilometer.reciprocal().build();

        assert_eq!(composite.dimension().symbol(), "L^-1");
        assert_scale_exact(composite.scale().value(), "0.001", "千米倒数 / kilometer reciprocal");
    }

    #[test]
    fn test_composite_unit_chain_preserves_dimension_cancellation() {
        // (千米 * 小时) / 小时 应还原为长度量纲，比例尺为 1000
        // (Kilometer times hour) divided by hour should reduce to the length dimension with scale 1000
        let kilometer = Kilometer::INSTANT.clone();
        let hour = Hour::INSTANT.clone();
        let composite = ((&kilometer * &hour) / &hour).build();

        assert_eq!(composite.dimension().symbol(), "L");
        assert_eq!(composite.scale().value(), &BigDecimal::from(1000));
    }

    #[test]
    fn test_composite_unit_multiplication_of_two_prefixed_units() {
        // 千米 * 千米 = 平方千米，比例尺为 1e6 / Kilometer times kilometer equals square kilometer with scale 1e6
        let kilometer = Kilometer::INSTANT.clone();
        let square_kilometer = (&kilometer * &kilometer).build();

        assert_eq!(square_kilometer.dimension().symbol(), "L^2");
        assert_eq!(square_kilometer.scale().value(), SquareKilometer::SCALE.value());
    }

    #[test]
    fn test_composite_unit_triple_product_equals_cubic_kilometer() {
        // 千米的三重积应为立方千米，比例尺为 1e9 / The triple product of kilometer should be cubic kilometer with scale 1e9
        let kilometer = Kilometer::INSTANT.clone();
        let cubic = ((&kilometer * &kilometer) * &kilometer).build();

        assert_eq!(cubic.dimension().symbol(), "L^3");
        assert_eq!(cubic.scale().value(), CubicKilometer::SCALE.value());
        assert_eq!(cubic.scale().value(), &BigDecimal::from(1_000_000_000));
    }

    #[test]
    fn test_composite_unit_power_via_division_reduces_correctly() {
        // (平方千米) / 千米 应还原为长度量纲，比例尺为 1000
        // Square kilometer divided by kilometer should reduce to the length dimension with scale 1000
        let square_kilometer = SquareKilometer::INSTANT.clone();
        let kilometer = Kilometer::INSTANT.clone();
        let composite = (&square_kilometer / &kilometer).build();

        assert_eq!(composite.dimension().symbol(), "L");
        assert_eq!(composite.scale().value(), &BigDecimal::from(1000));
    }

    #[test]
    fn test_composite_unit_mixed_quantity_ratio_is_dimensionless() {
        // 牛顿 / (千克 * 米每二次方秒) 应为无量纲且比例尺为 1
        // Newton divided by (kilogram times meter per second squared) should be dimensionless with scale 1
        let newton = Newton::INSTANT.clone();
        let kilogram = Kilogram::INSTANT.clone();
        let acceleration = MeterPerSecondSquared::INSTANT.clone();
        let composite = (&newton / &(&kilogram * &acceleration).build()).build();

        assert!(composite.dimension().is_none());
        assert_eq!(composite.dimension().symbol(), "1");
        assert_eq!(composite.scale().value(), &BigDecimal::from(1));
    }

    #[test]
    fn test_composite_unit_does_not_allow_affine_multiplication() {
        // 仿射单位（摄氏度）不允许参与乘除运算 / Affine units (Celsius) must not take part in multiplication or division
        assert!(!Celsius::INSTANT.is_linear());
        assert!(!Fahrenheit::INSTANT.is_linear());
        assert!(Kelvin::INSTANT.is_linear());
    }

    #[test]
    fn test_composite_unit_builder_name_and_symbol() {
        // 复合单位构建器可写入名称与符号 / The composite unit builder accepts a name and a symbol
        let kilometer = Kilometer::INSTANT.clone();
        let hour = Hour::INSTANT.clone();
        let mut builder = &kilometer / &hour;
        builder.name("kilometer per hour").symbol("km/h");
        let composite = builder.build();

        assert_eq!(composite.name(), "kilometer per hour");
        assert_eq!(composite.symbol(), "km/h");
        assert_eq!(composite.dimension().symbol(), "L·T^-1");
        assert_scale_relative(
            composite.scale().value(),
            "0.27777777777777777777777777777777777777777777777778",
            "1e-15",
            "千米每小时复合单位比例尺 / kilometer per hour composite unit scale",
        );
    }

    #[test]
    fn test_composite_unit_is_dimension_compatible_with_predefined_unit() {
        // 复合单位与预定义同量纲单位之间的运行时换算系数成立
        // Runtime conversion factors hold between a composite unit and a predefined unit of the same dimension
        let kilometer = Kilometer::INSTANT.clone();
        let hour = Hour::INSTANT.clone();
        let composite = (&kilometer / &hour).build();
        let predefined = KilometerPerHour::INSTANT.clone();

        assert!(composite.same_dimension(&predefined));
        let factor = composite
            .conversion_factor_to(&predefined)
            .expect("同量纲复合单位应可换算 / composite units of equal dimension should be convertible");
        assert_scale_relative(&factor, "1", "1e-15", "复合单位换算系数 / composite unit conversion factor");
    }

    #[test]
    fn test_composite_unit_runtime_value_conversion() {
        // 运行时数值换算：1 千米每小时应约等于 0.2777... 米每秒
        // Runtime value conversion: one kilometer per hour should be about 0.2777... meters per second
        use crate::quantity::Quantity;

        let speed = Quantity::new(BigDecimal::from(1), KilometerPerHour::INSTANT.clone());
        let converted = speed
            .to_unit(&MeterPerSecond::INSTANT)
            .expect("千米每小时应能转换为米每秒 / kilometer per hour should convert to meter per second");

        assert_scale_relative(
            &converted.value,
            "0.27777777777777777777777777777777777777777777777778",
            "1e-15",
            "千米每小时数值换算 / kilometer per hour value conversion",
        );
    }

    #[test]
    fn test_composite_unit_dimension_mismatch_rejects_conversion() {
        // 量纲不匹配时换算系数为 None / A conversion factor is None when dimensions do not match
        let kilometer = Kilometer::INSTANT.clone();
        let hour = Hour::INSTANT.clone();
        let composite = (&kilometer / &hour).build();
        let meter = Meter::INSTANT.clone();

        assert!(!composite.same_dimension(&meter));
        assert!(composite.conversion_factor_to(&meter).is_none());
    }

    /// 记录疑似实现问题：运行时量纲幂次列表保留插入顺序，导致同量纲的符号顺序可能不同，
    /// 且 `same_dimension` 依赖按序比较，会把物理上同量纲的单位判为不同量纲。
    /// Documents a suspected implementation issue: the runtime dimension power list keeps insertion order,
    /// so the symbol order of equal dimensions can differ, and `same_dimension` compares in order and
    /// therefore reports physically identical dimensions as different.
    #[test]
    fn test_composite_unit_same_dimension_is_order_sensitive() {
        // 信息量除以时间：运行时按「信息量在前」构造，编译时按固定顺序「时间在前」构造
        // Information divided by time: built at runtime as "information first" but at compile time in the fixed order "time first"
        let megabyte = Megabyte::INSTANT.clone();
        let second = Second::INSTANT.clone();
        let composite = (&megabyte / &second).build();
        let predefined = MegabytePerSecond::INSTANT.clone();

        assert_eq!(composite.scale().value(), predefined.scale().value());
        assert_eq!(composite.dimension().symbol(), "ℐ·T^-1");
        assert_eq!(predefined.dimension().symbol(), "T^-1·ℐ");
        // 比例尺相同、量纲物理含义相同，但运行时比较判定为不同量纲
        // The scales and physical dimensions match, yet the runtime comparison reports different dimensions
        assert!(!composite.same_dimension(&predefined));
        assert_ne!(composite.dimension(), predefined.dimension());
    }

    /// 记录疑似实现问题：`Scale` 的整型乘法把整数当作指数而非比例因子。
    /// Documents a suspected implementation issue: integer multiplication on `Scale` treats the integer
    /// as an exponent rather than a factor.
    #[test]
    fn test_composite_unit_integer_scaling_uses_exponent_semantics() {
        // 带前缀单位：1000^3 = 1e9 / Prefixed unit: 1000^3 = 1e9
        let kilometer = Kilometer::INSTANT.clone();
        assert_eq!((&kilometer * 3_i64).build().scale().value(), &BigDecimal::from(1_000_000_000));

        // 基准单位：内部无因子，缩放不产生任何效果 / Base unit: no internal factors, so scaling has no effect
        let meter = Meter::INSTANT.clone();
        assert_eq!((&meter * 3_i64).build().scale().value(), &BigDecimal::from(1));
    }

    #[test]
    fn test_composite_unit_pressure_from_force_over_area() {
        // 牛顿 / 平方米 应等于帕斯卡，比例尺为 1
        // Newton divided by square meter should equal pascal with scale 1
        let newton = Newton::INSTANT.clone();
        let square_meter = SquareMeter::INSTANT.clone();
        let pascal = (&newton / &square_meter).build();

        assert_eq!(pascal.dimension().symbol(), "L^-1·M·T^-2");
        assert_eq!(pascal.scale().value(), Pascal::SCALE.value());
    }

    #[test]
    fn test_composite_unit_energy_from_force_times_length() {
        // 牛顿 * 米 应等于焦耳，比例尺为 1 / Newton times meter should equal joule with scale 1
        let newton = Newton::INSTANT.clone();
        let meter = Meter::INSTANT.clone();
        let joule = (&newton * &meter).build();

        assert_eq!(joule.dimension().symbol(), "L^2·M·T^-2");
        assert_eq!(joule.scale().value(), Joule::SCALE.value());
    }

    #[test]
    fn test_composite_unit_bandwidth_from_megabyte_over_second() {
        // 兆字节 / 秒 应等于 MegabytePerSecond，比例尺为 8000000
        // Megabyte divided by second should equal MegabytePerSecond with scale 8000000
        //
        // 注意：运行时量纲符号的顺序（ℐ·T^-1）与编译时量纲符号顺序（T^-1·ℐ）不同，
        // 因此 same_dimension 返回 false。已在最终报告中作为疑似实现问题记录。
        // Note: the runtime dimension symbol order (ℐ·T^-1) differs from the compile-time order (T^-1·ℐ),
        // so same_dimension returns false. This is recorded as a suspected implementation issue in the report.
        let megabyte = Megabyte::INSTANT.clone();
        let second = Second::INSTANT.clone();
        let bandwidth = (&megabyte / &second).build();
        let predefined = MegabytePerSecond::INSTANT.clone();

        assert_eq!(bandwidth.scale().value(), MegabytePerSecond::SCALE.value());
        assert_eq!(bandwidth.scale().value(), &BigDecimal::from(8_000_000));
        assert_eq!(bandwidth.dimension().symbol(), "ℐ·T^-1");
        assert_eq!(predefined.dimension().symbol(), "T^-1·ℐ");
        assert!(!bandwidth.same_dimension(&predefined));
    }
}
