# 约束规划第二阶段：身份、远程契约与条件式原生增强计划

## 1. 文档状态

| 项目 | 内容 |
| --- | --- |
| 状态 | ImplementedWithBoundaries：必做 CP/远程基础能力已实施；稳定 ID 上游契约与条件式 native 能力保留明确边界 |
| 日期 | 2026-08-03 |
| 前置计划 | 原 `plans/constraint-programming.md` 已归档至 `plans/release.md`，并按声明的 MVP 能力边界完成 |
| 上游契约 | `plans/schema.md`，重点依赖 `OSPF-SOL-013`、`OSPF-SOL-022`、`OSPF-SOL-023` |
| 服务端计划 | `E:/workspace/ospf/ospf/framework/remote-solver/daily.md` |
| 后续计划 | `plans/cp3.md` 承接公共契约与源码收尾；`plans/solver_cp.md` 承接 JSCIPOpt 条件能力（当前 `BlockedByUpstream`） |
| 必做范围 | CP 稳定身份接入、远程 `SolveReport` 对齐、portable checkpoint 增强 |
| 条件范围 | JSCIP 增量能力、SCIP optional interval/variable duration 原生化、native checkpoint 调研 |
| 兼容策略 | 保留 model-rebuild、精确 MIP lowering、旧远程 DTO 兼容读取和 legacy checkpoint 读取 |

### 1.1 本轮收尾记录（2026-08-03）

- CP 统一报告直接复用 `SolveReport<Int64>`：CP 解、目标和诊断保留 `Int64`，求解统计固定使用 `Flt64`；已删除迁移期的 `TypedSolveReport`，线性/二次报告仍使用原有 `SolveReport<V>`。
- SCIP 只设置显式线程和随机种子；`deterministic=true` 且未显式指定线程时固定单线程，其余场景保留 SCIP 默认线程。实际线程、随机种子、native/binding/plugin 与运行库元数据通过 provenance 和运行时身份 fingerprint 记录，gap 使用 JSCIP 原生 `getGap()`。
- 远程 calculator 对结果 artifact 使用协议毫秒精度复验，raw/artifact 的终止状态、指纹、统计和诊断必须一致；CP 目标在线上传输只使用 JSON `Long` 的 `objectiveValueInt64`，调度 quantum 不参与语义配置 fingerprint；旧 V2 配置/solver 指纹可按已发布算法迁移读取，但历史 bound/gap 不作为恢复收敛依据。
- 已增加 Int64 目标/变量、MIP 精确重求值、非整毫秒 checkpoint、旧 V2 配置和 deterministic SCIP 回归测试。以下 3169/SCIP 15/服务端 304 统计是历史基线，不代表当前源码；最终收尾统计见“最终收尾复验”一节。数据库重启恢复、完整终态等价矩阵、代表性性能基线以及 `OSPF-SOL-013`、Benders/cut 仍是明确边界。

### 1.2 本轮审查收尾修复（2026-08-03）

- SCIP 当前 runtime fingerprint 已升级为明确的 `scip-runtime-2`：使用构建/plugin 版本、实际可配置的 SCIP/native 版本、binding 版本和原生库内容 SHA-256；查找路径、JAR URL、大小、mtime 与 `java.library.path` 不进入 solver identity，脱敏 provenance 仅记录 explicit/system 加载方式。旧 `scip-runtime-1` 同时包含路径/文件属性代和 502c50060 的内容 SHA 代：路径代只按当前环境可精确重建的 package implementation version/code-source URL、旧 native 优先级和路径属性匹配；内容代在当前库摘要可读时可枚举 explicit/system 两种 mode（摘要不可读时只保留当前 mode 的 unknown）。无法证明旧路径与文件属性相同的路径代跨路径 checkpoint 不兼容。原始固定 `sha256("scip-cp")` 兼容仍保留。
- portable checkpoint v2 恢复现在明确识别已发布的三套配置指纹算法（旧分隔符、旧原始结构化、上一版 task metadata 回退算法），并仅接受明确的历史 `sha256("scip-cp")` 或上述两代旧算法可精确重建的 `scip-runtime-1` 候选；未知 backend、版本推测或篡改值不会被当作 legacy。
- 服务端 provenance 写入 descriptor `pluginVersion`，Kotlin 客户端完整回填；`deterministic=true` 与显式非单线程配置现在返回结构化配置错误，不再产生语义不一致的 provenance。新增跨目录同二进制 fingerprint、历史配置算法、未知 solver fingerprint、pluginVersion 往返和 deterministic 冲突回归测试。
- 2026-08-04 历史全量基线：Kotlin `clean compile test-compile`、全量 `test` 和 `verify` 均通过，Surefire 为 3186 tests、0 failures、0 errors、6 skipped，Failsafe 为 SCIP 18 项、Gurobi 4 项，全部 0 failure、0 error；服务端此前使用 custom local Maven 仓库完成 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，Surefire 为 315 tests、0 failures、0 errors。该记录早于本轮 E2E 测试改动，仅作为历史对照。
- remote-solver 的 POM 仍以 Kotlin `1.1.0` 作为依赖坐标；该版本尚未发布，本轮通过隔离 custom Maven 仓库验证跨仓库源码联调。正式发布流程不属于本计划的 CP 正确性验收，不作为阻塞项。

### 1.3 本轮远程能力探测与协议 fixture（2026-08-04）

- 服务端新增 `/api/v1/capabilities`，返回能力 schema、支持的远程协议版本、在线节点模型类型以及 portable/native checkpoint 能力；HTTP CP 客户端在上传 payload 前强制探测，旧服务端或不支持 CP 的节点会在上传前返回结构化错误。
- 两个仓库共享同一份 canonical CP v2 `SerializedSolution` fixture，覆盖 `objectiveValueInt64`、稳定变量 ID、interval、终止状态、指纹 schema 和小数统计；两侧测试直接解码，fixture SHA-256 为 `EBAD2593BDD9279BFB1D536F1174A328AB774C93B64102234AB52599B26C8691`。
- 本轮只关闭协议字段表、双向 fixture 和 capability/version 探测；当时完整 HTTP + object-storage + dispatcher + calculator、所有终态等价矩阵、数据库重启和 CP2-206/RS-CP-7 其余验收仍未完成，后续已补充单场景 HTTP E2E 证据。

### 1.4 外部 CP worker 与旧协议兼容（2026-08-04）

- remote-solver 的 `OspfExternalProcessBridge` 现在将模型、checkpoint 和结果文件按内容物化到 worker 可读目录；结果 artifact 经过 JSON 校验后上传到租户作用域对象存储，不再把本地路径当作 `resultRef`。
- `RemoteSolverWorkerMain --model-format ospf-cp-snapshot-json` 调用真实 `OspfCpSnapshotExecutor`/SCIP 路径，返回版本化 `SerializedSolution` 和 portable checkpoint；未传入 CP format 时仍保留明确标注的旧进度兼容模式。
- 客户端与服务端新增同一份旧 v1 result artifact fixture；旧 raw-model 提交必须显式使用 `payloadMode=legacy-model`，新 CP payload 解析失败不会静默改变模型语义。
- 本轮只补齐外部执行与旧协议兼容的可验证基础路径；数据库重启、完整终态等价矩阵、发布版本关联和性能基线仍是 ImplementedWithBoundaries 的未完成验收项。

### 1.5 本轮跨仓库 E2E 与 checkpoint 防伪回归（2026-08-04）

- calculator 新增 v2 checkpoint 伪装 `legacy-v1` 回归：合法 digest 的 v2 envelope 不得通过 legacy 降级路径，执行器返回结构化 `BACKEND_FAILURE`，不会注入 incumbent。
- remote-solver dispatcher 新增 Failsafe `RemoteCpHttpE2EIT`：通过 HTTP 提交租户作用域 `SolvePayload`，由内存对象存储承载 payload/result/checkpoint，dispatcher 调度到进程内 SCIP calculator，随后从 HTTP 任务视图和对象存储 `SerializedSolution` 双向核对 `COMPLETED`、稳定变量值、`objectiveValueInt64`、runId 和 solver fingerprint schema；测试 1/1 通过。
- 本项只证明单场景内存对象存储 + 进程内 SCIP 的跨模块链路；数据库重启/回放、外部对象存储、跨模块完整终态组合、本地/远程 differential、性能基线以及 `OSPF-SOL-013`、Benders/cut 仍未关闭。

### 1.6 历史收尾基线（2026-08-04）

- 当时 Kotlin 工作区执行了 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，全部 `BUILD SUCCESS`；Surefire XML 汇总 3187 tests、0 failures、0 errors、6 skipped，Failsafe 汇总 22 tests（SCIP 18、Gurobi 4）、0 failures、0 errors。当前结果见 1.11。
- 当时 remote-solver 使用 `D:\temp\ospf-cp2-local-m2` 中的 Kotlin 制品执行了 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，全部 `BUILD SUCCESS`；Surefire XML 汇总 318 tests、0 failures、0 errors，Failsafe 汇总 1 test、0 failures、0 errors。当前结果见 1.11。
- 安装产物仅写入隔离本地 Maven 仓库；数据库重启、跨模块完整终态组合、稳定 ID 上游契约、Benders/cut 恢复和性能基线仍按边界保留。1.1.0 尚未发布，正式 GAV/revision 关联不属于本计划阻塞项。

