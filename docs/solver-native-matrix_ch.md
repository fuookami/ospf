# Solver 原生验收矩阵

:us: [English](solver-native-matrix.md) | :cn: 简体中文

本文定义 Rust solver 合同的可执行矩阵。它是验收流程，不是自动生成的测试报告。只有在
实际拥有指定 backend 动态库和许可证的环境中执行的命令，才可以作为原生证据。

## 规则

1. `cargo check` 只能证明 feature 接线和编译，不能证明原生 backend 可用。
2. 原生 backend 无法编译或加载时不得计为通过。环境明确不提供 backend 时记为
   `unsupported`；请求执行该门禁但失败时记为 `failed`。
3. 标记为 `ignored` 的测试必须使用 `-- --include-ignored`。无 feature 的合同 target
   应显示显式 ignored，而不是显示零测试后空通过。
4. 原生 golden/replay 比较至少包括 problem status、termination、incumbent objective、
   best bound、gap、solution vector、constraint residual、model/configuration/solver
   fingerprints 和 provenance。

## 矩阵

| 范围 | 命令形态 | 必须如何解释 |
| --- | --- | --- |
| 无 backend 的 core | `cargo test -p ospf-rust-core --test native_contract_suite` | 必须编译并报告 `ignored/unsupported`，不能算原生通过。 |
| Gurobi 10/11/12 shared contract | `cargo test -p ospf-rust-core --test native_contract_suite --features gurobi10`（按环境替换 feature） | 必须有匹配的 Gurobi 安装和许可证，原生合同与 replay 测试必须实际执行。 |
| SCIP shared contract | `cargo test -p ospf-rust-core --test native_contract_suite --features scip` | 必须有 SCIP header、library 和运行时配置；bundled/from-source 必须显式写 feature。 |
| 原生 release 终态矩阵 | `cargo test -p ospf-rust-core --test native_release_matrix --features gurobi10 -- --include-ignored` 及对应的 `scip` 命令 | 执行 limit、取消、无界和 incumbent 保留夹具。不带 `--include-ignored` 的 ignored 只是环境边界，不是通过。 |
| Gurobi 诊断与 QP | `cargo test -p ospf-rust-core --test gurobi_linear_dual_integration --features gurobi10`、`gurobi_linear_farkas_dual_integration`、`gurobi_iis_integration`、`gurobi_quadratic_model_integration` | 原生 dual、Farkas、IIS 和 QP 证据；每个 target 必须实际执行，不能只编译。 |
| SCIP report 与 observer | `cargo test -p ospf-rust-core --test scip_report_integration --features scip` 及 `scip_native_observer_integration` | 原生 report、证书、callback 和 observer 证据。 |
| core feature 接线 | `cargo check -p ospf-rust-core --no-default-features` 及各 backend feature | 仅为编译证据。 |
| framework 同步 | `cargo test -p ospf-rust-framework --no-default-features` | 覆盖无原生 backend 的 report-first framework adapter。 |
| framework async/remote | `cargo test -p ospf-rust-framework --features async` 和 `cargo test -p ospf-rust-framework --features remote-solver` | 覆盖取消、remote DTO、checkpoint 和 identity 合同。 |
| Network library gate | `cargo test -p ospf-rust-framework-network-scheduling` 加选定 native feature | 离线测试可以无 solver 通过；原生 branch-and-price 证据必须单独记录。 |
| Demo5 原生门禁 | 使用 `ospf-rust-example/src/framework/demo5/README.md` 中的命令并加 `-- --include-ignored` | parser/adapter target 不能替代 `--lib` direct-MIP、smoke 和 strict-proof 门禁。 |

## 已记录证据（2026-08-13）

以下命令已在当前 Windows 环境执行。原生版本以 backend 实际报告为准，不能从 Cargo
feature 名称推断：

| backend/范围 | 结果 |
| --- | --- |
| 无 feature：`native_contract_suite`、`native_release_matrix` | 不带 `--include-ignored` 时各报告 `1 ignored`；没有把它们算作 native 通过。 |
| Core/framework 离线（`--lib`） | core 无 feature `499/499`；core `serde` `508/508`；framework 无 feature `194/194`；framework `async` `159/159`；framework `remote-solver` `236/236`。旧的 `476/476` 与 `507/507` 属于回归测试补齐前的历史基线。 |
| Gurobi `10.0.1` / `gurobi10` | shared contract/replay `1/1`；release matrix `9/9`，连续三次完整运行均通过；dual `5/5`；Farkas `2/2`；IIS `4/4`；QP `23/23`；native observer `2/2`；Benders `3/3`。 |
| SCIP `9.2.4` / 非 bundled `scip` | shared contract/replay `1/1`；release matrix `10/10`（含 memory limit）；report `2/2`；native observer `3/3`。 |
| Network Scheduling | 默认 `50/50`；`serde` `50/50`；`big-decimal` `53/53`；Gurobi 与 SCIP library gate 各 `52/52`。结合已完成的跨计划合同矩阵，Network 交付按 `99/99` 正式关闭。 |
| Demo5 | Gurobi integration target `3/3`、SCIP integration target `2/2`；必需的 `--lib` direct-MIP、smoke 和 strict-proof 门禁也已执行。 |

Gurobi 11 和 12 的 feature check 可以编译，但当前没有匹配的原生安装，因此 native
行记为 `unsupported`。当前安装目录是 `gurobi1001`，使用 `gurobi11` feature 配置该目录
不能作为 Gurobi 11 证据。

`scip-bundled` 与 `scip-from-source` 探针记为 `unsupported`，原因是构建下载 Windows SCIP
包时远端 TLS 连接被关闭。非 bundled SCIP `9.2.4` 是独立证据，且已实际执行。

release fixture 覆盖 Gurobi/SCIP 的 node、iteration、time、gap、solution limit、取消、无界
以及有/无 incumbent 投影；每个有 incumbent 的 MIP limit fixture 还校验有限 best bound、
最大化 bound 方向，以及相对于 incumbent 和 bound 重算的 absolute/relative gap。Gurobi
time-limit-with-incumbent fixture 使用固定 seed 和非零短时限，完整 9 项矩阵连续三次运行均通过。
SCIP 另外覆盖 memory limit。`grb 3.0.1` 的 status enum 不暴露
Gurobi memory-limit 终态，因此该行明确记为 unsupported，不静默映射到其它 termination。
歧义 `InfOrUnbd` 由 binding mapping 和 native 消歧测试覆盖；backend 不会宣称未消歧结论为
精确结论。

截至 2026-08-13，feature 编译、framework sync/async/remote 测试、带 `--no-deps` 的严格
Clippy、局部格式、missing-docs rustdoc 和 `git diff --check` 均通过。包含 workspace 依赖的
Clippy 命令仍会触发既有 `ospf-rust-base`/`ospf-rust-math` warning；该结果记录为 workspace
依赖基线，不作为 solver crate 证据。

另见[统一求解合同](solve-contract_ch.md)与[source traceability](solver-traceability_ch.md)。
