//! 波数单位 / Wavenumber units

use super::length::Meter;
use crate::unit::{CTUnit, CTUnitReciprocal};

define_unit_by!(
    ReciprocalMeter,
    "reciprocal meter",
    "1/m",
    CTUnitReciprocal<Meter>
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_reciprocal_meter_symbol_and_name() {
        // 波数单位的符号与名称 / Symbol and name of the wavenumber unit
        assert_symbol_and_name::<ReciprocalMeter>("1/m", "reciprocal meter", "每米 / reciprocal meter");
    }

    #[test]
    fn test_reciprocal_meter_scale_is_unit() {
        // 米的倒数比例尺仍为 1 / The reciprocal of meter still has scale 1
        assert_scale_exact(ReciprocalMeter::SCALE.value(), "1", "每米比例尺 / reciprocal meter scale");
    }

    #[test]
    fn test_reciprocal_meter_dimension_is_inverse_length() {
        // 波数量纲为 L^-1 / Wavenumber dimension is L^-1
        assert_dimension_symbol::<ReciprocalMeter>("L^-1", "每米量纲 / reciprocal meter dimension");
    }

    #[test]
    fn test_reciprocal_meter_is_not_length() {
        // 波数与长度量纲不同 / Wavenumber differs in dimension from length
        assert_different_dimension::<ReciprocalMeter, Meter>("每米与米 / reciprocal meter vs meter");
    }

    #[test]
    fn test_reciprocal_meter_matches_kilometer_reciprocal_scale() {
        // 千米倒数的比例尺为 1e-3，与每米相差 1000 倍
        // The reciprocal of kilometer has scale 1e-3, differing from reciprocal meter by a factor of 1000
        assert_factor_exact::<CTUnitReciprocal<super::super::length::Kilometer>, ReciprocalMeter>(
            "0.001",
            "千米倒数到每米 / kilometer reciprocal to reciprocal meter",
        );
    }

    #[test]
    fn test_reciprocal_meter_round_trip_with_meter() {
        // 米与每米互逆：米 * 每米 的比例尺为 1
        // Meter and reciprocal meter are mutually inverse: the combined scale is 1
        assert_scale_exact(
            (&*Meter::SCALE * &*ReciprocalMeter::SCALE).value(),
            "1",
            "米与每米乘积 / meter times reciprocal meter",
        );
    }

    #[test]
    fn test_reciprocal_meter_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<ReciprocalMeter>("1/m", "reciprocal meter", "每米实例 / reciprocal meter instance");
    }
}
