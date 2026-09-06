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


## 子包概览

### solver/

| 子包 | 描述 |
|------|------|
| config | 求解器特定配置（COPT、Gurobi、SCIP）。 |
| heuristic | 元启发式框架（PSO、选择、交叉、变异）。 |
| iis | 不可约不可行子系统诊断。 |
| output | 求解器输出数据结构（可行/不可行）。 |
| alue | 值类型转换（IntoValue trait）。 |
| ackend | 特性门控后端适配器（Gurobi、SCIP）。 |

### model/

| 子包 | 描述 |
|------|------|
| asic | 基础接口、枚举和视图类型。 |
| mechanism | MetaModel、MechanismModel、约束/目标 DSL。 |
| intermediate | 标准形式模型（triad/tetrad）、稀疏矩阵。 |
| callback | 启发式求解器的回调模型接口。 |

### symbol/

| 子包 | 描述 |
|------|------|
| unction | 30+ 函数符号（Slack、If、Max、Piecewise 等）。 |
| latten | 表达式展平工具。 |
## 架构概览

`ospf-rust-core` 遵循 Kotlin 对齐的模型生命周期：

```text
用户定义层      ->  MetaModel<V>
    -> 机理层   ->  MechanismModel<V>
    -> 标准形式 ->  LinearTriadModel / QuadraticTetradModel
    -> solver层 ->  SolveReport（SolverOutput 仅作兼容 facade）
```

本 crate 显式保留模型构建、表达式展开、solver-order token 映射和结果提取，使上层 framework crate 可以组合它们，而不拥有底层建模细节。


## 约束规划

`model::constraint_programming` 模块提供整数域 CP 模型，支持不可变快照、稳定 ID 和来源验证。关键组件：

- **布尔字面量和变量**：`CpBoolVar`、`CpBoolLiteral` 用于布尔决策变量。
- **区间**：`CpInterval` 用于建模具有开始/结束/持续时间的时间区间。
- **全局约束**：`AllDifferent`、`Element`、`Table`（支持程度因后端而异）。
- **不可变快照**：`CpSnapshot` 捕获完整 CP 模型状态用于检查点/重启。
- **身份作用域**：当 ID 必须在模型重建后存活时，使用 `scope = "stable"`
- **可移植编解码器**：`ConstraintProgrammingCheckpointCodec` 写入可移植检查点信封。

### 求解器集成

`solver::constraint_programming` 模块提供：

- **求解器/会话 SPI**：CP 求解器后端的 trait 定义。
- **伪契约求解器**：用于测试和小规模穷举预言机。
- **SCIP 集成**：特性门控的 SCIP CP 后端（有限 MIP 支持门面）。
- **MIP 支持的降阶**：有限整数子集的精确降阶，验证边界；对不支持的约束返回结构化错误。

### 基于逻辑的 Benders

对于基于逻辑的 Benders 分解，使用 `ospf-rust-framework` 中的 `LogicBasedBendersEngine`。该引擎组合线性主问题和 CP 子问题，具有显式扩展点：

- 变量绑定和冲突/最优性切割预言机。
- 整数 no-good 编码。
- 迭代追踪和 `Exact`/`Heuristic` 证明门控。
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

