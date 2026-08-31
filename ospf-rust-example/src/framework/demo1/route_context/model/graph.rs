use super::{Edge, Node};

/// 图 / Graph (对齐 Kotlin Graph)
#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
