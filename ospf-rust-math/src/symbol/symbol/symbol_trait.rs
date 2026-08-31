//! 符号 trait 定义
//! Symbol trait definitions

use std::any::Any;
use std::fmt::{Debug, Display};
use dyn_clone::DynClone;
use super::{SymbolDynId, SymbolId};

// ============================================================================
// DynSymbol Trait - 动态符号（dyn compatible）
// ============================================================================

/// DynSymbol - 动态符号 trait（dyn compatible）
/// DynSymbol - Dynamic symbol trait (dyn compatible)
///
/// 用于 trait object，支持运行时多态。
/// Used for trait objects, supports runtime polymorphism.
///
/// # 设计说明 / Design Notes
///
/// 此 trait 仅包含 dyn compatible 的方法，可以被用于 `Box<dyn DynSymbol>`。
/// This trait only contains dyn compatible methods, can be used for `Box<dyn DynSymbol>`.
///
/// - `name()`: 内部标识名，用于调试和日志
/// - `display_name()`: 显示名称，用于用户界面
/// - `dyn_id()`: 动态标识符，用于比较和哈希
/// - 继承 `DynClone`: 支持通过 `dyn_clone::clone_box` 克隆
/// - 继承 `Display`: 支持格式化输出，默认应输出 `display_name()`
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::{DynSymbol, SymbolDynId};
/// use std::fmt::{Debug, Display, Formatter, Result};
/// use std::any::Any;
///
/// #[derive(Debug, Clone)]
/// struct SimpleSymbol {
///     id: usize,
///     name: String,
/// }
///
/// impl Display for SimpleSymbol {
///     fn fmt(&self, f: &mut Formatter<'_>) -> Result {
///         write!(f, "{}", self.display_name())
///     }
/// }
///
/// impl DynSymbol for SimpleSymbol {
///     fn name(&self) -> &str { &self.name }
///     fn display_name(&self) -> &str { &self.name }
///     fn dyn_id(&self) -> SymbolDynId<'_> {
///         SymbolDynId::standalone(self.id)
///     }
///     fn as_any(&self) -> &dyn Any { self }
/// }
///
/// let sym = SimpleSymbol { id: 1, name: "x".to_string() };
/// assert_eq!(format!("{}", sym), "x");
/// ```
pub trait DynSymbol: Display + Debug + Any + DynClone {
    /// 内部标识名 / Internal identifier name
    ///
    /// 用于调试和日志输出。
    /// Used for debugging and logging.
    fn name(&self) -> &str;

    /// 显示名称 / Display name
    ///
    /// 用于用户界面展示。
    /// Used for user interface display.
    fn display_name(&self) -> &str;

    /// 动态标识符 / Dynamic identifier
    ///
    /// 用于在运行时标识符号，支持比较和哈希。
    /// Used to identify symbols at runtime, supports comparison and hashing.
    fn dyn_id(&self) -> SymbolDynId<'_>;

    /// 将 self 转换为 Any 引用
    /// Convert self to Any reference
    fn as_any(&self) -> &dyn Any;
}

// 实现 DynClone 对 Box<dyn DynSymbol> 的支持
// Implement DynClone support for Box<dyn DynSymbol>
dyn_clone::clone_trait_object!(DynSymbol);

// ============================================================================
// Symbol Trait - 静态符号
// ============================================================================

/// Symbol - 静态符号 trait
/// Symbol - Static symbol trait
///
/// `Id` 作为关联类型，每个实现类型决定自己的 `Id` 类型。
/// `Id` as associated type, each implementation decides its own `Id` type.
///
/// # 设计说明 / Design Notes
///
/// 静态符号在编译期确定标识符类型，适合静态类型系统。
/// 与 `DynSymbol` 相比，提供了类型安全的标识符访问。
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::{Symbol, SymbolId, SymbolDynId, DynSymbol};
/// use std::fmt::{Debug, Display, Formatter, Result};
/// use std::any::Any;
///
/// #[derive(Debug, Clone)]
/// struct SimpleSymbol {
///     id: usize,
///     name: String,
/// }
///
/// impl Display for SimpleSymbol {
///     fn fmt(&self, f: &mut Formatter<'_>) -> Result {
///         write!(f, "{}", self.display_name())
///     }
/// }
///
/// impl DynSymbol for SimpleSymbol {
///     fn name(&self) -> &str { &self.name }
///     fn display_name(&self) -> &str { &self.name }
///     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
///     fn as_any(&self) -> &dyn Any { self }
/// }
///
/// impl Symbol for SimpleSymbol {
///     type Id = usize;
///     fn id(&self) -> Self::Id { self.id }
/// }
/// ```
pub trait Symbol: DynSymbol {
    /// 标识符类型 / Identifier type
    type Id: SymbolId;

    /// 唯一标识符 / Unique identifier
    fn id(&self) -> Self::Id;
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用的简单符号
    #[derive(Debug, Clone)]
    struct TestSymbol {
        id: usize,
        name: String,
    }

    impl Display for TestSymbol {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for TestSymbol {
        fn name(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl Symbol for TestSymbol {
        type Id = usize;

        fn id(&self) -> Self::Id {
            self.id
        }
    }

    #[test]
    fn test_dyn_symbol() {
        let symbol = TestSymbol {
            id: 42,
            name: "x".to_string(),
        };

        assert_eq!(symbol.name(), "x");
        assert_eq!(symbol.display_name(), "x");
        assert_eq!(symbol.dyn_id(), SymbolDynId::standalone(42));
    }

    #[test]
    fn test_symbol() {
        let symbol = TestSymbol {
            id: 42,
            name: "x".to_string(),
        };

        assert_eq!(symbol.id(), 42);
    }

    #[test]
    fn test_dyn_symbol_trait_object() {
        let symbol = TestSymbol {
            id: 42,
            name: "x".to_string(),
        };

        let dyn_sym: &dyn DynSymbol = &symbol;
        assert_eq!(dyn_sym.name(), "x");
    }

    #[test]
    fn test_clone_boxed() {
        let symbol = TestSymbol {
            id: 42,
            name: "x".to_string(),
        };

        let boxed: Box<dyn DynSymbol> = dyn_clone::clone_box(&symbol);
        assert_eq!(boxed.name(), "x");
        assert_eq!(boxed.dyn_id(), SymbolDynId::standalone(42));
    }
}