### 1.7 本轮 checkpoint 证据与运行时身份补强（2026-08-04）

- core checkpoint restore 现在会拒绝未知 assumption、constraint/domain/bound 成员以及非法 validity/minimality 枚举；远程 calculator 将服务端冲突证据映射到同一 core envelope 后再执行这套结构化复验。
- checkpoint 导出对 `infeasibility.assumptionIds` 和 `infeasibility.members` 的损坏 JSON 或空成员不再静默丢弃，无法形成可信证据时返回空引用；有效 assumptions、conflict members、validity、minimality 和 provenance 会随 v2 envelope 保存。
- SCIP runtime fingerprint 对缺失的相邻 `libscip` 依赖写入 `sha256:unknown`，而不是省略依赖身份；路径、mtime 和 `java.library.path` 仍不进入 solver identity。新增回归覆盖证据拒绝、冲突映射和 runtime identity 边界。
- 本轮定向回归已通过（core checkpoint 8 项、remote calculator 13 项）；随后两仓库全量编译、测试、verify 和 install 均已完成并通过。数据库重启、跨模块完整终态组合、稳定 ID 上游契约、Benders/cut 恢复和真实性能基线仍保持边界；1.1.0 尚未发布，正式唯一 GAV 发布不作为 CP2 阻塞项。

### 1.8 本轮增量证据（2026-08-04）

- SCIP runtime identity 按实际加载的 `jscip` 所在目录、Java/native 搜索路径解析非相邻的 `libscip`（含 Unix 版本化文件名），并将依赖内容摘要纳入 `scip-runtime-2`；路径、搜索目录和文件元数据仍不进入身份。新增回归确认替换非相邻依赖会改变指纹且不会泄露路径。
- portable checkpoint v2 新增重复 decode/restore 幂等回归：同一 envelope 多次恢复保持相同的完整性、父 checkpoint、incumbent、assumption/conflict 和审计字段；这只补齐幂等性质，不等同于真实进程重启回放。
- Benders 已提供 primitive-only、versioned cut serialization SPI、checkpoint codec 和 `resumeState` 主问题注入路径；测试覆盖 schema/ID/Exact proof 门禁、恢复 cut 去重和从指定迭代继续。跨模型重建稳定 binding、业务 cut 自动持久化仍保留边界。
- benchmark 模块新增 snapshot 编码、Fake CP 穷举、固定时长排程和 portable checkpoint 基准夹具；未将 Fake 时间冒充 SCIP/native 正式性能基线。

### 1.9 上一轮最终收尾复验（2026-08-05）

- core checkpoint codec 暴露带 KDoc 的 `withIntegrity` 规范化入口；remote-solver 在恢复前使用同一摘要算法，避免跨仓库 checkpoint 因缺失 core digest 被错误拒绝，同时继续保留严格完整性和模型复验。
- 上一轮 Kotlin 工作区已重新执行 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，全部 `BUILD SUCCESS`；Surefire 为 3195 tests、0 failures、0 errors、6 skipped，Failsafe 为 22 tests（SCIP 18、Gurobi 4）、0 failures、0 errors。
- 上一轮 remote-solver 已使用上述本地 Maven 制品重新执行 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，全部 `BUILD SUCCESS`；Surefire 为 318 tests、Failsafe 为 1 个 HTTP E2E，均 0 failures、0 errors。
- 上一轮验证日志位于 `D:\temp\cp2-final-kotlin-compile-rerun2.log`、`D:\temp\cp2-final-kotlin-test-rerun.log`、`D:\temp\cp2-final-kotlin-verify-rerun.log`、`D:\temp\cp2-final-kotlin-install-rerun.log` 以及对应的 `cp2-final-remote-*` 文件。数据库重启、跨模块完整终态组合、稳定 ID 上游契约、Benders/cut 跨模型恢复和真实性能基线仍按边界保留。

### 1.10 本轮计划收尾（2026-08-05）

- 复核并保留最大化/单目标门禁、SCIP 目标常数与原生 gap、runtime-2 依赖摘要、远程 fingerprint schema/请求绑定/诊断交叉校验、Int64 目标传输及 checkpoint legacy 防伪回归；这些能力已有源码和回归证据，不再重复声明为待修 P1/P2。
- `1.1.0` 尚未发布，隔离本地 Maven 安装只服务两个工作区的源码联调；正式 GAV/revision 发布不属于本计划的正确性关闭条件。
- 仍未完成的条目只保留可证据化边界：全仓库跨重建稳定 ID、真实数据库重启/回放与外部对象存储、跨模块完整终态组合、Benders 跨模型恢复、SCIP native resume 以及真实性能基线。客户端/协议/生命周期状态矩阵已有回归，不将其误写成生产跨模块证据。

### 1.11 本轮最终验收（2026-08-05）

- CP lowering 与 SCIP compiler 现在输出独立 artifact 映射：源变量保留稳定 origin，辅助变量、目标常数和编译约束使用独立 artifact ID；冲突 ID 自动加后缀，无法唯一归属时不伪造 origin。MIP lowerer 与 SCIP 集成回归均覆盖该映射。
- remote-solver 新增 V7 ObjectRef ETag 迁移，并在 Ktorm task/slice 合同测试中覆盖 payload、配置、snapshot、result 和 checkpoint 的 path/version/etag 往返；真实生产数据库迁移与重启回放仍保留环境边界。
- Kotlin 已执行 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，全部 `BUILD SUCCESS`；当前 Surefire XML 汇总为 3202 tests、0 failures、0 errors、6 skipped，Failsafe 为 25 tests（SCIP 21、Gurobi 4）、0 failures、0 errors。
- Kotlin 制品已成功安装到隔离本地仓库 `D:\temp\ospf-cp2-local-m2`；remote-solver 使用同一仓库执行 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，全部 `BUILD SUCCESS`；当前 Surefire 为 321 tests、Failsafe 为 1 个 `RemoteCpHttpE2EIT`，均 0 failures、0 errors。
- 当前完整日志：`D:\temp\cp2-final-kotlin-clean-current-4.log`、`D:\temp\cp2-final-kotlin-test-current-5.log`、`D:\temp\cp2-final-kotlin-verify-current-4.log`、`D:\temp\cp2-final-kotlin-install-current-6.log`，以及 `D:\temp\cp2-final-remote-clean-current-4.log`、`D:\temp\cp2-final-remote-test-current-4.log`、`D:\temp\cp2-final-remote-verify-current-4.log`、`D:\temp\cp2-final-remote-install-current-4.log`。数据库重启/回放、外部对象存储、跨模块完整终态组合、全仓库稳定 ID、Benders 跨模型恢复、SCIP native resume 和真实性能基线仍按边界保留。

### 1.12 当前 checkpoint/rebuild 复验（2026-08-05）

- `ConstraintProgrammingCheckpointProcessTest` 已包含 3 个可执行用例：独立 JVM 解码 checkpoint、无 incumbent 的时间限制恢复、序列化边界后的模型不兼容拒绝；独立 JVM 探针使用测试运行时 classpath 解码同一 JSON，避免测试方法返回非 `Unit` 而未被 Surefire 发现。定向日志 `D:\temp\cp2-checkpoint-process-test-final2.log` 为 3 tests、0 failures、0 errors。
- 该证据覆盖 portable envelope 的跨 JVM 编解码和 core model rebuild；它不等同于 SCIP 原生搜索树 resume，也不能替代真实服务重启、数据库回放或跨 native backend 的完整终态差分。上述场景继续作为环境边界，不将 portable decode 误报为 native resume。

### 1.13 最终全量复验（2026-08-05）

- 当前 Kotlin `clean compile test-compile`、全量 `test`、`verify` 和 `install` 均 `BUILD SUCCESS`；Surefire 汇总 3205 tests、0 failures、0 errors、6 skipped，Failsafe 汇总 25 tests（SCIP 21、Gurobi 4）、0 failures、0 errors。
- 当前完整日志：`D:\temp\cp2-final-kotlin-clean-current-final.log`、`D:\temp\cp2-final-kotlin-test-current-final.log`、`D:\temp\cp2-final-kotlin-verify-current-final.log`、`D:\temp\cp2-final-kotlin-install-current-final2.log`。制品已安装到 `D:\temp\ospf-cp2-local-m2`，供服务端锁步验证。

### 1.14 远程 CP 配置指纹收尾（2026-08-05）

- `RemoteSolverRuntimeConfig` 和 CP 客户端现在支持显式传递完整 `SolverConfig`（时间限制、解数量、gap、线程和扩展参数）；客户端不再以空配置覆盖调用方设置。
- 客户端按服务端相同的有效配置规则计算请求指纹：协议毫秒精度、默认 8 线程和排序后的参数均纳入语义指纹，调度 quantum 仍只属于切片预算，不参与 checkpoint/结果指纹。
- 新增回归测试验证配置字段完整上传且 quantum 不污染配置；客户端定向测试 31 项全部通过。真实数据库重启/回放、外部对象存储、跨重建稳定 ID、Benders 跨模型恢复、SCIP native resume 和真实性能基线仍按既定边界保留。

