//! 热力学温度单位 / Thermodynamic temperature units
//!
//! 提供热力学温度量纲的 SI 单位定义，包括开尔文等 / Provides SI unit definitions for thermodynamic temperature dimension, including kelvin, etc

use crate::dimension::derived::ThermodynamicTemperature;
use crate::scale::Scale;
use crate::unit::CTUnit;
use bigdecimal::BigDecimal;
use std::str::FromStr;

// ============================================================================
// 热力学温度单位 / Thermodynamic temperature units
// ============================================================================

define_unit!(Kelvin, "kelvin", "K", ThermodynamicTemperature);
define_unit!(
    Celsius,
    "celsius",
    "°C",
    ThermodynamicTemperature,
    Scale::new(),
    offset = BigDecimal::from_str("273.15")
        .expect("无法解析摄氏度偏移值 / Failed to parse Celsius offset value")
);
define_unit!(
    Fahrenheit,
    "fahrenheit",
    "°F",
    ThermodynamicTemperature,
    Scale::from_f64(5.0 / 9.0),
    offset = BigDecimal::from_str("255.37222222222222222222")
        .expect("无法解析华氏度偏移值 / Failed to parse Fahrenheit offset value")
);
define_unit!(
    Rankine,
    "rankine",
    "°R",
    ThermodynamicTemperature,
    Scale::from_f64(5.0 / 9.0)
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::conversion_value::UnitConversionCalculation;
    use crate::unit::derived::test_support::*;

    // 由 f64 构造的比例尺无法用十进制精确表示，统一使用相对容差比较。
    // Scales built from f64 cannot be represented exactly in decimal, so a relative tolerance is used.
    const FUZZ: &str = "1e-12";

    #[test]
    fn test_temperature_symbols_and_names() {
        // 热力学温度单位的符号与名称 / Symbols and names of thermodynamic temperature units
        assert_symbol_and_name::<Kelvin>("K", "kelvin", "开尔文 / kelvin");
        assert_symbol_and_name::<Celsius>("°C", "celsius", "摄氏度 / celsius");
        assert_symbol_and_name::<Fahrenheit>("°F", "fahrenheit", "华氏度 / fahrenheit");
        assert_symbol_and_name::<Rankine>("°R", "rankine", "兰氏度 / rankine");
    }

    #[test]
    fn test_kelvin_is_base_unit_with_unit_scale() {
        // 开尔文是热力学温度的基准单位，比例尺为 1 且无偏移
        // Kelvin is the base thermodynamic temperature unit with scale 1 and no offset
        assert_scale_exact(Kelvin::SCALE.value(), "1", "开尔文比例尺 / kelvin scale");
        assert_offset::<Kelvin>("0", "开尔文偏移 / kelvin offset");
        assert!(Kelvin::INSTANT.is_linear());
    }

    #[test]
    fn test_celsius_scale_is_unit_and_offset_is_273_point_15() {
        // 摄氏度比例尺为 1，偏移为 273.15 / Celsius has scale 1 and offset 273.15
        assert_scale_exact(Celsius::SCALE.value(), "1", "摄氏度比例尺 / celsius scale");
        assert_offset::<Celsius>("273.15", "摄氏度偏移 / celsius offset");
        assert!(!Celsius::INSTANT.is_linear());
        assert_scale_exact(&Celsius::INSTANT.conversion().offset(), "273.15", "摄氏度转换偏移 / celsius conversion offset");
    }

    #[test]
    fn test_celsius_converts_to_kelvin_with_offset() {
        // 0 摄氏度等于 273.15 开尔文 / Zero degrees Celsius equals 273.15 kelvin
        let unit = Celsius::INSTANT.clone();
        let value = unit
            .conversion()
            .to_standard_value_checked(decimal("0"))
            .expect("摄氏度转换应成功 / celsius conversion should succeed");
        assert_scale_exact(&value, "273.15", "摄氏度标准值 / celsius standard value");
    }

    #[test]
    fn test_kelvin_converts_to_celsius_with_offset() {
        // 273.15 开尔文等于 0 摄氏度 / 273.15 kelvin equals zero degrees Celsius
        let unit = Celsius::INSTANT.clone();
        let value = unit
            .conversion()
            .value_from_standard_checked(decimal("273.15"))
            .expect("摄氏度反转换应成功 / celsius reverse conversion should succeed");
        assert_scale_exact(&value, "0", "摄氏度反算值 / celsius reverse value");
    }

    #[test]
    fn test_fahrenheit_scale_and_offset() {
        // 华氏度比例尺为 5/9，与兰氏度相同 / Fahrenheit has scale 5/9, identical to Rankine
        assert_scale_relative(Fahrenheit::SCALE.value(), "0.55555555555555555556", FUZZ, "华氏度比例尺 / fahrenheit scale");
        assert_scale_equals::<Fahrenheit, Rankine>("华氏度与兰氏度比例尺 / fahrenheit vs rankine scale");
        assert_offset::<Fahrenheit>("255.37222222222222222222", "华氏度偏移 / fahrenheit offset");
        assert!(!Fahrenheit::INSTANT.is_linear());
    }

    #[test]
    fn test_rankine_scale_is_five_ninths() {
        // 兰氏度比例尺为 5/9，无偏移 / Rankine has scale 5/9 and no offset
        assert_scale_relative(Rankine::SCALE.value(), "0.55555555555555555556", FUZZ, "兰氏度比例尺 / rankine scale");
        assert_offset::<Rankine>("0", "兰氏度偏移 / rankine offset");
        assert!(Rankine::INSTANT.is_linear());
    }

    #[test]
    fn test_fahrenheit_converts_to_kelvin() {
        // 32 华氏度应等于 273.15 开尔文（冰点）/ 32 degrees Fahrenheit should equal 273.15 kelvin (the ice point)
        let unit = Fahrenheit::INSTANT.clone();
        let value = unit
            .conversion()
            .to_standard_value_checked(decimal("32"))
            .expect("华氏度转换应成功 / fahrenheit conversion should succeed");
        assert_scale_relative(&value, "273.15", "1e-15", "华氏度冰点标准值 / fahrenheit ice point standard value");
    }

    #[test]
    fn test_temperature_dimension_symbol() {
        // 热力学温度量纲符号为 Θ / Thermodynamic temperature dimension symbol is Θ
        assert_dimension_symbol::<Kelvin>("Θ", "开尔文量纲 / kelvin dimension");
    }

    #[test]
    fn test_temperature_units_share_dimension() {
        // 四种温标共享热力学温度量纲 / All four temperature scales share the thermodynamic temperature dimension
        assert_same_dimension::<Kelvin, Celsius>("开尔文与摄氏度 / kelvin vs celsius");
        assert_same_dimension::<Kelvin, Fahrenheit>("开尔文与华氏度 / kelvin vs fahrenheit");
        assert_same_dimension::<Kelvin, Rankine>("开尔文与兰氏度 / kelvin vs rankine");
        assert_different_dimension::<Kelvin, crate::unit::derived::Meter>("温度与长度 / temperature vs length");
    }

    #[test]
    fn test_affine_units_reject_conversion_factor() {
        // 仿射温标不提供线性换算系数 / Affine temperature scales do not provide a linear conversion factor
        assert!(Celsius::conversion_factor_to::<Kelvin>().is_none());
        assert!(Fahrenheit::conversion_factor_to::<Kelvin>().is_none());
        // 开尔文与兰氏度均为线性单位（兰氏度偏移为 0），可以换算
        // Kelvin and Rankine are both linear (Rankine has zero offset) and can be converted
        assert!(Kelvin::conversion_factor_to::<Rankine>().is_some());
    }

    #[test]
    fn test_temperature_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Celsius>("°C", "celsius", "摄氏度实例 / celsius instance");
        assert_instance_symbol_and_name::<Fahrenheit>("°F", "fahrenheit", "华氏度实例 / fahrenheit instance");
    }
}
