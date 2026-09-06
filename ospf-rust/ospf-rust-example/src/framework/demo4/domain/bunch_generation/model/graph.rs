//! 路线图模型模块 / Route graph model module
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

/// 节点 / Node (对齐 FSRA Graph.kt Node)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    /// 根节点 / Root node
    Root,
    /// 任务节点 / Task node
    Task {
        /// 任务标识 / Task identifier
        task_id: String,
        /// 任务时间 / Task time
        time: OffsetDateTime,
        /// 节点索引 / Node index
        index: u64,
    },
    /// 终止节点 / End node
    End,
}

impl Node {
    /// 获取节点索引 / Get node index
    pub fn index(&self) -> u64 {
        match self {
            Node::Root => 0,
            Node::Task { index, .. } => *index,
            Node::End => u64::MAX,
        }
    }

    /// 获取节点时间 / Get node time
    pub fn time(&self) -> Option<OffsetDateTime> {
        match self {
            Node::Root => None,
            Node::Task { time, .. } => Some(*time),
            Node::End => None,
        }
    }

    /// 是否为根节点 / Whether this is a root node
    pub fn is_root(&self) -> bool {
        matches!(self, Node::Root)
    }

    /// 是否为终止节点 / Whether this is an end node
    pub fn is_end(&self) -> bool {
        matches!(self, Node::End)
    }

    /// 是否为任务节点 / Whether this is a task node
    pub fn is_task(&self) -> bool {
        matches!(self, Node::Task { .. })
    }

    /// 获取任务标识 / Get task identifier
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Node::Task { task_id, .. } => Some(task_id),
            _ => None,
        }
    }
}

/// 边 / Edge (对齐 FSRA Graph.kt Edge)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edge {
    /// 起始节点 / Source node
    pub from: Node,
    /// 目标节点 / Target node
    pub to: Node,
}

/// 图 / Graph (对齐 FSRA Graph.kt Graph)
#[derive(Debug, Clone)]
pub struct Graph {
    /// 节点映射 / Node map (index -> Node)
    pub nodes: HashMap<u64, Node>,
    /// 边映射 / Edge map (Node -> Edge set)
    pub edges: HashMap<Node, HashSet<Edge>>,
}

impl Graph {
    /// 创建包含根节点和终止节点的新图 / Create new graph with root and end nodes
    pub fn new() -> Self {
        let mut graph = Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        };
        graph.put_node(Node::Root);
        graph.put_node(Node::End);
        graph
    }

    /// 添加节点 / Put node
    pub fn put_node(&mut self, node: Node) {
        self.nodes.insert(node.index(), node);
    }

    /// 添加边 / Put edge
    pub fn put_edge(&mut self, from: Node, to: Node) {
        self.edges
            .entry(from.clone())
            .or_insert_with(HashSet::new)
            .insert(Edge { from, to });
    }

    /// 获取节点 / Get node by index
    pub fn get_node(&self, index: u64) -> Option<&Node> {
        self.nodes.get(&index)
    }

    /// 获取节点的出边 / Get edges from node
    pub fn get_edges(&self, node: &Node) -> HashSet<Edge> {
        self.edges.get(node).cloned().unwrap_or_default()
    }

    /// 检查是否连接 / Check if connected
    pub fn connected(&self, from: &Node, to: &Node) -> bool {
        self.edges
            .get(from)
            .map(|edges| edges.iter().any(|e| &e.to == to))
            .unwrap_or(false)
    }

    /// 节点数量 / Node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 边数量 / Edge count
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|s| s.len()).sum()
    }

    /// BFS 查找从 from 到 to 的路径 / BFS find path
    pub fn has_path(&self, from: &Node, to: &Node) -> bool {
        let mut visited = HashSet::new();
        let mut queue = vec![from.clone()];
        while let Some(node) = queue.pop() {
            if &node == to {
                return true;
            }
            if !visited.insert(node.clone()) {
                continue;
            }
            for edge in self.get_edges(&node) {
                if !visited.contains(&edge.to) {
                    queue.push(edge.to);
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn task_node(id: &str, index: u64) -> Node {
        Node::Task {
            task_id: id.to_string(),
            time: datetime!(2026-06-07 08:00 UTC),
            index,
        }
    }

    #[test]
    fn new_graph_contains_root_and_end() {
        let graph = Graph::new();
        assert!(graph.get_node(0).unwrap().is_root());
        assert!(graph.get_node(u64::MAX).unwrap().is_end());
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn put_node_and_edge() {
        let mut graph = Graph::new();
        let n1 = task_node("T1", 1);
        let n2 = task_node("T2", 2);
        graph.put_node(n1.clone());
        graph.put_node(n2.clone());
        graph.put_edge(Node::Root, n1.clone());
        graph.put_edge(n1.clone(), n2.clone());
        graph.put_edge(n2.clone(), Node::End);

        assert_eq!(graph.node_count(), 4);
        assert!(graph.connected(&Node::Root, &n1));
        assert!(graph.connected(&n1, &n2));
        assert!(graph.connected(&n2, &Node::End));
        assert!(!graph.connected(&Node::Root, &n2));
    }

    #[test]
    fn has_path_finds_connected_nodes() {
        let mut graph = Graph::new();
        let n1 = task_node("T1", 1);
        let n2 = task_node("T2", 2);
        graph.put_node(n1.clone());
        graph.put_node(n2.clone());
        graph.put_edge(Node::Root, n1.clone());
        graph.put_edge(n1.clone(), n2.clone());
        graph.put_edge(n2.clone(), Node::End);

        assert!(graph.has_path(&Node::Root, &Node::End));
        assert!(graph.has_path(&Node::Root, &n2));
        assert!(!graph.has_path(&n2, &Node::Root));
    }

    #[test]
    fn get_edges_returns_empty_for_unknown_node() {
        let graph = Graph::new();
        let n1 = task_node("T1", 1);
        assert!(graph.get_edges(&n1).is_empty());
    }
}