### 1.15 当前工作区最终复验（2026-08-05）

- 本轮修改后的 Kotlin `clean compile test-compile`、全量 `test`、`verify` 和隔离仓库 `install` 均 `BUILD SUCCESS`；Surefire XML 汇总 3206 tests、0 failures、0 errors、6 skipped，Failsafe 汇总 25 tests（SCIP 21、Gurobi 4）、0 failures、0 errors。
- 当前完整日志：`D:\temp\cp2-final-kotlin-clean-current-next.log`、`D:\temp\cp2-final-kotlin-test-current-next.log`、`D:\temp\cp2-final-kotlin-verify-current-next.log`、`D:\temp\cp2-final-kotlin-install-current-next-retry.log`。制品已安装到 `D:\temp\ospf-cp2-local-m2`，供服务端锁步验证。

### 1.16 Benders 子问题模型身份门禁（2026-08-05）

- `PortableConstraintProgrammingBendersState` 和 framework `BendersResumeState` 新增可选的子问题 snapshot fingerprint；capture、restore 和 `LogicBasedBendersEngine` 共用同一规范编码与 SHA-256 计算，避免恢复时只凭 cut/变量 ID 接受不同模型。
- 使用同一稳定变量 ID 但不同 `origin` 重建的 CP 子问题会产生不同指纹；直接 restore 和 engine resume 均返回 `ORSolutionInvalid`，旧 checkpoint 中没有该字段时继续保留兼容读取。
- 新增回归覆盖 capture、codec restore 和 engine 入口三层门禁；该项只依赖 core snapshot 与 Fake solver，不需要 PostgreSQL、对象存储、外部 worker 或 native runtime。
- 业务 cut serializer 的持久化、全仓库稳定 ID（`OSPF-SOL-013`）、真实服务重启和 native 搜索树续跑仍保持各自边界，不因本地模型指纹门禁而宣称完成。
- 本轮最终本地验证已完成：`clean compile test-compile`、全量 `test`、`verify` 和隔离仓库 `install` 均 `BUILD SUCCESS`；Surefire XML 汇总 3207 tests、0 failures、0 errors、6 skipped，Failsafe 汇总 25 tests（SCIP 21、Gurobi 4）、0 failures、0 errors。完整日志分别为 `D:\temp\cp2-local-final-clean-current-retry.log`、`D:\temp\cp2-local-final-test-current.log`、`D:\temp\cp2-local-final-verify-current.log`、`D:\temp\cp2-local-final-install-current.log`。

### 1.17 本地稳定 ID 与领域 cut 持久化收尾（2026-08-05）

- `ModelElementIdentityRegistry` 已在 core 提供显式 variable/constraint/objective 注册、namespace/schema、稳定 scope/origin、重复 ID/重复绑定拒绝和 `model-local-*` fallback；linear triad 与 quadratic tetrad 的变量、约束批次、目标及诊断 helper 已接入该 registry。该实现只覆盖当前 core artifact 边界，不宣称机制模型、全部插件、组合求解器和远程 DTO 已完成 `OSPF-SOL-013`。
- framework 新增 `BendersCheckpointCodec` 的 versioned primitive-only document：schema `1.0`、完整性 SHA-256、cut ID/schema/payload/provenance、assumption/fixed binding 和 iteration 结构校验；`captureEncoded`/`restoreEncoded` 贯通既有 `BendersCutSerializationSpi`，Exact 模式恢复仍复验 cut validity/proof 和子问题模型指纹。该项不需要 PostgreSQL、对象存储、外部 worker 或 native runtime；领域 serializer 仍由调用方实现，业务对象不会被自动写入 checkpoint。
- 新增稳定 ID 回归 4 项，Benders checkpoint 往返/损坏摘要/未知 schema/重复 cut ID 回归与既有测试一起通过。最终 Kotlin `clean compile test-compile`、全量 `test`、`verify` 和隔离仓库 `install` 均 `BUILD SUCCESS`；Surefire 汇总 3212 tests、0 failures、0 errors、6 skipped，Failsafe 汇总 25 tests、0 failures、0 errors。完整日志为 `D:\temp\ospf-stable-id-benders-clean-compile-final.log`、`D:\temp\ospf-stable-id-benders-test-final.log`、`D:\temp\ospf-stable-id-benders-verify-final.log`、`D:\temp\ospf-stable-id-benders-install-final.log`；制品已安装到 `D:\environment\maven_repository`。数据库重启、外部对象存储、全仓库跨重建稳定 ID、Benders 跨模型恢复和真实性能基线仍按边界保留。

### 1.18 本轮失败路径收尾（2026-08-05）

- `ModelElementIdentityRegistry` 在显式稳定 ID 冲突时先完成冲突检查，再替换 model-local fallback；失败注册不会破坏既有身份映射。线性/二次 feasibility artifact 的辅助变量使用 `constraint:<stable-id>` 作为来源 origin，源约束自身的 scope/origin 继续保留。
- remote-solver worker 对 `BACKEND_FAILURE` 输出 `completed=false`；`OspfExternalProcessBridge` 仅在 `ospf-cp-snapshot-json` 严格模式下要求 checkpoint/result 文件真实存在并通过完整性校验，旧非 CP key=value 兼容模式仍允许可选路径缺失。
- 新增 registry fallback 冲突保持、worker 后端失败和外部 bridge 兼容模式回归；定向测试共 17 项（Kotlin registry 10、remote worker/bridge 7）全部通过。最终全量验证必须以本节修改后的最新日志为准。

### 1.19 历史全量收尾记录（2026-08-05）

- 本轮修改后的 Kotlin 已执行 `mvn clean compile test-compile -T 0.75C`、`mvn test -T 0.75C`、SCIP/Gurobi 插件 `mvn verify` 以及隔离仓库 `mvn install`，全部 `BUILD SUCCESS`。Surefire XML 汇总 3223 tests、0 failures、0 errors、6 skipped；Failsafe 汇总 25 tests（SCIP 21、Gurobi 4）、0 failures、0 errors。
- Kotlin 完整日志为 `D:\temp\cp2-final-kotlin-clean-current-20260805.log`、`D:\temp\cp2-final-kotlin-test-current-20260805.log`、`D:\temp\cp2-final-kotlin-verify-current-20260805.log`、`D:\temp\cp2-final-kotlin-install-current-20260805-retry.log`；制品安装到 `D:\temp\ospf-cp2-local-m2`。
- remote-solver 使用上述隔离制品执行 `mvn clean compile test-compile`、全量 `test`、`verify` 和 `install`，全部 `BUILD SUCCESS`。四模块 Surefire 合计 331 tests、0 failures、0 errors；Failsafe 为 `RemoteCpHttpE2EIT` 1 test、0 failures、0 errors。完整日志为 `D:\temp\cp2-final-remote-clean-current-20260805.log`、`D:\temp\cp2-final-remote-test-current-20260805.log`、`D:\temp\cp2-final-remote-verify-current-20260805.log`、`D:\temp\cp2-final-remote-install-current-20260805-retry2.log`。
- `git diff --check` 两仓库均无实际错误。PostgreSQL/生产对象存储、真实服务与外部 worker 重启回放、跨全仓库稳定 ID、Benders 领域对象跨进程恢复和生产性能基线仍按 `plans/release.md` 的外部或上游边界保留；本轮不关闭 `OSPF-SOL-013`。

### 1.20 本轮恢复与外部桥接门禁（2026-08-05）

- `OspfExternalProcessBridge` 恢复 checkpoint 时按同一 `runId` 接受历史 `attemptId`，不再要求旧 checkpoint 的 attempt 等于新 slice；legacy v1 仍走显式兼容分支，未知未来 v2 主版本、损坏摘要和伪装来源均返回结构化 backend failure。
- 外部 CP result 与 checkpoint 在上传前重新绑定当前 snapshot，复验变量集合和值域、全部约束、interval、目标、状态组合、assumption/conflict 成员、run/attempt、模型/配置/solver fingerprint 和 artifact digest；in-process 路径使用同一 checkpoint ownership 与数学复验门禁。
- checkpoint metadata 从已验证 envelope 提取 schema、三类 fingerprint 和 digest，LocalFS/S3 持久化保留新字段并兼容旧五列记录；对象引用的 version/etag 继续参与审计往返。
- `ModelElementIdentityRegistry` 的注册、fallback 和最终校验入口已同步化，模型构建完成时执行 namespace/schema/重复 ID 校验；Benders checkpoint 含 master 状态、assumption、binding、conflict 或收敛标记时必须提供显式恢复适配器，避免静默丢状态。
- 当时 remote-solver 全 reactor Surefire 测试 331 项、Kotlin 全仓库 Surefire 测试 3223 项均为 0 failures/0 errors（Kotlin skipped 6）；Kotlin Failsafe 25 项及 remote-solver Failsafe `RemoteCpHttpE2EIT` 1 项也全部通过。该历史日志不作为当前工作区最终统计。真实 PostgreSQL/S3、服务与 worker 重启组合、跨模块完整终态矩阵、全仓库 `OSPF-SOL-013` 和生产性能基线继续按 `plans/release.md` 保留边界。

