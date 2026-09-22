//! 流量单位 / Flow rate units
//!
//! 提供流量量纲的 SI 单位定义，包括立方米每秒、升每秒、升每分钟等 / Provides SI unit definitions for flow rate dimension, including cubic meter per second, liter per second, liter per minute, etc

use super::time::{Minute, Second};
use super::volume::{CubicMeter, Liter};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 流量单位 / Flow rate units
// ============================================================================

define_unit_by!(
    CubicMeterPerSecond,
    "cubic meter per second",
    "m³/s",
    CTUnitDiv<CubicMeter, Second>
);
define_unit_by!(
    LiterPerSecond,
    "liter per second",
    "L/s",
    CTUnitDiv<Liter, Second>
);
define_unit_by!(
    LiterPerMinute,
    "liter per minute",
    "L/min",
    CTUnitDiv<Liter, Minute>
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_flow_rate_symbols_and_names() {
        // 流量单位的符号与名称 / Symbols and names of flow rate units
        assert_symbol_and_name::<CubicMeterPerSecond>("m³/s", "cubic meter per second", "立方米每秒 / cubic meter per second");
        assert_symbol_and_name::<LiterPerSecond>("L/s", "liter per second", "升每秒 / liter per second");
        assert_symbol_and_name::<LiterPerMinute>("L/min", "liter per minute", "升每分钟 / liter per minute");
    }

    #[test]
    fn test_cubic_meter_per_second_scale_is_unit() {
        // 立方米每秒是流量的基准单位，比例尺为 1 / Cubic meter per second is the base flow rate unit with scale 1
        assert_scale_exact(CubicMeterPerSecond::SCALE.value(), "1", "立方米每秒比例尺 / cubic meter per second scale");
    }

    #[test]
    fn test_liter_per_second_equals_0_001_cubic_meter_per_second() {
        // 1 升每秒等于 0.001 立方米每秒 / One liter per second equals 0.001 cubic meters per second
        assert_factor_exact::<LiterPerSecond, CubicMeterPerSecond>("0.001", "升每秒到立方米每秒 / liter per second to cubic meter per second");
    }

    #[test]
    fn test_liter_per_minute_equals_one_sixtieth_liter_per_second() {
        // 1 升每分钟等于 1/60 升每秒 / One liter per minute equals 1/60 liter per second
        assert_factor_relative::<LiterPerMinute, LiterPerSecond>(
            "0.016666666666666666666666666666666666666666666666667",
            "1e-15",
            "升每分钟到升每秒 / liter per minute to liter per second",
        );
    }

    #[test]
    fn test_flow_rate_equals_volume_per_time() {
        // 流量单位应为体积单位除以时间单位，比例尺一致
        // A flow rate unit should equal the volume unit divided by the time unit, with a matching scale
        assert_scale_equals::<CubicMeterPerSecond, CTUnitDiv<CubicMeter, Second>>("立方米每秒分解 / cubic meter per second decomposition");
        assert_scale_equals::<LiterPerSecond, CTUnitDiv<Liter, Second>>("升每秒分解 / liter per second decomposition");
        assert_scale_equals::<LiterPerMinute, CTUnitDiv<Liter, Minute>>("升每分钟分解 / liter per minute decomposition");
    }

    #[test]
    fn test_flow_rate_dimension_symbol() {
        // 流量量纲符号为 L^3·T^-1 / Flow rate dimension symbol is L^3·T^-1
        assert_dimension_symbol::<CubicMeterPerSecond>("L^3·T^-1", "立方米每秒量纲 / cubic meter per second dimension");
    }

    #[test]
    fn test_flow_rate_units_share_dimension() {
        // 所有流量单位共享流量量纲 / All flow rate units share the flow rate dimension
        assert_same_dimension::<CubicMeterPerSecond, LiterPerMinute>("立方米每秒与升每分钟 / cubic meter per second vs liter per minute");
        assert_different_dimension::<CubicMeterPerSecond, CubicMeter>("流量与体积 / flow rate vs volume");
        assert_different_dimension::<CubicMeterPerSecond, super::super::velocity::MeterPerSecond>(
            "流量与速度 / flow rate vs velocity",
        );
    }

    #[test]
    fn test_flow_rate_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<LiterPerMinute>("L/min", "liter per minute", "升每分钟实例 / liter per minute instance");
    }
}
