//! 定价图模型 / Pricing graph model
//!
//! 定义列生成定价问题中的有向图结构，包含根节点、终止节点和任务节点。
//! Defines the directed graph structure for column generation pricing, with root, end, and task nodes.

use std::collections::HashMap;

// ============================================================================
// 节点类型 / Node Types
// ============================================================================

/// 节点索引类型 / Node index type
pub type NodeIndex = u64;

/// 节点 / Node
///
/// 定价图中的节点，分为根节点、终止节点和任务节点。
/// Nodes in the pricing graph: root, end, and task nodes.
#[derive(Debug, Clone)]
pub enum Node {
    /// 根节点（虚拟起点）/ Root node (virtual start)
    Root,
    /// 终止节点（虚拟终点）/ End node (virtual terminal)
    End,
    /// 任务节点 / Task node
    Task(TaskNode),
}

/// 任务节点 / Task node
///
/// 关联一个任务索引和时间点。
/// Associates a task index with a time point.
#[derive(Debug, Clone)]
pub struct TaskNode {
    /// 任务索引 / Task index
    pub task_index: usize,
    /// 节点索引 / Node index
    pub node_index: NodeIndex,
    /// 时间点（solver 值域）/ Time point in solver value domain
    pub time: f64,
}

impl TaskNode {
    /// 创建新的任务节点 / Create new task node
    pub fn new(task_index: usize, node_index: NodeIndex, time: f64) -> Self {
        Self {
            task_index,
            node_index,
            time,
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Node::Root, Node::Root) => true,
            (Node::End, Node::End) => true,
            (Node::Task(a), Node::Task(b)) => a.node_index == b.node_index,
            _ => false,
        }
    }
}

impl Eq for Node {}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Node::Root => 0u8.hash(state),
            Node::End => 1u8.hash(state),
            Node::Task(t) => {
                2u8.hash(state);
                t.node_index.hash(state);
            }
        }
    }
}

impl Node {
    /// 获取节点索引 / Get node index
    pub fn index(&self) -> NodeIndex {
        match self {
            Node::Root => 0,
            Node::End => NodeIndex::MAX,
            Node::Task(t) => t.node_index,
        }
    }

    /// 获取时间点 / Get time point
    pub fn time(&self) -> f64 {
        match self {
            Node::Root => f64::NEG_INFINITY,
            Node::End => f64::INFINITY,
            Node::Task(t) => t.time,
        }
    }

    /// 是否为任务节点 / Whether this is a task node
    pub fn is_task(&self) -> bool {
        matches!(self, Node::Task(_))
    }

    /// 获取任务索引（如果是任务节点）/ Get task index if task node
    pub fn task_index(&self) -> Option<usize> {
        match self {
            Node::Task(t) => Some(t.task_index),
            _ => None,
        }
    }
}

// ============================================================================
// 边 / Edge
// ============================================================================

/// 有向边 / Directed edge
#[derive(Debug, Clone)]
pub struct Edge {
    /// 起始节点 / Source node
    pub from: Node,
    /// 终止节点 / Target node
    pub to: Node,
}

impl Edge {
    /// 创建新的有向边 / Create new directed edge
    pub fn new(from: Node, to: Node) -> Self {
        Self { from, to }
    }
}

// ============================================================================
// 图 / Graph
// ============================================================================

/// 定价图 / Pricing graph
///
/// 有向图，用于建模列生成定价问题。
/// 任务节点通过边连接，表示可行的任务序列。
///
/// Directed graph for modeling the column generation pricing problem.
/// Task nodes are connected by edges representing feasible task sequences.
pub struct Graph {
    /// 节点索引到节点的映射 / Node index to node mapping
    nodes: HashMap<NodeIndex, Node>,
    /// 节点到出边集合的映射 / Node to outgoing edges mapping
    edges: HashMap<Node, Vec<Edge>>,
}

impl std::fmt::Debug for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graph")
            .field("node_count", &self.nodes.len())
            .field(
                "edge_count",
                &self.edges.values().map(|v| v.len()).sum::<usize>(),
            )
            .finish()
    }
}

