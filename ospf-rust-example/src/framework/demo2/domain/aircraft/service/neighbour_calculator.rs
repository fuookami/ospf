//! 邻接关系计算器 / Neighbour relationship calculator
use crate::framework::demo2::domain::aircraft::model::{Neighbour, NeighbourType, Position};

/// 邻接计算器 / Neighbour calculator
/// 对齐 Kotlin NeighbourCalculator
pub struct NeighbourCalculator;

impl NeighbourCalculator {
    /// 计算物理邻接关系 / Calculate physics neighbours
    /// 对齐 Kotlin NeighbourCalculator.calculatePhysics
    pub fn calculate_physics(positions: &[Position]) -> Vec<Neighbour> {
        let mut neighbours = Vec::new();
        // 物理邻接: 同一甲板上相邻的位置
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                // 如果两个位置在同一甲板且距离较近，则为物理邻接
                let pos_i = &positions[i];
                let pos_j = &positions[j];
                let long_diff = (pos_i.coordinate.longitudinal_arm() - pos_j.coordinate.longitudinal_arm()).abs();
                let lat_diff = (pos_i.coordinate.lateral_arm() - pos_j.coordinate.lateral_arm()).abs();
                if long_diff < 2.0 && lat_diff < 2.0 {
                    neighbours.push(Neighbour {
                        from: pos_i.id.clone(),
                        to: pos_j.id.clone(),
                        neighbour_type: NeighbourType::Physics,
                    });
                }
            }
        }
        neighbours
    }

    /// 计算间接物理邻接关系 / Calculate indirect physics neighbours
    /// 对齐 Kotlin NeighbourCalculator.calculateIndirectPhysics
    pub fn calculate_indirect_physics(positions: &[Position]) -> Vec<Neighbour> {
        let mut neighbours = Vec::new();
        // 间接物理邻接: 通过一个中间位置连接的两个位置
        let physics = Self::calculate_physics(positions);
        for i in 0..physics.len() {
            for j in (i + 1)..physics.len() {
                if physics[i].to == physics[j].from || physics[i].from == physics[j].to {
                    neighbours.push(Neighbour {
                        from: physics[i].from.clone(),
                        to: physics[j].to.clone(),
                        neighbour_type: NeighbourType::IndirectPhysics,
                    });
                }
            }
        }
        neighbours
    }

    /// 计算线性装载顺序邻接关系 / Calculate linear loading order neighbours
    /// 对齐 Kotlin NeighbourCalculator.calculateLinearLoadingOrder
    pub fn calculate_linear_loading_order(positions: &[Position]) -> Vec<Neighbour> {
        let mut neighbours = Vec::new();
        // 线性装载顺序: 按 loading_order 排序后相邻的位置
        let mut sorted_positions: Vec<&Position> = positions.iter().collect();
        sorted_positions.sort_by_key(|p| p.loading_order);
        for i in 0..sorted_positions.len().saturating_sub(1) {
            neighbours.push(Neighbour {
                from: sorted_positions[i].id.clone(),
                to: sorted_positions[i + 1].id.clone(),
                neighbour_type: NeighbourType::LinearLoadingOrder,
            });
        }
        neighbours
    }

    /// 计算拓扑装载顺序邻接关系 / Calculate topological loading order neighbours
    /// 对齐 Kotlin NeighbourCalculator.calculateTopologicalLoadingOrder
    pub fn calculate_topological_loading_order(positions: &[Position]) -> Vec<Neighbour> {
        let mut neighbours = Vec::new();
        // 拓扑装载顺序: 基于位置的空间关系
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let pos_i = &positions[i];
                let pos_j = &positions[j];
                // 如果位置在同一行（lateral_arm 相近），则按 longitudinal_arm 排序
                let lat_diff = (pos_i.coordinate.lateral_arm() - pos_j.coordinate.lateral_arm()).abs();
                if lat_diff < 0.5 {
                    let (from, to) = if pos_i.coordinate.longitudinal_arm() < pos_j.coordinate.longitudinal_arm() {
                        (pos_i.id.clone(), pos_j.id.clone())
                    } else {
                        (pos_j.id.clone(), pos_i.id.clone())
                    };
                    neighbours.push(Neighbour {
                        from,
                        to,
                        neighbour_type: NeighbourType::TopologicalLoadingOrder,
                    });
                }
            }
        }
        neighbours
    }
}