完整的 report、proof、取消、身份和 legacy 迁移合同见下文的
[统一求解合同](#统一求解合同)。

原生 feature 和许可证证据遵循下文的
[Solver 原生验收矩阵](#solver-原生验收矩阵)；source commit
覆盖见 [Source 追踪](#source-追踪)。只编译 feature
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

### 条件函数契约

条件函数使用以下稳定命名：`IfFunction` 是旧三元表达式，`IfElseFunction` 是首选的
显式二值条件三元形式，`ConditionalIndicatorFunction` 是可注册关系指示器，
`ConditionalIfFunction` 是不注册模型的分类器。`semantic::if_` 与 `semantic::if_named`
构造范围驱动指示器，`if_legacy` 保留旧阈值行为。`IfInFunction` 仍表示离散集合成员，
`IfInRangeFunction` 与 `RegisterableIfInRangeFunction` 表示并注册闭区间。
`ConditionalThenFunction`、`ConditionalImplyFunction` 是范围驱动的可注册形式，
`IfThenConstraintFunction`、`imply_constraint` 保留为旧 Big-M 兼容入口。
`SigmoidStepFunction` 是可注册关系阶跃形式，`SigmoidFunction` 仍是连续 PWL 形式。

令 `d = lhs - rhs`，关系指示器只在未定义间隔之外判定：

| 关系 | 真 | 假 |
| --- | --- | --- |
| `Greater` | `d >= g` | `d <= 0` |
| `GreaterEqual` | `d >= 0` | `d <= -g` |
| `Less` | `d <= -g` | `d >= 0` |
| `LessEqual` | `d <= 0` | `d >= g` |

`g` 是业务 `strict_boundary`，不是 solver 容差。可注册指示器不会推断 Big-M；调用方
必须提供覆盖条件多项式的有限有序范围，完全落在未定义间隔中的范围会被拒绝。
`evaluate` 返回 `None` 可能表示未定义或输入不可用，需要区分时使用 `classify`。
非恒定条件分支同样必须提供显式有限范围。

离散条件会根据线性系数和已注册 token 元数据推导格点证明：参与变量必须是整数，系数
必须是有限整数，`delta` 是绝对系数 gcd，常数按 `delta` 取模归一化。
`MetaModel::add_symbols` 与 `register_combination` 是原子事务；失败会恢复 token、符号、
约束和缓存绑定。第三方 `MutableTokenList`/`MutableTokenTable` 实现必须显式提供批量
原子实现，并使用 `try_add_tokens` 观察校验失败。并发 token 集合通过持有 `read()` guard
暴露借用的 trait 视图。旧 Big-M 工具拒绝非有限、零和负值；`IfInRangeFunction` 只接受
一个共享变量及有限有序的两侧边界。

## 统一求解合同

新的 solver 代码消费 `solver::SolveReport<V>`。`SolverOutput`、`FeasibleSolution`、
`SolveResult` 和 `SerializedSolution` 仅是兼容投影，不能用于推断证明或取消语义。
report 将 `problem_status`、`termination_reason`、已校验 incumbent、proof、统计、
diagnostics、provenance、fingerprint 和 trace 分开保存。可行 report 必须有 incumbent；
不可行/无界 report 不能携带 incumbent；最优证明必须有完成的终止状态和可靠完整证书。
limit 或 interruption 返回的 incumbent 只能作为候选解，不能关闭精确 bound。

`SolverErrorClass` 是稳定错误边界：`INPUT`、`MODELING`、`ENVIRONMENT`、`LICENSE`、
`CALLBACK`、`BACKEND`、`PARSING`、`NUMERICAL`、`INTERNAL_CONTRACT`、
`TERMINAL_PROJECTION` 和 `UNSUPPORTED`。取消是正常 report 终态，旧入口可将其投影为
`SolverError::Cancelled`。每次求解拥有幂等 `SolveHandle`；异步包装使用 Tokio blocking
线程池，资源释放需要 `cancel_and_wait`。report 使用确定性的模型、配置和 solver 环境
fingerprint；callback 属于 provenance 并标记为不可 replay。远程 report/checkpoint
保留 schema、run/attempt identity、父链、artifact digest、provenance、fingerprint、
proof、incumbent 和取消链；未知 schema 或身份不匹配会在恢复前拒绝。

声明的原生 backend 只有 Gurobi 和 SCIP。feature 编译只能证明 wiring；原生能力必须有
匹配的库、运行时和许可证探针。Gurobi 许可证错误（含代码 `10009`）归类为 `LICENSE`，
缺少动态库归类为 `ENVIRONMENT`。

### Solver 原生验收矩阵

`cargo check` 只是编译证据。无法加载 backend 的原生测试应记为 `unsupported`（主动要求
时记为 `failed`），被忽略测试必须使用 `-- --include-ignored`。golden/replay 比较必须
包含状态、终止原因、incumbent objective、best bound、gap、解向量、残差、fingerprint
和 provenance。主要 gate 如下：

| 范围 | 命令形态 |
| --- | --- |
| Core 合同 | `cargo test -p ospf-rust-core --test native_contract_suite` |
| Gurobi | 同一 target 加 `--features gurobi10`/`gurobi11`/`gurobi12` |
| SCIP | 同一 target 加 `--features scip`，bundled/from-source 必须明确指定 |
| 终态矩阵 | 原生 backend 下运行 `native_release_matrix` 并加 `-- --include-ignored` |
| Framework report | `cargo test -p ospf-rust-framework --no-default-features` 及 `async`/`remote-solver` |
| Network 与 Demo5 | 对应 crate README 的验证命令及选定 native feature |

### CP 能力边界

CP AST 使用精确 `i64` 值和不可变 snapshot。Gurobi、SCIP 仅通过精确有限
MIP-backed `ExactLowering` facade 暴露 CP，不宣称 native CP search。安全线性整数/布尔
变量、正 literal indicator、SOS1、状态元数据、取消、event handler 和作用域 probing，
只有显式探针成功时才是 Native。布尔重化、稀疏域、AllDifferent、Element、table 和有限
NoOverlap 属于 `ExactLowering`；Circuit、Automaton、Reservoir、增量 session、可选/变长
原生 interval 以及 CP 原生 conflict graph 属于 `Unsupported`。raw Cumulative 仅为
`Conditional` 研究路径，不构成生产能力。缺少库、许可证、bundled 下载或源码构建时，
必须记为 `not executed/unsupported`，不能把编译结果当作 Native。

### Solver 终态信息丢失清单

report 层在所有边界补回了原先丢失的终态信息：

| 边界 | 当前归属 |
| --- | --- |
| Core 状态/值投影 | `ProblemStatus`、`TerminationReason`、`SolveReport`、校验 builder 与显式旧投影 |
| Column Generation/Benders | report 聚合及 LP/dual/Farkas 证明 gate |
| Branch-and-Price | node conclusion、pricing 完成标记、继承/认证 bound 与证书 gate |
| 组合求解包装器 | 父子 attempt identity、完成线性化、loser trace 和取消快照 |
| 远程/checkpoint | 版本化 DTO、artifact/fingerprint 校验、provenance、父 attempt 和取消来源 |

### Source 追踪

统一迁移追踪以下 11 个不可变 Kotlin source commit 及其 Rust 归属：`b8d67c96`（report/progress）、
`5f616747`（终止传播）、`25bcb176`（Benders/branch-and-price 证明 gate）、`32f7d5aa`
（LP 不可行）、`e0bca1eb`（identity/fingerprint/remote/checkpoint）、`4efac629`
（capability/provenance）、`58930764`（聚合 provenance）、`e5089f18`（聚合 identity）、
`b9db32a8`（进行中取消）、`ae0b01fb`（完成点冻结）和 `b5b83d7d`（并行取消测试）。
Core report、proof、diagnostics、identity、fingerprint、progress、cancellation 和
checkpoint 在本 crate 实现；Gurobi/SCIP 通过 feature gate；framework 的组合、Benders
和 remote 路径在 `ospf-rust-framework` 实现；CPLEX、COPT、Hexaly、MindOPT、MOSEK 及
native CP search 明确排除。完整 manifest 可针对这些不可变 hash 使用
`git show --no-renames` 重现，命令输出不纳入版本控制。

完整 hash 依次为：`b8d67c96be2d29e6477838adbbb3ee6fec27ddf5`、
`5f61674788ae9543c4769b1eacbc74d24296012b`、
`25bcb176ebe3c4380f84eac6c3777f930f9ba47e`、
`32f7d5aa76fb9f7f5982d856497b480bbf7f3b3f`、
`e0bca1eb4d04e99fa8048deb9b2bb1731a1ac6b4`、
`4efac629a57571497695352cf8448980be4e418b`、
`589307646757d7f43afda299b866b2cfcf874ac2`、
`e5089f1886b0fb924b8f721966cf1bb511be7395`、
`b9db32a86af51e8ea976b81c2c15cbc3126dd006`、
`ae0b01fbb516a4fbd834adda5643b9a16d4b8041` 和
`b5b83d7d6f470c363e1044cd6b0266604ad5aaa1`。

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
