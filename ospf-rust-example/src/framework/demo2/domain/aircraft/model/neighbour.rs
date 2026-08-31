/// 邻接类型 / Neighbour type (对齐 Kotlin NeighbourType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeighbourType {
    Physics,
    IndirectPhysics,
    LinearLoadingOrder,
    TopologicalLoadingOrder,
}

impl NeighbourType {
    pub fn ordered(&self) -> bool {
        matches!(self, NeighbourType::LinearLoadingOrder | NeighbourType::TopologicalLoadingOrder)
    }
}

/// 邻接关系 / Neighbour
#[derive(Debug, Clone)]
pub struct Neighbour {
    pub from: String,
    pub to: String,
    pub neighbour_type: NeighbourType,
}
