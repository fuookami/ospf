//! 旅客取消模型模块 / Passenger cancel model module.
use super::passenger::Passenger;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use std::error::Error;
use std::sync::Arc;

/// 旅客取消 / Passenger cancel
/// 对齐 Kotlin PassengerCancel / Aligned with Kotlin PassengerCancel
#[derive(Debug, Clone)]
pub struct PassengerCancel {
    /// 旅客信息 / Passenger info
    pub passenger: Passenger,
    /// 航班标识 / Flight identifier
    pub flight_id: String,
}

impl PassengerCancel {
    /// 注册旅客取消符号到模型 / Register passenger cancel symbol to the model
    /// 对齐 Kotlin PassengerCancel.register / Aligned with Kotlin PassengerCancel.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn Error>> {
        let symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("passenger_cancel_{}_{}", self.passenger.id, self.flight_id),
            Vec::new(),
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        *next_id += 1;
        Ok(())
    }
}
