# Solver Source Traceability

:us: [English](solver-traceability.md) | :cn: 简体中文

本文档是统一 solver 迁移所覆盖 11 个 Kotlin solver-contract 提交的最终
source traceability 记录。参考源仓库为
`E:\workspace\ospf\ospf-kotlin`，Rust 仓库为当前工作区。

`文件`列给出 `git show` 记录的精确文件数和可重现完整清单的命令；后面的
Kotlin 文件列是该完整清单中的 solver 相关子清单。其余文件属于领域文档、
benchmark/example 接线或 CP 专属变更，由文末边界矩阵明确处置。

| # | Kotlin 提交 | 日期与主题 | 文件数 | Solver 相关 Kotlin 文件 | Rust 实现与测试 | 最终净行为 | 处置 |
| --- | --- | --- | ---: | --- | --- | --- | --- |
| 1 | `b8d67c96be2d29e6477838adbbb3ee6fec27ddf5` | 2026-07-28，`feat(solver): add unified solve reporting and progress contracts` | 29 | `ospf-kotlin-core/src/main/.../solver/LinearSolver.kt`；`QuadraticSolver.kt`；`SolveOptions.kt`；`config/SolverConfig.kt`；`progress/ProgressAdapters.kt`；`progress/SolverProgress.kt`；`report/NormalizedModel.kt`；`report/OfflineExperiment.kt`；`report/SolveReport.kt`；对应 core progress/report 测试；Gurobi/SCIP linear/quadratic adapter；framework `ColumnGenerationSolver.kt`、serial/parallel combinatorial solver、remote linear/quadratic client、`RemoteReportMapping.kt` | `ospf-rust-core/src/solver/report/mod.rs`、`progress.rs`、`solver.rs`、`solver_ext.rs`；`ospf-rust-framework/src/solver/column_generation_solver.rs`、serial/parallel wrapper 和 remote serializer；core/framework report 与 progress fixture | 引入一个经过校验的 report/progress 合同和首批 report-first adapter 入口。 | Gurobi/SCIP 与 framework 范围已实现；非 Gurobi/SCIP adapter 排除。 |
| 2 | `5f61674788ae9543c4769b1eacbc74d24296012b` | 2026-07-29，`fix(solver): propagate termination status through all adapters` | 24 | COPT `CoptLinearSolver.kt`/`CoptQuadraticSolver.kt`；CPLEX 对应文件；Hexaly 对应文件；MindOPT 对应文件；SCIP column-generation adapter；network status/algorithm/test；framework Benders/ColumnGeneration/remote client 与测试 | `ospf-rust-core/src/solver/report/mod.rs`、`solver.rs`；framework report gate、fallback policy 和 remote legacy projection 测试 | 在 adapter 边界保留终止原因、incumbent 存在性和 fallback 行为。 | COPT、CPLEX、Hexaly、MindOPT 源行为明确排除；Rust 验收仅覆盖声明的 Gurobi/SCIP 范围。 |
| 3 | `25bcb176ebe3c4380f84eac6c3777f930f9ba47e` | 2026-07-29，`fix(solver): enforce terminal-state contracts in Benders and branch-and-price` | 11 | example Benders solver；network `BranchNodeSolver.kt`；framework `ColumnGenerationSolver.kt`、solver README 与 value-conversion 测试 | `ospf-rust-framework/src/solver/linear_benders_decomposition_solver.rs`、`column_generation_solver.rs`、`core_extensions.rs`；solver contract fixture | 精确 master/subproblem gate 拒绝不完整 proof，并保留结构化终态。 | Part II 算法行为已实现；CP 仍属 Part III。 |
| 4 | `32f7d5aa76fb9f7f5982d856497b480bbf7f3b3f` | 2026-07-29，`fix(solver): preserve Gurobi LP infeasibility through branch-and-price` | 9 | Gurobi/Gurobi11/SCIP column-generation adapter；network B&P regression；example Benders terminal test | Gurobi/SCIP report 与 infeasibility certificate mapping；framework Benders/column-generation certificate gate 与测试 | 已验证 LP 不可行作为数学终态保留，不再改写为 node/backend failure。 | 仅 Gurobi/SCIP；CP plan 文件不由本行实现。 |
| 5 | `e0bca1eb4d04e99fa8048deb9b2bb1731a1ac6b4` | 2026-08-09，`feat(contract): complete solve report identity and remote adapter migration` | 196 | core `model/intermediate/*`；`solver/CoreSolverAsync.kt`、`LinearSolver.kt`、`QuadraticSolver.kt`、`ModelingPreparation.kt`、`SolverExt.kt`、`report/*`、`output/SolverOutput.kt`、`value/*`；Gurobi/SCIP solver、Benders、column-generation adapter/test；framework Benders/ColumnGeneration/combinatorial/remote 源码与测试；remote serialized-model fixture | `ospf-rust-core/src/solver/audit.rs`、`fingerprint/`、`report/`、`checkpoint.rs`；`ospf-rust-framework/src/solver/remote/domain.rs`、`ospf_serializer.rs`、`client.rs`；稳定身份、fingerprint、checkpoint、remote 测试 | 保留稳定 model identity、report provenance/fingerprint、remote schema 校验、artifact integrity 和 cancellation/checkpoint identity。 | CPLEX、COPT、Hexaly、MindOPT、MOSEK 与 CP 实现文件排除 Rust backend 验收；CP 专用 checkpoint 交付由 Part III 单独跟踪，当前已完成。 |
| 6 | `4efac629a57571497695352cf8448980be4e418b` | 2026-08-09，`fix(solver-report): complete capability, attempt trace, and provenance metadata` | 23 | Gurobi/SCIP runtime solver metadata/status test；core identity/provenance 源码与测试；framework combinatorial/remote client 与测试 | `ospf-rust-core/src/solver/solver.rs`、`report/mod.rs`、`fingerprint/`；framework attempt aggregation、provenance 和 remote metadata 路径 | runtime capability、effective configuration、provenance 与 attempt identity 在声明的 backend 范围内显式且确定。 | Gurobi/SCIP 与 backend-neutral framework 已实现；其它 native backend 排除。 |
| 7 | `589307646757d7f43afda299b866b2cfcf874ac2` | 2026-08-09，`fix(solver-report): preserve aggregate provenance and attempt traces` | 15 | core intermediate identity/report 源码与测试；framework `CombinatorialSolveSupport.kt`、serial/parallel combinatorial solver 与 identity/selection test | `ospf-rust-framework/src/solver/column_generation_solver.rs`、serial/parallel combinatorial linear/quadratic 与 column-generation wrapper；aggregation 测试 | aggregate report 保留 parent/child attempt identity、provenance、selected attempt 和确定性 trace 顺序。 | framework 已实现；不代表 CP 或新增 backend。 |
| 8 | `e5089f1886b0fb924b8f721966cf1bb511be7395` | 2026-08-10，`fix(solver-report): close aggregate identity and attempt cancellation boundaries` | 9 | core identity propagation test；framework serial/parallel combinatorial solver 与 identity test | framework cancellation linearization、completion snapshot 和 aggregate validation 测试 | backend 已完成后的迟到 shared cancellation 不能覆盖结果；aggregate identity 稳定。 | framework 已实现。 |
| 9 | `b9db32a86af51e8ea976b81c2c15cbc3126dd006` | 2026-08-10，`fix(solver-report): preserve in-flight cancellation reasons` | 6 | framework combinatorial support、serial/parallel combinatorial solver/test | `ospf-rust-framework/src/solver/column_generation_solver.rs`、serial/parallel wrapper 与 cancellation 测试 | in-flight cancellation origin 保留在 child attempt 和 aggregate report 中。 | framework 已实现。 |
| 10 | `ae0b01fbb516a4fbd834adda5643b9a16d4b8041` | 2026-08-10，`fix(solver-report): freeze combinatorial cancellation metadata at backend completion` | 6 | framework combinatorial support、serial/parallel combinatorial solver；parallel selection test | framework completion marker 和 stop-condition 测试 | cancellation metadata 在 backend completion linearization point 冻结。 | framework 已实现。 |
| 11 | `b5b83d7d6f470c363e1044cd6b0266604ad5aaa1` | 2026-08-10，`test(solver-report): cover parallel cancellation reason propagation` | 10 | core report identity 源码；framework combinatorial support、parallel selection test；删除 Kotlin CP plan 文件并更新 release/solver CP plan | framework parallel cancellation regression 与 shared report fixture | parallel loser cancellation 可观测，且不会改变已选择的 completed attempt。 | solver 行为已实现；删除的 CP-only Kotlin plan 文件明确排除 Rust 交付。 |

