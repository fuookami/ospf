//! 图模型 / Graph model

use super::{Edge, Node};

/// 图模型 / Graph model (对齐 Kotlin Graph)
#[derive(Debug, Clone)]
pub struct Graph {
    /// 节点列表 / Node list
    pub nodes: Vec<Node>,
    /// 边列表 / Edge list
    pub edges: Vec<Edge>,
}
