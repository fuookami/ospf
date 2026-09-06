//! VRPTW 分支遮罩 / VRPTW branch masks.

use std::collections::{BTreeMap, BTreeSet};

use super::{CustomerId, VehicleTypeId};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId};

/// 车辆类型维度的稳定弧 / Stable arc in a vehicle-type dimension.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceArc<K: Clone + Ord + Eq> {
    /// 资源键 / Resource key.
    pub resource_key: K,
    /// 稳定弧 ID / Stable arc ID.
    pub arc_id: NetworkArcId,
    /// 起点 / Origin.
    pub from: NetworkNodeId,
    /// 终点 / Destination.
    pub to: NetworkNodeId,
}

impl<K> ResourceArc<K>
where
    K: Clone + Ord + Eq,
{
    /// 创建资源弧 / Create a resource arc.
    pub fn new(
        resource_key: K,
        arc_id: NetworkArcId,
        from: NetworkNodeId,
        to: NetworkNodeId,
    ) -> Self {
        Self {
            resource_key,
            arc_id,
            from,
            to,
        }
    }
}

/// 不可变车辆类型分配与弧分支遮罩 / Immutable vehicle-type and arc branch mask.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BranchMask<K: Clone + Ord + Eq> {
    /// 起始仓库 / Start depot.
    pub start_depot: NetworkNodeId,
    /// 结束仓库 / End depot.
    pub end_depot: NetworkNodeId,
    /// 按资源禁止的节点 / Nodes forbidden by resource.
    pub forbidden_nodes: BTreeMap<K, BTreeSet<NetworkNodeId>>,
    /// 客户必须使用的资源 / Required resource per customer node.
    pub required_resources: BTreeMap<NetworkNodeId, K>,
    /// 禁止弧 / Forbidden arcs.
    pub forbidden_arcs: BTreeSet<ResourceArc<K>>,
    /// 必选弧 / Required arcs.
    pub required_arcs: BTreeSet<ResourceArc<K>>,
}

