pub mod context;
pub mod model;
pub mod service;

/// 旅客领域聚合 / Passenger domain aggregation
/// 对齐 Kotlin passenger Aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub passengers: Vec<model::Passenger>,
    pub amounts: Vec<model::PassengerAmount>,
    pub cancels: Vec<model::PassengerCancel>,
    pub changes: Vec<model::PassengerChange>,
}

impl Aggregation {
    /// 注册所有符号到模型
    /// 对齐 Kotlin passenger Aggregation.register
    pub fn register(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut next_id = 80000u64;

        for amount in &self.amounts {
            amount.register(model, &mut next_id)?;
        }
        for cancel in &self.cancels {
            cancel.register(model, &mut next_id)?;
        }
        for change in &self.changes {
            let _change_vars = change.register(model, &mut next_id)?;
        }

        Ok(())
    }
}
