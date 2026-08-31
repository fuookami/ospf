/// 节点类型 / Node kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Normal,
    Client,
}

/// 节点 / Node (对齐 Kotlin Node)
#[derive(Debug, Clone)]
pub struct Node {
    pub id: u64,
    pub kind: NodeKind,
    pub demand: f64,
    pub edges: Vec<usize>,
}

impl Node {
    pub fn normal(id: u64) -> Self {
        Self {
            id,
            kind: NodeKind::Normal,
            demand: 0.0,
            edges: Vec::new(),
        }
    }

    pub fn client(id: u64, demand: f64) -> Self {
        Self {
            id,
            kind: NodeKind::Client,
            demand,
            edges: Vec::new(),
        }
    }

    pub fn is_normal(&self) -> bool {
        self.kind == NodeKind::Normal
    }

    pub fn is_client(&self) -> bool {
        self.kind == NodeKind::Client
    }

    pub fn add_edge(&mut self, edge_index: usize) {
        self.edges.push(edge_index);
    }
}
