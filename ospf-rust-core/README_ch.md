# OSPF Rust Core

中文 | [English](README.md)

OSPF（运筹学求解器框架）Rust 实现的核心模块，提供优化建模的基础数据结构和抽象。

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

### MetaModel 快捷接口示例

`MetaModel` 已补齐高频建模快捷入口，典型包括：

1. 带元数据的线性约束快捷添加（`group/lazy/priority/args`）
2. symbolic 约束批量添加
3. 按系数或索引列表构造 partition 约束

```rust,ignore
use std::sync::Arc;
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearInequality, MetaModel, SymbolicLinearInequality,
};

let mut model = MetaModel::<f64>::new("shortcut_demo");

// 1) 带元数据的快捷接口
let g = Arc::new(ConstraintGroup::new("logic"));
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

### 特性标志

- `async` - 启用 async/await 支持
- `serde` - 启用序列化/反序列化
- `nightly` - 启用可调用求解器包装器（`as_fn`）
- `gurobi` - 启用 Gurobi 求解器绑定
- `gurobi10`, `gurobi11`, `gurobi12` - Gurobi 版本特定绑定
- `scip` - 启用 SCIP 求解器绑定
- `scip-quadratic` - 启用 SCIP 二次支持

### 求解器后端说明（Gurobi / SCIP）

后端专用说明文档：

1. Gurobi: [solver/solvers/gurobi/README.md](src/solver/solvers/gurobi/README.md)
2. SCIP: [solver/solvers/scip/README.md](src/solver/solvers/scip/README.md)

常见 feature 示例：

```bash
# Gurobi 12
cargo test -p ospf-rust-core --features gurobi12

# SCIP（本机安装）
cargo test -p ospf-rust-core --features scip

# SCIP bundled
cargo test -p ospf-rust-core --features scip-bundled
```

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
