//! 频率单位 / Frequency units
//!
//! 提供频率量纲的 SI 单位定义，包括赫兹、千赫、兆赫、吉赫等 / Provides SI unit definitions for frequency dimension, including hertz, kilohertz, megahertz, gigahertz, etc

use crate::dimension::derived::Frequency;
use crate::scale::{GIGA, KILO, MEGA, Scale};
use crate::unit::CTUnit;

// ============================================================================
// 频率单位 / Frequency units
// ============================================================================

define_unit!(Hertz, "hertz", "Hz", Frequency, Scale::new());
define_unit!(Kilohertz, "kilohertz", "kHz", Frequency, KILO.clone());
define_unit!(Megahertz, "megahertz", "MHz", Frequency, MEGA.clone());
define_unit!(Gigahertz, "gigahertz", "GHz", Frequency, GIGA.clone());

// 每小时周期数 / Cycle per hour
define_unit!(
    CyclePerHour,
    "cycle per hour",
    "cph",
    Frequency,
    Scale::from_f64(0.0002777777777777778)
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_frequency_symbols_and_names() {
        // 频率单位的符号与名称 / Symbols and names of frequency units
        assert_symbol_and_name::<Hertz>("Hz", "hertz", "赫兹 / hertz");
        assert_symbol_and_name::<Kilohertz>("kHz", "kilohertz", "千赫兹 / kilohertz");
        assert_symbol_and_name::<Megahertz>("MHz", "megahertz", "兆赫兹 / megahertz");
        assert_symbol_and_name::<Gigahertz>("GHz", "gigahertz", "吉赫兹 / gigahertz");
        assert_symbol_and_name::<CyclePerHour>("cph", "cycle per hour", "每小时周期数 / cycle per hour");
    }

    #[test]
    fn test_hertz_is_base_unit_with_unit_scale() {
        // 赫兹是频率的基准单位，比例尺为 1 / Hertz is the base frequency unit with scale 1
        assert_scale_exact(Hertz::SCALE.value(), "1", "赫兹比例尺 / hertz scale");
    }

    #[test]
    fn test_frequency_conversion_to_hertz() {
        // 频率单位到赫兹的换算系数 / Conversion factors of frequency units to hertz
        assert_factor_exact::<Kilohertz, Hertz>("1000", "千赫兹到赫兹 / kilohertz to hertz");
        assert_factor_exact::<Megahertz, Hertz>("1000000", "兆赫兹到赫兹 / megahertz to hertz");
        assert_factor_exact::<Gigahertz, Hertz>("1000000000", "吉赫兹到赫兹 / gigahertz to hertz");
    }

    #[test]
    fn test_frequency_prefix_series() {
        // 频率单位按 1000 递进 / Frequency units advance by 1000
        assert_factor_exact::<Megahertz, Kilohertz>("1000", "兆赫兹到千赫兹 / megahertz to kilohertz");
        assert_factor_exact::<Gigahertz, Megahertz>("1000", "吉赫兹到兆赫兹 / gigahertz to megahertz");
    }

    #[test]
    fn test_cycle_per_hour_is_reciprocal_hour() {
        // 每小时 1 周期等于 1/3600 赫兹 / One cycle per hour equals 1/3600 hertz
        assert_factor_relative::<CyclePerHour, Hertz>(
            "0.00027777777777777777777777777777777777777777777778",
            "1e-15",
            "每小时周期数到赫兹 / cycle per hour to hertz",
        );
    }

    #[test]
    fn test_frequency_is_reciprocal_time() {
        // 频率量纲为 T^-1，与时间互为倒数 / Frequency dimension is T^-1, the reciprocal of time
        assert_dimension_symbol::<Hertz>("T^-1", "赫兹量纲 / hertz dimension");
        assert_different_dimension::<Hertz, crate::unit::derived::Second>("赫兹与秒 / hertz vs second");
    }

    #[test]
    fn test_frequency_units_share_dimension() {
        // 所有频率单位共享频率量纲 / All frequency units share the frequency dimension
        assert_same_dimension::<Hertz, Gigahertz>("赫兹与吉赫兹 / hertz vs gigahertz");
        assert_same_dimension::<Hertz, CyclePerHour>("赫兹与每小时周期数 / hertz vs cycle per hour");
    }

    #[test]
    fn test_frequency_scale_matches_reciprocal_second() {
        // 千赫兹与千分之一秒的倒数比例尺一致 / Kilohertz matches the reciprocal of a millisecond
        use crate::unit::{CTUnitReciprocal, Millisecond};
        assert_scale_equals::<Kilohertz, CTUnitReciprocal<Millisecond>>("千赫兹与毫秒倒数 / kilohertz vs millisecond reciprocal");
    }

    #[test]
    fn test_frequency_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Megahertz>("MHz", "megahertz", "兆赫兹实例 / megahertz instance");
        assert_instance_symbol_and_name::<Gigahertz>("GHz", "gigahertz", "吉赫兹实例 / gigahertz instance");
    }
}
