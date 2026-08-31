use std::collections::HashMap;
use super::super::super::aircraft::model::{AircraftModel, FlightPhase, FuelConstant, Fuselage};
use super::payload::Payload;
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;

/// 总重变量索引 / Total weight variable indices
#[derive(Debug, Clone)]
pub struct TotalWeightVariables {
    /// estimateTotalWeight[phase] = 各阶段估算总重
    pub estimate_total_weight: HashMap<FlightPhase, usize>,
    /// actualTotalWeight[phase] = 各阶段实际总重
    pub actual_total_weight: HashMap<FlightPhase, usize>,
}

/// 总重 / Total weight (对齐 Kotlin TotalWeight)
#[derive(Debug)]
pub struct TotalWeight {
    pub max_total_weight: HashMap<FlightPhase, f64>,
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

        for phase in [FlightPhase::ZeroFuel, FlightPhase::TakeOff, FlightPhase::Landing] {
            let fuel_weight = fuel.get(&phase).map(|f| f.weight).unwrap_or(0.0);
            let dow = fuselage.dow;

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
