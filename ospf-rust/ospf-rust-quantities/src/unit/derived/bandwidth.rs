//! 带宽单位 / Bandwidth units
//!
//! 提供带宽量纲的 SI 单位定义，带宽为信息量除以时间，包括比特每秒、千比特每秒等 / Provides SI unit definitions for bandwidth dimension, bandwidth is information divided by time, including bit per second, kilobit per second, etc

use super::information::{
    Bit, Byte, Exabit, Gigabit, Kilobit, Kilobyte, Megabit, Megabyte, Petabit, Terabit,
};
use super::time::Second;
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 带宽单位 / Bandwidth units
// ============================================================================

define_unit_by!(
    BitPerSecond,
    "bit per second",
    "bit/s",
    CTUnitDiv<Bit, Second>
);
define_unit_by!(
    KilobitPerSecond,
    "kilobit per second",
    "kbit/s",
    CTUnitDiv<Kilobit, Second>
);
define_unit_by!(
    MegabitPerSecond,
    "megabit per second",
    "Mbit/s",
    CTUnitDiv<Megabit, Second>
);
define_unit_by!(
    GigabitPerSecond,
    "gigabit per second",
    "Gbit/s",
    CTUnitDiv<Gigabit, Second>
);
define_unit_by!(
    TerabitPerSecond,
    "terabit per second",
    "Tbit/s",
    CTUnitDiv<Terabit, Second>
);
define_unit_by!(
    PetabitPerSecond,
    "petabit per second",
    "Pbit/s",
    CTUnitDiv<Petabit, Second>
);
define_unit_by!(
    ExabitPerSecond,
    "exabit per second",
    "Ebit/s",
    CTUnitDiv<Exabit, Second>
);
define_unit_by!(
    BytePerSecond,
    "byte per second",
    "B/s",
    CTUnitDiv<Byte, Second>
);
define_unit_by!(
    KilobytePerSecond,
    "kilobyte per second",
    "kB/s",
    CTUnitDiv<Kilobyte, Second>
);
define_unit_by!(
    MegabytePerSecond,
    "megabyte per second",
    "MB/s",
    CTUnitDiv<Megabyte, Second>
);
