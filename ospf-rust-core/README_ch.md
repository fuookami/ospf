# OSPF Rust Core

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-core` 是 OSPF Rust workspace 的核心建模 crate。它拥有从变量和符号表达式，到 `MetaModel` 构建与展开，再到 solver 抽象、结果输出、IIS diagnostics 和 feature-gated backend adapter 的优化模型生命周期。

## 作用范围

本 crate 覆盖：

1. 变量与 token 系统。
2. 符号表达式和函数符号系统。
3. `MetaModel`、mechanism model、intermediate standard-form model 和 callback model 层。
4. solver trait、options、outputs、value conversion、heuristic interface、IIS diagnostics 和 Gurobi/SCIP adapter 边界。

明确非目标：

1. framework 层 column-generation 编排、Benders 组合器、持久化和远程求解；这些属于 `ospf-rust-framework`。
2. 分切、装箱或排程等领域专用建模。
3. solver 安装和许可证管理，除 backend setup notes 外不在本 crate 处理。

## 模块结构

| Rust 模块 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| `variable` | `core/variable` | 变量类型系统、变量项、组合和范围。 |
| `token` | `core/token` | 变量-token 映射、token list/table、缓存值和 solver result access。 |
| `symbol` | `core/symbol` | 表达式符号、monomial cell、函数符号、中间符号和 flattening helper。 |
| `model` | `core/model` | `MetaModel`、mechanism model、intermediate triad/tetrad model、callback model、constraint/objective DSL 和 model state。 |
| `solver` | `core/solver` | solver trait、options、backend config、output type、value conversion、heuristic helper、IIS diagnostics 和 backend adapter。 |
| `error` | `core/error` | core error 和 result 类型。 |

## 架构概览

`ospf-rust-core` 遵循 Kotlin 对齐的模型生命周期：

```text
用户定义层      ->  MetaModel<V>
    -> 机理层   ->  MechanismModel<V>
    -> 标准形式 ->  LinearTriadModel / QuadraticTetradModel
    -> solver层 ->  SolveReport（SolverOutput 仅作兼容 facade）
```

本 crate 显式保留模型构建、表达式展开、solver-order token 映射和结果提取，使上层 framework crate 可以组合它们，而不拥有底层建模细节。

## 核心概念

1. 变量描述 binary、integer 和 continuous 等 solver decision domain。
2. token 把变量和符号连接到 solver-order index 与缓存结果。
3. symbol 表达线性/二次表达式、函数符号和中间值。
4. `MetaModel` 是面向用户的装配层；mechanism 与 intermediate model 为 solver 消费而生成。
5. solver trait 和 backend adapter 把 standard-form model 转为后端调用并返回结构化 output。

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `MetaModel<V>` | 主要用户侧模型装配对象。 | stable within migration |
| `Variable`、`VariableRange`、`VariableType` | 决策变量定义和范围。 | stable within migration |
| `Token`、`TokenList`、`TokenTable` | solver-order token 映射和 result/cache access。 | stable within migration |
| `symbol::function` | 推荐函数符号路径。 | stable within migration |
| `symbol::flatten` | 推荐表达式展开路径。 | stable within migration |
| `model::mechanism` | mechanism model 与 constraint/objective lowering。 | migration |
| `LinearTriadModel`、`QuadraticTetradModel` | standard-form solver input model。 | migration |
| `Solver`、`SolverExt`、`SolveOptions` | 统一 solver trait 和 solve options。 | migration |
| `solver::backend::{GurobiSolver, ScipSolver}` | feature-gated backend adapter。 | migration |

## 泛型数值边界

`MetaModel<V>` 对建模值类型泛型化。backend adapter 当前通过后端数值域求解，最常见为 `f64`。转换应保留在 solver、flattening 和 extraction 边界。`big-rational` 与 `big-decimal` feature path 通过 core/framework conversion policy 支持协同。

## 求解生命周期

常见求解路径：

1. 使用变量、符号、约束和目标构建 `MetaModel<V>`。
2. 通过 mechanism 与 flattening 层降低符号表达式。
3. dump 为线性或二次 standard form。
4. 调用 `Solver` 实现。
5. 把 solver-order value 写回 token，并暴露结构化 output。

## 使用方式

添加依赖：

```toml
[dependencies]
ospf-rust-core = { path = "path/to/ospf-rust-core" }
```

### 统一求解入口

完整的 report、proof、取消、身份和 legacy 迁移合同见
[`docs/solve-contract_ch.md`](../docs/solve-contract_ch.md)。

原生 feature 和许可证证据遵循
[`solver-native-matrix_ch.md`](../docs/solver-native-matrix_ch.md)；source commit
覆盖见 [`solver-traceability_ch.md`](../docs/solver-traceability_ch.md)。只编译 feature
不构成原生 solver 证据。

高频路径建议直接从 `MetaModel` 调用：

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{SolveOptions, SolveReport, SolverExt};

fn solve_model<S: ospf_rust_core::solver::Solver>(
    meta_model: &MetaModel<f64>,
    solver: &S,
) -> ospf_rust_core::error::Result<SolveReport<f64>> {
    let _report = meta_model.solve_report(solver)?;

    let options = SolveOptions::new();
    solver.solve_report_with_options(meta_model, &options)
}
```