### 1.21 当前恢复链与诊断复验（2026-08-06）

- external bridge 将输入 checkpoint 仅作为当前切片的恢复起点；输出 checkpoint 必须是当前 `runId/sliceId` 的新 v2 envelope，并指向输入 checkpoint 的 `parentCheckpointId`。worker 未产生新 checkpoint 时不会重复导出旧引用，而是返回已完成的结构化 `BACKEND_FAILURE`，避免切片无限从同一点重启。
- `BACKEND_FAILURE` 携带 incumbent 时，无论 snapshot 是否有目标都执行同一套变量、值域、约束、interval 和目标复验；缺失目标只允许由 checkpoint exporter 基于 snapshot 重算，其他终态不能借此绕过目标校验。
- external result/checkpoint 共享 snapshot 数学复验和完整身份/审计门禁；诊断 source、枚举、成员、变量界、assumption 和 issue 条目必须属于当前 snapshot，合法成员可通过，未知成员结构化拒绝。
- Benders checkpoint 中存在主问题 incumbent/bound、绑定、冲突或收敛标记时必须由显式 adapter 返回逐字段恢复证据；没有 adapter 或证据不一致时返回结构化错误，不宣称通用主问题状态已经恢复。
- 本轮新增的外部 bridge、calculator diagnostics、checkpoint 及 Benders 回归已通过定向测试；随后 Kotlin `mvn clean compile test-compile -T 0.75C`、全量 `mvn test -T 0.75C`、SCIP/Gurobi `mvn verify` 和隔离仓库 `mvn install` 均 `BUILD SUCCESS`。Kotlin Surefire 为 3223 tests、0 failures、0 errors、6 skipped；Failsafe 为 25 tests（SCIP 21、Gurobi 4）、0 failures、0 errors。日志：`D:\temp\cp2-final-kotlin-20260806-clean-compile.log`、`D:\temp\cp2-final-kotlin-20260806-test.log`、`D:\temp\cp2-final-kotlin-20260806-verify.log`、`D:\temp\cp2-final-kotlin-20260806-install.log`。
- remote-solver 使用上述隔离制品完成 clean compile/test-compile、全量 test、verify 和 install，均 `BUILD SUCCESS`；Surefire 为 337 tests、0 failures、0 errors，Failsafe `RemoteCpHttpE2EIT` 为 1 test、0 failures、0 errors。日志：`D:\temp\cp2-final-remote-20260806-clean-compile.log`、`D:\temp\cp2-final-remote-20260806-test.log`、`D:\temp\cp2-final-remote-20260806-verify.log`、`D:\temp\cp2-final-remote-20260806-install.log`。

### 1.22 本轮证据一致性与兼容回归（2026-08-06）

- Benders schema 1.0 在加入 `masterFingerprint` 后保留旧 DTO 摘要迁移；旧 V2 Benders envelope 与 Kotlin framework 文档均通过冻结形状回归。包含主问题状态的 checkpoint 没有主问题指纹时返回结构化错误，不生成不可恢复文档。
- 线性/二次中间模型捕获 stable identity 校验结果，`dumpResult`、统一求解扩展和机制模型主求解路径传播失败；身份错误不再只依赖调用方显式读取私有校验字段。
- external bridge 在对象存储写入前完成 checkpoint/result 的完整性、状态、数学解、diagnostics、assumption/conflict、bound/gap 和 incumbent 对称复验；失败上下文可由 `fetchFinalResult()` 读取，普通 CP 结果不得携带无法由结果证明的 Benders 证据。
- 本轮定向回归：Kotlin Benders 23 项；remote protocol 9 项、calculator 23 项、external bridge 5 项，均 0 failures、0 errors。完整全量验收需以本节之后重新生成的 clean compile/test/verify/install 日志为准。
- 真实 PostgreSQL/S3、服务与 worker 重启组合、跨模块完整终态矩阵、领域 cut serializer 跨进程持久化、全仓库 `OSPF-SOL-013` 和生产性能基线仍属于 `plans/release.md` 中的外部或上游边界；本轮不关闭 CP2。

### 1.23 本轮最终本地验收与剩余门禁（2026-08-06）

- plain CP checkpoint 现在拒绝包含 Benders 主状态的 envelope，避免把 cuts、master state 静默降级成普通 CP 恢复；external bridge 支持无目标模型的合法 incumbent/checkpoint 配对。
- `RemoteLinearSolver` 与 `RemoteQuadraticSolver` 在序列化提交前传播 stable identity `Result`；core checkpoint codec 增加缺少 `masterFingerprint` 的已发布 V2 Benders 摘要迁移，并继续执行完整性与模型指纹门禁。
- external diagnostics 增加 source、validity、minimality、members 与 proof status 的组合语义校验；新增定向测试：core checkpoint 7 项、calculator 26 项、external bridge 6 项，均 0 failures、0 errors。
- 本轮最新全量 Kotlin `clean compile test-compile`、`test`、`verify` 和隔离仓库 `install` 均 `BUILD SUCCESS`。最新 Surefire XML 汇总为 3226 tests、0 failures、0 errors、6 skipped；Failsafe 为 25 tests（SCIP 21、Gurobi 4），0 failures、0 errors。日志：`D:\temp\cp2-final-k-clean-compile-20260806-final.log`、`D:\temp\cp2-final-k-test-20260806-final.log`、`D:\temp\cp2-final-k-verify-20260806-final.log`、`D:\temp\cp2-final-k-install-20260806-final-retry.log`。
- remote-solver 使用上述隔离制品完成 `clean compile test-compile`、全量 `test`、`verify` 和 `install`，均 `BUILD SUCCESS`。最新 Surefire XML 为 343 tests、0 failures、0 errors；Failsafe `RemoteCpHttpE2EIT` 为 1 test、0 failures、0 errors。日志：`D:\temp\cp2-final-remote-clean-compile-20260806-final.log`、`D:\temp\cp2-final-remote-test-20260806-final.log`、`D:\temp\cp2-final-remote-verify-20260806-final.log`、`D:\temp\cp2-final-remote-install-20260806-final.log`。
- 上述统计以本轮 XML 为准，早期章节中的 3223/337 等数字均为历史记录。PostgreSQL/S3、真实服务与 worker 重启回放、跨模块完整终态矩阵、领域 cut serializer 跨进程持久化、全仓库 `OSPF-SOL-013` 和生产性能基线仍保持边界，计划状态继续为 `ImplementedWithBoundaries`。

以上修复均不依赖 PostgreSQL、S3/MinIO、真实 worker 重启或生产硬件。全仓库 `OSPF-SOL-013`、领域业务 cut serializer 的实际接入、真实数据库/对象存储重启回放、跨模块完整终态矩阵和生产性能基线仍按 `plans/release.md` 保留边界。

### 1.24 本轮源码最终复验（2026-08-06）

- 针对 plain CP 最优证明、BackendFailure 目标重算、旧 V2 Benders `decode -> restore`、diagnostics 冗余集合一致性及远程线性/二次 stable identity 传播的回归均已通过；未放宽任何外部环境边界。
- 当前 Kotlin 工作区已重新执行 `mvn clean compile test-compile -T 0.75C`、`mvn test -T 0.75C`、SCIP/Gurobi 子模块 `mvn verify` 和隔离仓库 `mvn install`，均为 `BUILD SUCCESS`。Surefire XML 汇总 3228 tests、0 failures、0 errors、6 skipped；Failsafe 为 25 tests（SCIP 21、Gurobi 4）、0 failures、0 errors。日志：`D:\temp\cp2-current-k-clean-compile-test-compile.log`、`D:\temp\cp2-current-k-test.log`、`D:\temp\cp2-current-k-plugin-child-verify.log`、`D:\temp\cp2-current-k-install.log`。
- remote-solver 使用上述隔离制品重新执行 `mvn clean compile test-compile`、全量 `test`、`verify` 和 `install`，均为 `BUILD SUCCESS`。Surefire XML 汇总 345 tests、0 failures、0 errors；Failsafe `RemoteCpHttpE2EIT` 为 1 test、0 failures、0 errors。日志：`D:\temp\cp2-current-r-clean-compile-test-compile.log`、`D:\temp\cp2-current-r-test.log`、`D:\temp\cp2-current-r-verify.log`、`D:\temp\cp2-current-r-install.log`。
- 3226/343 及更早数字保留为历史统计；本节为当前源码的唯一权威本地验收口径。真实 PostgreSQL/S3、服务/worker 重启回放、跨模块完整终态矩阵、领域 cut serializer 跨进程持久化、全仓库 `OSPF-SOL-013` 和生产性能基线仍未关闭，计划状态继续为 `ImplementedWithBoundaries`。

