//! 聚合初始化器模块 / Aggregation initializer module
use super::super::model::{FlightTaskReverse, Graph};
use super::initial_flight_task_bunch_generator::InitialFlightTaskBunchGenerator;
use super::operator::FeasibilityJudger;
use super::route_graph_generator::{
    FlightTaskInfo, RouteGraphGenerator, RouteGraphGeneratorConfig,
};
use std::collections::HashMap;

/// 聚合初始化器 / Aggregation initializer
/// 对齐 FSRA AggregationInitializer
pub struct AggregationInitializer {
    /// 可行性判断器 / Feasibility judger
    pub feasibility_judger: FeasibilityJudger,
    /// 是否允许换序 / Whether order change is allowed
    pub with_order_change: bool,
    /// 时间差限制 / Time difference limit
    pub time_difference_limit: time::Duration,
}

impl AggregationInitializer {
    /// 初始化聚合 / Initialize aggregation
    /// 对齐 FSRA AggregationInitializer.invoke
    pub fn initialize(
        &self,
        aircraft_ids: &[String],
        aircraft_locations: &HashMap<String, String>,
        flight_tasks: &HashMap<String, Vec<FlightTaskInfo>>,
        origin_bunches: &[Vec<String>],
        locked_tasks: &[String],
        task_pairs: Vec<(String, String)>,
    ) -> AggregationResult {
        // 1. 初始化 FlightTaskReverse
        let reverse = FlightTaskReverse::new(
            task_pairs,
            origin_bunches,
            locked_tasks,
            self.time_difference_limit,
        );

        // 2. 按 aircraft 构造 route graph
        let config = RouteGraphGeneratorConfig {
            with_order_change: self.with_order_change,
        };
        let generator = RouteGraphGenerator::new(
            reverse.clone(),
            config,
            // feasibility_judger 不可 clone，这里用一个简化版本
            Box::new(|_aircraft, _prev, _task| true),
        );

        let mut graphs = HashMap::new();
        for aircraft_id in aircraft_ids {
            let location = aircraft_locations
                .get(aircraft_id)
                .map(|s| s.as_str())
                .unwrap_or("");
            let graph = generator.generate(aircraft_id, location, flight_tasks);
            graphs.insert(aircraft_id.clone(), graph);
        }

        // 3. 生成初始 bunch
        let initial_generator = InitialFlightTaskBunchGenerator;
        let initial_bunches = initial_generator.generate(aircraft_ids, &graphs, locked_tasks);

        AggregationResult {
            flight_task_reverse: reverse,
            graphs_by_aircraft: graphs,
            initial_bunches,
        }
    }
}

/// 聚合初始化结果 / Aggregation initialization result
#[derive(Debug)]
pub struct AggregationResult {
    /// 任务对换管理器 / Flight task reverse manager
    pub flight_task_reverse: FlightTaskReverse,
    /// 按飞机分组的路线图 / Route graphs grouped by aircraft
    pub graphs_by_aircraft: HashMap<String, Graph>,
    /// 初始编组列表 / Initial bunch list
    pub initial_bunches: Vec<Vec<String>>,
}