精确消费方必须使用 report certificate helper。limit 或 interruption 可以保留 incumbent，
但不能据此当作最优证明。原生 License 失败归类为 `LICENSE`；缺少动态库和环境配置仍归类为
`ENVIRONMENT`。

### 约束规划边界

core crate 同时提供整数 CP AST，包括布尔文字、整数表达式、不可变 snapshot、规范化
snapshot artifact，以及 `NoOverlap` 等全局约束。CP 值域和目标求值使用精确 `i64`，
CP 路径不会静默把整数语义降为 `f64`。

Gurobi 和 SCIP 通过 feature-gated、MIP-backed 的 `ExactLowering` 暴露 CP facade。
这是一条带 snapshot 校验和统一 `SolveReport` proof 校验的精确 formulation 路径，
不代表 backend 提供 native CP search。native optional-interval tree、增量 CP session，
以及没有显式 lowering 的全局约束不属于该 facade；在能力门禁关闭前，它们保持
`Conditional` 或 `Unsupported`。选择 backend 前应先检查 capability matrix。

启用 `nightly` feature 时，还可使用 callable wrapper：

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

启用 `async` feature 时，可把阻塞式求解调用移到 Tokio blocking 线程池：

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

### MetaModel 快捷接口

`MetaModel` 提供高频建模快捷入口：

1. 带元数据的线性约束快捷添加（`group/lazy/priority/args`）。
2. symbolic 约束批量添加。
3. 按系数或索引列表构造 partition 约束。

```rust,ignore
use std::sync::Arc;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearInequality, MetaModel, SymbolicLinearInequality,
};

let mut model = MetaModel::<f64>::new("shortcut_demo");

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
    true,
    20,
    Some("{\"tag\":\"demo\"}".to_string()),
)?;

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

model.partition_linear_coefficients(&[(0, 1.0), (1, 1.0)], "p_coeff")?;
model.partition_linear_indices(&[2, 3, 4], "p_idx")?;
```

### Builder 输入

`MetaModel` 也提供 Kotlin 对齐 builder 输入，减少手写稀疏数组：

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

## Feature Flags

- `async`：启用 async 支持。
- `serde`：启用序列化/反序列化。
- `nightly`：启用 callable solver wrapper（`as_fn`）。
- `gurobi`：`gurobi10` 的别名。
- `gurobi10`、`gurobi11`、`gurobi12`：Gurobi 版本特定绑定。
- `scip`：启用 SCIP solver binding。
- `scip-bundled`：启用来自 `russcip` 的 bundled SCIP。
- `scip-from-source`：通过 `russcip` 从源码构建 SCIP。
- `scip-quadratic`：启用 SCIP 二次支持。

## Solver Backend

Rust 不按 Maven module 拆分 solver backend。依赖哪个 crate，就在该 crate 上通过 Cargo feature 启用对应后端。

后端专用说明：

1. Gurobi: [solver/solvers/gurobi/README_ch.md](src/solver/solvers/gurobi/README_ch.md)
2. SCIP: [solver/solvers/scip/README_ch.md](src/solver/solvers/scip/README_ch.md)

常见 feature 示例：

```toml
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core", features = ["gurobi10"] }
# 或
ospf-rust-core = { path = "../ospf-rust-core", features = ["scip-bundled"] }
```

```bash
cargo test -p ospf-rust-core --features gurobi10
cargo test -p ospf-rust-core --features scip
cargo test -p ospf-rust-core --features scip-bundled
```

推荐后端导入：

```rust,ignore
use ospf_rust_core::solver::backend::{GurobiSolver, ScipSolver};
```

`SCIPSolver` 和 `SCIPConfig` 作为兼容名称保留。新的 Rust 代码建议使用 `ScipSolver` 和 `ScipConfig`。

## Kotlin 对齐公共路径

- `symbol::function` 是新的推荐入口；旧 `symbol::functions` 保留兼容。
- `symbol::flatten` 是新的推荐入口；旧 `model::flatten` 保留兼容。
- 已引入 `solver::config`、`solver::output`、`solver::value` 和 `solver::backend` 对齐路径。

## 本地验证

```powershell
cargo check -p ospf-rust-core
cargo test -p ospf-rust-core --no-run
cargo test -p ospf-rust-core --lib
bash scripts/phase5_gate.sh
powershell -File scripts/phase5_gate.ps1
```

## 依赖

- `ospf-rust-base`
- `ospf-rust-math`
- `ospf-rust-multiarray`
- `thiserror`

## 相关模块

- [根 README](../README_ch.md)
- [Framework README](../ospf-rust-framework/README_ch.md)
- [Kotlin core README](../../ospf-kotlin/ospf-kotlin-core/README_ch.md)

## 许可证

本项目与主 OSPF Rust 项目使用相同许可证。