impl<K> BranchMask<K>
where
    K: Clone + Ord + Eq,
{
    /// 创建并校验分支遮罩 / Create and validate a branch mask.
    pub fn new(
        start_depot: NetworkNodeId,
        end_depot: NetworkNodeId,
        forbidden_nodes: BTreeMap<K, BTreeSet<NetworkNodeId>>,
        mut required_resources: BTreeMap<NetworkNodeId, K>,
        forbidden_arcs: BTreeSet<ResourceArc<K>>,
        required_arcs: BTreeSet<ResourceArc<K>>,
    ) -> Result<Self> {
        if start_depot == end_depot {
            return Err(NetworkSchedulingError::validation(
                "起止仓库必须不同 / start and end depots must differ",
            ));
        }
        for arc in &required_arcs {
            if forbidden_arcs.contains(arc) {
                return Err(NetworkSchedulingError::validation(
                    "弧不能同时被要求和禁止 / an arc cannot be both required and forbidden",
                ));
            }
            if arc.from != start_depot
                && let Some(other) = required_arcs.iter().find(|candidate| {
                    candidate.resource_key == arc.resource_key
                        && candidate.from == arc.from
                        && candidate != &arc
                })
            {
                return Err(NetworkSchedulingError::validation(format!(
                    "节点 {} 存在冲突必选出弧 {} / node {} has conflicting required outgoing arcs {}",
                    arc.from, arc.arc_id, arc.from, other.arc_id
                )));
            }
            if arc.to != end_depot
                && let Some(other) = required_arcs.iter().find(|candidate| {
                    candidate.resource_key == arc.resource_key
                        && candidate.to == arc.to
                        && candidate != &arc
                })
            {
                return Err(NetworkSchedulingError::validation(format!(
                    "节点 {} 存在冲突必选入弧 {} / node {} has conflicting required incoming arcs {}",
                    arc.to, arc.arc_id, arc.to, other.arc_id
                )));
            }
            for node in [
                (&arc.from, arc.from != start_depot),
                (&arc.to, arc.to != end_depot),
            ] {
                if !node.1 {
                    continue;
                }
                if forbidden_nodes
                    .get(&arc.resource_key)
                    .is_some_and(|nodes| nodes.contains(node.0))
                {
                    return Err(NetworkSchedulingError::validation(format!(
                        "节点 {} 的必选弧资源同时被禁止 / required arc resource for node {} is forbidden",
                        node.0, node.0
                    )));
                }
                if let Some(previous) =
                    required_resources.insert(node.0.clone(), arc.resource_key.clone())
                    && previous != arc.resource_key
                {
                    return Err(NetworkSchedulingError::validation(format!(
                        "节点 {} 存在冲突必选车辆类型 / node {} has conflicting required resources",
                        node.0, node.0
                    )));
                }
            }
        }
        for (node, resource) in &required_resources {
            if forbidden_nodes
                .get(resource)
                .is_some_and(|nodes| nodes.contains(node))
            {
                return Err(NetworkSchedulingError::validation(format!(
                    "节点 {} 的必选资源同时被禁止 / required resource for node {} is forbidden",
                    node, node
                )));
            }
        }
        Ok(Self {
            start_depot,
            end_depot,
            forbidden_nodes,
            required_resources,
            forbidden_arcs,
            required_arcs,
        })
    }

    /// 创建空遮罩 / Create an empty mask.
    pub fn empty(start_depot: NetworkNodeId, end_depot: NetworkNodeId) -> Result<Self> {
        Self::new(
            start_depot,
            end_depot,
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        )
    }

    /// 判断资源能否访问节点 / Check whether a resource may visit a node.
    pub fn allows_node(&self, resource: &K, node: &NetworkNodeId) -> bool {
        if node == &self.start_depot || node == &self.end_depot {
            return true;
        }
        if self
            .forbidden_nodes
            .get(resource)
            .is_some_and(|nodes| nodes.contains(node))
        {
            return false;
        }
        self.required_resources
            .get(node)
            .is_none_or(|required| required == resource)
    }

    /// 判断资源能否使用弧 / Check whether a resource may use an arc.
    pub fn allows_arc(
        &self,
        resource: &K,
        arc_id: &NetworkArcId,
        from: &NetworkNodeId,
        to: &NetworkNodeId,
    ) -> bool {
        if !self.allows_node(resource, from) || !self.allows_node(resource, to) {
            return false;
        }
        let candidate =
            ResourceArc::new(resource.clone(), arc_id.clone(), from.clone(), to.clone());
        if self.forbidden_arcs.contains(&candidate) {
            return false;
        }
        self.required_arcs.iter().all(|required| {
            let touches_required_origin =
                required.from != self.start_depot && from == &required.from;
            let touches_required_destination = required.to != self.end_depot && to == &required.to;
            required.resource_key != *resource
                || required == &candidate
                || (!touches_required_origin && !touches_required_destination)
        })
    }

    /// 判断完整路线是否兼容 / Check complete-route compatibility.
    pub fn is_route_compatible(
        &self,
        resource: &K,
        stops: &[NetworkNodeId],
        arc_ids: &[NetworkArcId],
    ) -> bool {
        if stops.len() < 2
            || stops.first() != Some(&self.start_depot)
            || stops.last() != Some(&self.end_depot)
        {
            return false;
        }
        if stops.iter().any(|node| !self.allows_node(resource, node)) {
            return false;
        }
        let ids = if arc_ids.is_empty() {
            stops
                .windows(2)
                .map(|pair| crate::domain::vrp::model::default_arc_id(&pair[0], &pair[1]))
                .collect::<Vec<_>>()
        } else {
            arc_ids.to_vec()
        };
        ids.len() == stops.len() - 1
            && ids
                .iter()
                .zip(stops.windows(2))
                .all(|(arc, pair)| self.allows_arc(resource, arc, &pair[0], &pair[1]))
    }
}

/// 将客户分支所需的 ID 统一保留在应用层的便捷键 / Convenience key retaining customer and vehicle IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleAssignment {
    /// 车辆类型 / Vehicle type.
    pub vehicle_type_id: VehicleTypeId,
    /// 客户 / Customer.
    pub customer_id: CustomerId,
}
