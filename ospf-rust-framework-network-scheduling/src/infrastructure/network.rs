//! 泛型有向网络图 / Generic directed network graph.

use std::collections::HashMap;

use super::{NetworkArcId, NetworkNodeId};
use crate::error::{NetworkSchedulingError, Result};

/// 带泛型 payload 的网络节点 / Network node with a typed payload.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkNode<N> {
    /// 稳定节点 ID / Stable node ID.
    pub id: NetworkNodeId,
    /// 节点业务 payload / Node payload.
    pub payload: N,
}

impl<N> NetworkNode<N> {
    /// 创建网络节点 / Create a network node.
    pub fn new(id: impl Into<NetworkNodeId>, payload: N) -> Self {
        Self {
            id: id.into(),
            payload,
        }
    }
}

/// 带泛型 payload 的有向弧 / Directed arc with a typed payload.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkArc<A> {
    /// 稳定弧 ID / Stable arc ID.
    pub id: NetworkArcId,
    /// 起点节点 / Origin node.
    pub from: NetworkNodeId,
    /// 终点节点 / Destination node.
    pub to: NetworkNodeId,
    /// 弧业务 payload / Arc payload.
    pub payload: A,
}

impl<A> NetworkArc<A> {
    /// 创建网络弧 / Create a network arc.
    pub fn new(
        id: impl Into<NetworkArcId>,
        from: impl Into<NetworkNodeId>,
        to: impl Into<NetworkNodeId>,
        payload: A,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            payload,
        }
    }
}

/// 不可变图快照 / Immutable graph snapshot.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkGraph<N, A> {
    nodes: Vec<NetworkNode<N>>,
    arcs: Vec<NetworkArc<A>>,
    node_indices: HashMap<NetworkNodeId, usize>,
    arc_indices: HashMap<NetworkArcId, usize>,
    outgoing: HashMap<NetworkNodeId, Vec<usize>>,
    incoming: HashMap<NetworkNodeId, Vec<usize>>,
}

impl<N, A> NetworkGraph<N, A> {
    /// 从节点与弧构造并校验不可变图 / Build and validate an immutable graph.
    pub fn new(nodes: Vec<NetworkNode<N>>, arcs: Vec<NetworkArc<A>>) -> Result<Self> {
        let mut node_indices = HashMap::with_capacity(nodes.len());
        for (index, node) in nodes.iter().enumerate() {
            if node.id.as_str().is_empty() {
                return Err(NetworkSchedulingError::structure(
                    "节点 ID 不能为空 / node ID cannot be empty",
                ));
            }
            if node_indices.insert(node.id.clone(), index).is_some() {
                return Err(NetworkSchedulingError::structure(format!(
                    "节点 ID 重复：{} / duplicate node ID: {}",
                    node.id, node.id
                )));
            }
        }

        let mut arc_indices = HashMap::with_capacity(arcs.len());
        let mut outgoing: HashMap<NetworkNodeId, Vec<usize>> = HashMap::new();
        let mut incoming: HashMap<NetworkNodeId, Vec<usize>> = HashMap::new();
        for (index, arc) in arcs.iter().enumerate() {
            if arc.id.as_str().is_empty() {
                return Err(NetworkSchedulingError::structure(
                    "弧 ID 不能为空 / arc ID cannot be empty",
                ));
            }
            if arc_indices.insert(arc.id.clone(), index).is_some() {
                return Err(NetworkSchedulingError::structure(format!(
                    "弧 ID 重复：{} / duplicate arc ID: {}",
                    arc.id, arc.id
                )));
            }
            if !node_indices.contains_key(&arc.from) || !node_indices.contains_key(&arc.to) {
                return Err(NetworkSchedulingError::structure(format!(
                    "弧 {} 的端点不存在 / endpoint of arc {} does not exist",
                    arc.id, arc.id
                )));
            }
            outgoing.entry(arc.from.clone()).or_default().push(index);
            incoming.entry(arc.to.clone()).or_default().push(index);
        }

        Ok(Self {
            nodes,
            arcs,
            node_indices,
            arc_indices,
            outgoing,
            incoming,
        })
    }

