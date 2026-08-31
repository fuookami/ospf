//! 编组生成上下文模块 / Bunch generation context module
use std::collections::HashMap;
use super::model::{FlightTaskReverse, Graph};
use super::service::{
    AggregationInitializer, FlightTaskBunchGenerator, GeneratedBunch,
    RouteGraphGeneratorConfig,
};

/// Bunch 生成上下文 / Bunch generation context
/// 对齐 FSRA BunchGenerationContext
pub struct BunchGenerationContext {
    /// 飞机标识列表 / Aircraft identifier list
    pub aircraft_ids: Vec<String>,
    /// 飞机位置映射 / Aircraft location map
    pub aircraft_locations: HashMap<String, String>,
    /// 锁定任务列表 / Locked task list
    pub locked_tasks: Vec<String>,
    /// 最大编组数量 / Maximum bunch count
    pub max_bunches: usize,
    /// 缩减成本阈值 / Reduced cost threshold
    pub reduced_cost_threshold: f64,
    /// 是否允许换序 / Whether order change is allowed
    pub with_order_change: bool,
}

impl BunchGenerationContext {
    /// 创建新的编组生成上下文 / Create new bunch generation context
    pub fn new(
        aircraft_ids: Vec<String>,
        aircraft_locations: HashMap<String, String>,
        locked_tasks: Vec<String>,
    ) -> Self {
        Self {
            aircraft_ids,
            aircraft_locations,
            locked_tasks,
            max_bunches: 100,
            reduced_cost_threshold: -1e-6,
            with_order_change: false,
        }
    }

    /// 初始化 / Initialize
    pub fn initialize(
        &self,
        flight_tasks: &HashMap<String, Vec<super::service::FlightTaskInfo>>,
        origin_bunches: &[Vec<String>],
        task_pairs: Vec<(String, String)>,
    ) -> super::service::AggregationResult {
        let initializer = AggregationInitializer {
            feasibility_judger: Box::new(|_aircraft, _prev, _task| true),
            with_order_change: self.with_order_change,
            time_difference_limit: FlightTaskReverse::DEFAULT_TIME_DIFFERENCE_LIMIT,
        };
        initializer.initialize(
            &self.aircraft_ids,
            &self.aircraft_locations,
            flight_tasks,
            origin_bunches,
            &self.locked_tasks,
            task_pairs,
        )
    }

    /// 生成新束 / Generate new bunches (pricing)
    pub fn generate_bunches(
        &self,
        graph: &Graph,
        shadow_prices: &HashMap<String, f64>,
        cost_calculator: &dyn Fn(&str, Option<&str>, &str) -> f64,
        connection_time_calculator: &dyn Fn(&str, &str) -> f64,
    ) -> Vec<GeneratedBunch> {
        let generator = FlightTaskBunchGenerator::new(self.max_bunches, self.reduced_cost_threshold);
        generator.generate(graph, shadow_prices, cost_calculator, connection_time_calculator)
    }
}
