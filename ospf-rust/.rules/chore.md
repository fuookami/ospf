# 项目规范

## 1. 编码风格

### 1.1 注释语言
编写注释时，要中英双语。中文在前，英文在后，单行用 ` / ` 分隔，多行分行书写。

```rust
/// 基本模型 / Basic Model
///
/// 只包含变量和约束的基本模型层，不包含目标函数。
/// Basic model layer containing only variables and constraints, without objective.
```

### 1.2 版权声明
不需要添加版权声明。项目级许可统一在仓库根目录 `LICENSE` 文件中维护。

### 1.3 ReadMe 文件
英文 ReadMe：`README.md`，中文 ReadMe：`README_ch.md`，要添加超链接能互相跳转。

### 1.4 Shell 工具
PowerShell 用：`pwsh.exe`。

### 1.5 函数参数规范
超过 2 个参数时，优先使用多行参数列表。当参数具有配置性质时，使用结构体或 Builder 模式聚合参数。

```rust
// 少量简单参数，单行
pub fn add_variable(&mut self, name: &str, value: V) -> Result<usize>

// 超过 2 个参数，多行
pub fn register_auto_variable_with_range<VT: VariableTypeTrait>(
    &mut self,
    name: &str,
    range: VariableRange<VT::Value>,
) -> Result<usize>

// 配置性质参数，使用结构体
let config = ModelConfig {
    name: "my_model".into(),
    tolerance: 1e-6,
    max_iterations: 1000,
};
model.build(config)?;
```

### 1.6 Rust 文件排版与 use 规范

本节约束 Rust 源文件的基础排版风格。重构时应优先保持本项目既有手写风格，不使用会大幅改写 use、空行和换行的自动格式化结果作为最终形态。

#### 1.6.1 文件整体结构

文件各部分的排列顺序和间距必须遵循以下规范：

1. `#![...]` crate 级属性（如 `#![cfg_attr(...)]`、`#![allow(...)]`）放在文件最开头。
2. `#[...]` 模块级属性（如 `#[macro_use]`、`#[cfg(...)]`）紧随其后。
3. `extern crate` 声明（如有）与属性之间不空行。
4. `use` 语句块与上方声明之间空一行。
5. 最后一个 `use` 语句与首个顶层声明（`mod`、`struct`、`impl`、`fn` 等）之间空一行。
6. 文件末尾必须有且仅有一个换行符。

#### 1.6.2 use 排列

**整体顺序**：

1. 所有 `use` 连续排列，中间不按来源分组插空行。
2. use 来源层级按以下顺序排列（被依赖方排在前面）：
   - `std::*`（标准库）
   - 第三方库（如 `bigdecimal`、`chrono`、`thiserror` 等）
   - `ospf_rust_base::*`（基础工具，被所有模块依赖）
   - `ospf_rust_multiarray::*`（多维数组）
   - `ospf_rust_math::*`（数学库）
   - `ospf_rust_quantities::*`（物理量）
   - `ospf_rust_core::*`（优化核心）
   - `ospf_rust_framework::*`（应用框架）
3. 同一模块层级内，按路径段字典序排列。
4. 当前 crate 内部引用使用 `crate::` 前缀，跨模块引用使用 crate 名。
5. 同模块下多类型引用允许使用 `{}` 分组；跨模块仅引用少数明确类型时，可使用显式 use。
6. 禁止 use 末尾使用分号以外的任何多余标点。

**正确示例**（以 `ospf-rust-core` 模块中的文件为例）：

```rust
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

use bigdecimal::BigDecimal;

use ospf_rust_base::container::ContainerTrait;
use ospf_rust_base::error::{Ret, Error};

use crate::error::{ModelError, Result};
use crate::symbol::{IntermediateSymbol, Token};
```

说明：
- `std` → 第三方库 → `ospf_rust_base`（最底层）→ `crate` 内部引用

#### 1.6.3 换行、缩进与空行

**基础规则**：

- 使用 4 空格缩进，不使用 tab。
- 顶层声明之间保留一行空行。
- `impl` 块内方法之间保留一行空行。
- struct/enum 结束前不保留多余空行。
- 禁止保留重复文档注释或连续重复注释块。

**函数声明与调用**：

- 1-2 个参数且语义简单时可单行书写。
- 超过 2 个参数时，按 `1.5 函数参数规范` 使用多行。
- 多行参数列表中，参数缩进一层；闭合括号与函数签名起始位置对齐。

**trait bound 与 where 子句**：

- 简短 trait bound 可内联写在泛型参数声明处。
- 复杂或多个 trait bound 必须使用 `where` 子句，每个 bound 独占一行。

```rust
// 简短 bound 内联
pub fn add<V: Clone>(&self, other: &V) -> V

// 复杂 bound 使用 where 子句
pub fn checked_add(&self, other: &Self) -> Ret<Quantity<V, Unit>>
where
    V: Add<Output = V> + Sub<Output = V> + Clone + Mul<V, Output = V> + Div<V, Output = V>,
    BigDecimal: Into<V>,
```

#### 1.6.4 注释与分段

- 公共 struct、trait、enum、重要公共方法使用中英双语 `///` 文档注释。
- 模块级文档使用 `//!`，同样双语。
- 简短字段可使用单行双语注释，如 `/// 模型名称 / Model name`。
- 大段逻辑分隔允许使用 `// ============================================================================` 形式分段，并附双语说明。
- 注释应说明业务意图、加载态语义或分段边界；避免重复描述代码本身。

#### 1.6.5 文档注释覆盖要求

- 公共 struct / enum 的每个公共字段必须有文档注释。
- 公共 trait 的每个必须方法必须有文档注释。
- 公共函数的参数和返回值语义不明显时，应在文档注释中说明。
- 泛型类型参数语义非常规时应在文档注释中说明。

### 1.7 泛型化命名规范

对外 API 使用业务自然名表达稳定抽象，不使用迁移期技术命名。

- 泛型化后的主 trait、主 struct、主服务占用自然名。
- 不使用 `V`、`Typed`、`Generic` 作为迁移痕迹型前后缀（类型参数 `V` 本身除外）。
- 需要保留的 `Flt64` 专用接口、桥接接口或兼容入口，使用 `Flt64` 后缀显式标识。
- 类型参数使用单个大写字母缩写（`V`、`U`、`T`、`I`），关联类型使用全称。
- 内部变量、测试名、文档示例应尽量同步上述命名，避免保留迁移期表达。

### 1.8 Rust 惯用法

- 能使用泛型尽可能使用泛型以保证编译期优化，避免运行时开销。
- 关联类型要使用全称，类型参数使用缩写。
- 优先使用 `Result<T, E>` 和 `?` 操作符进行错误传播，避免 `unwrap()` 在非测试代码中使用。
- 优先使用迭代器和方法链代替显式循环，当性能敏感时除外。
- 测试统一放在 `#[cfg(test)] mod tests` 块中。
