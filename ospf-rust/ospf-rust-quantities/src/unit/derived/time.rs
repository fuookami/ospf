//! 时间单位 / Time units
//!
//! 提供时间量纲的单位定义，包括秒、毫秒、微秒、纳秒、分、时、天、周等 / Provides unit definitions for time dimension, including second, millisecond, microsecond, nanosecond, minute, hour, day, week, etc

use crate::dimension::derived::Time;
use crate::scale::{MICRO, MILLI, NANO, SEXAGESIMAL, Scale};
use crate::unit::CTUnit;

// ============================================================================
// 时间单位 / Time units
// ============================================================================

define_unit!(Second, "second", "s", Time);
define_unit!(Millisecond, "millisecond", "ms", Time, MILLI.clone());
define_unit!(Microsecond, "microsecond", "μs", Time, MICRO.clone());
define_unit!(Nanosecond, "nanosecond", "ns", Time, NANO.clone());
define_unit!(Minute, "minute", "min", Time, SEXAGESIMAL.clone());
define_unit!(Hour, "hour", "h", Time, &*Minute::SCALE * &*SEXAGESIMAL);
define_unit!(Day, "day", "d", Time, &*Hour::SCALE * &Scale::from_int(24));
define_unit!(Week, "week", "w", Time, &*Day::SCALE * &Scale::from_int(7));

// 年 / Year (365.25 days)
define_unit!(
    Year,
    "year",
    "yr",
    Time,
    &*Day::SCALE * &Scale::from_f64(365.25)
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_time_unit_symbols_and_names() {
        // 时间单位的符号与名称 / Symbols and names of time units
        assert_symbol_and_name::<Second>("s", "second", "秒 / second");
        assert_symbol_and_name::<Millisecond>("ms", "millisecond", "毫秒 / millisecond");
        assert_symbol_and_name::<Microsecond>("μs", "microsecond", "微秒 / microsecond");
        assert_symbol_and_name::<Nanosecond>("ns", "nanosecond", "纳秒 / nanosecond");
        assert_symbol_and_name::<Minute>("min", "minute", "分 / minute");
        assert_symbol_and_name::<Hour>("h", "hour", "时 / hour");
        assert_symbol_and_name::<Day>("d", "day", "天 / day");
        assert_symbol_and_name::<Week>("w", "week", "周 / week");
        assert_symbol_and_name::<Year>("yr", "year", "年 / year");
    }

    #[test]
    fn test_time_conversion_to_second() {
        // 时间单位到秒的换算系数 / Conversion factors of time units to second
        assert_factor_exact::<Second, Second>("1", "秒到秒 / second to second");
        assert_factor_exact::<Millisecond, Second>("0.001", "毫秒到秒 / millisecond to second");
        assert_factor_exact::<Microsecond, Second>("0.000001", "微秒到秒 / microsecond to second");
        assert_factor_exact::<Nanosecond, Second>("1e-9", "纳秒到秒 / nanosecond to second");
        assert_factor_exact::<Minute, Second>("60", "分到秒 / minute to second");
        assert_factor_exact::<Hour, Second>("3600", "时到秒 / hour to second");
        assert_factor_exact::<Day, Second>("86400", "天到秒 / day to second");
        assert_factor_exact::<Week, Second>("604800", "周到秒 / week to second");
        assert_factor_exact::<Year, Second>("31557600", "年到秒 / year to second");
    }

    #[test]
    fn test_time_unit_internal_ratios() {
        // 时间单位之间的比例关系 / Ratios between time units
        assert_factor_exact::<Second, Millisecond>("1000", "秒到毫秒 / second to millisecond");
        assert_factor_exact::<Millisecond, Microsecond>("1000", "毫秒到微秒 / millisecond to microsecond");
        assert_factor_exact::<Microsecond, Nanosecond>("1000", "微秒到纳秒 / microsecond to nanosecond");
        assert_factor_exact::<Minute, Second>("60", "分到秒 / minute to second");
        assert_factor_exact::<Hour, Minute>("60", "时到分 / hour to minute");
        assert_factor_exact::<Day, Hour>("24", "天到时 / day to hour");
        assert_factor_exact::<Week, Day>("7", "周到天 / week to day");
    }

    #[test]
    fn test_year_uses_julian_year_of_365_point_25_days() {
        // 年按儒略年 365.25 天定义 / The year is defined as the Julian year of 365.25 days
        assert_factor_exact::<Year, Day>("365.25", "年到天 / year to day");
    }

    #[test]
    fn test_time_dimension_symbol_and_domain() {
        // 时间量纲符号为 T，取值域为连续 / Time dimension symbol is T and the domain is continuous
        assert_dimension_symbol::<Second>("T", "秒量纲 / second dimension");
        assert_domain::<Second>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "秒取值域 / second domain",
        );
    }

    #[test]
    fn test_second_is_base_unit_with_unit_scale() {
        // 秒是时间的基准单位，比例尺为 1 / Second is the base unit of time with scale 1
        assert_scale_exact(Second::SCALE.value(), "1", "秒比例尺 / second scale");
    }

    #[test]
    fn test_time_units_share_time_dimension() {
        // 所有时间单位共享时间量纲 / All time units share the time dimension
        assert_same_dimension::<Second, Year>("秒与年 / second vs year");
        assert_different_dimension::<Second, crate::unit::derived::Meter>("秒与米 / second vs meter");
        assert_different_dimension::<Second, crate::unit::derived::Kilogram>("秒与千克 / second vs kilogram");
    }

    #[test]
    fn test_time_unit_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Hour>("h", "hour", "时实例 / hour instance");
        assert_instance_symbol_and_name::<Millisecond>("ms", "millisecond", "毫秒实例 / millisecond instance");
    }
}
