//! 编组编制领域服务 / Bunch compilation domain service.
/// 编组编译约束模块 / Bunch compilation constraints module
pub mod limits;

/// 空闲飞机选择器 / Free aircraft selector
/// 对齐 Kotlin FreeAircraftSelector
pub struct FreeAircraftSelector;

impl FreeAircraftSelector {
    /// 选择空闲飞机 / Select free aircraft
    /// 对齐 Kotlin FreeAircraftSelector.select
    pub fn select(&self, used_aircraft: &[String], all_aircraft: &[String]) -> Vec<String> {
        // 返回未使用的飞机
        all_aircraft
            .iter()
            .filter(|a| !used_aircraft.contains(a))
            .cloned()
            .collect()
    }
}

/// Bunch 编译管道列表生成器 / Bunch compilation pipeline list generator
/// 对齐 Kotlin bunch_compilation PipelineListGenerator
pub fn generate_pipelines(
    aggregation: &super::Aggregation,
    model: &mut ospf_rust_core::model::MetaModel<f64>,
) -> Result<(), Box<dyn std::error::Error>> {
    limits::apply_fleet_balance_limit(
        model,
        &aggregation.compilations,
        &aggregation.fleet_balances,
    )?;
    limits::apply_flight_link_limit(model, &aggregation.compilations, &aggregation.flight_links)?;
    Ok(())
}