本文档中的“CP2”表示约束规划能力的第二阶段增强，不是
原 `plans/constraint-programming.md` 中历史 Phase CP2A/CP2B/CP2C 的延续编号；原计划全文已归档至 `plans/release.md`。

## 2. 背景与目标

第一阶段已经完成 core CP AST、snapshot、Fake/MIP-backed solver、SCIP 编译与求解、
Logic-Based Benders、结构化不可行诊断、远程 CP 结果物化和 portable snapshot checkpoint。
当前仍有三类需要明确边界的事项：

1. **跨链路基础契约**：CP snapshot 内的显式身份已经闭环；全仓库跨重建稳定性仍归 `OSPF-SOL-013`，不能由 CP 局部测试替代。
2. **可移植恢复能力**：checkpoint 已保存并复验 incumbent、assumption、冲突证据、Benders cut/trace 和审计字段；Benders 子问题 snapshot 指纹与 versioned primitive cut document 已提供本地跨模型/完整性拒绝门禁，领域 serializer 仍由调用方提供，数据库重启回放和原生搜索树续跑仍未承诺。
3. **有证据的性能增强**：当前 JSCIP binding 缺少已验收的增量 bound、probing 和 conflict graph；SCIP native optional interval、variable duration 与真实性能基线仍须 API 探针和生产代表性基准决定。

本计划的目标是：

- 让 CP 模型、解、诊断、Benders binding、远程传输和 checkpoint 使用同一套稳定身份契约。
- 让本地与远程 CP 求解无损表达问题结论、终止原因、解、证明、诊断、来源和指纹。
- 建立可跨后端重建的恢复协议，并明确区分“重建恢复”和“原生搜索状态续跑”。
- 只在等价性、资源安全和可测性能收益都成立时启用 SCIP/JSCIP 原生增强。
- 始终保留正确的降级路径，不把 unsupported、exact lowering 或 warm restart 冒充 native 能力。

## 3. 优先级与决策

| 能力 | 优先级 | 结论 | 原因 |
| --- | --- | --- | --- |
| `OSPF-SOL-013` 跨重建稳定 ID 及 CP 接入 | P0 | 必做 | 是诊断、远程物化、checkpoint、重放和 Benders 关联的共同前提 |
| `OSPF-SOL-022/023` 统一远程 `SolveReport` 及 CP 对齐 | P1 | 必做 | 避免本地/远程状态和证明语义继续分裂 |
| portable checkpoint v2 | P1 | 必做 | 跨后端、跨进程、可审计，实际价值高于绑定特定 native 状态 |
| JSCIP 增量 bound/probing/conflict graph | P2 | 条件实施 | 只在 binding 可支持且 model-rebuild 已成为可测瓶颈时有价值 |
| SCIP native optional interval | P2 | 条件实施 | 需先证明 SCIP/JSCIP 存在等价原生语义并优于当前精确 lowering |
| SCIP native variable duration | P3 | 条件实施 | API 和语义不确定性更高，现有 MIP-backed 精确路径已可用 |
| backend native checkpoint/resume | P3 | 仅调研 | SCIP 问题导出、warm start 或重优化不等于完整搜索状态恢复 |

P0/P1 表示 CP2 的交付优先级，不改变 `schema.md` 对公共契约的所有权。
公共 ID 和远程报告 DTO 必须在 solver 通用层实现，CP2 只增加 CP 特有的接入、物化和验收，不复制一套 CP 私有协议。

### 3.1 实施决策记录

| 工作包 | 决策 | 证据与保留路径 |
| --- | --- | --- |
| `OSPF-SOL-013` 全仓库稳定 ID | `BlockedByUpstream` | `plans/schema.md` 仍未关闭；CP snapshot 已支持 `stable/model-local` scope、显式 origin 和重复 ID 拒绝，但不宣称机制模型及所有插件已跨重建稳定。 |
| `OSPF-SOL-022/023` 远程报告 | `BlockedByUpstream`（CP 接入已完成） | CP 客户端、protocol、dispatcher、calculator 已共享正交状态字段；schema 计划仍未完成线性/二次全链路验收。 |
| CP2-4 JSCIP 增量能力 | `RejectedByEvidence` | 当前 binding 未提供可验证的增量 bound/probing/conflict-graph API，且没有达到 rebuild 成本与收益门禁；继续逐轮 model-rebuild。 |
| CP2-5 SCIP native interval | `RejectedByEvidence` | SCIP 路径使用已验证的 exact lowering，未找到 optional interval/variable duration 的等价 native 语义或 differential/基准证据；不声明 `Native`。 |
| CP2-6 native checkpoint/resume | `RejectedByEvidence` | 当前只保存 portable snapshot、incumbent、指纹和完整性摘要；问题导出、warm start、reoptimization 不构成搜索树恢复，继续使用 `RebuildFromSnapshot`。 |

CP2 的“完成”因此仅覆盖已通过测试的公共 CP/远程协议、稳定 snapshot scope、结果物化和 portable checkpoint 基础能力；未关闭的上游契约和条件式工作包不得在 capability matrix 中标为 `Native` 或跨重建稳定。

## 4. 架构原则

### 4.1 身份先于序列化和恢复

- `VariableId`、`ConstraintId`、`ObjectiveId` 和 interval ID 在公开建模边界确定，进入 snapshot 后不可改写。
- 跨重建稳定身份必须来自调用方稳定业务键或确定性的身份工厂；名称、集合遍历顺序、solver index、JVM hash 和随机 UUID 都不能隐式承担该语义。
- 无稳定来源的自动 ID 必须标记为 model-local，不能在诊断、远程结果或 checkpoint 中宣称跨重建稳定。
- lowering/native compiler 生成的辅助元素使用独立命名空间，并携带可选 origin ID；辅助 ID 不得覆盖或伪装成源元素 ID。
- 重名元素、注册顺序变化和 backend 重建不得影响已有稳定 ID 的回查结果。

### 4.2 统一报告，不扩展旧输出模型

- CP 报告复用 `SolveReport` 的正交状态、证明、诊断、provenance、fingerprint 和 attempt 语义。
- 远程任务生命周期与数学问题结论保持正交；任务完成不等于最优，任务失败也不能覆盖已经获得且可信的 incumbent 或诊断。
- 正常的 Infeasible、Unbounded、TimeLimit、Cancelled 和限制终止返回可信报告；协议损坏、模型不一致或无法形成可信报告时才进入 `Failed`/`Fatal`。
- 旧 `ConstraintProgrammingSolverOutput` 和旧远程 DTO 只作为有明确映射规则的兼容 facade，不再承载新增字段。

### 4.3 Portable-first checkpoint

- `RebuildFromSnapshot` 表示重建模型、恢复已验证 incumbent/cut/assumption 并继续求解，不表示恢复原生搜索树。
- 只有完整保留并恢复 backend 声明的搜索状态，且通过中断续跑一致性测试，才能声明 `Native`。
- 写问题文件、读取 incumbent、warm start、重优化或复用编译缓存都不能单独标记为 native checkpoint/resume。
- checkpoint 必须带 schema 版本、身份命名空间、模型/配置/solver 指纹和完整性校验，错误或不兼容时返回结构化失败，禁止静默忽略字段。

### 4.4 能力声明必须可证

- `Native`、`ExactLowering` 和 `Unsupported` 的含义保持互斥；原生 handler 内部若仍采用 OSPF 通用大 M 分解，不得声明为 `Native`。
- 条件式能力必须同时通过 API 探针、语义等价测试、资源生命周期测试和代表性基准。
- 未达到实施门禁时继续使用现有 model-rebuild 或 exact MIP-backed 路径，不影响正确性和本计划关闭。
- OSPF 不依赖、不引入也不适配 OR-Tools；SCIP、Gurobi 与 OSPF CP 抽象保持独立工具库/后端关系。

## 5. 分阶段实施计划

### Phase CP2-0：基线与公共契约对齐

- [x] `OSPF-CP2-001` 复核 `schema.md` 和当前实现，记录 `OSPF-SOL-013/022/023` 的真实完成状态、剩余差距和所有权，避免以已有类型名替代端到端验收。
- [x] `OSPF-CP2-002` 冻结 CP2 capability 术语、ID scope、checkpoint support level 和远程协议兼容期。
- [x] `OSPF-CP2-003` 建立代表性基线：benchmark 模块新增 direct CP、固定/可选/可变时长排程、Logic-Based Benders、snapshot 编码和 checkpoint capture/restore fixture；`D:\temp\cp2-benchmark-results-all-current.json` 记录了当前 Fake、小规模、单 fork smoke 的 build/solve/restore 时间。该数据不冒充 native SCIP 的节点、峰值内存或生产 SLO 基线，native/完整资源指标仍属边界。
- [x] `OSPF-CP2-004` 为每个条件式工作包建立决策记录，结论限定为 `Accepted`、`RejectedByEvidence` 或 `BlockedByUpstream`，并附 API 探针、版本和基准证据。

