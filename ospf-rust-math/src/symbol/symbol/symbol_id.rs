//! 符号标识符定义
//! Symbol identifier definitions

use std::any::Any;
use std::fmt::Debug;
use std::hash::Hash;

// ============================================================================
// SymbolId Trait - 静态符号标识符
// ============================================================================

/// SymbolId - 静态符号标识符 trait
/// SymbolId - Static symbol identifier trait
///
/// 用于静态类型符号的标识符约束。
/// Constraint for static symbol identifiers.
///
/// # 要求 / Requirements
///
/// - `Clone`: 可克隆
/// - `Debug`: 可调试输出
/// - `Eq`: 可判等
/// - `Hash`: 可哈希（用于 HashMap/HashSet）
/// - `Any`: 可类型检查
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::SymbolId;
///
/// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// struct MyId(usize);
///
/// // MyId 自动实现 SymbolId
/// fn use_symbol_id<Id: SymbolId>(id: Id) {
///     // ...
/// }
/// ```
pub trait SymbolId: Clone + Debug + Eq + Hash + Any {}

// ============================================================================
// 为类型自动实现 SymbolId
// Auto-implement SymbolId for types
// ============================================================================

/// 为满足约束的类型自动实现 SymbolId
/// Auto-implement SymbolId for types satisfying constraints
impl<T> SymbolId for T where T: Clone + Debug + Eq + Hash + Any {}

// ============================================================================
// SymbolDynId - 动态符号标识符
// ============================================================================

/// SymbolDynId - 符号动态标识符
/// SymbolDynId - Symbol dynamic identifier
///
/// 用于在运行时标识符号，支持高维符号（如向量、矩阵）的分量。
/// Used to identify symbols at runtime, supports components of high-dimensional symbols.
///
/// # 字段说明 / Field Description
///
/// - `parent_type`: 父符号类型名（空字符串表示独立符号）
/// - `parent_id`: 父符号的唯一标识
/// - `index`: 在父符号中的序号（0 = 父符号本身或独立符号）
///
/// # 高维符号示例 / High-dimensional Symbol Example
///
/// | 符号 | `SymbolDynId` | 含义 |
/// |------|---------------|------|
/// | `x`（独立符号） | `("", 0, 0)` | 独立原子符号 |
/// | `v`（3维向量） | `("VectorSymbol", 1, 0)` | 向量本身 |
/// | `v[0]` | `("VectorSymbol", 1, 1)` | 向量第1个分量 |
/// | `v[1]` | `("VectorSymbol", 1, 2)` | 向量第2个分量 |
/// | `v[2]` | `("VectorSymbol", 1, 3)` | 向量第3个分量 |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymbolDynId<'a> {
    /// 父符号类型名 / Parent symbol type name
    /// 空字符串表示独立符号 / Empty string for standalone symbol
    pub parent_type: &'a str,

    /// 父符号唯一标识 / Parent symbol unique id
    pub parent_id: usize,

    /// 在父符号中的序号 / Index in parent symbol
    /// 0 = 父符号本身或独立符号 / 0 = parent itself or standalone
    pub index: usize,
}

impl<'a> SymbolDynId<'a> {
    /// 创建独立符号的标识符
    /// Create identifier for standalone symbol
    ///
    /// # 示例 / Examples
    ///
    /// ```
    /// use ospf_rust_math::symbol::SymbolDynId;
    ///
    /// let id = SymbolDynId::standalone(42);
    /// assert_eq!(id.parent_type, "");
    /// assert_eq!(id.parent_id, 42);
    /// assert_eq!(id.index, 0);
    /// ```
    pub const fn standalone(id: usize) -> Self {
        Self {
            parent_type: "",
            parent_id: id,
            index: 0,
        }
    }

    /// 创建高维符号分量的标识符
    /// Create identifier for high-dimensional symbol component
    ///
    /// # 示例 / Examples
    ///
    /// ```
    /// use ospf_rust_math::symbol::SymbolDynId;
    ///
    /// let id = SymbolDynId::component("VectorSymbol", 1, 2);
    /// assert_eq!(id.parent_type, "VectorSymbol");
    /// assert_eq!(id.parent_id, 1);
    /// assert_eq!(id.index, 2);
    /// ```
    pub const fn component(parent_type: &'a str, parent_id: usize, index: usize) -> Self {
        Self {
            parent_type,
            parent_id,
            index,
        }
    }

    /// 判断是否为独立符号
    /// Check if this is a standalone symbol
    ///
    /// # 示例 / Examples
    ///
    /// ```
    /// use ospf_rust_math::symbol::SymbolDynId;
    ///
    /// let standalone = SymbolDynId::standalone(0);
    /// assert!(standalone.is_standalone());
    ///
    /// let component = SymbolDynId::component("VectorSymbol", 1, 0);
    /// assert!(!component.is_standalone());
    /// ```
    pub const fn is_standalone(&self) -> bool {
        self.parent_type.is_empty()
    }

    /// 判断是否为父符号本身
    /// Check if this is the parent symbol itself
    ///
    /// 当 `index == 0` 时，表示父符号本身（对于高维符号）或独立符号。
    /// When `index == 0`, it represents the parent symbol itself (for high-dimensional symbols)
    /// or a standalone symbol.
    pub const fn is_parent(&self) -> bool {
        self.index == 0
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_id_auto_impl() {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        struct TestId(usize);

        fn accept_symbol_id<Id: SymbolId>(_: Id) {}

        accept_symbol_id(TestId(42));
    }

    #[test]
    fn test_standalone_id() {
        let id = SymbolDynId::standalone(42);

        assert!(id.is_standalone());
        assert!(id.is_parent());
        assert_eq!(id.parent_type, "");
        assert_eq!(id.parent_id, 42);
        assert_eq!(id.index, 0);
    }

    #[test]
    fn test_component_id() {
        let id = SymbolDynId::component("VectorSymbol", 1, 2);

        assert!(!id.is_standalone());
        assert!(!id.is_parent());
        assert_eq!(id.parent_type, "VectorSymbol");
        assert_eq!(id.parent_id, 1);
        assert_eq!(id.index, 2);
    }

    #[test]
    fn test_parent_component() {
        // 高维符号本身（index = 0）
        let parent = SymbolDynId::component("VectorSymbol", 1, 0);

        assert!(!parent.is_standalone());
        assert!(parent.is_parent());
    }

    #[test]
    fn test_id_equality() {
        let id1 = SymbolDynId::standalone(42);
        let id2 = SymbolDynId::standalone(42);
        let id3 = SymbolDynId::standalone(43);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(SymbolDynId::standalone(42));
        set.insert(SymbolDynId::standalone(42));
        set.insert(SymbolDynId::standalone(43));

        assert_eq!(set.len(), 2);
    }
}
