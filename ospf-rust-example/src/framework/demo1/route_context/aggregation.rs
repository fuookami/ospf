use super::model::{Assignment, Edge, Graph, Node, Service};

pub struct Aggregation {
    pub graph: Graph,
    pub services: Vec<Service>,
    pub assignment: Assignment,
}

impl Aggregation {
    pub fn new(graph: Graph, services: Vec<Service>, assignment: Assignment) -> Self {
        Self {
            graph,
            services,
            assignment,
        }
    }
}
