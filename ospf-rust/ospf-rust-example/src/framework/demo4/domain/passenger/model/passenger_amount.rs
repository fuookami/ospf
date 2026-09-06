//! 旅客数量模型模块 / Passenger amount model module.
use super::passenger::Passenger;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use std::error::Error;
use std::sync::Arc;

/// 旅客数量 / Passenger amount
/// 对齐 Kotlin PassengerAmount / Aligned with Kotlin PassengerAmount
#[derive(Debug, Clone)]
pub struct PassengerAmount {
    /// 旅客信息 / Passenger info
    pub passenger: Passenger,
    /// 旅客数量 / Passenger amount
    pub amount: u64,
}

impl PassengerAmount {
    /// 注册旅客数量符号到模型 / Register passenger amount symbol to the model
    /// 对齐 Kotlin PassengerAmount.register / Aligned with Kotlin PassengerAmount.register
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