## 完整 manifest

每一行的完整文件清单（包括非 solver 文件）都可使用完整 hash 重现：

```text
git -C E:\workspace\ospf\ospf-kotlin show --no-renames --format="COMMIT %H%nSUBJECT %s%nDATE %ad" --date=short --name-status <full-hash>
```

按表格顺序记录的 manifest 文件数为：`29, 24, 11, 9, 196, 23, 15,
9, 6, 6, 10`。因此短 hash 或过滤后的表格不会被误认为是 source manifest。

上述不可变完整 hash 和复现命令就是受版本控制的 manifest 证据。
`git show` 输出属于生成的验证材料，按仓库规则不提交。

## Rust 所有权与排除矩阵

| 源范围 | Rust 处置 | 证据边界 |
| --- | --- | --- |
| core report、proof、diagnostics、identity、fingerprint、progress、cancellation、checkpoint | 在 `ospf-rust-core` 实现 | fake contract fixture、report/proof 测试和 feature check |
| Gurobi | 已实现并 feature-gated | Gurobi 可用时运行 native/shared contract；不可用时显式 unsupported |
| SCIP | 已实现并 feature-gated | SCIP 可用时运行 native/shared contract；不可用时显式 unsupported |
| framework serial/parallel、Benders、column generation、remote | 已实现 | framework solver 测试与 remote round-trip/legacy 测试 |
| Network/Gantt consumer | 仅消费方接线 | Network 由 crate README 所有并已按 `99/99` 关闭；Gantt 继续由领域 README 与测试所有。 |
| CPLEX、COPT、MindOPT、Hexaly、MOSEK | 排除 | 不提供 Rust adapter、placeholder capability 或验收声明 |
| PSO | 既有无关 heuristic backend | 不计入 native solver-contract backend |
| CP、Logic-Based Benders、portable checkpoint rebuild | Part III Rust 交付 | CP 声明范围已完成（`66/66`）；exact lowering、verified conflict、remote materialization、portable rebuild 和 Gantt differential 均由 CP 自有源码/测试覆盖。原生增强能力明确为 `Conditional`/`Unsupported`，不冒充 Native。 |

