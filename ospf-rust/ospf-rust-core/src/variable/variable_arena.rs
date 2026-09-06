//! 变量 Arena 定义
//! Variable Arena Definitions

use super::{VariableData, VariableItem, VariableRange, VariableTypeTrait, new_standalone_id};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;
use typed_arena::Arena;

// ============================================================================
// Flt64VariableArena - 变量 Arena
// ============================================================================

/// 变量 Arena / Variable Arena
///
/// 用于批量分配变量数据，减少内存碎片和分配开销。
/// Used for batch allocation of variable data, reducing memory fragmentation and allocation overhead.
///
/// `typed_arena` 提供类型化的 Arena 分配，比通用 Arena 更高效。
/// `typed_arena` provides typed arena allocation, more efficient than generic arenas.
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::variable::{Flt64VariableArena, Flt64VariableData, VariableType, VariableRange};
///
/// let arena = Flt64VariableArena::new();
///
/// // 批量创建变量 / Batch create variables
/// let vars: Vec<_> = (0..1000)
///     .map(|i| arena.alloc(Flt64VariableData {
///         id: i as u64,
///         index: i,
///         name: format!("x_{}", i),
///         display_name: None,
///         var_type: VariableType::Continuous,
///         range: VariableRange::bounded(0.0, 100.0),
///     }))
///     .collect();
/// ```
pub struct Flt64VariableArena {
    /// 类型化 Arena 分配器 / Typed arena allocator
    arena: Arena<Flt64VariableData>,
}

/// 变量数据（Flt64 专用版本）/ Variable Data (Flt64-specific version)
#[derive(Debug, Clone)]
pub struct Flt64VariableData {
    /// 唯一标识符 / Unique identifier
    pub id: u64,
    /// 索引 / Index
    pub index: usize,
    /// 名称 / Name
    pub name: String,
    /// 显示名称 / Display name
    pub display_name: Option<String>,
    /// 变量类型 / Variable type
    pub var_type: super::VariableType,
    /// 变量范围 / Variable range
    pub range: super::VariableRange<f64>,
}

impl Flt64VariableArena {
    /// 创建新的 Arena / Create new arena
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }

    /// 创建带预留容量的 Arena / Create arena with reserved capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
        }
    }

    /// 在 Arena 中分配变量数据 / Allocate variable data in arena
    ///
    /// 返回的 `Flt64VariableItem` 持有 Arena 中数据的引用。
    /// The returned `Flt64VariableItem` holds a reference to data in the arena.
    pub fn alloc(&self, data: Flt64VariableData) -> Flt64VariableItem {
        Flt64VariableItem {
            data: Arc::new(self.arena.alloc(data).clone()),
        }
    }

    /// 自动分配独立变量（使用全局递增 ID）/ Auto-allocate standalone variable with global incremental ID
    pub fn alloc_auto(
        &self,
        name: &str,
        var_type: super::VariableType,
        range: VariableRange<f64>,
    ) -> Flt64VariableItem {
        let id = new_standalone_id();
        self.alloc(Flt64VariableData {
            id: id.unique_id(),
            index: id.index_in_group,
            name: name.to_string(),
            display_name: None,
            var_type,
            range,
        })
    }

    /// 自动分配带显示名称的独立变量（使用全局递增 ID）/ Auto-allocate standalone variable with display name and global incremental ID
    pub fn alloc_auto_with_display_name(
        &self,
        name: &str,
        display_name: &str,
        var_type: super::VariableType,
        range: VariableRange<f64>,
    ) -> Flt64VariableItem {
        let id = new_standalone_id();
        self.alloc(Flt64VariableData {
            id: id.unique_id(),
            index: id.index_in_group,
            name: name.to_string(),
            display_name: Some(display_name.to_string()),
            var_type,
            range,
        })
    }

    /// 批量分配变量 / Batch allocate variables
    ///
    /// 使用 Arena 分配器批量创建变量，性能更优。
    /// Batch create variables using arena allocator for better performance.
    pub fn alloc_iter<I: IntoIterator<Item = Flt64VariableData>>(
        &self,
        iter: I,
    ) -> Vec<Flt64VariableItem> {
        iter.into_iter().map(|data| self.alloc(data)).collect()
    }

    /// 预留容量 / Reserve capacity
    ///
    /// 预留指定数量的元素空间。
    /// Reserves space for the specified number of elements.
    pub fn reserve(&mut self, additional: usize) {
        self.arena.reserve_extend(additional);
    }

    /// 获取已分配数量 / Get allocated count
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.arena.len() == 0
    }
}

impl Default for Flt64VariableArena {
    fn default() -> Self {
        Self::new()
    }
}

