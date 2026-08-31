//! 变量 ID 定义
//! Variable ID Definitions

use once_cell::sync::Lazy;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// VariableId - 变量唯一标识符
// ============================================================================

/// 变量 ID / Variable ID
///
/// 变量的唯一标识符，由两部分组成：
/// Unique identifier for a variable, composed of two parts:
///
/// - `group_id`: 由 ID 生成器递增生成的组 ID
/// - `index_in_group`: 在变量组中的序号（独立变量为 0）
///
/// - `group_id`: Group ID incremented by ID generator
/// - `index_in_group`: Index within the variable group (0 for standalone variables)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariableId {
    /// 组 ID（由 ID 生成器递增生成）/ Group ID (incremented by ID generator)
    pub group_id: usize,
    /// 组内索引 / Index within group
    pub index_in_group: usize,
}

impl VariableId {
    /// 创建新的变量 ID / Create new variable ID
    pub fn new(group_id: usize, index_in_group: usize) -> Self {
        Self {
            group_id,
            index_in_group,
        }
    }

    /// 创建独立变量 ID / Create standalone variable ID
    ///
    /// 独立变量的 `index_in_group` 恒为 0。
    /// Standalone variables always have `index_in_group` equal to 0.
    pub fn standalone(group_id: usize) -> Self {
        Self {
            group_id,
            index_in_group: 0,
        }
    }

    /// 检查是否为独立变量 / Check if standalone variable
    pub fn is_standalone(&self) -> bool {
        self.index_in_group == 0
    }

    /// 获取唯一数值 ID / Get unique numeric ID
    ///
    /// 将组 ID 和组内索引组合为单一数值。
    /// Combines group ID and index into a single numeric value.
    pub fn unique_id(&self) -> u64 {
        // 将 group_id 和 index_in_group 编码为单个 u64
        // Encode group_id and index_in_group into a single u64
        // 使用高 32 位存储 group_id，低 32 位存储 index_in_group
        // Use high 32 bits for group_id, low 32 bits for index_in_group
        ((self.group_id as u64) << 32) | (self.index_in_group as u64)
    }

    /// 从唯一数值 ID 解析 / Parse from unique numeric ID
    pub fn from_unique_id(id: u64) -> Self {
        Self {
            group_id: (id >> 32) as usize,
            index_in_group: (id & 0xFFFFFFFF) as usize,
        }
    }
}

impl fmt::Display for VariableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_standalone() {
            write!(f, "Var({})", self.group_id)
        } else {
            write!(f, "Var({}:{})", self.group_id, self.index_in_group)
        }
    }
}

// ============================================================================
// VariableIdGenerator - 变量 ID 生成器
// ============================================================================

/// 变量 ID 生成器 / Variable ID Generator
///
/// 用于生成唯一的变量组 ID。
/// Used to generate unique variable group IDs.
#[derive(Debug)]
pub struct VariableIdGenerator {
    next_id: AtomicUsize,
}

impl VariableIdGenerator {
    /// 创建新的 ID 生成器 / Create new ID generator
    pub fn new() -> Self {
        Self {
            next_id: AtomicUsize::new(0),
        }
    }

    /// 创建带起始 ID 的生成器 / Create generator with starting ID
    pub fn with_start(start: usize) -> Self {
        Self {
            next_id: AtomicUsize::new(start),
        }
    }

    /// 生成下一个组 ID / Generate next group ID
    ///
    /// 返回一个新的唯一组 ID。
    /// Returns a new unique group ID.
    pub fn next_group_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// 生成独立变量 ID / Generate standalone variable ID
    pub fn next_standalone_id(&self) -> VariableId {
        VariableId::standalone(self.next_group_id())
    }

    /// 生成变量组 ID 列表 / Generate variable group ID list
    ///
    /// 生成一个变量组中所有变量的 ID。
    /// Generates IDs for all variables in a group.
    pub fn next_group_ids(&self, count: usize) -> Vec<VariableId> {
        let group_id = self.next_group_id();
        (0..count).map(|i| VariableId::new(group_id, i)).collect()
    }

    /// 获取当前计数 / Get current count
    pub fn current_count(&self) -> usize {
        self.next_id.load(Ordering::SeqCst)
    }

    /// 重置计数器 / Reset counter
    pub fn reset(&self) {
        self.next_id.store(0, Ordering::SeqCst);
    }
}

impl Default for VariableIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for VariableIdGenerator {
    fn clone(&self) -> Self {
        Self {
            next_id: AtomicUsize::new(self.next_id.load(Ordering::SeqCst)),
        }
    }
}

// ============================================================================
// 全局变量 ID 生成器 / Global Variable ID Generator
// ============================================================================

/// 全局变量 ID 生成器 / Global variable ID generator
///
/// 用于全局变量 ID 分配。
/// Used for global variable ID allocation.
pub static GLOBAL_VARIABLE_ID_GENERATOR: Lazy<VariableIdGenerator> =
    Lazy::new(VariableIdGenerator::new);

/// 生成新的独立变量 ID / Generate new standalone variable ID
pub fn new_standalone_id() -> VariableId {
    GLOBAL_VARIABLE_ID_GENERATOR.next_standalone_id()
}

/// 生成新的组 ID / Generate new group ID
///
/// 返回一个唯一的组 ID，可用于创建 VariableCombination。
/// Returns a unique group ID that can be used to create a VariableCombination.
pub fn new_group_id() -> usize {
    GLOBAL_VARIABLE_ID_GENERATOR.next_group_id()
}

/// 生成新的变量组 ID / Generate new variable group IDs
pub fn new_group_ids(count: usize) -> Vec<VariableId> {
    GLOBAL_VARIABLE_ID_GENERATOR.next_group_ids(count)
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_id_creation() {
        let id = VariableId::new(1, 0);
        assert_eq!(id.group_id, 1);
        assert_eq!(id.index_in_group, 0);
        assert!(id.is_standalone());
    }

    #[test]
    fn test_standalone_id() {
        let id = VariableId::standalone(5);
        assert_eq!(id.group_id, 5);
        assert_eq!(id.index_in_group, 0);
        assert!(id.is_standalone());
    }

    #[test]
    fn test_unique_id() {
        let id = VariableId::new(1, 2);
        let unique = id.unique_id();
        let parsed = VariableId::from_unique_id(unique);
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_id_generator() {
        let generator = VariableIdGenerator::new();
        let id1 = generator.next_standalone_id();
        let id2 = generator.next_standalone_id();
        assert_ne!(id1, id2);
        assert!(id1.group_id < id2.group_id);
    }

    #[test]
    fn test_group_ids() {
        let generator = VariableIdGenerator::new();
        let ids = generator.next_group_ids(5);
        assert_eq!(ids.len(), 5);
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(id.index_in_group, i);
        }
    }

    #[test]
    fn test_display() {
        let id1 = VariableId::standalone(1);
        assert_eq!(format!("{}", id1), "Var(1)");

        let id2 = VariableId::new(1, 2);
        assert_eq!(format!("{}", id2), "Var(1:2)");
    }
}
