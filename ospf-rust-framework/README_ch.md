# OSPF Rust Framework

OSPF（运筹学求解器框架）Rust 实现的框架层，提供高层求解器抽象和模型管理工具。

🇺🇸 [English](README.md) | 🇨🇳 简体中文

## 概述

`ospf-rust-framework` 提供：

- **模型层**：列生成算法的管道管理和 Shadow Price 工具
- **求解器层**：线性和二次规划的求解器抽象 trait 和组合器

## 功能模块

### Model 模块

- **Pipeline (`pipeline.rs`)**：`CGPipeline` trait，用于列生成管道管理
- **Shadow Price (`shadow_price.rs`)**：Shadow Price 数据结构和管理工具

### Solver 模块

- **列生成 (`column_generation_solver.rs`)**：列生成求解器的核心 trait 定义
- **并行模式 (`parallel_combinatorial_mode.rs`)**：并行执行模式配置
- **线性求解器**：
  - `parallel_combinatorial_linear_solver.rs` - 并行线性求解器组合器
  - `serial_combinatorial_linear_solver.rs` - 串行线性求解器组合器
- **二次求解器**：
  - `parallel_combinatorial_quadratic_solver.rs` - 并行二次求解器组合器
  - `serial_combinatorial_quadratic_solver.rs` - 串行二次求解器组合器
- **列生成求解器**：
  - `parallel_combinatorial_column_generation_solver.rs` - 并行列生成求解器组合器
  - `serial_combinatorial_column_generation_solver.rs` - 串行列生成求解器组合器
- **Benders 分解 (`linear_benders_decomposition_solver.rs` / `quadratic_benders_decomposition_solver.rs`)**：Benders 分解算法支持
- **远程求解 (`remote-solver` feature)**：async 远程求解客户端、HTTP 任务客户端、本地对象存储和 OSPF 模型序列化 helper

## 使用方法

在 `Cargo.toml` 中添加：

```toml
[dependencies]
ospf-rust-framework = { path = "path/to/ospf-rust-framework" }
```

### 统一求解入口（推荐）

框架层建议优先使用 `solve(...)` 最短路径；当存在模型外参数时，统一使用
`solve_with_options(...)`，减少记忆多参数接口。
推荐选项类型为 `FrameworkSolveOptions`（`SolveOptions` 作为兼容别名保留）。

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};

fn run_column_generation<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let _result = solver.solve(meta_model)?;

    let options = FrameworkSolveOptions::new()
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
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};

fn run_with_policy<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let options = FrameworkSolveOptions::new()
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

### 默认验收命令

```bash
cargo check -p ospf-rust-framework
cargo test -p ospf-rust-framework --no-run
cargo test -p ospf-rust-framework --lib
```

如果触碰 async 路径，建议额外执行：

```bash
cargo check -p ospf-rust-framework --features async
```

远程求解代码受 feature gate 控制，默认 framework 测试不会覆盖。请显式执行：

```bash
cargo test -p ospf-rust-framework --features remote-solver remote
cargo check -p ospf-rust-framework --features "remote-solver remote-solver-http-reqwest"
```

### 特性标志

