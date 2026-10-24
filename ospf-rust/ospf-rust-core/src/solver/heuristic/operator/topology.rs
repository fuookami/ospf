//! 拓扑算子 / Topology operators

use crate::solver::heuristic::entropy::Entropy;

use super::unit;

/// 拓扑算子契约 / Topology contract
///
/// 定义个体之间的邻域关系；随机拓扑需要消费熵源，因此熵源作为参数传入而不是被拓扑持有。
/// Defines the neighbourhood relation between individuals. Random topologies consume entropy, so the entropy source is
/// passed in rather than owned by the topology.
pub trait Topology {
    /// 获取邻居下标集合 / Get the neighbour index set
    fn neighbours(&self, index: usize, size: usize, entropy: &mut dyn Entropy) -> Vec<usize>;
}

/// 全局拓扑：任意个体都能看到所有个体 / Global topology where every individual sees everyone
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalTopology;

impl Topology for GlobalTopology {
    fn neighbours(&self, _index: usize, size: usize, _entropy: &mut dyn Entropy) -> Vec<usize> {
        (0..size).collect()
    }
}

/// 环形拓扑：只与最近邻相连 / Ring topology connecting only adjacent individuals
#[derive(Debug, Clone, Copy, Default)]
pub struct RingTopology;

impl Topology for RingTopology {
    fn neighbours(&self, index: usize, size: usize, _entropy: &mut dyn Entropy) -> Vec<usize> {
        if size <= 1 {
            return vec![index.min(size.saturating_sub(1))];
        }
        let mut result = vec![(index + size - 1) % size, index, (index + 1) % size];
        result.sort_unstable();
        result.dedup();
        result
    }
}

/// k 邻环形拓扑 / k-neighbour ring topology
#[derive(Debug, Clone, Copy)]
pub struct RingKTopology {
    /// 邻域半径 / Neighbourhood radius
    pub radius: usize,
}

impl RingKTopology {
    /// 创建 k 邻环形拓扑 / Create a k-neighbour ring topology
    pub fn new(radius: usize) -> Self {
        Self { radius }
    }
}

impl Topology for RingKTopology {
    fn neighbours(&self, index: usize, size: usize, _entropy: &mut dyn Entropy) -> Vec<usize> {
        if size <= 1 {
            return vec![0];
        }
        let mut result = Vec::with_capacity(self.radius * 2 + 1);
        let radius = self.radius as isize;
        for offset in -radius..=radius {
            let neighbour = (index as isize + offset).rem_euclid(size as isize) as usize;
            result.push(neighbour);
        }
        result.sort_unstable();
        result.dedup();
        result
    }
}

/// 冯·诺依曼网格拓扑 / Von Neumann grid topology
#[derive(Debug, Clone, Copy)]
pub struct VonNeumannTopology {
    /// 网格宽度 / Grid width
    pub width: usize,
}

impl VonNeumannTopology {
    /// 创建网格拓扑 / Create a grid topology
    pub fn new(width: usize) -> Self {
        Self { width }
    }
}

impl Topology for VonNeumannTopology {
    fn neighbours(&self, index: usize, size: usize, _entropy: &mut dyn Entropy) -> Vec<usize> {
        if size <= 1 || self.width == 0 {
            return vec![index.min(size.saturating_sub(1))];
        }
        let row = index / self.width;
        let column = index % self.width;
        let mut result = Vec::with_capacity(5);
        for (row_offset, column_offset) in
            [(0_isize, 0_isize), (-1, 0), (1, 0), (0, -1), (0, 1)]
        {
            let neighbour_row = row as isize + row_offset;
            let neighbour_column = column as isize + column_offset;
            if neighbour_row < 0
                || neighbour_column < 0
                || neighbour_column >= self.width as isize
            {
                continue;
            }
            let neighbour = neighbour_row as usize * self.width + neighbour_column as usize;
            if neighbour < size {
                result.push(neighbour);
            }
        }
        result.sort_unstable();
        result.dedup();
        result
    }
}

/// 随机拓扑，每次查询返回随机的若干邻居 / Random topology returning random neighbours per query
#[derive(Debug, Clone, Copy)]
pub struct RandomTopology {
    /// 邻居数量 / Neighbour amount
    pub amount: usize,
}

impl RandomTopology {
    /// 创建随机拓扑 / Create a random topology
    pub fn new(amount: usize) -> Self {
        Self { amount }
    }
}

impl Topology for RandomTopology {
    fn neighbours(&self, index: usize, size: usize, entropy: &mut dyn Entropy) -> Vec<usize> {
        if size <= 1 {
            return vec![0];
        }
        let mut result = vec![index];
        let target = self.amount.min(size).max(1);
        let mut attempts = 0;
        while result.len() < target && attempts < size * 4 {
            let candidate = ((unit(entropy) * size as f64) as usize).min(size - 1);
            if !result.contains(&candidate) {
                result.push(candidate);
            }
            attempts += 1;
        }
        result.sort_unstable();
        result
    }
}
