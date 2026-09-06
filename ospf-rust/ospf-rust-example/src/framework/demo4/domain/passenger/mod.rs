//! 旅客领域模块 / Passenger domain module.
/// 旅客上下文模块 / Passenger context module
pub mod context;
/// 旅客模型模块 / Passenger model module
pub mod model;
/// 旅客服务模块 / Passenger service module
pub mod service;

/// 旅客领域聚合 / Passenger domain aggregation
/// 对齐 Kotlin passenger Aggregation / Aligned with Kotlin passenger Aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 旅客列表 / Passenger list
    pub passengers: Vec<model::Passenger>,
    /// 旅客数量列表 / Passenger amount list
    pub amounts: Vec<model::PassengerAmount>,
    /// 旅客取消列表 / Passenger cancel list
    pub cancels: Vec<model::PassengerCancel>,
    /// 旅客变更列表 / Passenger change list
    pub changes: Vec<model::PassengerChange>,
}

impl Aggregation {
    /// 注册所有符号到模型 / Register all symbols to the model
    /// 对齐 Kotlin passenger Aggregation.register / Aligned with Kotlin passenger Aggregation.register
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
