//! UnitTrait - 单位 super trait
//! UnitTrait - Unit super trait
//!
//! 统一编译时单位和运行时单位的核心接口
//! Core interface unifying compile-time and runtime units

use bigdecimal::BigDecimal;

/// UnitTrait - 单位 super trait
/// UnitTrait - Unit super trait
///
/// 统一编译时单位和运行时单位的核心接口
/// Core interface unifying compile-time and runtime units
///
/// # 类型参数 / Type Parameters
/// - `Dimension`: 量纲类型
///
/// # 实现者 / Implementors
/// - `Unit`: 运行时单位
/// - 所有实现 `CTUnit` 的编译时单位标记类型
/// - All compile-time unit marker types implementing `CTUnit`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_quantities::unit::concept::UnitTrait;
/// use ospf_rust_quantities::unit::physical_unit::CTUnit;
/// use ospf_rust_quantities::unit::derived::length::Meter;
///
/// // 运行时单位
/// let unit = Meter::INSTANT.clone();
/// assert_eq!(unit.symbol(), "m");
///
/// // 编译时单位
/// let ct_unit = Meter;
/// assert_eq!(ct_unit.symbol(), "m");
/// ```
pub trait UnitTrait {
    /// 量纲类型 / Dimension type
    type Dimension;

    /// 获取单位符号 / Get unit symbol
    fn symbol(&self) -> &str;

    /// 获取单位名称 / Get unit name
    fn name(&self) -> &str;

    /// 获取量纲符号 / Get dimension symbol
    fn dimension_symbol(&self) -> String;

    /// 获取比例尺值 / Get scale value
    fn scale_value(&self) -> BigDecimal;
}