## 兼容性证据

除模块内部的 report 测试外，兼容窗口还由以下源码级回归测试目标验证：

```text
cargo test -p ospf-rust-core --test solver_legacy_api_compat
cargo test -p ospf-rust-framework --test solver_legacy_api_compat
cargo test -p ospf-rust-framework --features remote-solver --test solver_remote_legacy_api_compat
```

core 目标验证旧 solver trait、deprecated 的 `solver_output` 与
`solver_config` 路径、typed 转换，以及有损的
`SolveReport -> SolverOutput -> SolveReport` 投影。framework 目标验证
`FeasibleSolution` 与 `LPResult` 往返，并拒绝没有证书的 LP 报告。remote
目标验证旧 `SerializedSolution` JSON、不可行/无界构造器、报告投影、未知
报告 schema 拒绝，以及 `stop_legacy` 布尔兼容 facade。

这些测试保留旧 API 的读取/转发窗口，同时禁止新算法继续扩展旧终态模型；
新代码消费 `SolveReport` 及其 certificate gate。

## 审计规则

- 每个 source commit 必须映射到实现、测试或明确排除；只看 source diff
  不能作为完成证据。
- 完整 manifest 可由上述不可变 hash 重现；表中的 solver 子清单不表示忽略
  了其它文件。生成的 manifest 输出不纳入版本控制。
- native 行遵守 [`solver-native-matrix_ch.md`](solver-native-matrix_ch.md)；
  只编译不构成 native 证据。
- 本文档是权威的来源迁移状态与追溯记录；公共行为由
  [`solve-contract_ch.md`](solve-contract_ch.md) 定义，Network 交付与验收由
  [Network Scheduling README](../ospf-rust-framework-network-scheduling/README_ch.md) 定义。
