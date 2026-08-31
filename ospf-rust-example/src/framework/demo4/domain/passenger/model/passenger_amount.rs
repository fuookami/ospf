use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use super::passenger::Passenger;

/// 旅客数量 / Passenger amount
/// 对齐 Kotlin PassengerAmount
#[derive(Debug, Clone)]
pub struct PassengerAmount {
    pub passenger: Passenger,
    pub amount: u64,
}

impl PassengerAmount {
    /// 注册旅客数量符号到模型
    /// 对齐 Kotlin PassengerAmount.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn Error>> {
        let symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("passenger_amount_{}", self.passenger.id),
            Vec::new(),
            self.amount as f64,
        );
        model.add_symbol(Arc::new(symbol))?;
        *next_id += 1;
        Ok(())
    }
}
