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

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_bandwidth_bit_symbols_and_names() {
        // 比特类带宽单位的符号与名称 / Symbols and names of bit based bandwidth units
        assert_symbol_and_name::<BitPerSecond>("bit/s", "bit per second", "比特每秒 / bit per second");
        assert_symbol_and_name::<KilobitPerSecond>("kbit/s", "kilobit per second", "千比特每秒 / kilobit per second");
        assert_symbol_and_name::<MegabitPerSecond>("Mbit/s", "megabit per second", "兆比特每秒 / megabit per second");
        assert_symbol_and_name::<GigabitPerSecond>("Gbit/s", "gigabit per second", "吉比特每秒 / gigabit per second");
        assert_symbol_and_name::<TerabitPerSecond>("Tbit/s", "terabit per second", "太比特每秒 / terabit per second");
        assert_symbol_and_name::<PetabitPerSecond>("Pbit/s", "petabit per second", "拍比特每秒 / petabit per second");
        assert_symbol_and_name::<ExabitPerSecond>("Ebit/s", "exabit per second", "艾比特每秒 / exabit per second");
    }

    #[test]
    fn test_bandwidth_byte_symbols_and_names() {
        // 字节类带宽单位的符号与名称 / Symbols and names of byte based bandwidth units
        assert_symbol_and_name::<BytePerSecond>("B/s", "byte per second", "字节每秒 / byte per second");
        assert_symbol_and_name::<KilobytePerSecond>("kB/s", "kilobyte per second", "千字节每秒 / kilobyte per second");
        assert_symbol_and_name::<MegabytePerSecond>("MB/s", "megabyte per second", "兆字节每秒 / megabyte per second");
    }

    #[test]
    fn test_byte_per_second_equals_eight_bit_per_second() {
        // 1 字节每秒等于 8 比特每秒 / One byte per second equals eight bits per second
        assert_factor_exact::<BytePerSecond, BitPerSecond>("8", "字节每秒到比特每秒 / byte per second to bit per second");
    }

    #[test]
    fn test_kilobyte_per_second_conversions() {
        // 1 千字节每秒等于 1000 字节每秒 / 8000 比特每秒
        // One kilobyte per second equals 1000 bytes per second / 8000 bits per second
        assert_factor_exact::<KilobytePerSecond, BytePerSecond>("1000", "千字节每秒到字节每秒 / kilobyte per second to byte per second");
        assert_factor_exact::<KilobytePerSecond, BitPerSecond>("8000", "千字节每秒到比特每秒 / kilobyte per second to bit per second");
    }

    #[test]
    fn test_megabyte_per_second_conversions() {
        // 1 兆字节每秒等于 1000 千字节每秒 / 8000000 比特每秒
        // One megabyte per second equals 1000 kilobytes per second / 8000000 bits per second
        assert_factor_exact::<MegabytePerSecond, KilobytePerSecond>("1000", "兆字节每秒到千字节每秒 / megabyte per second to kilobyte per second");
        assert_factor_exact::<MegabytePerSecond, BytePerSecond>("1000000", "兆字节每秒到字节每秒 / megabyte per second to byte per second");
        assert_factor_exact::<MegabytePerSecond, BitPerSecond>("8000000", "兆字节每秒到比特每秒 / megabyte per second to bit per second");
    }

    #[test]
    fn test_bit_bandwidth_prefix_series() {
        // 比特类带宽单位按 1000 递进 / Bit based bandwidth units advance by 1000
        assert_factor_exact::<KilobitPerSecond, BitPerSecond>("1000", "千比特每秒到比特每秒 / kilobit per second to bit per second");
        assert_factor_exact::<MegabitPerSecond, KilobitPerSecond>("1000", "兆比特每秒到千比特每秒 / megabit per second to kilobit per second");
        assert_factor_exact::<GigabitPerSecond, MegabitPerSecond>("1000", "吉比特每秒到兆比特每秒 / gigabit per second to megabit per second");
        assert_factor_exact::<TerabitPerSecond, GigabitPerSecond>("1000", "太比特每秒到吉比特每秒 / terabit per second to gigabit per second");
        assert_factor_exact::<PetabitPerSecond, TerabitPerSecond>("1000", "拍比特每秒到太比特每秒 / petabit per second to terabit per second");
        assert_factor_exact::<ExabitPerSecond, PetabitPerSecond>("1000", "艾比特每秒到拍比特每秒 / exabit per second to petabit per second");
    }

    #[test]
    fn test_bit_per_second_is_base_bandwidth_unit() {
        // 比特每秒是带宽的基准单位，比例尺为 1 / Bit per second is the base bandwidth unit with scale 1
        assert_scale_exact(BitPerSecond::SCALE.value(), "1", "比特每秒比例尺 / bit per second scale");
    }

    #[test]
    fn test_bandwidth_units_share_dimension() {
        // 所有带宽单位共享带宽量纲 / All bandwidth units share the bandwidth dimension
        assert_same_dimension::<BitPerSecond, MegabytePerSecond>("比特每秒与兆字节每秒 / bit per second vs megabyte per second");
        assert_different_dimension::<BitPerSecond, crate::unit::derived::MeterPerSecond>(
            "带宽与速度 / bandwidth vs velocity",
        );
        assert_different_dimension::<BitPerSecond, crate::unit::derived::Bit>(
            "带宽与信息量 / bandwidth vs information",
        );
    }

    #[test]
    fn test_bandwidth_dimension_symbol() {
        // 带宽量纲符号按固定基础量纲顺序生成：先时间后信息量，即 T^-1·ℐ
        // The bandwidth dimension symbol follows the fixed base dimension order: time then information, i.e. T^-1·ℐ
        assert_dimension_symbol::<BitPerSecond>("T^-1·ℐ", "比特每秒量纲 / bit per second dimension");
    }

    #[test]
    fn test_bandwidth_is_bit_per_second_dimension_ratio() {
        // 带宽单位应为信息量单位除以秒，比例尺与直接定义一致
        // A bandwidth unit should equal the information unit divided by second, with a matching scale
        use crate::unit::{CTUnitDiv, Second};
        type Expected = CTUnitDiv<Megabyte, Second>;
        assert_scale_equals::<MegabytePerSecond, Expected>("兆字节每秒分解 / megabyte per second decomposition");
    }

    #[test]
    fn test_bandwidth_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<MegabitPerSecond>("Mbit/s", "megabit per second", "兆比特每秒实例 / megabit per second instance");
        assert_instance_symbol_and_name::<BytePerSecond>("B/s", "byte per second", "字节每秒实例 / byte per second instance");
    }
}
