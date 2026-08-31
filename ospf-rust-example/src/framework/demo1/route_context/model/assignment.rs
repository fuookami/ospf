/// 分配 / Assignment (对齐 Kotlin Assignment)
#[derive(Debug, Clone)]
pub struct Assignment {
    pub normal_node_indices: Vec<usize>,
    pub x_idx: Vec<Vec<usize>>,
}
