//! 总重模型 / Total weight model
use super::super::super::aircraft::model::{AircraftModel, FlightPhase, FuelConstant, Fuselage};
use super::super::super::shared::units::{
    quantity_value_in_unit, quantity_value_in_unit_or_default, weight_unit,
};
use super::payload::Payload;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

/// 总重变量索引 / Total weight variable indices
#[derive(Debug, Clone)]
pub struct TotalWeightVariables {
    /// 各阶段估算总重 / Estimate total weight by flight phase
    pub estimate_total_weight: HashMap<FlightPhase, usize>,
    /// 各阶段实际总重 / Actual total weight by flight phase
    pub actual_total_weight: HashMap<FlightPhase, usize>,
}

/// 总重 / Total weight (对齐 Kotlin TotalWeight)
#[derive(Debug)]
pub struct TotalWeight {
    /// 各阶段最大总重 / Maximum total weight by flight phase
    pub max_total_weight: HashMap<FlightPhase, f64>,
    /// 各阶段计算总重 / Computed total weight by flight phase
    pub computed_total_weight: HashMap<FlightPhase, f64>,
}

impl TotalWeight {
    /// 注册总重中间符号到模型
    /// 对齐 Kotlin TotalWeight.register:
    /// - estimateTotalWeight[phase] = dow + fuel[phase] + estimatePayload
    /// - actualTotalWeight[phase] = dow + fuel[phase] + actualPayload
    pub fn register(
        &self,
        _aircraft_model: &AircraftModel,
        fuselage: &Fuselage,
        fuel: &HashMap<FlightPhase, FuelConstant>,
        model: &mut MetaModel<f64>,
        payload_estimate_idx: usize,
        payload_actual_idx: usize,
    ) -> Result<TotalWeightVariables, Box<dyn Error>> {
        let mut next_id = 50000u64;
        let mut estimate_map = HashMap::new();
        let mut actual_map = HashMap::new();
        let wu = weight_unit();

        for phase in [
            FlightPhase::ZeroFuel,
            FlightPhase::TakeOff,
            FlightPhase::Landing,
        ] {
            let fuel_weight =
                quantity_value_in_unit_or_default(fuel.get(&phase).map(|f| &f.weight), &wu, 0.0)?;
            let dow = quantity_value_in_unit(&fuselage.dow, &wu)?;

            // estimateTotalWeight = dow + fuel + estimatePayload
            let est_symbol = LinearExpressionSymbol::new(
                next_id,
                &format!("estimate_total_weight_{:?}", phase),
                vec![LinearMonomial::new(1.0, payload_estimate_idx)],
                dow + fuel_weight,
            );
            model.add_symbol(Arc::new(est_symbol))?;
            estimate_map.insert(phase, next_id as usize);
            next_id += 1;

            // actualTotalWeight = dow + fuel + actualPayload
            let act_symbol = LinearExpressionSymbol::new(
                next_id,
                &format!("actual_total_weight_{:?}", phase),
                vec![LinearMonomial::new(1.0, payload_actual_idx)],
                dow + fuel_weight,
            );
            model.add_symbol(Arc::new(act_symbol))?;
            actual_map.insert(phase, next_id as usize);
            next_id += 1;
        }

        Ok(TotalWeightVariables {
            estimate_total_weight: estimate_map,
            actual_total_weight: actual_map,
        })
    }
}
