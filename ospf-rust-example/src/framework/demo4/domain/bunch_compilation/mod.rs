pub mod context;
pub mod model;
pub mod service;

/// Bunch 编译聚合 / Bunch compilation aggregation
/// 对齐 Kotlin BunchCompilationAggregation
#[derive(Debug)]
pub struct Aggregation {
    pub compilations: Vec<model::Compilation>,
    pub fleet_balances: Vec<model::FleetBalance>,
    pub flight_capacities: Vec<model::FlightCapacity>,
    pub flight_links: Vec<model::FlightLink>,
}

impl Aggregation {
    /// 注册所有符号到模型
    /// 对齐 Kotlin BunchCompilationAggregation.register
    pub fn register(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut next_id = 70000u64;

        for capacity in &self.flight_capacities {
            capacity.register(model, &mut next_id)?;
        }
        for balance in &self.fleet_balances {
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

    /// 添加新列
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
