use super::aircraft_model::AircraftModel;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;
use super::super::super::shared::units;

/// 公式 / Formula (对齐 Kotlin Formula)
#[derive(Debug, Clone)]
pub struct Formula {
    pub lip: Quantity<f64, Unit>,
    pub chord: Quantity<f64, Unit>,
    pub standard_datum: Quantity<f64, Unit>,
    pub force_distance_coefficient: f64,
    pub doi_correction: f64,
}

impl Formula {
    pub fn balanced_arm(&self, dow: f64, doi: f64, liferaft_weight: f64, liferaft_arm: f64) -> f64 {
        let numerator = dow * self.standard_datum.value + doi * self.chord.value + liferaft_weight * liferaft_arm;
        let denominator = dow + doi + liferaft_weight;
        if denominator > 0.0 { numerator / denominator } else { 0.0 }
    }

    pub fn arm(&self, _weight: f64, arm_distance: f64) -> f64 {
        arm_distance - self.standard_datum.value
    }

    pub fn index(&self, weight: f64, arm_distance: f64) -> f64 {
        let mac_arm = self.arm(weight, arm_distance);
        mac_arm * weight * self.force_distance_coefficient + self.doi_correction
    }

    pub fn mac(&self, weight: f64, arm_distance: f64) -> f64 {
        // MAC 百分比计算
        let index = self.index(weight, arm_distance);
        index / self.chord.value * 100.0
    }
}
