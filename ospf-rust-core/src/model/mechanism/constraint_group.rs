//! 约束组定义
//! Constraint Group Definition

/// 约束组 / Constraint Group
///
/// 用于批量管理约束。
/// For batch management of constraints.
#[derive(Debug)]
pub struct ConstraintGroup {
    /// 唯一标识符 / Unique identifier
    pub id: u64,
    /// 名称 / Name
    pub name: String,
    /// 约束数量 / Constraint count
    pub count: usize,
}

impl ConstraintGroup {
    /// 创建新约束组 / Create new constraint group
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            count: 0,
        }
    }

    /// 增加约束数量 / Increment constraint count
    pub fn increment(&mut self) {
        self.count += 1;
    }
}

impl Clone for ConstraintGroup {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            count: self.count,
        }
    }
}

impl PartialEq for ConstraintGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ConstraintGroup {}

impl std::hash::Hash for ConstraintGroup {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
