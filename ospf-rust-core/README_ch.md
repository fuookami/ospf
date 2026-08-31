# OSPF Rust Core

OSPF（运筹学求解器框架）Rust 实现的核心模块，提供优化建模的基础数据结构和抽象。

🇺🇸 [English](README.md) | 🇨🇳 简体中文

## 概述

`ospf-rust-core` 实现了一个运筹学建模框架，支持：

- 线性规划 (LP)
- 混合整数规划 (MIP)
- 二次规划 (QP)

## 核心模块

### 变量系统 (`variable`)
- 变量类型定义（二元、整数、连续）
- 变量 Arena，用于高效内存管理
- 变量组合和范围

### Token 系统 (`token`)
- 表达式中变量的 Token 表示
- Token 列表和表，用于高效查找
- 变量数据管理

### 符号系统 (`symbol`)
- 表达式符号（单项式、多项式）
- 自定义表达式的函数符号
- 中间符号表示
- 单项式单元操作

### 模型系统 (`model`)
- **基本模型**：核心模型结构
- **配置**：模型配置选项
- **平展系统**：表达式平展，用于求解器输入
- **机理模型**：约束组和基于机理的建模
- **中间模型**：线性和二次模型表示
  - `LinearTriadModel` - 线性模型 (A, b, c)
  - `QuadraticTetradModel` - 二次模型 (Q, A, b, c)
- **回调模型**：多目标和基于回调的建模

### 求解器接口 (`solver`)
- 求解器配置
- 求解器输出结构
- 启发式算法
  - 种群管理
  - 选择、交叉、变异算子
  - 归一化技术
- IIS（不可约不一致子系统）分析

## 使用方法

在 `Cargo.toml` 中添加：

```toml
[dependencies]
ospf-rust-core = { path = "path/to/ospf-rust-core" }
```

### 统一求解入口（推荐）

高频路径建议直接从 `MetaModel` 调用：

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{SolveOptions, SolverExt};

fn solve_model<S: ospf_rust_core::solver::Solver>(
    meta_model: &MetaModel<f64>,
    solver: &S,
) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
    // 最短路径
    let _output = meta_model.solve(solver)?;

    // 带参数对象
    let options = SolveOptions::new();
    solver.solve_with_options(meta_model, &options)
}
```

启用 `nightly` 特性时，还可使用可调用包装器：

```rust
#[cfg(feature = "nightly")]
fn solve_with_callable<S: ospf_rust_core::solver::Solver>(
    meta_model: &MetaModel<f64>,
    solver: &S,
) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
    let f = solver.as_fn();
    f(meta_model)
}
```

启用 `async` 特性时，可以把阻塞式求解调用移到 Tokio blocking 线程池：

```rust
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{solve_async_with_callback, SolvingStatusCallback, Solver};

#[cfg(feature = "async")]
async fn solve_in_background<S: Solver + 'static>(
    meta_model: MetaModel<f64>,
    solver: Arc<S>,
    callback: SolvingStatusCallback,
) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
    solve_async_with_callback(solver, meta_model, callback).await
}
```

### MetaModel 快捷接口示例

`MetaModel` 已补齐高频建模快捷入口，典型包括：

1. 带元数据的线性约束快捷添加（`group/lazy/priority/args`）
2. symbolic 约束批量添加
3. 按系数或索引列表构造 partition 约束

```rust,ignore
use std::sync::Arc;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearInequality, MetaModel, SymbolicLinearInequality,
};

let mut model = MetaModel::<f64>::new("shortcut_demo");

// 1) 带元数据的快捷接口
let g = Arc::new(ConstraintGroup::new(1001, "logic"));
let ineq = LinearInequality::new(
    Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
    ConstraintRelation::LessEqual,
    10.0,
);
model.add_inequality_with_metadata(
    ineq,
    "c_meta",
    Some(g),
    true,   // lazy
    20,     // priority
    Some("{\"tag\":\"demo\"}".to_string()),
)?;

// 2) 批量 symbolic 约束
model.add_symbolic_inequalities(vec![
    (
        SymbolicLinearInequality::new(Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0), ConstraintRelation::GreaterEqual, 0.0),
        "sym_lb",
    ),
    (
        SymbolicLinearInequality::new(Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0), ConstraintRelation::LessEqual, 5.0),
        "sym_ub",
    ),
]);

// 3) partition 快捷入口
model.partition_linear_coefficients(&[(0, 1.0), (1, 1.0)], "p_coeff")?;
model.partition_linear_indices(&[2, 3, 4], "p_idx")?;
```

### Phase4 Builder 接口

`MetaModel` 额外提供 Kotlin 风格的 builder 输入，减少手写稀疏数组：

```rust,ignore
use ospf_rust_core::model::{LinearExpressionBuilder, MetaModel, ObjectiveCategory};