验收：后续性能改造有可重复基线；公共计划与 CP2 不存在重复 DTO、重复 ID 类型或相互矛盾的 capability 定义。

### Phase CP2-1：稳定身份接入（P0）

本阶段消费并验收 `OSPF-SOL-013`，公共身份实现仍归 `schema.md` 所属计划。

- [x] `OSPF-CP2-101` 让 CP variable、constraint、objective 和 interval 在模型、snapshot、codec、solver output 与诊断证据中保留同一稳定 ID 和 scope（仅对 CP snapshot scope 完成；全仓库稳定性受 `OSPF-SOL-013` 约束）。
- [x] `OSPF-CP2-102` 为 SCIP compiler 和 MIP lowerer 建立 origin-to-artifact 映射；辅助变量、辅助约束和激活 literal 使用可追溯但不冒充源元素的 ID（已由 compiler/lowerer artifact registry 及 SCIP/MIP 回归覆盖；跨全仓库稳定 ID 仍受 `OSPF-SOL-013` 约束）。
- [x] `OSPF-CP2-103` 让 Logic-Based Benders master binding、subproblem binding、assumption、conflict member 和 cut provenance 优先使用显式 `subproblemVariableId`；回归覆盖跨重建同一 CP ID。未提供稳定来源时仍明确降级为 model-local，不能替代全仓库 `OSPF-SOL-013`。
- [x] `OSPF-CP2-104` 为 snapshot/remote/checkpoint 增加身份 schema 版本与兼容读取；重复 ID、scope 冲突和 origin 丢失返回双语结构化错误。
- [x] `OSPF-CP2-105` 增加跨重建、注册顺序变化、重名、序列化往返、SCIP/MIP differential 和辅助元素投影测试（`ConstraintProgrammingEnhancementTest`、`ConstraintProgrammingLowererTest` 与 `ScipConstraintProgrammingSolverIT` 已覆盖 snapshot/codec、稳定投影、跨后端和辅助 artifact；全仓库跨重建稳定 ID 仍受 `OSPF-SOL-013` 约束）。

验收：同一逻辑模型独立构建两次后，所有显式稳定元素 ID 一致；诊断、远程解和 checkpoint 可回查原元素；无稳定来源的元素仍明确为 model-local。

### Phase CP2-2：远程统一报告（P1）

本阶段消费 `OSPF-SOL-022` 的版本化 DTO，并与 `OSPF-SOL-023` 的线性/二次 adapter 共享映射基础设施。

- [x] `OSPF-CP2-201` 为 CP 增加统一报告入口，完整映射 `problemStatus`、`terminationReason`、`solutionPresence`、`proof`、statistics、diagnostics、provenance 和 fingerprints。
- [x] `OSPF-CP2-202` 让远程 CP 请求、raw result 和 `resultRef` artifact 使用显式 schemaVersion、artifact digest、model fingerprint 和 run/attempt 关联。
- [x] `OSPF-CP2-203` 继续校验 Int64 值域、interval 语义、全部 snapshot 约束和目标值，并新增报告、proof、diagnostics、fingerprint 与 artifact 的交叉一致性校验。
- [x] `OSPF-CP2-204` 正确保留“超时/取消但有 incumbent”“不可行但诊断失败”“Unknown 且无证明”等组合，不从状态字符串或 `feasible` 布尔值反推更强结论。
- [x] `OSPF-CP2-205` 保留一个明确版本窗口的旧 DTO 兼容读取；未知新版本、字段丢失、摘要不一致和旧协议无法表达的状态返回结构化 warning/error。
- [x] `OSPF-CP2-206` 建立 core 状态映射与远程物化等价 fixture，覆盖 Optimal、Feasible、Infeasible、Unknown、TimeLimit、NodeLimit、SolutionLimit、Cancelled、Interrupted、BackendFailure、有/无 incumbent 及诊断失败；跨真实 native/远程进程的差分仍按环境边界保留。

验收：同一 snapshot 的本地与远程报告在声明容差内语义等价；远程传输不提升 proof 等级、不丢 incumbent、不把任务状态当作问题状态。

### Phase CP2-3：portable checkpoint v2（P1）

- [x] `OSPF-CP2-301` 定义版本化 checkpoint envelope，至少包含 CP snapshot、稳定 ID namespace、model/configuration/solver fingerprints、run/attempt/parent checkpoint、创建时间和完整性摘要。
- [x] `OSPF-CP2-302` 保存并恢复经 snapshot 复验的 incumbent、目标值、solver hint 和 interval 解；历史 best bound/gap 作为审计数据保存，只有存在可复验的 bound 证明且模型/配置一致时才能恢复为收敛依据。不可信 incumbent 必须拒绝，不能作为 warm start 注入。
- [x] `OSPF-CP2-303` 保存 CP assumptions、固定 binding、冲突证据及其 validity/minimality/provenance；`BendersCheckpointCodec` 和 `LogicBasedBendersEngine` 已接入字段捕获、schema/成员/domain/模型复验，回归覆盖 round-trip 与稳定 ID。真实跨进程注入和数据库重启回放仍属边界。
- [x] `OSPF-CP2-304` 为 Logic-Based Benders 保存迭代号、master incumbent/best bound、去重后的 cut pool、cut validity/provenance、收敛门禁和 iteration trace（`BendersCheckpointCodec` + `LogicBasedBendersOptions.resumeState` 已接入；document 具有 schema/digest/结构校验，跨模型稳定 binding 仍受 `OSPF-SOL-013` 约束）。
- [x] `OSPF-CP2-305` 为领域 cut payload 提供版本化 serialization SPI 和完整性保护的 primitive document；闭包、backend native handle 和不可序列化业务对象不得进入 checkpoint（领域 serializer 仍由调用方拥有，当前 core/framework 实现不依赖外部数据库或对象存储）。
- [x] `OSPF-CP2-306` 实现 v1 legacy checkpoint 读取和 v2 写入策略；损坏、截断、未知 schema、模型错配、solver capability 降级都返回 `Ret` 错误或结构化 warning。
- [x] `OSPF-CP2-307` 增加中断前后等价、跨进程重建、跨兼容 backend 重建、重复恢复幂等、cut 去重和不兼容恢复拒绝测试（已覆盖 digest、legacy read、独立 JVM decode、模型错配、incumbent 复验、重复 decode/restore 幂等和 Benders cut 去重；真实服务重启、原生搜索树续跑及完整跨模块中断矩阵仍是明确环境边界）。

验收：portable resume 可以在不持有任何 native 对象的情况下重建并继续求解；Exact Benders 只恢复仍可验证为 exact 的 cut 和门禁状态；恢复后的结论不强于现有证明。

### Phase CP2-4：JSCIP 增量求解与冲突增强（P2，条件式）

- [x] `OSPF-CP2-401` 对目标 JSCIP/SCIP 版本探测增量 bound change、约束启停、probing、reoptimization、conflict analysis/graph 和安全中断 API，并记录线程与 stage 限制（`RejectedByEvidence`：当前 binding 未暴露可用探针/API）。
- [x] `OSPF-CP2-402` 将 runtime capability 与编译期 binding capability 分开声明；API 不可用时保持 `incrementalBoundUpdates=false`、`probing=false`、`conflictGraph=false`（`RejectedByEvidence`：继续使用 rebuild capability）。
- [x] `OSPF-CP2-403` 只有当代表性 Benders workload 中 model rebuild/compile 成为显著瓶颈时，才实现增量 session；core SPI 不暴露 JSCIP/JNI 类型（`RejectedByEvidence`：未达到性能门禁，未引入 session）。
- [x] `OSPF-CP2-404` 若 conflict graph 可用，增加 origin ID 投影、证据等级和最终不可行复验；无法投影或复验失败时降级到现有 assumptions/shrink 路径（`RejectedByEvidence`：保留现有 assumptions/shrink）。
- [x] `OSPF-CP2-405` 对增量路径与每轮重建路径执行状态、解、目标、conflict、取消和资源释放 differential test（`RejectedByEvidence`：没有第二条增量路径可执行差分）。
- [x] `OSPF-CP2-406` 仅在重复基准显示稳定收益且无正确性/内存回归后默认启用；否则保持 opt-in 或 `RejectedByEvidence`（当前结论为 `RejectedByEvidence`）。

实施门禁：目标 binding 必须提供受支持 API，且基线显示 rebuild/compile 至少占代表性端到端耗时的 20%，或已超过明确的业务延迟预算。收益门禁以同机同版本重复基准为准，默认要求端到端中位耗时改善至少 15%，并报告 p95 和峰值内存；未达门禁不实施或不默认启用。

### Phase CP2-5：SCIP 排程原生能力（P2/P3，条件式）

