# 统一求解合同

本文说明 `ospf-rust-core` 与 `ospf-rust-framework` 共享的公共求解合同。英文版本见
[`solve-contract.md`](./solve-contract.md)。

## 交付状态

统一 solver 迁移已于 2026-08-14 正式关闭，`201/201` 项实现与验收全部完成：Part I
公共执行合同 `42/42`、Part II Gurobi/SCIP 与算法终态证书迁移 `66/66`、Part III
CP/Logic-Based Benders `66/66`，以及 27 条跨部分验收标准。原生 backend 范围仍仅为
Gurobi 与 SCIP。验收矩阵中冻结为 `Conditional` 或 `Unsupported` 的能力是明确产品边界，
不能把未执行工作写成 native 支持。

## 报告优先

新的 solver-facing 代码应消费 `ospf_rust_core::solver::SolveReport<V>`。
`SolverOutput`、`FeasibleSolution`、`SolveResult` 和 `SerializedSolution` 仍是兼容投影，
新算法代码不得从它们推断证明或取消语义。

报告正交保存：

- `problem_status`：`Feasible`、`Unknown`、`Infeasible`、`Unbounded` 或
  `InfeasibleOrUnbounded`；
- `termination_reason`：完成、后端限制、取消、中断、数值失败或后端失败；
- `solution`：实际通过验证的 incumbent 及其目标值；
- `proof`：最优性、不可行或无界证明；
- `statistics`、`diagnostics`、`provenance`、`fingerprints` 和算法 trace。

进度回调使用经过校验的 `SolveProgressSnapshot` 和稳定 stage path。已知百分比
会限制在 `0..=100`，未知总量报告为 indeterminate。新代码应保持报告和进度
路径分离：

```rust
let report = solver.solve_linear_report(&model)?;
if report.is_optimal() {
    consume_verified_report(&report)?;
}
```

旧的 `solve_linear`/`SolverOutput` 投影继续用于兼容，但不携带 proof、provenance、
fingerprint 或取消详情；新的精确算法门禁不得从该投影推断这些语义。

报告必须通过受校验 builder 构造。可行报告必须有 incumbent，不可行或无界报告不能携带
incumbent；已验证最优证明必须对应完成终止和可靠、完整的证明。只有同时存在 incumbent
目标和有效下界且二者一致时，gap 字段才有效。

## 错误边界

`SolverErrorClass` 是稳定的错误分类边界，包含 `INPUT`、`MODELING`、
`ENVIRONMENT`、`LICENSE`、`CALLBACK`、`BACKEND`、`PARSING`、`NUMERICAL`、
`INTERNAL_CONTRACT`、`TERMINAL_PROJECTION` 和 `UNSUPPORTED`。Gurobi/SCIP
原生执行失败仍归类为 `BACKEND`；模型装配、非法报告、回调失败、损坏
artifact 和运行时误用分别保留自己的分类。取消等正常终态由 `SolveReport`
表达；旧接口可以将其投影为 `SolverError::Cancelled`，该错误属于终态投影，
不属于后端失败。

## 证明门禁

精确算法必须使用 `ospf_rust_core::solver` 中的集中 helper：

- 最优 LP 消费方要求完成的可行报告、已验证最优证明、匹配的模型指纹、对偶向量、维度、
  目标值和残差检查；
- 不可行消费方要求完成的不可行报告、已验证完整证明和匹配的模型指纹；
- IIS 与 Farkas 是附着在报告上的诊断证据。诊断失败不能覆盖已经确定的数学结论。

限制或中断时返回的 incumbent 可以作为候选解，但不能关闭精确下界或生成最优证明。

## 取消与异步执行

每次求解拥有独立的 `SolveHandle`。取消幂等，记录首次 `CancellationOrigin` 和时间，
并调用所有已注册的后端 interrupter。`spawn_solve_report_with_options` 把阻塞求解放入
Tokio blocking pool；`cancel`/`abort` 会先请求后端中断，需要等待资源释放完成时使用
`cancel_and_wait`。

组合 wrapper 会把取消来源保留在子 attempt，并将 loser attempt 写入最终 trace。若取消发生在
attempt 的 `completedAt` 线性化点之后，不得改写已经完成的报告。

## 身份与重放

报告携带稳定模型元素映射，并分别保存模型、实际配置和 solver 环境指纹。指纹使用确定性
规范化编码、排序后的稀疏项、显式数值标签和 SHA-256 domain separation；不得使用对象地址、
进程全局 ID、默认哈希迭代顺序、callback 地址或 callback Debug 文本作为重放身份。

Callback 属于执行行为而不是可重放配置。后端 provenance 记录 callback 是否存在；注册 callback
时，报告增加 `NonReplayableCallback` warning。

## Remote 边界

版本化 remote report DTO 保留报告 schema、run/attempt identity、artifact digest、provenance、
fingerprints、diagnostics、proof、incumbent、bound 和 solution pool。task 生命周期与数学结论
分离。未知未来 schema、身份错配和 artifact 错配必须结构化拒绝。`stop` 返回
`StopAcknowledgement`；布尔 `stop_legacy` 仅为兼容入口。

Portable checkpoint artifact 使用 schema `1.0` 的 `RemoteCheckpointArtifactDto`。
`checkpoint_artifact_to_json` 与 `checkpoint_artifact_from_json` 会校验 schema
和状态摘要；`store_checkpoint_artifact` 与 `load_checkpoint_artifact` 提供对象
存储边界。读取时会拒绝对象缺失、ETag 不匹配、artifact 损坏，以及通过
`CheckpointResumeExpectation` 提供的 run/model/config/solver 指纹不匹配。
该类型和兼容入口 `validate_resume` 不建立源 attempt 证明；精确恢复必须使用
`CheckpointResumeExpectationWithAttempt`、`load_checkpoint_artifact_from` 和
`validate_resume_from`，校验源 attempt、父子 attempt 链、保持不变的 provenance、
匹配的指纹以及取消链前缀。HTTP execution port 在发送服务端 resume 请求前完成
artifact 身份检查，因此被拒绝的 artifact 不会启动远程 attempt。若远程 action
提供了这些元数据，`StopAcknowledgement` 会透传相同的 run/attempt identity、指纹、
provenance、取消来源和消息。子 attempt 必须是带显式 parent 的新 attempt，不能覆盖
源 checkpoint 身份。

## 后端范围

本次迁移的精确 backend 范围是 Gurobi 和 SCIP。Cargo feature 存在不代表原生库或许可证可用，
因此 runtime descriptor 在 backend probe 成功前只能报告 conditional 能力。缺少 native 环境的
项目必须在矩阵中标记 ignored 或 unsupported，不得计为通过。
原生 backend 错误与普通后端执行失败分开分类：Gurobi License 错误（包括原生错误码
`10009`）使用稳定的 `LICENSE` 类；缺少动态库或环境配置仍使用 `ENVIRONMENT`。

另见：

- [`ospf-rust-core` README](../ospf-rust-core/README_ch.md)
- [`ospf-rust-framework` README](../ospf-rust-framework/README_ch.md)
- [native 验收矩阵](./solver-native-matrix_ch.md)
- [source traceability](./solver-traceability_ch.md)
- [CP 原生能力矩阵](./constraint-programming-native-matrix_ch.md)
