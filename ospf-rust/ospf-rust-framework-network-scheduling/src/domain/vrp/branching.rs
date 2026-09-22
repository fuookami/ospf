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

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vrp::default_arc_id;

    // ========================================================================
    // 固定装置 / Fixtures
    // ========================================================================

    fn node(id: &str) -> NetworkNodeId {
        NetworkNodeId::from(id)
    }

    fn arc_id(id: &str) -> NetworkArcId {
        NetworkArcId::from(id)
    }

    fn nodes(values: &[&str]) -> BTreeSet<NetworkNodeId> {
        values
            .iter()
            .map(|value| NetworkNodeId::from(*value))
            .collect()
    }

    fn resource_arc(
        resource: &'static str,
        arc: &str,
        from: &str,
        to: &str,
    ) -> ResourceArc<&'static str> {
        ResourceArc::new(
            resource,
            NetworkArcId::from(arc),
            NetworkNodeId::from(from),
            NetworkNodeId::from(to),
        )
    }

    fn mask(
        forbidden_nodes: BTreeMap<&'static str, BTreeSet<NetworkNodeId>>,
        required_resources: BTreeMap<NetworkNodeId, &'static str>,
        forbidden_arcs: BTreeSet<ResourceArc<&'static str>>,
        required_arcs: BTreeSet<ResourceArc<&'static str>>,
    ) -> Result<BranchMask<&'static str>> {
        BranchMask::new(
            NetworkNodeId::from("start"),
            NetworkNodeId::from("end"),
            forbidden_nodes,
            required_resources,
            forbidden_arcs,
            required_arcs,
        )
    }

    fn empty_mask() -> BranchMask<&'static str> {
        mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .expect("空遮罩合法 / an empty mask is valid")
    }

    fn assert_validation_error(error: &NetworkSchedulingError, fragment: &str) {
        assert!(
            matches!(error, NetworkSchedulingError::Validation { .. }),
            "期望 Validation 错误，实际为 {error} / expected a Validation error, got {error}"
        );
        assert!(
            error.to_string().contains(fragment),
            "错误消息应包含 {fragment}，实际为 {error} / error message should contain {fragment}, got {error}"
        );
    }

    // ========================================================================
    // 空遮罩与基础可见性 / Empty mask and basic visibility
    // ========================================================================

    #[test]
    fn empty_mask_accepts_every_arc_between_allowed_nodes() {
        let mask = empty_mask();

        assert!(mask.allows_node(&"v1", &node("c1")));
        assert!(mask.allows_arc(&"v1", &arc_id("a1"), &node("c1"), &node("c2")));
        assert!(mask.start_depot == node("start") && mask.end_depot == node("end"));
        assert!(mask.forbidden_nodes.is_empty());
        assert!(mask.required_resources.is_empty());
        assert!(mask.forbidden_arcs.is_empty());
        assert!(mask.required_arcs.is_empty());
    }

    #[test]
    fn mask_rejects_identical_depot_nodes() {
        let error = BranchMask::<&'static str>::new(
            node("start"),
            node("start"),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .expect_err("起止仓库相同必须被拒绝 / identical depots must be rejected");
        assert_validation_error(&error, "起止仓库必须不同");
    }

    #[test]
    fn forbidden_node_blocks_visits_and_incident_arcs() {
        let mask = mask(
            BTreeMap::from([("v1", nodes(&["c1"]))]),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .expect("合法遮罩 / valid mask");

        assert!(!mask.allows_node(&"v1", &node("c1")));
        assert!(mask.allows_node(&"v2", &node("c1")));
        assert!(!mask.allows_arc(&"v1", &arc_id("a1"), &node("c1"), &node("c2")));
        assert!(!mask.allows_arc(&"v1", &arc_id("a2"), &node("c0"), &node("c1")));
        assert!(mask.allows_arc(&"v1", &arc_id("a3"), &node("c0"), &node("c2")));
    }

    #[test]
    fn depot_nodes_are_never_forbidden() {
        let mask = mask(
            BTreeMap::from([("v1", nodes(&["start", "end", "c1"]))]),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .expect("合法遮罩 / valid mask");

        assert!(mask.allows_node(&"v1", &node("start")));
        assert!(mask.allows_node(&"v1", &node("end")));
        assert!(!mask.allows_node(&"v1", &node("c1")));
    }

    #[test]
    fn forbidden_arc_is_rejected_while_parallel_arcs_stay_allowed() {
        let mask = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::from([resource_arc("v1", "a1", "c1", "c2")]),
            BTreeSet::new(),
        )
        .expect("合法遮罩 / valid mask");

        assert!(!mask.allows_arc(&"v1", &arc_id("a1"), &node("c1"), &node("c2")));
        assert!(mask.allows_arc(&"v1", &arc_id("a2"), &node("c1"), &node("c2")));
        assert!(mask.allows_arc(&"v2", &arc_id("a1"), &node("c1"), &node("c2")));
    }

    // ========================================================================
    // 必选弧累积语义 / Required-arc accumulation semantics
    // ========================================================================

    #[test]
    fn required_arc_forces_the_outgoing_choice() {
        let mask = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::from([resource_arc("v1", "a1", "c1", "c2")]),
        )
        .expect("合法遮罩 / valid mask");

        assert!(mask.allows_arc(&"v1", &arc_id("a1"), &node("c1"), &node("c2")));
        assert!(!mask.allows_arc(&"v1", &arc_id("a2"), &node("c1"), &node("c3")));
        // 必选弧同时把两个端点锁定给该资源。
        // A required arc also pins both endpoints to that resource.
        assert_eq!(mask.required_resources.get(&node("c1")), Some(&"v1"));
        assert_eq!(mask.required_resources.get(&node("c2")), Some(&"v1"));
        assert!(!mask.allows_node(&"v2", &node("c1")));
        assert!(!mask.allows_arc(&"v2", &arc_id("a2"), &node("c1"), &node("c3")));
        assert!(mask.allows_arc(&"v2", &arc_id("a3"), &node("c3"), &node("c4")));
    }

    #[test]
    fn required_arc_forces_the_incoming_choice() {
        let mask = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::from([resource_arc("v1", "a1", "c1", "c2")]),
        )
        .expect("合法遮罩 / valid mask");

        assert!(mask.allows_arc(&"v1", &arc_id("a1"), &node("c1"), &node("c2")));
        assert!(!mask.allows_arc(&"v1", &arc_id("a2"), &node("c3"), &node("c2")));
        assert!(!mask.allows_arc(&"v2", &arc_id("a2"), &node("c3"), &node("c2")));
        assert!(mask.allows_arc(&"v2", &arc_id("a3"), &node("c3"), &node("c4")));
    }

    #[test]
    fn required_arc_at_a_depot_does_not_constrain_other_arcs() {
        let mask = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::from([
                resource_arc("v1", "a1", "start", "c1"),
                resource_arc("v1", "a2", "c2", "end"),
            ]),
        )
        .expect("合法遮罩 / valid mask");

        assert!(mask.allows_arc(&"v1", &arc_id("a9"), &node("start"), &node("c3")));
        assert!(mask.allows_arc(&"v1", &arc_id("a8"), &node("c3"), &node("end")));
        // 仓库一侧的必选弧不锁定其它出入弧，但仍把客户端点锁定给该资源。
        // A depot-side required arc does not pin other depot arcs, yet still pins its customer endpoint.
        assert!(mask.allows_arc(&"v1", &arc_id("a7"), &node("c1"), &node("c3")));
        assert!(mask.allows_node(&"v1", &node("c1")));
        assert!(!mask.allows_node(&"v2", &node("c1")));
    }

    #[test]
    fn required_arc_that_is_also_forbidden_is_rejected() {
        let arc = resource_arc("v1", "a1", "c1", "c2");

        let error = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::from([arc.clone()]),
            BTreeSet::from([arc]),
        )
        .expect_err("弧同时必选与禁止必须被拒绝 / an arc cannot be both required and forbidden");
        assert_validation_error(&error, "弧不能同时被要求和禁止");
    }

    #[test]
    fn conflicting_required_outgoing_arcs_are_rejected() {
        let error = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::from([
                resource_arc("v1", "a1", "c1", "c2"),
                resource_arc("v1", "a2", "c1", "c3"),
            ]),
        )
        .expect_err("冲突出弧必须被拒绝 / conflicting required outgoing arcs must be rejected");
        assert_validation_error(&error, "存在冲突必选出弧");
    }

    #[test]
    fn conflicting_required_incoming_arcs_are_rejected() {
        let error = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::from([
                resource_arc("v1", "a1", "c2", "c1"),
                resource_arc("v1", "a2", "c3", "c1"),
            ]),
        )
        .expect_err("冲突入弧必须被拒绝 / conflicting required incoming arcs must be rejected");
        assert_validation_error(&error, "存在冲突必选入弧");
    }

    // ========================================================================
    // 必选资源累积语义 / Required-resource accumulation semantics
    // ========================================================================

    #[test]
    fn required_resource_restricts_node_visits() {
        let mask = mask(
            BTreeMap::new(),
            BTreeMap::from([(node("c1"), "v2")]),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .expect("合法遮罩 / valid mask");

        assert!(mask.allows_node(&"v2", &node("c1")));
        assert!(!mask.allows_node(&"v1", &node("c1")));
        assert!(mask.allows_node(&"v1", &node("c2")));
        assert!(mask.allows_node(&"v1", &node("start")));
    }

    #[test]
    fn required_arc_on_a_forbidden_node_is_rejected() {
        let error = mask(
            BTreeMap::from([("v1", nodes(&["c1"]))]),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::from([resource_arc("v1", "a1", "c1", "c2")]),
        )
        .expect_err("必选弧落在被禁节点必须被拒绝 / a required arc on a forbidden node must be rejected");
        assert_validation_error(&error, "必选弧资源同时被禁止");
    }

    #[test]
    fn required_arc_conflicting_with_a_required_resource_is_rejected() {
        let error = mask(
            BTreeMap::new(),
            BTreeMap::from([(node("c1"), "v2")]),
            BTreeSet::new(),
            BTreeSet::from([resource_arc("v1", "a1", "c1", "c2")]),
        )
        .expect_err("必选弧与必选资源冲突必须被拒绝 / a required arc conflicting with a required resource must be rejected");
        assert_validation_error(&error, "存在冲突必选车辆类型");
    }

    #[test]
    fn required_resource_on_a_forbidden_node_is_rejected() {
        let error = mask(
            BTreeMap::from([("v1", nodes(&["c1"]))]),
            BTreeMap::from([(node("c1"), "v1")]),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .expect_err("必选资源落在被禁节点必须被拒绝 / a required resource on a forbidden node must be rejected");
        assert_validation_error(&error, "必选资源同时被禁止");
    }

    // ========================================================================
    // 路线兼容累积 / Route compatibility
    // ========================================================================

    #[test]
    fn route_compatibility_requires_depot_endpoints_and_aligned_arc_ids() {
        let mask = empty_mask();
        let stops = vec![node("start"), node("c1"), node("end")];

        assert!(mask.is_route_compatible(&"v1", &stops, &[]));
        assert!(!mask.is_route_compatible(&"v1", &stops[..2], &[]));
        assert!(!mask.is_route_compatible(&"v1", &[node("c1"), node("end")], &[]));
        assert!(!mask.is_route_compatible(&"v1", &stops, &[arc_id("a1")]));

        let derived = vec![
            default_arc_id(&stops[0], &stops[1]),
            default_arc_id(&stops[1], &stops[2]),
        ];
        assert!(mask.is_route_compatible(&"v1", &stops, &derived));
    }

    #[test]
    fn route_compatibility_rejects_arcs_blocked_by_the_mask() {
        let mask = mask(
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::from([resource_arc("v1", "a1", "start", "c1")]),
            BTreeSet::new(),
        )
        .expect("合法遮罩 / valid mask");
        let stops = vec![node("start"), node("c1"), node("end")];
        let arc_ids = vec![arc_id("a1"), arc_id("a2")];

        assert!(!mask.is_route_compatible(&"v1", &stops, &arc_ids));
        assert!(mask.is_route_compatible(&"v2", &stops, &arc_ids));
        assert!(mask.is_route_compatible(&"v1", &stops, &[]));
    }
}
