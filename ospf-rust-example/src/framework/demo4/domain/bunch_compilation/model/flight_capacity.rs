use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;

/// 航班容量 / Flight capacity
/// 对齐 Kotlin FlightCapacity
#[derive(Debug, Clone)]
pub struct FlightCapacity {
    pub flight_id: String,
    pub passenger_capacity: u64,
    pub cargo_capacity: f64,
}

impl FlightCapacity {
    /// 注册容量符号到模型
    /// 对齐 Kotlin FlightCapacity.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn Error>> {
        // 旅客容量符号
        let pax_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("flight_capacity_pax_{}", self.flight_id),
            Vec::new(),
            self.passenger_capacity as f64,
        );
        model.add_symbol(Arc::new(pax_symbol))?;
        *next_id += 1;

        // 货物容量符号
        let cargo_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("flight_capacity_cargo_{}", self.flight_id),
            Vec::new(),
            self.cargo_capacity,
        );
        model.add_symbol(Arc::new(cargo_symbol))?;
        *next_id += 1;

        Ok(())
    }
}
