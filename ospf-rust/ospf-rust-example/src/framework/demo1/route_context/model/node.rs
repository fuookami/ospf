//! 节点模型 / Node model

/// 节点类型 / Node kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// 普通节点 / Normal node
    Normal,
    /// 客户节点 / Client node
    Client,
}

/// 节点模型 / Node model (对齐 Kotlin Node)
#[derive(Debug, Clone)]
pub struct Node {
    /// 节点标识 / Node identifier
    pub id: u64,
    /// 节点类型 / Node kind
    pub kind: NodeKind,
    /// 带宽需求 / Bandwidth demand
    pub demand: f64,
    /// 关联的边索引列表 / Associated edge index list
    pub edges: Vec<usize>,
}

impl Node {
    /// 创建普通节点 / Create a normal node
    pub fn normal(id: u64) -> Self {
        Self {
            id,
            kind: NodeKind::Normal,
            demand: 0.0,
            edges: Vec::new(),
        }
    }

    /// 创建客户节点 / Create a client node
    pub fn client(id: u64, demand: f64) -> Self {
        Self {
            id,
            kind: NodeKind::Client,
            demand,
            edges: Vec::new(),
        }
    }

    /// 判断是否为普通节点 / Check if this is a normal node
    pub fn is_normal(&self) -> bool {
        self.kind == NodeKind::Normal
    }

    /// 判断是否为客户节点 / Check if this is a client node
    pub fn is_client(&self) -> bool {
        self.kind == NodeKind::Client
    }

    /// 添加关联边索引 / Add associated edge index
    pub fn add_edge(&mut self, edge_index: usize) {
        self.edges.push(edge_index);
    }
}