    /// 获取节点快照 / Get node snapshot.
    pub fn nodes(&self) -> &[NetworkNode<N>] {
        &self.nodes
    }

    /// 获取弧快照 / Get arc snapshot.
    pub fn arcs(&self) -> &[NetworkArc<A>] {
        &self.arcs
    }

    /// 按稳定 ID 获取节点 / Get a node by stable ID.
    pub fn node(&self, id: &NetworkNodeId) -> Option<&NetworkNode<N>> {
        self.node_indices
            .get(id)
            .and_then(|index| self.nodes.get(*index))
    }

    /// 按稳定 ID 获取弧 / Get an arc by stable ID.
    pub fn arc(&self, id: &NetworkArcId) -> Option<&NetworkArc<A>> {
        self.arc_indices
            .get(id)
            .and_then(|index| self.arcs.get(*index))
    }

    /// 获取节点局部索引 / Get a node local index.
    pub fn node_index(&self, id: &NetworkNodeId) -> Option<usize> {
        self.node_indices.get(id).copied()
    }

    /// 获取弧局部索引 / Get an arc local index.
    pub fn arc_index(&self, id: &NetworkArcId) -> Option<usize> {
        self.arc_indices.get(id).copied()
    }

    /// 获取出弧 / Get outgoing arcs.
    pub fn outgoing(&self, node: &NetworkNodeId) -> impl Iterator<Item = &NetworkArc<A>> {
        self.outgoing
            .get(node)
            .into_iter()
            .flatten()
            .filter_map(|index| self.arcs.get(*index))
    }

    /// 获取入弧 / Get incoming arcs.
    pub fn incoming(&self, node: &NetworkNodeId) -> impl Iterator<Item = &NetworkArc<A>> {
        self.incoming
            .get(node)
            .into_iter()
            .flatten()
            .filter_map(|index| self.arcs.get(*index))
    }

    /// 按条件创建弧过滤视图 / Create a filtered arc view.
    pub fn filtered<'a, F>(&'a self, predicate: F) -> impl Iterator<Item = &'a NetworkArc<A>>
    where
        F: Fn(&NetworkArc<A>) -> bool + 'a,
    {
        self.arcs.iter().filter(move |arc| predicate(arc))
    }

    /// 按弧 ID 生成有序路线签名 / Build an ordered arc-ID route signature.
    pub fn arc_signature<'a, I>(&self, ids: I) -> String
    where
        I: IntoIterator<Item = &'a NetworkArcId>,
    {
        let mut signature = String::from("arc-sequence-v1;");
        for id in ids {
            signature.push_str(&id.as_str().len().to_string());
            signature.push(':');
            signature.push_str(id.as_str());
            signature.push(';');
        }
        signature
    }
}

/// 图构建器 / Graph builder.
#[derive(Debug, Clone)]
pub struct NetworkGraphBuilder<N, A> {
    nodes: Vec<NetworkNode<N>>,
    arcs: Vec<NetworkArc<A>>,
}

impl<N, A> Default for NetworkGraphBuilder<N, A> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            arcs: Vec::new(),
        }
    }
}

impl<N, A> NetworkGraphBuilder<N, A> {
    /// 创建空构建器 / Create an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加节点 / Add a node.
    pub fn add_node(&mut self, node: NetworkNode<N>) -> &mut Self {
        self.nodes.push(node);
        self
    }

    /// 添加弧；允许相同端点的平行弧 / Add an arc; parallel arcs are allowed.
    pub fn add_arc(&mut self, arc: NetworkArc<A>) -> &mut Self {
        self.arcs.push(arc);
        self
    }

    /// 消费构建器并生成图快照 / Consume the builder and create a graph snapshot.
    pub fn build(self) -> Result<NetworkGraph<N, A>> {
        NetworkGraph::new(self.nodes, self.arcs)
    }
}
