use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use super::passenger::Passenger;
use crate::framework::demo4::infrastructure::PassengerClass;

/// 旅客变更 / Passenger change
/// 对齐 Kotlin PassengerChange
#[derive(Debug, Clone)]
pub struct PassengerChange {
    pub passenger: Passenger,
    pub from_flight: String,
    pub to_flight: String,
    pub from_class: PassengerClass,
    pub to_class: PassengerClass,
}

impl PassengerChange {
    /// 注册旅客变更符号到模型
    /// 对齐 Kotlin PassengerChange.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn Error>> {
        // 舱位变更符号
        let class_change_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("passenger_class_change_{}_{}", self.passenger.id, self.from_flight),
            Vec::new(),
            0.0,
        );
        model.add_symbol(Arc::new(class_change_symbol))?;
        *next_id += 1;

        // 航班变更符号
        let flight_change_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("passenger_flight_change_{}_{}", self.passenger.id, self.from_flight),
            Vec::new(),
            0.0,
        );
        model.add_symbol(Arc::new(flight_change_symbol))?;
        *next_id += 1;

        Ok(())
    }
}
