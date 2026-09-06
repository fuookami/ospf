# ospf-rust

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust` 是 OSPF 的 Rust 实现与迁移 workspace，提供基础工具、数学基础、物理量、优化核心建模、framework 求解编排，以及 BPP3D、CSP1D、Gantt Scheduling 等领域框架。

更完整的 OSPF 项目与发布文档见：

- ospf：<https://github.com/fuookami/ospf>
- 文档：<https://fuookami.github.io/ospf/>

## 作用范围

本 workspace 负责可复用 Rust crate 与 framework 迁移内容，沉淀领域无关基础设施、优化建模原语和可复用领域框架。

明确非目标：

1. 业务专用请求协议、租户上下文、公式语言和运行时部署适配。
2. 外部 renderer 实现。
3. solver 安装与许可证管理，除 feature-gated adapter 文档说明外不在仓库内处理。

## 模块结构

| Rust crate | Kotlin 边界 | 职责 |
| --- | --- | --- |
| [`ospf-rust-base`](ospf-rust-base/README_ch.md) | `ospf-kotlin-utils` foundation | 错误处理、索引类型、集合、容器、迭代器和可克隆函数辅助。 |
| [`ospf-rust-multiarray`](ospf-rust-multiarray/README_ch.md) | `ospf-kotlin-multiarray` | 泛型多维数组、shape、view、存储顺序和 block array。 |
| [`ospf-rust-math`](ospf-rust-math/README_ch.md) | `ospf-kotlin-math` | 代数、几何、普通数学、运算符、混沌系统、分形、组合数学和符号计算。 |
| [`ospf-rust-quantities`](ospf-rust-quantities/README_ch.md) | `ospf-kotlin-quantities` | 物理维度、单位、编译时/运行时物理量和物理量算术。 |
| [`ospf-rust-core`](ospf-rust-core/README_ch.md) | `ospf-kotlin-core` | 变量、token、symbol、`MetaModel`、flatten、solver trait、solver output、IIS 和 backend adapter。 |
| [`ospf-rust-framework`](ospf-rust-framework/README_ch.md) | `ospf-kotlin-framework` | pipeline 建模、shadow price、列生成、Benders、组合求解器、持久化契约、远程求解客户端和心跳工具。 |
| [`ospf-rust-framework-bpp3d`](ospf-rust-framework-bpp3d/README_ch.md) | `ospf-kotlin-framework-bpp3d` | 可复用三维装箱框架，包括 BPP3D context、层生成/分配、packing、CSV fixture 和 renderer DTO。 |
| [`ospf-rust-framework-csp1d`](ospf-rust-framework-csp1d/README_ch.md) | `ospf-kotlin-framework-csp1d` | 可复用一维分切框架，包括 material、generation、produce、yield、waste、length 和 application flow。 |
| [`ospf-rust-framework-gantt-scheduling`](ospf-rust-framework-gantt-scheduling/README_ch.md) | `ospf-kotlin-framework-gantt-scheduling` | 可复用 Gantt 排程框架，包括 task、bunch、capacity、resource、produce 和 branch-and-price flow。 |
| [`ospf-rust-framework-network-scheduling`](ospf-rust-framework-network-scheduling/README_ch.md) | `ospf-kotlin-framework-network-scheduling` | 通用 network flow、VRPTW、ESPPRC 定价、路线编译和 Branch-and-Price 框架。 |
| [`ospf-rust-example`](ospf-rust-example/README_ch.md) | `ospf-kotlin-example` | 可运行示例和迁移兼容 demo。 |

## 架构概览

workspace 采用分层结构：

1. `base`、`multiarray`、`math` 和 `quantities` 提供可复用基础。
2. `core` 拥有优化建模原语与面向 solver 的模型转换。
3. `framework` 增加求解编排、pipeline 抽象、shadow price、持久化契约和远程求解。
4. 领域 framework crate 围绕 `MetaModel` 装配可复用业务领域建模 context。
5. `example` 展示当前 public flow 与迁移兼容路径。

framework 领域 crate 应把优化语义放在 context / aggregation / model component / pipeline 层。application service 负责 solver 选择、生命周期、trace/KPI/render 组装和 recovery 边界。

## 约束规划边界

`ospf-rust-core` 提供精确 `i64` 约束规划模型，包含 immutable snapshot、稳定 ID、源模型复验和统一
`SolveReport<i64>`。通用 MIP lowerer 在内部使用 checked `i128`，只有当整数系数、边界和生成的 Big-M
都能无损表示时才跨过现有 `f64` solver 边界；当前数值门禁为 `2^53`。

SCIP CP 入口是 feature-gated 的严格有限 MIP-backed facade，不是 native SCIP/CIP CP backend。CP
声明能力范围已经完成：通用 MIP lowering 对已支持的有限子集返回经过复验的 `ExactLowering`，
对 `Cumulative`、`Circuit`、`Automaton` 和 `Reservoir` 明确返回结构化 `Unsupported`；cumulative
raw FFI 是 `Conditional` 研究探针，真实增量 CP session 仍是 `Unsupported`。snapshot-rebuild
session 是正确但可能较慢的重建路径，不得描述为 native incremental resume。fake CP solver
仅用于合同测试和小型穷举 oracle，不是生产级搜索器。

## 文档模板

新增或整理 README 时应参考：

- [English template](docs/README_TEMPLATE.md)
- [中文模板](docs/README_TEMPLATE_ch.md)

crate 级 README 使用完整模板。内部 `src/domain`、`src/application`、`src/infrastructure` README 可使用精简骨架：职责、文件结构、Public API、扩展点、生命周期/数据流、验证和相关模块。

## 使用方式

在本仓库开发时使用 path dependency：

```toml
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core" }
ospf-rust-framework = { path = "../ospf-rust-framework" }
```

领域框架可直接引入：

```toml
[dependencies]
ospf-rust-framework-csp1d = { path = "../ospf-rust-framework-csp1d" }
ospf-rust-framework-gantt-scheduling = { path = "../ospf-rust-framework-gantt-scheduling" }
ospf-rust-framework-network-scheduling = { path = "../ospf-rust-framework-network-scheduling" }
```

## 本地验证

```powershell
cargo check --workspace
cargo test --workspace --no-run
```

聚焦开发时优先使用 package 级命令：

```powershell
cargo check -p ospf-rust-core
cargo test -p ospf-rust-framework-csp1d
```

solver-backed 测试需要对应 Cargo feature 和本地 solver 安装或 bundled 支持。core solver 说明见 [Gurobi](ospf-rust-core/src/solver/solvers/gurobi/README_ch.md) 与 [SCIP](ospf-rust-core/src/solver/solvers/scip/README_ch.md)。

## 当前边界

本仓库正在把 Kotlin framework 能力迁移到 Rust。部分领域 crate 已暴露 Kotlin 对齐 public surface，但生命周期中的一些阶段仍使用 Rust 侧确定性、fake 或 feature-gated solver 路径。每个领域 crate README 会记录自身覆盖范围与已知差距。

`ospf-rust-framework-network-scheduling` 已接入 workspace，`99/99` 迁移全部完成，并具备 graph/flow、VRPTW、ESPPRC、路线编译和 Branch-and-Price 的离线覆盖。其 Gurobi/SCIP Demo5 验证仍由 feature 控制并依赖本机 native solver 环境；长期数值、正确性与 E2E 边界记录在 crate README 中。

## 相关模块

- [OSPF Kotlin workspace](../ospf-kotlin/README_ch.md)
- [OSPF Rust examples](ospf-rust-example/README_ch.md)

## 许可证

本项目基于 MIT 许可证发布。详情见 [LICENSE](LICENSE)。
