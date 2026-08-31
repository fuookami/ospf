// TailBinAssignmentConstraint - 尾箱赋值约束 / Tail bin assignment constraint
// ============================================================================

/// 尾箱赋值约束 / Tail bin assignment constraint
///
/// 确保尾箱（最后一个使用的箱）在箱序列中排在最后。
/// 对于已使用的箱 b_i 和 b_j (i < j)，如果 b_j 未使用，则 b_i 必须未使用：
/// - `v[b_j] <= v[b_i]` for all i < j
///
/// Ensures the tail bin (last used bin) appears last in the bin sequence.
/// For used bins b_i and b_j (i < j), if b_j is unused, b_i must be unused:
/// - `v[b_j] <= v[b_i]` for all i < j
#[derive(Debug)]
pub struct TailBinAssignmentConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 箱使用标记变量索引 / Bin usage marker variable indices
    pub v_indices: Vec<usize>,
}

impl TailBinAssignmentConstraint {
    /// 创建尾箱赋值约束 / Create tail bin assignment constraint
    pub fn new(v_indices: Vec<usize>) -> Self {
        Self {
            name: "tail_bin_assignment_constraint".to_string(),
            group: None,
            v_indices,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TailBinAssignmentConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        // v[b_j] <= v[b_i] => v[b_j] - v[b_i] <= 0
        for i in 0..self.v_indices.len() {
            for j in (i + 1)..self.v_indices.len() {
                let v_i = self.v_indices[i];
                let v_j = self.v_indices[j];
                if let Err(e) = model.add_le_constraint(
                    &[(v_j, 1.0), (v_i, -1.0)],
                    0.0,
                    &format!("{}_{}_{}", self.name, i, j),
                ) {
                    log::warn!("Failed to register {}_{}_{}: {:?}", self.name, i, j, e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