impl Graph {
    /// 创建空图 / Create empty graph
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(0, Node::Root);
        nodes.insert(NodeIndex::MAX, Node::End);

        Self {
            nodes,
            edges: HashMap::new(),
        }
    }

    /// 添加节点 / Add node
    pub fn add_node(&mut self, node: Node) {
        let idx = node.index();
        self.nodes.insert(idx, node);
    }

    /// 添加边 / Add edge
    pub fn add_edge(&mut self, from: Node, to: Node) {
        let edge = Edge::new(from.clone(), to);
        self.edges.entry(from).or_default().push(edge);
    }

    /// 获取节点 / Get node by index
    pub fn get_node(&self, index: NodeIndex) -> Option<&Node> {
        self.nodes.get(&index)
    }

    /// 获取节点的出边 / Get outgoing edges from node
    pub fn edges_from(&self, node: &Node) -> &[Edge] {
        self.edges.get(node).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 检查两个节点是否连通 / Check if two nodes are connected
    pub fn connected(&self, from: &Node, to: &Node) -> bool {
        self.edges_from(from).iter().any(|e| &e.to == to)
    }

    /// 获取根节点 / Get root node
    pub fn root(&self) -> &Node {
        self.nodes.get(&0).expect("Root node always exists")
    }

    /// 获取终止节点 / Get end node
    pub fn end(&self) -> &Node {
        self.nodes
            .get(&NodeIndex::MAX)
            .expect("End node always exists")
    }

    /// 节点数量 / Number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 边数量 / Number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }

    /// 反转图 / Reverse the graph
    ///
    /// 所有边的方向反转，RootNode 和 EndNode 互换。
    /// Reverses all edge directions, swapping RootNode and EndNode.
    pub fn reverse(&self) -> Graph {
        let mut reversed = Graph::new();
        // 添加所有节点（Root 和 End 互换）
        for node in self.nodes.values() {
            match node {
                Node::Root => {} // 已存在为 End
                Node::End => {}  // 已存在为 Root
                Node::Task(_) => reversed.add_node(node.clone()),
            }
        }
        // 反转所有边
        for edges in self.edges.values() {
            for edge in edges {
                let rev_from = match &edge.to {
                    Node::Root => Node::End,
                    Node::End => Node::Root,
                    n => n.clone(),
                };
                let rev_to = match &edge.from {
                    Node::Root => Node::End,
                    Node::End => Node::Root,
                    n => n.clone(),
                };
                reversed.add_edge(rev_from, rev_to);
            }
        }
        reversed
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_basic() {
        let mut graph = Graph::new();
        assert_eq!(graph.node_count(), 2); // Root + End

        let task_node = Node::Task(TaskNode::new(0, 1, 100.0));
        graph.add_node(task_node.clone());
        assert_eq!(graph.node_count(), 3);

        graph.add_edge(Node::Root, task_node.clone());
        graph.add_edge(task_node.clone(), Node::End);
        assert_eq!(graph.edge_count(), 2);

        assert!(graph.connected(&Node::Root, &task_node));
        assert!(graph.connected(&task_node, &Node::End));
        assert!(!graph.connected(&Node::End, &task_node));
    }

    #[test]
    fn test_graph_reverse() {
        let mut graph = Graph::new();
        let task_node = Node::Task(TaskNode::new(0, 1, 100.0));
        graph.add_node(task_node.clone());
        graph.add_edge(Node::Root, task_node.clone());
        graph.add_edge(task_node.clone(), Node::End);

        let reversed = graph.reverse();
        assert!(reversed.connected(&Node::Root, &task_node));
        assert!(reversed.connected(&task_node, &Node::End));
    }

    #[test]
    fn test_node_equality() {
        let n1 = Node::Task(TaskNode::new(0, 1, 100.0));
        let n2 = Node::Task(TaskNode::new(0, 1, 200.0)); // same node_index
        let n3 = Node::Task(TaskNode::new(1, 2, 100.0));
        assert_eq!(n1, n2); // equality by node_index
        assert_ne!(n1, n3);
        assert_eq!(Node::Root, Node::Root);
        assert_eq!(Node::End, Node::End);
        assert_ne!(Node::Root, Node::End);
    }
}
