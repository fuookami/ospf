//! 编组生成领域模块 / Bunch generation domain module.
/// 编组生成聚合模块 / Bunch generation aggregation module
pub mod aggregation;
/// 编组生成上下文模块 / Bunch generation context module
pub mod context;
/// 编组生成模型模块 / Bunch generation model module
pub mod model;
/// 编组生成服务模块 / Bunch generation service module
pub mod service;

/// Bunch 生成聚合 / Bunch generation aggregation
/// 对齐 FSRA bunch_generation_context Aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 全局路线图 / Global route graph
    pub graph: model::Graph,
    /// 任务对换管理器 / Flight task reverse manager
    pub flight_task_reverse: Option<model::FlightTaskReverse>,
    /// 按飞机分组的路线图 / Route graphs grouped by aircraft
    pub graphs_by_aircraft: std::collections::HashMap<String, model::Graph>,
    /// 初始编组列表 / Initial bunch list
    pub initial_bunches: Vec<Vec<String>>,
    /// 已生成的编组列表 / Generated bunch list
    pub generated_bunches: Vec<Vec<String>>,
}

impl Aggregation {
    /// 创建新的聚合 / Create new aggregation
    pub fn new(graph: model::Graph) -> Self {
        Self {
            graph,
            flight_task_reverse: None,
            graphs_by_aircraft: std::collections::HashMap::new(),
            initial_bunches: Vec::new(),
            generated_bunches: Vec::new(),
        }
    }
}
