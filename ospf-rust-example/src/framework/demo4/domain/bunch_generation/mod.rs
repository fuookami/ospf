pub mod aggregation;
pub mod context;
pub mod model;
pub mod service;

/// Bunch 生成聚合 / Bunch generation aggregation
/// 对齐 FSRA bunch_generation_context Aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub graph: model::Graph,
    pub flight_task_reverse: Option<model::FlightTaskReverse>,
    pub graphs_by_aircraft: std::collections::HashMap<String, model::Graph>,
    pub initial_bunches: Vec<Vec<String>>,
    pub generated_bunches: Vec<Vec<String>>,
}

impl Aggregation {
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