- [x] `OSPF-CP2-501` 明确定义 fixed-duration、optional interval、variable duration 在 presence=false、start/end/duration、NoOverlap/Cumulative 和解回填上的精确语义。
- [x] `OSPF-CP2-502` 探测目标 SCIP/JSCIP 是否存在可表达 optional job/interval 的原生约束语义；仅有 OSPF big-M 分解时继续声明 `ExactLowering`（`RejectedByEvidence`：未发现可验证 native 语义）。
- [x] `OSPF-CP2-503` 若 optional interval 可原生表达，实现 compiler、origin 映射、capability 和资源释放，并与 Fake/MIP-backed oracle 做小规模穷举 differential test（`RejectedByEvidence`：继续 exact lowering，未接入伪 native compiler）。
- [x] `OSPF-CP2-504` 独立探测 variable duration；不得因 optional fixed-duration 可用而推断 variable duration 也为 native（`RejectedByEvidence`：native API/等价证据不足）。
- [x] `OSPF-CP2-505` 若 variable duration 可原生表达，实现 start + duration = end、duration domain、presence 和资源约束的完整映射及负路径测试（`RejectedByEvidence`：保留 core/MIP exact lowering）。
- [x] `OSPF-CP2-506` 建立 fixed/optional/variable-duration Job Shop 与 Gantt 代表性基准，比较 native、SCIP exact decomposition 和 MIP-backed lowering 的构建时间、求解时间、节点、内存和解质量（已增加 Fake/SCIP exact/MIP-backed 的小规模 benchmark smoke 与结果夹具；SCIP/JSCIP native optional/variable interval 探针结论为 `RejectedByEvidence`，因此不虚构 native 对照数据；节点/峰值内存和生产代表性性能仍需部署环境基线）。
- [x] `OSPF-CP2-507` 只有 native 路径稳定优于现有路径或满足明确业务 SLO 时才评估 Gantt framework 迁移；不因 API 可用就改写现有生产建模路径（`RejectedByEvidence`：不迁移生产 Gantt）。

验收：被声明为 `Native` 的每种 interval 组合均与 core 语义双向等价；超过原生能力边界时返回 `Unsupported` 或选择已声明的 `ExactLowering`，不静默弱化约束。

### Phase CP2-6：native checkpoint/resume 调研（P3，不承诺实现）

- [x] `OSPF-CP2-601` 调研目标 SCIP/JSCIP 版本对搜索树、开放节点、cut pool、incumbent pool、伪成本、随机状态和参数的保存/恢复能力（`RejectedByEvidence`：当前版本未提供完整搜索状态 API）。
- [x] `OSPF-CP2-602` 区分 problem export、solution/warm start、reoptimization 与真正 native resume，并形成 capability 判定清单。
- [x] `OSPF-CP2-603` 若缺少完整且稳定的 API，记录 `RejectedByEvidence` 或 `BlockedByUpstream`，继续使用 portable checkpoint v2，不增加伪 native facade（结论：`RejectedByEvidence`）。
- [x] `OSPF-CP2-604` 只有 API 足够时才制作崩溃/中断续跑原型，验证同 backend 版本、平台、参数、模型指纹、证明连续性、取消和资源释放（未达到 API 门禁，不制作原型）。
- [x] `OSPF-CP2-605` 原型通过后另行评审是否进入生产；native artifact 必须显式绑定 backend/native 版本，且 portable fallback 始终保留（未进入生产评审）。

本阶段的完成定义是形成有证据的能力结论，而不是必须交付 native resume。

### Phase CP2-7：文档、兼容与发布

- [x] `OSPF-CP2-701` 更新 core、SCIP plugin、framework remote/Benders 的中英文 README 和 capability matrix。
- [x] `OSPF-CP2-702` 提供 stable ID、远程报告、portable checkpoint v2 和条件式 native capability 的最小迁移示例。
- [x] `OSPF-CP2-703` 记录 schema 兼容窗口、弃用入口、升级失败语义和回滚到 rebuild/exact lowering 的方式。
- [x] `OSPF-CP2-704` 汇总当前可验证的单元、性质、SCIP/Gurobi differential、集成、远程终态和 benchmark smoke 报告；不可用 native library/license 按 `Skipped`/`RejectedByEvidence` 记录，不计为通过。跨进程/数据库重启、全仓库稳定 ID、完整生产性能基线继续明确列为边界。

## 6. 测试矩阵

| 层级 | 必须覆盖 |
| --- | --- |
| 稳定身份 | 独立重建、注册顺序变化、重名、重复 ID、model-local scope、origin/auxiliary 投影 |
| snapshot/codec | 新旧 schema 往返、Int64 边界、未知版本、损坏字段、确定性编码 |
| 远程报告 | 全终态组合、incumbent 保留、proof 不提升、诊断失败、raw/resultRef 不一致、digest 错误 |
| portable checkpoint | incumbent/目标复验、指纹错配、assumption/cut 恢复、重复恢复、跨进程重建 |
| Benders resume | master bound、cut pool、validity、trace、收敛门禁、Unknown/取消后恢复 |
| JSCIP 增量 | 与 rebuild 的解/状态/conflict 等价、stage 限制、重复 session、取消、native 资源释放 |
| interval native | presence、variable duration、NoOverlap、Cumulative、边界 horizon、不可行与最优 oracle |
| 性能 | build/compile/solve/shrink/restore 分项、p50/p95、节点、迭代、峰值内存、版本/provenance |

所有 exact/native 路径必须有小规模穷举或可信 reference formulation 的双向等价测试。
随机 fixture 固定种子并记录模型、配置和 solver fingerprint；性能数据不得代替正确性验收。

## 7. 主要风险与控制措施

| 风险 | 影响 | 控制措施 |
| --- | --- | --- |
| 自动 ID 仍依赖注册顺序 | 重建、诊断和恢复关联错误 | 显式稳定键/身份工厂；无稳定来源则保留 model-local scope |
| CP 私建远程 DTO | 与线性/二次报告再次分裂 | 公共 DTO 归 `schema.md`；CP 只扩展类型安全 artifact |
| checkpoint 恢复无效 cut | Exact Benders 错误收敛 | 保存 validity/provenance，恢复后重新验证并拒绝不可信 cut |
| 把 warm start 称为 native resume | capability 和审计失真 | support level 三态与能力判定清单 |
| JSCIP stage/API 使用错误 | native 崩溃或状态污染 | API 探针、session 状态机、边界异常转 `Ret`、资源测试 |
| optional interval 仅单向等价 | 漏解或错误排除可行解 | 双向 formulation 审查、穷举 oracle、跨 backend differential |
| 原生路径收益不稳定 | 复杂度增加但生产无收益 | 实施门禁、重复基准、opt-in 和可回退 capability |
| backend 版本锁定 | checkpoint/原生行为不可移植 | provenance、版本绑定、portable fallback 和兼容矩阵 |

## 8. 明确非目标

- 不引入 OR-Tools 依赖、adapter、协议类型或其搜索引擎实现。
- 不在 CP2 中重复实现 `schema.md` 已拥有的公共 ID、`SolveReport` 或远程基础 DTO。
- 不承诺把任意 CP 全局约束自动降为 MIP 或 SCIP native constraint。
- 不承诺跨 SCIP/native 主版本恢复原生搜索状态。
- 不把 portable rebuild、problem export、warm start 或 reoptimization 宣称为 native resume。
- 不因 SCIP interval API 可用就自动迁移 Gantt 或其它生产 framework。
- 不让 backend/JNI 对象进入 core、domain、remote payload 或 portable checkpoint。

## 9. 关闭标准

以下必做条件全部满足后，CP2 可以关闭：

1. CP 路径已经消费统一稳定身份契约，跨重建稳定与 model-local scope 可机器区分。
2. CP 模型、snapshot、solver artifact、诊断、Benders、远程结果和 checkpoint 的 ID 可以端到端回查。
3. 本地与远程 CP 使用统一报告语义，全终态、incumbent、proof、诊断、provenance 和 fingerprints 无损映射。
4. portable checkpoint v2 可以恢复经验证的 incumbent、assumption、Benders cut/trace 和审计关联，且不夸大证明。
5. 旧 snapshot/remote/checkpoint 的兼容读取期和失败语义已经文档化并有测试。
6. 所有条件式工作包都有版本化探针、基准和明确决策；已接受项完成正确性与资源验收，未接受项保留准确 capability 和 fallback。
7. 中英文文档、示例、capability matrix、正确性测试和性能报告同步完成。
8. 全量编译、全量测试和可用 SCIP/Gurobi 集成测试通过，`git diff --check` 无实际错误。

SCIP native optional interval、native variable duration 和 native checkpoint/resume **不是强制交付项**。
如果探针或基准不满足门禁，以 `RejectedByEvidence`/`BlockedByUpstream` 关闭并保留现有正确降级路径，即满足本计划要求。

## 10. 构建与验收策略

任务进行中只运行受影响模块的增量编译和定向测试，并把每次 Maven 输出完整保存到仓库外日志。
所有改动完成后按项目规则执行一次全量编译和一次全量测试：

```powershell
$cp2LogDir = Join-Path $env:TEMP 'ospf-cp2-verification'
New-Item -ItemType Directory -Force -Path $cp2LogDir | Out-Null
$cp2CompileLog = Join-Path $cp2LogDir 'compile.log'
$cp2TestLog = Join-Path $cp2LogDir 'test.log'
$cp2PluginLog = Join-Path $cp2LogDir 'plugin-verify.log'

mvn clean compile test-compile -T 0.75C *> $cp2CompileLog
mvn test -T 0.75C *> $cp2TestLog
mvn verify -pl ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip,ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-gurobi -am -T 0.75C *> $cp2PluginLog
git diff --check
```

