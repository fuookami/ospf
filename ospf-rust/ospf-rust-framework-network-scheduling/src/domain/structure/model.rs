//! 根有向树及独立设施分区归属。 / Rooted directed trees and independent facility-partition ownership.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::design::{NetworkConstructionSolution, NetworkDesignProblem};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId};

/// 设施资格明确的分区覆盖请求。 / Partition coverage request with explicit facility eligibility.
#[derive(Debug, Clone)]
pub struct FacilityPartitionRequest {
    /// 稳定且唯一的分区 ID / Stable, unique partition ID.
    pub id: String,
    /// 可负责的设施节点；未声明为候选设施的节点固定启用 / Eligible facility nodes; nodes not declared as candidate facilities are fixed open.
    pub eligible_facilities: BTreeSet<NetworkNodeId>,
    /// 必须恰好归属一个设施 / Exactly one owner is required.
    pub required: bool,
}

/// 设施被分配的分区数量配额，与物理流量无关。 / Facility assignment-count quota independent of physical flows.
#[derive(Debug, Clone)]
pub struct FacilityAssignmentQuota {
    /// 设施节点 / Facility node.
    pub facility: NetworkNodeId,
    /// 最少归属数量；超过可分配数量仍是正常模型不可行 / Minimum count; exceeding assignable partitions is ordinary model infeasibility.
    pub minimum: usize,
    /// 可选最多归属数量 / Optional maximum count.
    pub maximum: Option<usize>,
}

/// 不可变根树与设施归属问题；不注册设计问题中的商品流。 / Immutable tree and facility-ownership problem; commodity flows in the design are not registered.
#[derive(Debug, Clone)]
pub struct NetworkStructureProblem<V: SolveValue + UnitConversionValue> {
    design: NetworkDesignProblem<V>,
    root: NetworkNodeId,
    terminals: BTreeSet<NetworkNodeId>,
    steiner_nodes: BTreeSet<NetworkNodeId>,
    tree_nodes: BTreeSet<NetworkNodeId>,
    partitions: Vec<FacilityPartitionRequest>,
    quotas: Vec<FacilityAssignmentQuota>,
}

impl<V: SolveValue + UnitConversionValue> NetworkStructureProblem<V> {
    /// 校验引用、重复定义与配额；合法断联和无合格所有者留给模型判定。 / Validate references, duplicates and quotas; leave valid disconnection and missing eligible owners to model feasibility.
    pub fn new(
        design: NetworkDesignProblem<V>,
        root: NetworkNodeId,
        terminals: BTreeSet<NetworkNodeId>,
        steiner_nodes: BTreeSet<NetworkNodeId>,
        partitions: Vec<FacilityPartitionRequest>,
        quotas: Vec<FacilityAssignmentQuota>,
    ) -> Result<Self> {
        let tree_nodes = std::iter::once(root.clone()).chain(terminals.iter().cloned())
            .chain(steiner_nodes.iter().cloned()).collect::<BTreeSet<_>>();
        if tree_nodes.len() > u32::MAX as usize || tree_nodes.iter().any(|id| id.as_str().trim().is_empty() || design.flow().node(id).is_none()) {
            return Err(NetworkSchedulingError::structure("根、终端或中转节点未知或数量超限 / unknown root, terminal or relay, or excessive node count"));
        }
        let mut ids = BTreeSet::new();
        for partition in &partitions {
            if partition.id.trim().is_empty() || !ids.insert(partition.id.clone())
                || partition.eligible_facilities.iter().any(|id| design.flow().node(id).is_none())
            {
                return Err(NetworkSchedulingError::validation("分区 ID 非法、重复或引用未知设施节点 / invalid or duplicate partition ID or unknown facility node"));
            }
        }
        let mut facilities = BTreeSet::new();
        for quota in &quotas {
            if !facilities.insert(quota.facility.clone()) || design.flow().node(&quota.facility).is_none()
                || quota.maximum.is_some_and(|maximum| maximum < quota.minimum)
                || quota.minimum > u32::MAX as usize || quota.maximum.is_some_and(|maximum| maximum > u32::MAX as usize)
            {
                return Err(NetworkSchedulingError::validation("设施配额重复、非法或引用未知节点 / duplicate or invalid facility quota or unknown node"));
            }
        }
        Ok(Self { design, root, terminals, steiner_nodes, tree_nodes, partitions, quotas })
    }

    /// 只读建设问题 / Read-only construction problem.
    pub fn design(&self) -> &NetworkDesignProblem<V> { &self.design }

    /// 必选根节点 / Required root node.
    pub fn root(&self) -> &NetworkNodeId { &self.root }

    /// 必须由根可达的终端 / Terminals that must be reachable from the root.
    pub fn terminals(&self) -> &BTreeSet<NetworkNodeId> { &self.terminals }

    /// 可选中转候选；根和终端始终必选 / Optional relay candidates; root and terminals remain mandatory.
    pub fn steiner_nodes(&self) -> &BTreeSet<NetworkNodeId> { &self.steiner_nodes }

    /// 根、终端和中转节点的去重集合 / Deduplicated root, terminal and relay nodes.
    pub fn tree_nodes(&self) -> &BTreeSet<NetworkNodeId> { &self.tree_nodes }

    /// 独立于树流的分区覆盖请求 / Partition requests independent of tree flows.
    pub fn partitions(&self) -> &[FacilityPartitionRequest] { &self.partitions }

    /// 设施归属数量配额 / Facility assignment-count quotas.
    pub fn quotas(&self) -> &[FacilityAssignmentQuota] { &self.quotas }
}

/// 经生产校验的建设、树与分区归属结果。 / Production-validated construction, tree and partition-ownership result.
#[derive(Debug, Clone)]
pub struct NetworkStructureSolution<V> {
    /// 纯建设选择与费用 / Construction-only choices and cost.
    pub construction: NetworkConstructionSolution<V>,
    /// 稳定 ID 标识的选中树弧 / Selected tree arcs identified by stable IDs.
    pub selected_tree_arcs: BTreeSet<NetworkArcId>,
    /// 根树中的全部节点 / All nodes in the rooted tree.
    pub connected_nodes: BTreeSet<NetworkNodeId>,
    /// 每个非根活动节点的唯一父弧 / Unique parent arc for each active non-root node.
    pub parent_arcs: BTreeMap<NetworkNodeId, NetworkArcId>,
    /// 已归属分区的合格设施 / Eligible owner of each assigned partition.
    pub partition_owners: BTreeMap<String, NetworkNodeId>,
}
