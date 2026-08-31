# symbol

🇺🇸 [English](README.md) | 🇨🇳 简体中文

Core symbol definitions for the symbolic mathematics system. This module provides the fundamental abstractions for representing mathematical symbols in optimization and algebraic computations.

## Key Types

| Type | Description |
|------|-------------|
| `SymbolId<T>` | Typed symbol identifier with stable hashing |
| `SymbolDynId` | Dynamic (untyped) symbol identifier for runtime use |
| `DynSymbol` | Trait for dynamic symbol objects with runtime dispatch |
| `Symbol` | Trait for static-typed symbols with typed identifiers |
| `OwnedSymbol` | Owned wrapper for any symbol implementing `DynSymbol` |

## Architecture

- `SymbolId<T>` - Generic typed identifier wrapping an inner ID type
- `SymbolDynId` - Component-based dynamic ID for heterogeneous collections
- `DynSymbol` trait - Provides `name()`, `display_name()`, `dyn_id()`, and `as_any()` for downcasting
- `Symbol` trait - Extends `DynSymbol` with typed `id()` method returning `Self::Id`
- `OwnedSymbol` - Heap-allocated container implementing `DynSymbol` via type erasure

## Usage

```rust
use ospf_rust_math::symbol::{Symbol, DynSymbol, OwnedSymbol, SymbolId};

// Implement a custom symbol
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

## License

This project is licensed under the MIT License.