/// 变量项（非泛型版本）/ Variable Item (non-generic version)
#[derive(Debug, Clone)]
pub struct Flt64VariableItem {
    /// 指向实际数据的 Arc 指针 / Arc pointer to actual data
    data: Arc<Flt64VariableData>,
}

impl Flt64VariableItem {
    /// 获取唯一标识符 / Get unique identifier
    pub fn id(&self) -> u64 {
        self.data.id
    }

    /// 获取索引 / Get index
    pub fn index(&self) -> usize {
        self.data.index
    }

    /// 获取名称 / Get name
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// 获取显示名称 / Get display name
    pub fn display_name(&self) -> Option<&str> {
        self.data.display_name.as_deref()
    }

    /// 获取变量类型 / Get variable type
    pub fn var_type(&self) -> super::VariableType {
        self.data.var_type
    }

    /// 获取变量范围 / Get variable range
    pub fn range(&self) -> &super::VariableRange<f64> {
        &self.data.range
    }
}

// ============================================================================
// Flt64VariableArena - 泛型变量 Arena
// ============================================================================

/// 泛型变量 Arena / Generic Variable Arena
///
/// 支持任意类型变量的 Arena 分配器。
/// Arena allocator supporting arbitrary variable types.
///
/// # 类型参数 / Type Parameters
///
/// - `VT`: 变量类型标记（实现 `VariableTypeTrait`）
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::variable::{VariableArena, Binary, VariableData, new_standalone_id};
///
/// let arena: VariableArena<Binary> = VariableArena::new();
///
/// // 创建二进制变量 / Create binary variable
/// let binary_var = arena.alloc(VariableData::<Binary>::new(
///     new_standalone_id(),
///     "x",
/// ));
/// ```
pub struct VariableArena<VT: VariableTypeTrait> {
    arena: Arena<VariableData<VT>>,
    _marker: PhantomData<VT>,
}

impl<VT: VariableTypeTrait> VariableArena<VT> {
    /// 创建新的 Arena / Create new arena
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            _marker: PhantomData,
        }
    }

    /// 创建带预留容量的 Arena / Create arena with reserved capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
            _marker: PhantomData,
        }
    }

    /// 在 Arena 中分配变量数据 / Allocate variable data in arena
    pub fn alloc(&self, data: VariableData<VT>) -> VariableItem<VT> {
        VariableItem::new(self.arena.alloc(data).clone())
    }

    /// 自动分配独立变量（使用全局递增 ID）/ Auto-allocate standalone variable with global incremental ID
    pub fn alloc_auto(&self, name: &str) -> VariableItem<VT> {
        self.alloc(VariableData::new(new_standalone_id(), name))
    }

    /// 自动分配带范围的独立变量（使用全局递增 ID）/ Auto-allocate ranged standalone variable with global incremental ID
    pub fn alloc_auto_with_range(
        &self,
        name: &str,
        range: VariableRange<VT::Value>,
    ) -> VariableItem<VT> {
        self.alloc(VariableData::with_range(new_standalone_id(), name, range))
    }

    /// 自动分配带显示名称的独立变量（使用全局递增 ID）/ Auto-allocate standalone variable with display name and global incremental ID
    pub fn alloc_auto_with_display_name(&self, name: &str, display_name: &str) -> VariableItem<VT> {
        self.alloc(VariableData::with_display_name(
            new_standalone_id(),
            name,
            display_name,
        ))
    }

    /// 批量分配变量 / Batch allocate variables
    pub fn alloc_iter<I: IntoIterator<Item = VariableData<VT>>>(
        &self,
        iter: I,
    ) -> Vec<VariableItem<VT>> {
        iter.into_iter().map(|data| self.alloc(data)).collect()
    }

    /// 预留容量 / Reserve capacity
    pub fn reserve(&mut self, additional: usize) {
        self.arena.reserve_extend(additional);
    }

    /// 获取已分配数量 / Get allocated count
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.arena.len() == 0
    }
}

impl<VT: VariableTypeTrait> Default for VariableArena<VT> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ConcurrentVariableArena - 线程安全的变量 Arena
// ============================================================================

/// 线程安全的变量 Arena / Thread-safe variable arena
///
/// 支持多线程并发分配（内部使用 `RefCell`）。
/// Supports multi-threaded concurrent allocation (uses `RefCell` internally).
///
/// # 注意 / Note
///
/// 此类型不是线程安全的，不应跨线程共享。
/// 如果需要跨线程使用，请使用外部同步机制。
///
/// This type is not thread-safe and should not be shared across threads.
/// If cross-thread usage is needed, use external synchronization.
pub struct ConcurrentVariableArena {
    arena: RefCell<Arena<Flt64VariableData>>,
}

