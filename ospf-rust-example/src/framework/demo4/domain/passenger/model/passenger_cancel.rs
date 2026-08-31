use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use super::passenger::Passenger;

/// 旅客取消 / Passenger cancel
/// 对齐 Kotlin PassengerCancel
#[derive(Debug, Clone)]
pub struct PassengerCancel {
    pub passenger: Passenger,
    pub flight_id: String,
}

impl PassengerCancel {
    /// 注册旅客取消符号到模型
    /// 对齐 Kotlin PassengerCancel.register
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
