# symbol

:us: [English](README.md) | :cn: 简体中文

符号数学系统的核心符号定义。本模块为优化和代数计算中的数学符号表示提供基础抽象。

## 核心类型

| 类型 | 描述 |
|------|------|
| `SymbolId<T>` | 类型化符号标识符，使用稳定哈希 |
| `SymbolDynId` | 动态（无类型）符号标识符，用于运行时 |
| `DynSymbol` | 动态符号对象的 trait，支持运行时分发 |
| `Symbol` | 静态类型符号的 trait，具有类型化标识符 |
| `OwnedSymbol` | 拥有权包装器，包装任何实现 `DynSymbol` 的符号 |

## 架构设计

- `SymbolId<T>` - 泛型类型化标识符，包装内部 ID 类型
- `SymbolDynId` - 基于组件的动态 ID，用于异构集合
- `DynSymbol` trait - 提供 `name()`、`display_name()`、`dyn_id()` 和 `as_any()` 用于向下转型
- `Symbol` trait - 扩展 `DynSymbol`，添加类型化的 `id()` 方法，返回 `Self::Id`
- `OwnedSymbol` - 堆分配容器，通过类型擦除实现 `DynSymbol`

## 使用示例

```rust
use ospf_rust_math::symbol::{Symbol, DynSymbol, OwnedSymbol, SymbolId};

// 实现自定义符号
struct MyVariable {
    name: String,
}

impl DynSymbol for MyVariable {
    fn name(&self) -> &str { &self.name }
    fn display_name(&self) -> &str { &self.name }
    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::component("MyVariable", 0, 0)
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

impl Symbol for MyVariable {
    type Id = String;
    fn id(&self) -> Self::Id { self.name.clone() }
}
```

## 许可证

本项目采用 MIT 许可证。