- `async` - 启用求解器操作的 async/await 支持
- `nightly` - 透传 `ospf-rust-core` 的 nightly 可调用能力
- `big-rational` - 启用 `BigRational` 的 MetaModel 值类型支持（透传到 core）
- `big-decimal` - 启用 `BigDecimal` 的 MetaModel 值类型支持（透传到 core）
- `gurobi` - `gurobi10` 的别名
- `gurobi10`, `gurobi11`, `gurobi12` - 启用 core 中对应版本的 Gurobi 后端
- `scip` - 启用 core 中的 SCIP 后端
- `scip-bundled` - 通过 core 启用 bundled SCIP
- `scip-from-source` - 通过 core 从源码构建 SCIP
- `scip-quadratic` - 通过 core 启用 SCIP 二次支持
- `persistence` - 启用通用表达式仓储契约、排序/更新描述、DTO/记录类型
- `persistence-sqlx`, `persistence-sqlx-postgres`, `persistence-sqlx-mysql`, `persistence-sqlx-sqlite` - 启用参数化 SQL 语句构建器
- `persistence-sea-orm` - 启用 SeaORM translator 与 async repository adapter
- `persistence-diesel`, `persistence-diesel-postgres`, `persistence-diesel-mysql`, `persistence-diesel-sqlite` - 启用 Diesel typed field planning helper
- `persistence-rbatis` - 启用 Rbatis 命名的参数化 SQL 语句构建器
- `persistence-toasty` - 启用 Toasty typed repository plan
- `persistence-cornucopia` - 启用 Cornucopia 生成查询函数绑定 helper
- `persistence-mongodb` - 启用 MongoDB JSON filter/update helper
- `persistence-redis` - 启用 Redis key/value 命令 helper
- `remote-solver` - 启用 async 远程求解领域模型、端口、客户端编排、本地文件对象存储和 OSPF 模型序列化器
- `remote-solver-http-reqwest` - 启用远程任务客户端的 reqwest HTTP transport

### Remote Solver 说明

远程求解遵循 Cargo feature 模型，不按 Maven 模块拆分后端。公共 feature 提供：

- `RemoteSolverClient`、`RemoteLinearSolver`、`RemoteQuadraticSolver`
- async `SolverExecutionPort` 与 `ObjectStoragePort`
- `RemoteSolverHttpClient`，支持可替换 transport 与 Kotlin 兼容的任务路由
- `LocalFileObjectStoragePort`，包含路径逃逸保护、metadata sidecar 和 SHA-256 etag
- `OspfRemoteModelSerializer`，支持 `LinearTriadModel` 与 `QuadraticTetradModel`
- `SerializedSolution` / `SolveResult` 到 `SolverOutput` 的转换 helper

`RemoteLinearSolver` 和 `RemoteQuadraticSolver` 已实现 core 的同步 solver trait，方便作为替换项使用。
如果代码已经运行在 current-thread Tokio runtime 内，建议使用 `solve_remote(...)` /
`solve_remote_with_options(...)`，同步 trait 入口会主动拒绝阻塞该 runtime flavor。

对于 `RemoteSolverHttpExecutionPort`，当前 HTTP 任务 API 的规范 task id 由服务端分配。
调用方传入的 `task_id` 会作为 `requestId` 和 payload 路径关联键，返回的 handle 保存服务端 task id。
当前 `/resume` 路由恢复任务自身最新 checkpoint；如果要选择指定 checkpoint，需要服务端 API 支持。

reqwest transport 仅在启用 `remote-solver-http-reqwest` 时编译：

```toml
[dependencies]
ospf-rust-framework = { path = "../ospf-rust-framework", features = ["remote-solver-http-reqwest"] }
```

```rust,ignore
use std::time::Duration;
use ospf_rust_framework::solver::{
    RemoteSolveContext, RemoteSolveOptions, RemoteSolverClient,
};

let options = RemoteSolveOptions::new()
    .with_quantum(Duration::from_secs(4))
    .with_max_rounds(64);
```

### Persistence 后端说明

持久化后端遵循 Cargo feature 模型，不按 Maven 模块拆分。公共层提供
`ExpressionRepository`、`RepositoryQuery`、`SortBy`、`UpdateAssignments`、
`PredicateSchema`、`FieldPath` 以及请求/响应 DTO 与记录类型。

当前后端边界：
- SQLx 构建参数化 `SELECT` / `COUNT` / `UPDATE` / `DELETE` SQL 语句。
- SeaORM 将表达式翻译为 SeaQuery/SeaORM 类型，并提供 async repository adapter。
- Rbatis 复用参数化 SQL builder，但提供 Rbatis 命名入口。
- Diesel 与 Toasty 保持 typed ORM 边界，通过 planning helper 交给调用方接入具体模型。
- Cornucopia 绑定生成查询函数名，不强行做动态 SQL。
- MongoDB 与 Redis 提供 JSON/document 和命令 helper，不绑定具体 client 类型。

