# Solver 终态丢失清单

:us: [English](solver-terminal-loss-inventory.md) | :cn: 简体中文

本清单记录 Rust 边界曾经丢失的终态信息，以及当前替代合同的归属。它是审计地图；native
环境条目仍受各自明确的能力边界约束。

| 边界 | 过去的丢失或歧义 | 当前 Rust 所有者 | 回归面 |
| --- | --- | --- | --- |
| Core status | 一个 `SolverStatus` 混合表达问题结论、termination 和执行阶段。 | `ospf-rust-core/src/solver/report/` 的 `ProblemStatus`、`TerminationReason`、`SolveReport` 和 builder。 | core report 合同测试与共享 fixture。 |
| Typed/value projection | `FeasibleSolution`/typed conversion 丢失 status、proof 和 provenance。 | report-first solver extension 与显式 legacy projection。 | core solver extension、typed conversion 测试。 |
| Column Generation | LP/MILP 入口只能返回 feasible DTO，使 dual 看起来像证明。 | framework column-generation report aggregation 和 optimal-LP certificate helper。 | column-generation、serial/parallel、dual gate 测试。 |
| Benders | 仅有 solution 就可能进入 subproblem 或生成没有可靠证明的 cut。 | master/subproblem report gate、dual/Farkas evidence helper 和结构化 stop result。 | 线性/二次 sync/async Benders 合同测试。 |
| Branch-and-Price | node bound、pricing completeness 与 solver termination 没有正交表达。 | node conclusion、solver report、pricing-complete、inherited/certified bound 和 certificate gate。 | Gantt 与 Network branch-node 测试；Network 跨计划关闭已完成。 |
| Combinatorial wrapper | fallback 与 First/Best selection 压缩 attempt 和取消原因。 | serial/parallel attempt trace、parent/child identity、完成线性化、loser trace 和 cancellation snapshot。 | combinatorial selection、取消和 progress 测试。 |
| Remote boundary | task lifecycle 与 solve conclusion 混合；legacy status 丢失结构化元数据。 | versioned report/stop DTO、稳定 identity、fingerprint、provenance 和 artifact 校验。 | remote serializer/client/HTTP 测试。 |
| Checkpoint/resume | run、源 attempt、parent、provenance 和取消来源可能丢失。 | portable checkpoint artifact、严格 `*_from` 恢复 API、对象存储 ETag 检查和 stop acknowledgement metadata。 | core checkpoint、remote checkpoint chain、HTTP 测试。 |

## 明确保留的边界

- `CoreError`/`SolverError` 的稳定分类已完成，覆盖输入、建模、环境、许可证、callback、backend、解析、数值、内部合同、终态投影和 unsupported；穷举回归明确 `Err` 与正常终态兼容投影边界。
- 当前可用 Gurobi `10.0.1` 和 SCIP `9.2.4` release 行已有原生证据。Gurobi 11/12
  与 SCIP bundled/from-source 仍明确为 `unsupported`；feature 编译不构成 native 证据。
  跨计划 Network 最终矩阵已关闭。
- Part III CP/Logic-Based Benders 已有版本化 remote result materialization 和 portable
  `RebuildFromSnapshot` checkpoint 路径。两者都不序列化 backend pointer，也不宣称 native
  search-tree resume；真正的增量/native resume 仍在 CP 能力矩阵中明确为 `Unsupported`。
- Rust CP 声明能力范围已完成（`66/66`）。未执行的 native library、bundled 下载或许可证行仍
  记为 `not executed/unsupported`，不能计为 native 通过。
- Network 交付已按 `99/99` 关闭；长期合同与验收边界由 Network Scheduling README
  和 solver native matrix 维护。
