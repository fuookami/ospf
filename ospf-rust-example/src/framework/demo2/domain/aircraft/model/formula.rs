//! 气动公式定义 / Aerodynamic formula definitions
use super::aircraft_model::AircraftModel;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;
use super::super::super::shared::units;

/// 公式 / Formula (对齐 Kotlin Formula)
#[derive(Debug, Clone)]
pub struct Formula {
    /// 前缘位置 / Leading edge position (LIP)
    pub lip: Quantity<f64, Unit>,
    /// 平均气动弦长 / Mean aerodynamic chord
    pub chord: Quantity<f64, Unit>,
    /// 标准基准 / Standard datum
    pub standard_datum: Quantity<f64, Unit>,
    /// 力矩距离系数 / Force-distance coefficient
    pub force_distance_coefficient: f64,
    /// 干运行指数修正 / DOI correction
    pub doi_correction: f64,
}

impl Formula {
    /// 计算平衡力臂 / Calculate balanced arm
    pub fn balanced_arm(&self, dow: f64, doi: f64, liferaft_weight: f64, liferaft_arm: f64) -> f64 {
        let numerator = dow * self.standard_datum.value + doi * self.chord.value + liferaft_weight * liferaft_arm;
        let denominator = dow + doi + liferaft_weight;
        if denominator > 0.0 { numerator / denominator } else { 0.0 }
    }

    /// 计算力臂 / Calculate arm distance
    pub fn arm(&self, _weight: f64, arm_distance: f64) -> f64 {
        arm_distance - self.standard_datum.value
    }

    /// 计算指数 / Calculate index
    pub fn index(&self, weight: f64, arm_distance: f64) -> f64 {
        let mac_arm = self.arm(weight, arm_distance);
        mac_arm * weight * self.force_distance_coefficient + self.doi_correction
    }

    /// 计算平均气动弦百分比 / Calculate MAC percentage
    pub fn mac(&self, weight: f64, arm_distance: f64) -> f64 {
        // MAC 百分比计算
        let index = self.index(weight, arm_distance);
        index / self.chord.value * 100.0
    }
}