每条 Maven 命令只执行一次并完整利用对应日志。失败时从同一日志读取全部错误和必要上下文，禁止通过 `head`、`tail` 或截断管道反复启动构建。

## 11. 推荐实施顺序

```text
CP2-0 基线、术语和外部依赖复核
  -> CP2-1 OSPF-SOL-013 + CP 稳定身份接入
  -> CP2-2 OSPF-SOL-022/023 + 远程 CP SolveReport
  -> CP2-3 portable checkpoint v2
  -> CP2-4 JSCIP 增量能力探针与条件实施
  -> CP2-5 SCIP interval 原生能力探针与条件实施
  -> CP2-6 native checkpoint 能力结论
  -> CP2-7 文档、兼容与最终验收
```

`CP2-4`、`CP2-5` 和 `CP2-6` 的探针可以在 CP2-1～CP2-3 期间并行开展，但任何实现不得绕过稳定 ID、统一报告和 capability 语义。

### 11.1 跨仓库同步执行步骤

`cp2.md` 与远程求解服务端的 `daily.md` 是同一个跨仓库项目的两个工作流：

- 本计划负责公共契约、core/SCIP/framework 客户端实现、稳定身份、统一 `SolveReport`、CP artifact 和 portable checkpoint 语义。
- `E:/workspace/ospf/ospf/framework/remote-solver/daily.md` 负责服务端协议接收、dispatcher、calculator、对象存储、持久化、恢复和部署验收。
- 两份计划不要求所有任务逐项同步，也不能先完整关闭一份再开始另一份；公共契约先冻结，随后客户端与服务端使用相同 fixture 锁步实现。

按以下门禁执行：

| 门禁 | CP2 工作 | 服务端同步工作 |
| --- | --- | --- |
| G0：契约准备 | 执行 CP2-0，冻结术语、所有权、版本策略和兼容窗口 | 同步执行 RS-CP-0，建立请求/响应/artifact 双向 fixture；可先修复不依赖最终 DTO 的路径与 artifact 结构问题 |
| G1：身份冻结 | 完成 CP2-1 的稳定 ID、scope、origin 和 identity schema | 接入稳定身份传输、校验、origin 投影和持久化摘要；不得自行定义服务端私有 ID 语义 |
| G2：报告冻结 | 冻结 CP2-2 的公共 `SolveReport`、远程 DTO、schemaVersion、digest、run/attempt 和兼容规则 | 锁步实施服务端 protocol、dispatcher、calculator、结果 artifact、报告持久化及客户端映射，并完成本地/远程等价测试 |
| G3：恢复冻结 | 冻结 CP2-3 的 portable checkpoint v2 envelope、指纹和 support level | 实施 checkpoint artifact 存储、恢复、legacy 读取及跨进程/重启测试 |
| G4：条件增强 | CP2-4～CP2-6 分别形成 `Accepted`、`RejectedByEvidence` 或 `BlockedByUpstream` 结论 | 仅为已接受能力增量扩展 capability 和执行路径；不得改变 G1～G3 已冻结的基础语义 |

具体步骤：

1. CP2-0 与服务端 RS-CP-0 同步启动，先用跨仓库 fixture 固定线格式，不以两个仓库中的同名 Kotlin DTO 代替协议验收。
2. CP2-1 先冻结稳定身份契约，服务端随后完成 G1 接入；身份未冻结前不得固化相关数据库 schema 或对外承诺跨重建稳定性。
3. CP2-2 与服务端 RS-CP-1、RS-CP-3、RS-CP-4、报告相关 RS-CP-5 及 RS-CP-6 锁步实施，每次公共字段变更必须同时更新协议表和双向 fixture。
4. 本地/远程全终态、incumbent、proof、diagnostics 和 fingerprint 等价测试通过后，服务端才能声明并启用基础 CP capability。
5. CP2-3 与服务端 checkpoint v2 存储、恢复和重启验收同步执行；该阶段不反向阻塞不使用 resume 的基础远程 CP 求解。
6. CP2-4～CP2-6 的探针可以与必做阶段并行，但条件式实现只能在对应决策被接受后接入，且不能阻塞 CP2-1～CP2-3 及基础服务端交付。
7. CP2-7 汇总两个仓库的兼容矩阵、测试日志、迁移文档和 capability 证据；客户端或服务端任一侧未完成相应必做验收时，不得关闭 CP2。

同步约束：

- 服务端不应等待 CP2 全部完成后才开始，否则 CP2-206 的本地/远程等价验收无法成立。
- G2 前不得把当前过渡 DTO 固化为最终公开 API 或不可迁移的数据库结构。
- G2 后远程协议变更必须同时更新本计划、服务端 `daily.md`、双向 fixture 和兼容矩阵。
- G3 后 checkpoint 协议变更必须同时验证 portable fallback，不能以 native 能力替代公共恢复契约。
- 两个仓库分别记录任务状态和提交历史，但以跨仓库 fixture 与端到端测试作为联合完成证据。

### 11.2 证据链收尾修复（2026-08-06）

- Benders schema 1.0 保留旧 DTO 摘要迁移；旧文档解码只允许读取，包含主问题状态但没有 `masterFingerprint` 的状态在恢复前拒绝，不能生成新的不可恢复 checkpoint。
- CP checkpoint codec 对 Benders 主问题状态执行指纹门禁；线性、二次、SCIP 和 Gurobi 直接求解入口传播 stable identity 的 `Result` 失败，不再让空 namespace/schema 模型静默进入 native solver。
- external bridge 在 checkpoint/result 成对复验完成前不写入对象存储；失败上下文可通过 `fetchFinalResult()` 读取，checkpoint 的 assumptions、conflicts、bound、gap、incumbent 和后端失败目标语义执行对称校验，并保留完整审计 metadata。
- 服务端 checkpoint 导出拒绝非 incumbent 携带目标值；新增无目标字段回归测试。Kotlin Benders 定向测试 23 项、remote calculator 定向测试 24 项及协议 9 项均通过。
- CP2 仍保持 `ImplementedWithBoundaries`：真实 PostgreSQL/S3、生产进程重启组合、跨模块全终态矩阵、领域 cut serializer 的跨进程接入、全仓库 OSPF-SOL-013 和生产性能基线继续写入 `plans/release.md`，不得仅凭本地测试关闭计划。

### 11.3 本轮增量复验（2026-08-06）

- 修复 external bridge 在严格 CP 非终态且未生成新 checkpoint 时丢失已复验 incumbent 的问题：现在返回 `BACKEND_FAILURE + INCUMBENT`，保留精确目标、诊断、provenance 和 fingerprints，但不写入孤立 result/checkpoint artifact。
- 补齐 checkpoint 证据回归夹具的 `infeasibility.source`，继续保持未知来源和不完整诊断拒绝，不放宽服务端与客户端的结构化诊断契约。
- 本轮定向复验：Kotlin stable identity 10 项、Benders 23 项；remote calculator 24 项、protocol 9 项、external bridge 5 项，均为 0 failures、0 errors。
- 以上为源码级和内存对象存储证据；真实 PostgreSQL/S3、服务/worker 重启组合、跨模块完整终态矩阵、领域 cut serializer 跨进程接入、全仓库 OSPF-SOL-013 和生产性能基线仍按 `plans/release.md` 保留边界。

### 11.4 历史本地验收记录（2026-08-06，早于 1.23）

- Kotlin 已执行 `clean compile test-compile`、全量 `test`、SCIP/Gurobi `verify`，并安装到隔离 Maven 仓库 `D:\temp\ospf-cp2-local-m2`；remote-solver 使用该制品完成 `clean compile test-compile`、全量 `test`、`verify` 和 `install`。
- 当时报告统计为 Kotlin Surefire 3100 tests、6 skipped，Failsafe 25 tests；remote-solver Surefire 340 tests、Failsafe 1 test；两仓库均 0 failures、0 errors，仅作历史记录。
- 当时完整日志：`D:\temp\cp2-final-k-clean-compile-20260806.log`、`D:\temp\cp2-final-k-test-20260806.log`、`D:\temp\cp2-final-k-plugin-verify-20260806.log`、`D:\temp\cp2-final-k-install-20260806.log`、`D:\temp\cp2-final-r2-clean-compile-20260806.log`、`D:\temp\cp2-final-r2-test-20260806.log`、`D:\temp\cp2-final-r2-verify-20260806.log`、`D:\temp\cp2-final-r2-install-20260806.log`；当前权威统计见 1.23。
- 本地构建和内存 E2E 证据不关闭外部边界：真实 PostgreSQL/S3、服务/worker 重启组合、跨模块完整终态矩阵、领域 cut serializer 跨进程接入、全仓库 OSPF-SOL-013 和生产性能基线仍须外部环境或上游证据。
