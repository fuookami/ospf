//! Time units - 时间单位
//! Time units - SI time units (second, minute, hour, etc.)
//!
//! 提供时间量纲的单位定义，包括秒、毫秒、微秒、纳秒、分、时、天、周等。
//! Provides unit definitions for time dimension, including second, millisecond, microsecond, nanosecond, minute, hour, day, week, etc.

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