let mut model = MetaModel::<f64>::new("builder_demo");

let c = LinearExpressionBuilder::new()
    .term(0, 1.0)
    .term(1, 2.0)
    .constant(-3.0)
    .le(0.0, "capacity");
model.add_linear_constraint_input(c)?;

let objective = LinearExpressionBuilder::new()
    .term(0, 4.0)
    .term(1, 5.0)
    .maximize("profit")
    .category(ObjectiveCategory::Maximum);
model.set_linear_objective_input(objective);
```

### Phase5 门禁

默认门禁与扫描：

```bash
bash scripts/phase5_gate.sh
powershell -File scripts/phase5_gate.ps1
```

### 特性标志

- `async` - 启用 async/await 支持
- `serde` - 启用序列化/反序列化
- `nightly` - 启用可调用求解器包装器（`as_fn`）
- `gurobi` - `gurobi10` 的别名
- `gurobi10`, `gurobi11`, `gurobi12` - Gurobi 版本特定绑定
- `scip` - 启用 SCIP 求解器绑定
- `scip-bundled` - 启用 `russcip` bundled SCIP
- `scip-from-source` - 通过 `russcip` 从源码构建 SCIP
- `scip-quadratic` - 启用 SCIP 二次支持

### 求解器后端说明（Gurobi / SCIP）

Rust 侧不按 Maven module 拆分求解器后端；依赖哪个 crate，就在该 crate 上通过 Cargo
feature 启用对应后端。

后端专用说明文档：

1. Gurobi: [solver/solvers/gurobi/README.md](src/solver/solvers/gurobi/README.md)
2. SCIP: [solver/solvers/scip/README.md](src/solver/solvers/scip/README.md)

常见 feature 示例：

```toml
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core", features = ["gurobi10"] }
# 或
ospf-rust-core = { path = "../ospf-rust-core", features = ["scip-bundled"] }
```

```bash
# Gurobi 10
cargo test -p ospf-rust-core --features gurobi10

# SCIP（本机安装）
cargo test -p ospf-rust-core --features scip

# SCIP bundled
cargo test -p ospf-rust-core --features scip-bundled
```

推荐后端导入路径：

```rust,ignore
use ospf_rust_core::solver::backend::{GurobiSolver, ScipSolver};
```

`SCIPSolver` 和 `SCIPConfig` 作为兼容名称保留。新的 Rust 代码建议优先使用
`ScipSolver` 和 `ScipConfig`。

后端配置提供常用调参别名：

```rust,ignore
use ospf_rust_core::solver::backend::{GurobiConfig, GurobiSolver, ScipConfig, ScipSolver};

let gurobi = GurobiSolver::with_config(
    GurobiConfig::new()
        .with_gap(1e-4)
        .with_memory_limit_gb(8.0)
        .with_improve_threshold(1e-6),
);

let scip = ScipSolver::with_config(
    ScipConfig::new()
        .with_gap(1e-4)
        .with_memory_limit_mb(2048.0)
        .with_improve_threshold(1e-6),
);
```

多解调用统一使用 `solution_amount`。Gurobi 和 SCIP 会在后端支持时优先走原生
solution pool 路径：

```rust,ignore
use ospf_rust_core::solver::{SolveOptions, SolverExt};

let options = SolveOptions::new().with_solution_amount(5);
let multi = solver.solve_multi_with_options(&meta_model, &options)?;
```

### Kotlin 对齐公共路径（迁移说明）

- `symbol::function` 作为新主路径（旧 `symbol::functions` 保留兼容）。
- `symbol::flatten` 作为新主路径（旧 `model::flatten` 保留兼容）。
- 已引入 `solver::config`、`solver::output`、`solver::value`、`solver::backend` 对齐路径。

## 依赖

- `ospf-rust-base` - 基础工具和集合
- `ospf-rust-math` - 数学类型和运算
- `ospf-rust-multiarray` - 多维数组支持
- `thiserror` - 错误处理派生宏

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                      用户代码                            │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                     模型层                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │  回调    │ │  中间    │ │  机理    │ │   平展    │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                     符号层                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │ 表达式   │ │  函数    │ │  中间    │                │
│  │  符号    │ │  符号    │ │  符号    │                │
│  └──────────┘ └──────────┘ └──────────┘                │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                     Token 层                            │
│  ┌──────────┐ ┌──────────┐                             │
│  │  Token   │ │ TokenList│                             │
│  └──────────┘ └──────────┘                             │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                     变量层                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │ 变量ID   │ │   变量   │ │  Arena   │                │
│  └──────────┘ └──────────┘ └──────────┘                │
└─────────────────────────────────────────────────────────┘
```

## 许可证

本项目与主 OSPF Rust 项目使用相同的许可证。