### SCIP 环境说明

启用 `scip` 时，除 `SCIPOPTDIR` 外，还需确保 `libclang` 与 SCIP 运行时 DLL 可被发现：

- `LIBCLANG_PATH` 指向包含 `libclang.dll` 的目录
- `PATH` 需包含 `<SCIPOPTDIR>\\bin`，以便运行/测试时加载 `libscip.dll`

### 求解器后端说明（Gurobi / SCIP）

framework 遵循 Rust feature 模型：只有在 `ospf-rust-framework` 上启用对应 feature 时，
后端适配器才会编译，并且该 feature 会透传到 `ospf-rust-core`。

framework 侧扩展入口在：

1. `src/solver/gurobi_extension.rs`
2. `src/solver/scip_extension.rs`

core 侧后端准备说明见：

1. Gurobi: `../ospf-rust-core/src/solver/solvers/gurobi/README.md`
2. SCIP: `../ospf-rust-core/src/solver/solvers/scip/README.md`

### Benders dual/Farkas 说明

对于 LP 与可线性化子问题，framework 会优先使用后端返回的 row dual/Farkas 证书。
如果 LP row dual 的目标值与 primal objective 不一致，或 Farkas 证书缺失，
framework 会 fallback 到显式 dual 或 Farkas dual model。

对于真实二次子问题，后端对完整 dual/Farkas 证书的支持有限。
当二次子问题不能表示为线性 surrogate，且后端没有返回可用乘子时，
framework 会返回可行/不可行结果，但该轮可能无法生成完整的 quadratic Benders cut。

framework 常见 feature 示例：

```toml
[dependencies]
ospf-rust-framework = { path = "../ospf-rust-framework", features = ["gurobi10"] }
# 或
ospf-rust-framework = { path = "../ospf-rust-framework", features = ["scip-bundled"] }
```

```bash
# Gurobi 10 扩展路径
cargo test -p ospf-rust-framework --features gurobi10

# SCIP 扩展路径
cargo test -p ospf-rust-framework --features scip

# SCIP bundled + async 扩展路径
cargo test -p ospf-rust-framework --features "scip-bundled async"
```

推荐 framework 导入路径：

```rust,ignore
use ospf_rust_framework::solver::{
    ColumnGenerationSolver, FrameworkSolveOptions,
    GurobiColumnGenerationSolver, ScipColumnGenerationSolver,
};
```

常见 Kotlin 概念参数提供 Rust 风格 builder：

```rust,ignore
let options = FrameworkSolveOptions::new()
    .with_solution_amount(5)
    .with_benders_iteration_limit(100)
    .with_benders_stall_iteration_limit(3);
```

后端 wrapper 也透出常用调参别名：

```rust,ignore
let gurobi = GurobiColumnGenerationSolver::new()
    .with_gap(1e-4)
    .with_memory_limit_gb(8.0)
    .with_improve_threshold(1e-6);

let scip = ScipColumnGenerationSolver::new()
    .with_gap(1e-4)
    .with_memory_limit_mb(2048.0)
    .with_improve_threshold(1e-6)
    .with_lp_subproblem_defaults();
```

对于 ColumnGeneration/Benders 子问题，如果需要更稳定的 LP row dual，SCIP wrapper
建议使用 `with_lp_subproblem_defaults()`。该 helper 会在子问题后端配置上使用单线程，
并关闭 presolving 与 heuristics。

## 依赖

- `ospf-rust-core` - 核心数据结构和 trait
- `ospf-rust-base` - 基础工具
- `parking_lot` - 高性能同步原语
- `log` - 日志门面
- `async-trait`（可选）- Async trait 支持

## 许可证

本项目与主 OSPF Rust 项目使用相同的许可证。