impl ConcurrentVariableArena {
    /// 创建新的 Arena / Create new arena
    pub fn new() -> Self {
        Self {
            arena: RefCell::new(Arena::new()),
        }
    }

    /// 创建带预留容量的 Arena / Create arena with reserved capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: RefCell::new(Arena::with_capacity(capacity)),
        }
    }

    /// 在 Arena 中分配变量数据 / Allocate variable data in arena
    pub fn alloc(&self, data: Flt64VariableData) -> Flt64VariableItem {
        Flt64VariableItem {
            data: Arc::new(self.arena.borrow_mut().alloc(data).clone()),
        }
    }

    /// 批量分配变量 / Batch allocate variables
    pub fn alloc_iter<I: IntoIterator<Item = Flt64VariableData>>(
        &self,
        iter: I,
    ) -> Vec<Flt64VariableItem> {
        iter.into_iter().map(|data| self.alloc(data)).collect()
    }
}

impl Default for ConcurrentVariableArena {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// 二进制变量 Arena / Binary variable arena
pub type BinaryArena = VariableArena<super::Binary>;

/// 连续变量 Arena / Continuous variable arena
pub type ContinuousArena = VariableArena<super::Continuous>;

/// 整数变量 Arena / Integer variable arena
pub type IntegerArena = VariableArena<super::Integer>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::{
        Binary, BinaryCombination1D, Continuous, VariableCombination, VariableId, VariableRange,
        VariableType,
    };
    use ospf_rust_multiarray::Shape;

    #[test]
    fn test_variable_arena() {
        let arena = Flt64VariableArena::new();
        let data = Flt64VariableData {
            id: 0,
            index: 0,
            name: "x".to_string(),
            display_name: None,
            var_type: VariableType::Continuous,
            range: VariableRange::bounded(0.0, 100.0),
        };
        let item = arena.alloc(data);
        assert_eq!(item.name(), "x");
        assert_eq!(item.var_type(), VariableType::Continuous);
    }

    #[test]
    fn test_generic_variable_arena() {
        let arena = BinaryArena::new();
        let data = VariableData::<Binary>::new(VariableId::standalone(0), "x");
        let item = arena.alloc(data);
        assert_eq!(item.name(), "x");
        assert_eq!(item.var_type(), VariableType::Binary);
    }

    #[test]
    fn test_batch_allocation() {
        let arena = Flt64VariableArena::with_capacity(100);
        let items: Vec<_> = arena.alloc_iter((0..10).map(|i| Flt64VariableData {
            id: i as u64,
            index: i,
            name: format!("x_{}", i),
            display_name: None,
            var_type: VariableType::Continuous,
            range: VariableRange::bounded(0.0, 1.0),
        }));
        assert_eq!(items.len(), 10);
        assert_eq!(arena.len(), 10);
    }

    #[test]
    fn test_variable_arena_auto_allocates_unique_ids() {
        let arena = Flt64VariableArena::new();
        let x = arena.alloc_auto(
            "x_auto",
            VariableType::Continuous,
            VariableRange::bounded(0.0, 1.0),
        );
        let y = arena.alloc_auto(
            "y_auto",
            VariableType::Continuous,
            VariableRange::bounded(0.0, 1.0),
        );
        assert_ne!(x.id(), y.id());
        assert_eq!(x.index(), 0);
        assert_eq!(y.index(), 0);
    }

    #[test]
    fn test_generic_variable_arena_auto_allocates_unique_ids() {
        let arena = BinaryArena::new();
        let x = arena.alloc_auto("x_auto");
        let y = arena.alloc_auto("y_auto");
        assert_ne!(x.id(), y.id());
        assert!(x.id().is_standalone());
        assert!(y.id().is_standalone());
    }

    #[test]
    fn test_generic_variable_arena_auto_with_range_preserves_bounds() {
        let arena: VariableArena<Continuous> = VariableArena::new();
        let z = arena.alloc_auto_with_range("z_auto", VariableRange::bounded(-2.0, 3.0));
        assert_eq!(z.range().lower_bound, Some(-2.0));
        assert_eq!(z.range().upper_bound, Some(3.0));
    }

    #[test]
    fn test_generic_arena_and_combination_share_global_id_generator() {
        let arena = BinaryArena::new();
        let single_before = arena.alloc_auto("single_before");
        let combination: BinaryCombination1D = VariableCombination::new(Shape::new([2]), "combo");
        let single_after = arena.alloc_auto("single_after");

        let before_group = single_before.id().group_id;
        let combination_group = combination.group_id();
        let after_group = single_after.id().group_id;

        assert!(before_group < combination_group);
        assert!(combination_group < after_group);
    }
}
