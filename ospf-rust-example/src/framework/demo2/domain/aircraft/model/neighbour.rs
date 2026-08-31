//! 邻接关系定义 / Neighbour relationship definitions
/// 邻接类型 / Neighbour type (对齐 Kotlin NeighbourType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeighbourType {
    /// 物理邻接 / Physical adjacency
    Physics,
    /// 间接物理邻接 / Indirect physical adjacency
    IndirectPhysics,
    /// 线性装载顺序 / Linear loading order
    LinearLoadingOrder,
    /// 拓扑装载顺序 / Topological loading order
    TopologicalLoadingOrder,
}

impl NeighbourType {
    /// 是否为有序邻接类型 / Whether this is an ordered neighbour type
    pub fn ordered(&self) -> bool {
        matches!(self, NeighbourType::LinearLoadingOrder | NeighbourType::TopologicalLoadingOrder)
    }
}

/// 邻接关系 / Neighbour
#[derive(Debug, Clone)]
pub struct Neighbour {
    /// 起始舱位 / From position
    pub from: String,
    /// 目标舱位 / To position
    pub to: String,
    /// 邻接类型 / Neighbour type
    pub neighbour_type: NeighbourType,
}
