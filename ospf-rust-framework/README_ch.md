# OSPF Rust Framework

中文 | [English](README.md)

OSPF（运筹学求解器框架）Rust 实现的框架层，提供高层求解器抽象和模型管理工具。

## 概述

`ospf-rust-framework` 提供：

- **模型层**：列生成算法的管道管理和 Shadow Price 工具
- **求解器层**：线性和二次规划的求解器抽象 trait 和组合器

## 功能模块

### Model 模块

- **Pipeline (`pipeline.rs`)**：`CGPipeline` trait，用于列生成管道管理
- **Shadow Price (`shadow_price.rs`)**：Shadow Price 数据结构和管理工具

### Solver 模块

- **列生成 (`column_generation.rs`)**：列生成求解器的核心 trait 定义
- **并行模式 (`parallel_mode.rs`)**：并行执行模式配置
- **线性求解器**：
  - `parallel_linear.rs` - 并行线性求解器组合器
  - `serial_linear.rs` - 串行线性求解器组合器
- **二次求解器**：
  - `parallel_quadratic.rs` - 并行二次求解器组合器
  - `serial_quadratic.rs` - 串行二次求解器组合器
- **列生成求解器**：
  - `parallel_column_generation.rs` - 并行列生成求解器组合器
  - `serial_column_generation.rs` - 串行列生成求解器组合器
- **Benders 分解 (`benders_decomposition.rs`)**：Benders 分解算法支持

## 使用方法

在 `Cargo.toml` 中添加：

```toml
[dependencies]
ospf-rust-framework = { path = "path/to/ospf-rust-framework" }
```

### 统一求解入口（推荐）

框架层建议优先使用 `solve(...)` 最短路径；当存在模型外参数时，统一使用
`solve_with_options(...)`，减少记忆多参数接口。

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::{ColumnGenerationSolver, SolveOptions};

fn run_column_generation<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let _result = solver.solve(meta_model)?;

    let options = SolveOptions::new()
        .with_name("cg_case")
        .with_log_model(true);
    let _result_with_options = solver.solve_with_options(meta_model, options)?;
    Ok(())
}
```

相同模式也适用于：
- `LinearMetaModelSolverExt`
- `QuadraticMetaModelSolverExt`
- `LinearBendersDecompositionSolver`
- `QuadraticBendersDecompositionSolver`

同时提供面向 MetaModel 的入口：
- 线性/二次 MetaModel 扩展：`solve_meta(...)` / `solve_meta_with_options(...)`
- 线性 Benders MetaModel 入口：`solve_meta(...)` / `solve_meta_with_options(...)`
- 二次 Benders MetaModel 入口：`solve_meta_quadratic(...)` / `solve_meta_quadratic_with_options(...)`

### 值类型与精度策略

框架层 Meta 入口接受 `MetaModel<V>`，并通过 `SolveValueConversionPolicy` 转换到后端数值域：

- `Strict`（默认）：禁止有损转换
- `AllowRounding`：允许受控舍入到 `f64`

当前值类型支持：
- `f64`（默认，无需额外 feature）
- `BigRational`（启用 `big-rational` feature）
- `BigDecimal`（启用 `big-decimal` feature）

所有转换失败都会显式报错（避免静默降精度），典型错误包括：
- `NonFinite`
- `PrecisionLoss`
- `Overflow`

示例：

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::SolveValueConversionPolicy;
use ospf_rust_framework::solver::{ColumnGenerationSolver, SolveOptions};

fn run_with_policy<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let options = SolveOptions::new()
        .with_value_conversion_policy(SolveValueConversionPolicy::Strict);
    let _result = solver.solve_with_options(meta_model, options)?;
    Ok(())
}
```

### 接口收敛说明

对应用层公开的主路径收敛为：
- `solve(...)`
- `solve_with_options(...)`

对于 Benders 与扩展 trait 路径，`solve_meta*` 仍可用于显式 MetaModel 工作流；
但应用层建议优先使用上述两个统一入口，降低接口记忆成本。

### 特性标志

- `async` - 启用求解器操作的 async/await 支持
- `nightly` - 透传 `ospf-rust-core` 的 nightly 可调用能力
- `big-rational` - 启用 `BigRational` 的 MetaModel 值类型支持（透传到 core）
- `big-decimal` - 启用 `BigDecimal` 的 MetaModel 值类型支持（透传到 core）

### SCIP 环境说明

启用 `scip` 时，除 `SCIPOPTDIR` 外，还需确保 `libclang` 与 SCIP 运行时 DLL 可被发现：

- `LIBCLANG_PATH` 指向包含 `libclang.dll` 的目录
- `PATH` 需包含 `<SCIPOPTDIR>\\bin`，以便运行/测试时加载 `libscip.dll`

### 求解器后端说明（Gurobi / SCIP）

framework 侧扩展入口在：

1. `src/solver/gurobi_extension.rs`
2. `src/solver/scip_extension.rs`

core 侧后端准备说明见：

1. Gurobi: `../ospf-rust-core/src/solver/solvers/gurobi/README.md`
2. SCIP: `../ospf-rust-core/src/solver/solvers/scip/README.md`

framework 常见 feature 示例：

```bash
# Gurobi 12 扩展路径
cargo test -p ospf-rust-framework --features gurobi12

# SCIP 扩展路径
cargo test -p ospf-rust-framework --features scip

# SCIP bundled + async 扩展路径
cargo test -p ospf-rust-framework --features "scip-bundled async"
```

## 依赖

- `ospf-rust-core` - 核心数据结构和 trait
- `ospf-rust-base` - 基础工具
- `parking_lot` - 高性能同步原语
- `log` - 日志门面
- `async-trait`（可选）- Async trait 支持

## 许可证

本项目与主 OSPF Rust 项目使用相同的许可证。
