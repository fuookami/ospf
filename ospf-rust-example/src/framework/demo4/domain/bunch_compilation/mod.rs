//! 编组编制领域模块 / Bunch compilation domain module.
/// 编组编制上下文 / Bunch compilation context
pub mod context;
/// 编组编制模型 / Bunch compilation model
pub mod model;
/// 编组编制服务 / Bunch compilation service
pub mod service;

/// Bunch 编译聚合 / Bunch compilation aggregation
/// 对齐 Kotlin BunchCompilationAggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 编译结果列表 / Compilation result list
    pub compilations: Vec<model::Compilation>,
    /// 机队平衡列表 / Fleet balance list
    pub fleet_balances: Vec<model::FleetBalance>,
    /// 航班容量列表 / Flight capacity list
    pub flight_capacities: Vec<model::FlightCapacity>,
    /// 航班链接列表 / Flight link list
    pub flight_links: Vec<model::FlightLink>,
}

impl Aggregation {
    /// 注册所有符号到模型 / Register all symbols to model
    /// 对齐 Kotlin BunchCompilationAggregation.register
    pub fn register(
        &mut self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut next_id = 70000u64;

        for capacity in &self.flight_capacities {
            capacity.register(model, &mut next_id)?;
        }
        for balance in &mut self.fleet_balances {
            balance.register(model, &mut next_id)?;
        }
        for link in &self.flight_links {
            link.register(model, &mut next_id)?;
        }
        for compilation in &self.compilations {
            compilation.register(model, &mut next_id)?;
        }

        Ok(())
    }

    /// 添加新列 / Add new columns
    /// 对齐 Kotlin BunchCompilationAggregation.addColumns
    pub fn add_columns(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
        new_bunches: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        for balance in &self.fleet_balances {
            balance.add_columns(model, new_bunches)?;
        }
        for link in &self.flight_links {
            link.add_columns(model, new_bunches)?;
        }
        Ok(())
    }
}
