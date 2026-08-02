# Remote Solver 事件驱动异构调度系统设计

## 一、概述

### 1.1 业务目标

在多供应商、多性能、多计费模式的求解器资源池中，以**最低成本**完成全部算法任务，并满足可接受的时效性要求。

### 1.2 技术目标

1. 复用现有原型的事件驱动架构思想（Client/Dispatcher/Solver）。
2. 支持异构节点调度（高性能高价 + 低性能低价）。
3. 支持复杂非实时任务的时间片轮转与断点续算。
4. 对不支持原生中断或搜索树恢复的求解器，利用可移植 checkpoint、warm start 与模型重建实现近似可抢占。
5. 支持线性、二次与约束规划（CP）任务的统一调度和能力匹配。
6. 对复杂实时任务，按策略降级处理为复杂非实时任务。

### 1.3 非目标（明确边界）

1. 不直接保证复杂实时任务的硬实时 SLA。
2. 不在第一阶段实现跨数据中心强一致多活。
3. 不强绑定具体求解器 SDK，实现统一抽象后由插件接入。
4. portable checkpoint 不承诺保存求解器原生搜索树、线程状态或进程句柄。

### 1.4 核心约束

**任务约束：**
1. 每个任务有业务优先级、是否实时、模型复杂度、预计求解规模。
2. 任务必须在可接受时间内给出可用结果（最优或可行解）。
3. 复杂任务可能需要多轮执行和多次状态恢复。

**资源约束：**
1. 节点能力差异：CPU/GPU、并行数、求解器类型、许可证上限。
2. 计费差异：按秒/按分钟、最小计费粒度、并发附加费、许可证占用费。
3. 部分求解器不支持"立即中断"或原生搜索状态恢复，只能导出可移植 incumbent、模型快照和诊断证据后重建求解。

### 1.5 优化目标

最小化总体成本并控制超时风险：

\[
\min \sum_j (\text{price}_{n_j} \cdot \text{runtime}_j + \text{licenseCost}_{n_j}) + \lambda \cdot \text{SLA\_penalty}
\]

其中：
1. \(n_j\) 为任务或时间片分配的节点。
2. \(\lambda\) 为 SLA 风险惩罚权重，可按业务线配置。

### 1.6 关键权衡结论

1. 简单任务不是架构敏感点，直接空闲分配即可。
2. 复杂非实时任务是核心，必须做时间片轮转与状态续算。
3. 对不支持中断的求解器，使用"受控自然返回 + 可移植 checkpoint + 重建恢复"替代硬抢占。
4. 复杂实时任务当前不处理，按复杂非实时处理是合理工程折中。
5. CP 恢复是 rebuild-based warm start，不表示原生搜索树恢复；对外能力必须明确区分 portable 与 native checkpoint。

---

## 二、架构设计

### 2.1 总体架构

采用事件驱动架构，组件包括：

| 组件 | 职责 |
|------|------|
| Kotlin/Rust Client | 能力探测、任务提交、状态查询、停止/恢复和结果映射 |
| `protocol` 模块 | 跨模块值类型、线格式 DTO、对象引用和执行 Port |
| `dispatcher` 模块 | HTTP API、准入、排队、调度、预算、事件、状态持久化和结果终结 |
| `Scheduler Engine` | 成本、时限、模型能力和节点容量感知的决策器 |
| `calculator` 模块 | `SolverExecutionPort` 实现、OSPF 模型重建和 checkpoint 编排 |
| External Worker | 独立进程执行真实求解，输出结果文件和可选 checkpoint 文件 |
| State Store | 任务、切片、节点、预算、成本和调度审计状态 |
| `Event Bus` | 系统事件总线（主题订阅 + 定向消息） |
| Object Store | 模型、配置、snapshot、结果和 checkpoint artifact |

### 2.2 模块与信任边界

模块依赖方向固定为：

```text
dispatcher -> calculator -> protocol
     |                         ^
     +-------------------------+
```

1. `protocol` 不依赖 dispatcher 或 calculator，服务端与跨语言客户端以其线格式为契约基准。
2. dispatcher 在调度前加载并校验 `SolvePayload`，统一应用一次租户对象路径作用域，并按 `NormalizedModelType` 过滤节点。
3. calculator 负责把协议载荷转换或重建为 OSPF 可执行模型；求解器厂商能力不得泄漏到 dispatcher 和客户端业务层。
4. 外部 worker 输出的是本地文件路径。只有 bridge 读取、校验并上传后的内容才是可信的 tenant-scoped artifact；本地路径字符串本身不得作为远程结果引用。
5. Kotlin 客户端位于 `ospf-kotlin-framework`，Rust 客户端位于 `ospf-rust-framework`；本工程不再维护客户端模块。

### 2.3 事件主题

1. `SolvingRequest` - 任务请求
2. `SolvingControl` - 控制指令（accept/confirm/stop）
3. `TaskDispatch` - 任务分发
4. `SolverHeartBeat` - 求解器心跳
5. `SliceLifecycle` - 切片生命周期（slice-start/slice-end/slice-suspend/slice-resume）
6. `TaskResult` - 任务结果
7. `CostAndCapabilityUpdate` - 成本与能力更新
8. `MonitorAlert` - 监控告警

### 2.4 关键设计点

1. `TaskStatePort`/`NodeStatePort` 等状态存储是当前状态权威来源；重要状态变化同步发布领域事件，便于审计和回放，但不把事件日志误作唯一状态源。
2. 关键实体（任务、切片、节点）都具备幂等 ID。
3. 分发确认采用双向握手，避免多 Dispatcher 重复受理。
4. 模型、配置、checkpoint 和结果通过 `ObjectRef(path, version, etag)` 传递；数据库重建不得丢失引用完整性信息。
5. 严格协议结果在进入任务状态和对象存储前完成 schema、归属、状态一致性与摘要校验。

### 2.5 技术栈抽象层（Port/Adapter）

**设计原则：**
1. 核心调度逻辑只依赖抽象接口（Port），不依赖具体中间件。
2. 每种技术栈通过适配器（Adapter）接入，支持替换和并行灰度。
3. 先实现 `InMemory Adapter` 跑通，再替换为生产适配器。
4. 所有 Port 都要有契约测试（Contract Test）。

**核心 Port 列表：**

| Port | 职责 | 开发实现 | 生产实现 |
|------|------|----------|----------|
| `EventPort` / `TaskEventQueryPort` | 事件发布、订阅、确认、重试和任务事件查询 | InMemory | Kafka，可选镜像双写 |
| `TaskStatePort` | 任务与切片状态读写、CAS 和重建 | InMemory | Ktorm/PostgreSQL |
| `NodeStatePort` | 节点能力、心跳和容量状态 | InMemory | Ktorm/PostgreSQL |
| `ObjectStoragePort` | 模型、snapshot、结果对象存取 | InMemory / Local FS | S3 / MinIO |
| `CheckpointPort` | checkpoint 元数据、版本和保留策略 | InMemory / Local FS | S3 / MinIO |
| `DistributedLockPort` | 分布式锁与租约 | JVM Mutex | Ktorm/PostgreSQL |
| `BudgetPort` / `CostLedgerPort` | 预算控制、成本预留与执行账本 | InMemory | Ktorm/PostgreSQL |
| `SolverExecutionPort` | 求解启动、恢复、等待切片、结果和停止 | InMemory 模拟器 | OSPF bridge + external worker |
| `SchedulerConfigAuditPort` | 热更新快照和审计记录 | InMemory / Local FS | Ktorm/PostgreSQL |
| `TaskFamilyPort` / `ScoringModelPort` / `ExperimentPort` | 任务族、学习评分与实验 | InMemory | Ktorm/PostgreSQL |
| `MetricsPort` / `MetricsScrapePort` | 规范指标记录和抓取 | InMemory | Prometheus |
| `TracingPort` | 链路上下文和 span | InMemory | 当前仍为进程内实现，OpenTelemetry 为后续适配目标 |
| `AuthPort` | 身份认证和角色校验 | 关闭认证或测试替身 | JWT |

Port 接口以 `ospf-remote-solver-protocol/.../protocol/port` 和 dispatcher 的 `port` 包源码为唯一权威定义，本文档只描述稳定职责，不复制完整方法签名。

客户端协议、状态模型与交互流程仍作为跨语言客户端实现依据保留在本文档和 protocol 模块中。Kotlin 侧 `RemoteSolverClient`、`RemoteLinearSolver`、`RemoteQuadraticSolver` 等实现由 `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote` 提供；其它语言客户端应按本文档的 HTTP/API、任务状态与对象存储协议实现。

**适配器切换方式：**
1. 启动配置选择实现：`event.adapter=inmemory|kafka`、`storage.adapter=inmemory|localfs|s3` 等。
2. 所有实现通过依赖注入装配，业务层不感知具体技术栈。
3. `MirroringEventPort` 支持主事件端口与镜像端口双写灰度；镜像失败是否阻断主链路由 fail-open 配置决定。

---

## 三、领域模型

### 3.1 任务分类

| 维度 | 类型 | 调度策略 |
|------|------|----------|
| 复杂度 | 简单 | 直接分配到空闲节点，优先低成本满足时限 |
| 复杂度 | 复杂 | 时间片轮转 + 状态持久化 + warm start |
| 时效性 | 实时 | 优先级提高，尽量选更高性能节点 |
| 时效性 | 非实时 | 成本优先，允许等待 |

**复杂实时任务策略：** 统一按"复杂非实时"处理，理由：
1. 现实业务中通常可改造为简单模型迭代。
2. 强行做硬实时调度会显著抬高成本并恶化系统稳定性。

### 3.2 模型与结果协议

`ModelData` 通过互斥载荷形态和 `NormalizedModelType` 区分模型：

| 模型类型 | 载荷形态 | 执行路径 |
|----------|----------|----------|
| `LINEAR` | `SerializedLinearModel` 或对象引用 | OSPF 线性求解桥接 |
| `QUADRATIC` | `SerializedQuadraticModel` 或对象引用 | OSPF 二次求解桥接 |
| `CP` | `rawBytes` + `format=ospf-cp-snapshot-json` | CP snapshot 规范化、重建和 SCIP CP 执行 |
| `UNKNOWN` | 无法推断的引用或格式 | 仅允许在后续信息可明确类型时继续，否则拒绝调度 |

**稳定身份约束：**
1. 模型、变量、约束和目标使用稳定 identity ID，不得以展示名称或注册顺序代替。
2. identity scope、namespace、schema 和 provenance 在序列化、calculator 重建、checkpoint 恢复及结果映射中保持一致。
3. 兼容 snapshot 必须先规范化，再计算模型指纹或注册到 OSPF 模型，避免等价线格式产生不同恢复身份。

**结果语义：**
1. `problemStatus`、`terminationReason`、`solutionPresence` 和 `proofStatus` 是相互正交的事实，不能仅由 `feasible`/`optimal` 两个布尔值推断完整语义。
2. 线性和二次协议 v1 可继续使用浮点 `objectiveValue` 及兼容布尔字段。
3. CP 协议 v2 使用 `objectiveValueInt64`、按稳定 ID 索引的变量值和 interval 值；精确整数不得经过浮点转换。
4. 引用结果 artifact 时，`resultRef` 与 `artifactDigest` 必须成对出现；严格结果还必须携带匹配的 fingerprint/fingerprint schema、`runId` 和 `attemptId`。

字段级线格式由 [Remote CP Protocol Field Table](docs/remote-cp-protocol.md) 维护，本文档不复制完整字段表。

### 3.3 节点能力画像

每个节点维护：
1. `solverType`：gurobi/scip/heuristic/...
2. `performanceScore`：历史吞吐与收敛效率估计。
3. `pricePerSecond`：单位时间成本。
4. `minBillingUnit`：最小计费粒度。
5. `supportsInterrupt`：是否支持硬中断。
6. `supportsCheckpoint`：是否支持导出状态。
7. `supportsWarmStart`：是否支持恢复启动。
8. `parallelUnits`：可并发槽位。
9. `supportedModelTypes`：明确声明可执行的 `LINEAR`、`QUADRATIC`、`CP` 类型集合。
10. `licenseCostPerSlice`：每个切片额外产生的许可证费用。

客户端通过 `GET /api/v1/capabilities` 获取在线节点能力聚合。当前服务声明支持 protocol `2.0` 和 portable checkpoint，同时明确 `supportsNativeCheckpoint=false`；这两个能力不得合并为单一的“支持恢复”标志。

### 3.4 任务状态机

```text
CREATED(兼容/瞬时) -> QUEUED -> ACCEPTED -> DISPATCHING -> RUNNING -> COMPLETED
RUNNING -> SUSPENDED -> ACCEPTED
QUEUED/ACCEPTED -> WAITING_FOR_BUDGET -> QUEUED
```

异常分支：
1. 可控制状态可进入 `STOPPING -> STOPPED`。
2. 调度或运行态可进入 `FAILED`（节点故障、恢复失败、协议校验失败、超时失败）。
3. `WAITING_FOR_BUDGET` 表示当前预算无法覆盖候选切片；预算释放或降级成功后必须显式回到 `QUEUED` 再执行 accept/confirm，无法降级时终结为预算失败。
4. 正常 HTTP 提交流程直接创建 `QUEUED` 任务；`CREATED` 仅保留为兼容或未来分阶段准入状态。

### 3.5 切片状态机

```text
PLANNED -> RUNNING -> CHECKPOINTING -> SUSPENDED
              |             |
              +-------------+-> COMPLETED
              +-------------+-> FAILED
```

`FAILED` 切片保留错误原因和已产生的可信 artifact 引用；其所属任务根据 checkpoint 可用性进入 `SUSPENDED`、重新 `QUEUED` 或终结为 `FAILED`。

### 3.6 持久化模型

**task_state 核心字段：**
- 身份与隔离：`task_id`、`request_id`、`tenant_id`
- 调度状态：`status`、`priority`、`complexity`、`time_sensitivity`、`deadline_epoch_ms`、`assigned_node_id`
- 载荷：model/config/snapshot 的 `ObjectRef(path, version, etag)`、模型格式、内联 `SolverConfig`、`TaskMeta` 和扩展字段
- 最新产物：结果报告、结果/快照/checkpoint 的 `ObjectRef` 和结构化消息
- 预算与时间：`budget_scope`、`budget_limit`、`consumed_cost`、`created_at`、`updated_at`

关键索引语义：
- `(status, priority desc, created_at asc)`：队列出队
- request/tenant 查询：幂等受理和租户隔离
- model type：能力匹配和 CP 任务筛选

**slice_state 核心字段：**
- `slice_id`、`task_id`、`dispatch_id`、`status`、`node_id`
- `quantum_ms`、checkpoint/result 的 `ObjectRef(path, version, etag)`
- `started_at`、`finished_at`、`error`
- `slice_id` 全局唯一，同一任务的 dispatch 不得重复执行

**cost_ledger 核心字段：**
- `task_id`、`tenant_id`、`budget_scope`、`slice_id`、`node_id`
- `runtime_ms`、`billed_seconds`、`price_per_second`、`license_cost`、`total_cost`
- `created_at`

这里描述的是逻辑持久化边界。物理表名、字段、索引和迁移顺序以 [数据库迁移说明](ospf-remote-solver-dispatcher/deploy/sql/README.md) 及同目录 V1-V7 脚本为唯一权威来源。

---

## 四、调度设计

### 4.1 总体策略

采用"两层调度 + 一层执行编排"：
1. 第一层：任务分类与准入（简单/复杂，实时/非实时）。
2. 第二层：节点选择（成本、性能、能力约束）。
3. 第三层：复杂任务时间片轮转执行。

### 4.2 简单任务调度

**流程：**
1. 过滤可执行节点（`supportedModelTypes` 匹配 + 求解器兼容 + 空闲槽位 > 0）。
2. 估计每个候选节点的完成时间 `eta(node)`。
3. 在满足时限的候选中选最小 `estimatedCost(node)`。
4. 若无节点满足时限，选 `deadlineRisk` 最低且成本次优节点。

**评分函数：**
\[
score = w_c \cdot cost + w_t \cdot deadlineRisk + w_q \cdot queueDelay
\]
`score` 越小越优。

### 4.3 复杂任务调度

使用**时间片轮转 + 断点续算**。

**时间片长度（Quantum）：**
\[
Q = clamp(Q_{min}, Q_{max}, \alpha \cdot E[T_{solve}] - \beta \cdot E[T_{checkpoint}])
\]

原则：
1. checkpoint 代价越高，时间片越长。
2. 节点越贵，不宜给过长时间片（避免高价节点长期独占）。

**轮转队列优先级：**
\[
priority = p_1 \cdot urgency + p_2 \cdot waitingAge + p_3 \cdot progressNeed - p_4 \cdot costSensitivity
\]

**切片执行步骤：**
1. 选中任务与节点。
2. 载入并校验最近 checkpoint（若存在），确认租户、task、历史 attempt、模型与配置身份。
3. 下发本切片 quantum；task-level time/solution limit 仍保持独立语义。
4. 切片结束后先校验结果和 checkpoint，再上传 artifact 并持久化引用。
5. 已完成任务进入终态；未完成任务保存可信恢复点后回到轮转队列。

### 4.4 不支持中断的求解器处理

1. 不发求解器无法保证一致性的硬中断指令。
2. 通过切片 quantum、求解器 time limit 或阶段性迭代阈值，让求解受控返回。
3. 在返回点导出可验证的 incumbent、snapshot、指纹和诊断证据，形成 portable checkpoint。
4. 下一轮重建模型并注入已验证 incumbent；除非节点明确声明，否则不得把该过程描述为原生搜索状态恢复。
5. 当前 CP calculator 的恢复语义固定为 rebuild-based，服务能力对外声明 `supportsPortableCheckpoint=true`、`supportsNativeCheckpoint=false`。

### 4.5 成本控制机制

**统一成本模型：**
每个执行片段记录：节点单价、实际运行时长、计费粒度折算后费用、许可证占用费用。

**成本守护策略：**
1. 任务预算上限（单任务可配置）。
2. 业务线预算上限（时间窗口内总预算）。
3. 超预算时自动降级：优先低价节点，降低并行度。

**性价比动态学习：**
根据历史任务回放更新：
1. `performanceScore`（单位时间进展）。
2. 各节点"目标改善/秒"的分布。
3. 同类任务的最优节点族。

---

## 五、集成设计

### 5.1 OSPF 对接

**统一接口约束（强制）：**
1. 业务层、调度层、`ospf-remote-solver-calculator` 的 OSPF 适配层之外，**禁止直接调用** gurobi/scip 原生 API。
2. 所有求解请求必须先转换为中间模型，再通过 OSPF 提供的统一求解接口执行。
3. 厂商差异只能在 OSPF 适配层内处理，不允许向上泄漏。

**任务载荷抽象：**
- `modelData`：对象引用、内联线性/二次 DTO，或带格式标识的原始模型字节
- `configRef` / `config`：外部配置引用或内联 `SolverConfig`
- `snapshotRef`：输入 snapshot/checkpoint 引用（可空）
- `taskMeta`：目标类型、任务级时间限制、解数量和规模估计
- `extension`：协议保留的字符串扩展字段

`NormalizedModelType` 只负责跨模块能力分类，不复制一套求解模型继承体系。线性/二次任务使用共享序列化 DTO；CP 任务使用 OSPF CP snapshot codec 规范化和重建。

**CP 执行路径：**
1. 客户端先探测 `/api/v1/capabilities`，确认 protocol `2.0` 和 `CP` 模型类型同时可用。
2. `payloadRef` 指向序列化 `SolvePayload` artifact，而不是裸 snapshot；dispatcher 加载完整载荷、应用一次租户路径作用域并校验 `ModelData.format`。
3. calculator 对兼容 snapshot 先做 canonicalization，再进行身份恢复、指纹计算和 OSPF 模型注册。
4. `OspfExternalProcessBridge` 向 worker 传递模型路径、格式、task/slice/node/tenant、quantum、内联配置和可选 checkpoint 路径。
5. worker 写出 `SerializedSolution` JSON 和可选 portable checkpoint 文件；bridge 校验文件内容后上传到租户对象存储，并返回 `ObjectRef`。
6. 当前实现使用 SCIP CP 路径，不引入 OR-Tools，也不宣称原生 optional interval 或原生搜索树 checkpoint 能力。

**Remote Solver 客户端入口：**
- `RemoteSolverClient`：面向远程调度器的基础客户端，负责提交任务、等待切片、导出 checkpoint、获取最终结果。
- `RemoteLinearSolver : LinearSolver`：线性模型远程求解封装，负责将线性模型转换为远程求解 payload，并把远程结果映射回求解器结果。
- `RemoteQuadraticSolver : QuadraticSolver`：二次模型远程求解封装，负责将二次模型转换为远程求解 payload，并把远程结果映射回求解器结果。

职责：能力探测、远程任务提交、状态或切片轮询、checkpoint 管理、结果映射，不直接触达 gurobi/scip API。CP 客户端必须使用共享 protocol v2 字段，并拒绝把缺少 CP 能力的服务端当作线性服务端降级调用。

客户端必须按服务端 protocol 中的 `TaskId`、`SliceId`、`NodeId`、`TenantId`、`ObjectRef`、`SolvePayload`、`SliceResult`、`SolveResult` 等协议模型交互。

客户端侧协议设计继续保留，用于 Kotlin 以外语言实现远程求解客户端。Kotlin 客户端封装由 `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote` 提供；Rust 客户端已在 `ospf-rust-framework/src/solver/remote` 实现。本工程保留 dispatcher、calculator、protocol 等服务端职责。

### 5.2 事件契约

**统一事件包络：**
- `eventId`：事件唯一 ID（幂等主键）
- `eventType`：业务事件类型
- `schemaVersion`：整数事件结构版本
- `occurredAtEpochMs`：事件生成时间戳
- `producer`：生产方标识
- `traceId` / `spanId`：可观测链路字段
- `payload`：序列化业务载荷字节
- `attributes`：事件附加属性；`tenantId` 作为必需的规范 header/attribute 全链路传递

**关键事件结构：**
- `TaskDispatchPayload`：dispatchId, taskId, sliceId, nodeId, quantumMs, checkpointInRef
- `SliceLifecyclePayload`：taskId, sliceId, dispatchId, action, status, taskStatus, nodeId, quantumMs, reason
- `TaskResultPayload`：taskId, status, reasonCode, message
- `SolvingControlPayload`：taskId, action, status, dispatch/node/dispatcher、状态转换、原因和操作者信息
- `CostUpdatePayload`：task/slice/node、费用、运行时长、性能和在线状态
- `HeartbeatPayload`：nodeId, heartbeatAt
- `TaskSummaryPayload`：taskId, status, priority

**版本演进规则：**
1. 事件 `schemaVersion` 使用整数版本并由 `EventSchemaRegistry` 显式登记；严格模式拒绝未登记版本，兼容演进新增字段应保持可选，payload 消费端忽略未知字段。
2. 结果和 checkpoint artifact 使用独立 schema 策略，不能套用事件的宽松反序列化规则。
3. 线性/二次 v1 DTO 保留兼容字段；CP 严格结果当前只接受精确 `2.0`，服务端和客户端都拒绝未知未来主版本。
4. 严格 artifact 缺少摘要、run/attempt 归属、fingerprint schema 或存在状态冲突时必须拒绝，不得静默降级。
5. 跨仓库 fixture 必须由客户端与服务端直接解码，防止两侧各自定义私有 CP DTO 后产生表面兼容。

### 5.3 API 设计

**能力协商：**
- `GET /api/v1/capabilities` - 返回协议版本、在线节点模型类型及 portable/native checkpoint 能力

**任务管理：**
- `POST /api/v1/tasks` - 提交任务
- `GET /api/v1/tasks/{taskId}` - 查询任务
- `POST /api/v1/tasks/{taskId}/stop` - 停止任务
- `POST /api/v1/tasks/{taskId}/resume` - 恢复任务

**调度管理：**
- `POST /api/v1/scheduler/config/hot-reload` - 热更新调度参数
- `POST /api/v1/scheduler/config/rollback` - 回滚调度参数
- `GET /api/v1/scheduler/config/audits` - 查询审计日志

**监控与回放：**
- `GET /api/v1/monitor/overview` - 监控概览
- `GET /api/v1/tasks/{taskId}/timeline` - 任务时序回放
- `GET /monitor` - 监控看板页面
- `GET /metrics` - Prometheus 指标

**健康检查：**
- `GET /health` - 健康状态
- `GET /health/ready` - 就绪探针
- `GET /health/live` - 存活探针

### 5.4 错误码分层

| 层级 | 错误码示例 |
|------|------------|
| 参数/协议层 | `INVALID_ARGUMENT` |
| 状态层 | `INVALID_TASK_STATE_TRANSITION` |
| 资源层 | `NO_ELIGIBLE_NODE_AVAILABLE`, `NO_COMPATIBLE_NODE_AVAILABLE`, `NODE_OFFLINE` |
| 预算层 | `TASK_FAILED_BUDGET_EXCEEDED` |
| 执行层 | `SOLVER_EXECUTION_FAILED`, `CHECKPOINT_EXPORT_FAILED`, `TASK_FAILED_HARD_TIMEOUT`, `TASK_FAILED_SLICE_TIMEOUT` |
| 系统层 | `EVENT_PUBLISH_FAILED`, `STORAGE_IO_FAILED`, `INTERNAL_ERROR` |

**返回规范：**
1. 对外 API 返回稳定 `code + message + traceId`。
2. 内部异常统一映射领域错误码，禁止透传底层异常类名。
3. 失败的 `TaskResultPayload` 必须包含稳定 `reasonCode`。
4. 协议、checkpoint 或结果校验失败必须在 artifact 持久化和任务终结前返回结构化错误，不得以兼容字段掩盖不一致。

---

## 六、可靠性设计

### 6.1 幂等机制

1. `(tenantId, requestId)` 是客户端提交幂等键，`taskId`、`dispatchId`、`sliceId` 是服务端执行身份。
2. 相同 `sliceId` 重复投递只执行一次。
3. 结果和切片状态回写使用 compare-and-set，防止过期 worker 覆盖较新的终态。
4. event 使用稳定 idempotency key，重试不得产生第二次业务状态转换。

### 6.2 故障恢复

1. Dispatcher 重启后从持久化任务/切片状态重建队列，并使用事件查询补齐时间线和审计信息。
2. Solver 节点心跳超时后将其活动切片标记为 `FAILED`，回收容量并清除任务节点分配。
3. 有可信 checkpoint 的任务进入 `SUSPENDED` 后恢复；无 checkpoint 的任务重新 `QUEUED`，从最近稳定输入重跑。
4. CP 恢复必须重新加载原始 payload、内联配置与 ObjectRef ETag，规范化 snapshot 后核对模型、配置、求解器、run 和历史 attempt 身份。
5. 恢复失败不能回退到未校验 artifact；应保留原始错误原因并按策略重试或终结任务。

### 6.3 Artifact 完整性与信任链

1. `payloadRef`、`resultRef`、`snapshotRef` 和 checkpoint 引用必须经过租户作用域解析，且只允许作用域化一次。
2. `RemoteResultValidator` 在 dispatcher 边界校验 schema 主版本、task/slice 归属、正交状态、目标值类型、指纹集合以及 `resultRef`/摘要配对关系。
3. `PortableCheckpointCodec` 对 v2 envelope 做规范编码和完整性摘要校验；legacy checkpoint 只能通过显式兼容迁移路径进入 v2，不能伪装为原生 v2。
4. calculator/bridge 负责求解器相关的数学解、snapshot、incumbent、冲突证据和配置一致性校验；通用协议校验器不替代后端数学复验。
5. 外部 worker 的退出码、标准输出和本地文件路径均视为不可信输入。bridge 必须读取并校验实际文件内容，成功上传后才可发布远程 `ObjectRef`。
6. ObjectRef 的 version 和 ETag 必须随任务、切片及数据库重建完整保留，避免恢复时读取到不同对象版本。

### 6.4 超时策略

| 超时类型 | 说明 |
|----------|------|
| 任务总超时 | hard timeout，任务级别最大运行时间 |
| 切片超时 | slice timeout，单次时间片最大运行时间 |
| 心跳超时 | node offline，节点无响应判定 |

task-level time limit、slice quantum 和 dispatcher timeout 是三个独立约束：task limit 控制业务总求解窗口，quantum 控制本轮执行配额，dispatcher timeout 负责识别失联或超时执行。不得用其中一个字段替代另一个。

### 6.5 灰度发布策略

**适配器灰度：**
1. 先 `inmemory` 单机回归。
2. 开启 `event.mirror.enabled=true` 做 in-memory + Kafka 双写观测。
3. 仅当消费一致性与延迟达标后，切主为 `event.adapter=kafka`。

**风险开关：**
- `event.mirror.fail-open=true`：镜像失败不阻断主链路
- `scheduler.performance-learning.enabled=true`：可快速回退到固定评分
- `checkpoint.retention.max-per-task`：防止对象存储无限增长

**CP protocol v2 发布门禁：**
1. 先应用数据库 V5-V7 迁移，确认模型能力、内联配置、结果报告和 ObjectRef ETag 可重建。
2. 服务端能力接口仅在可用节点真实声明 CP 时返回 `CP`，客户端必须在上传大对象前探测。
3. 服务端、Kotlin 客户端和 Rust 客户端使用同一 canonical fixture 做直接解码测试。
4. portable checkpoint 验收不能替代 native checkpoint 能力；当前始终对外声明 native 为 false。

---

## 七、运营与治理

### 7.1 可观测性

**关键指标：**
1. 任务成功率、SLA 达成率。
2. 平均成本/任务、P95 成本。
3. 节点利用率与排队时长。
4. checkpoint 开销占比。
5. 单位成本目标改善率。
6. protocol/checkpoint/result 校验失败率和 artifact 摘要不一致次数。

**告警规则：**
1. 成本突增告警。
2. 轮转队列积压告警。
3. 快照失败率告警。
4. 单节点异常失败率告警。
5. 严格协议校验失败或 artifact 完整性错误突增告警。

### 7.2 配置治理

**配置分层：**
- `base`：跨环境一致的语义开关
- `env`：环境差异项（连接串、bucket、topic）
- `runtime`：可热更新项（权重、阈值、降级开关）

**变更原则：**
1. 热更新仅允许白名单键；非白名单必须重启生效。
2. 每次变更必须记录：`operator/changeSet/effectiveAt/rollbackKey`。
3. 同一时间仅允许一个生效中的调度参数变更窗口。

### 7.3 SLO 与容量

**建议 SLO：**
| 指标 | 目标 |
|------|------|
| 提交成功率 | >= 99.9%（5分钟窗口） |
| 调度决策延迟 P95 | <= 200ms |
| 简单任务完成时延 P95 | <= 5s |
| checkpoint 写入失败率 | <= 0.5% |

**容量基线：**
| 维度 | 指标 |
|------|------|
| 调度侧 | 单实例 >= 500 事件/秒 |
| API 侧 | 200 并发下错误率 < 0.1% |
| 存储侧 | checkpoint 写入时延 < 150ms |

**超阈值自动动作：**
1. `queue_length` 超阈值：提高 `w_q` 权重。
2. `budget_burn_rate` 超阈值：触发低价节点优先策略。
3. `node_offline_ratio` 超阈值：降低单节点分配上限。

### 7.4 安全与多租户

**最小安全要求：**
1. API 层校验 `tenantId`，并在任务全链路透传。
2. 对象存储路径按租户隔离：`/{tenantId}/models|checkpoints|results/...`。
3. 事件规范 header/attribute 包含 `tenantId`，跨租户消费默认拒绝。
4. worker 本地路径不构成租户隔离边界，只有 bridge 上传后的 tenant-scoped `ObjectRef` 可进入协议和持久化状态。

**多租户调度策略：**
1. 基线配额：每租户保底并发槽位。
2. 弹性借用：空闲配额可借用，超出时按优先级回收。
3. 预算隔离：`budgetScope` 默认包含租户前缀，防止串账。

**审计要求：**
1. 所有 stop/resume 操作记录操作者、来源、原因。
2. 关键配置变更与预算变更进入审计日志，不可静默覆盖。

---

## 附录

### A. 事件审计投影示例

以下 JSON 是便于阅读和回放的逻辑投影。运行时 `EventEnvelope.payload` 为序列化字节，`tenantId` 通过规范事件 header/attribute 传递。

**TaskDispatch：**
```json
{
  "eventId": "evt-8b93f9d2",
  "eventType": "TaskDispatch",
  "schemaVersion": 1,
  "occurredAtEpochMs": 1760000000000,
  "producer": "dispatcher-core",
  "traceId": "tr-6d1f",
  "spanId": "sp-19",
  "tenantId": "tenant-a",
  "payload": {
    "dispatchId": "disp-1001",
    "taskId": "task-9001",
    "sliceId": "slice-0003",
    "nodeId": "node-a",
    "quantumMs": 4000,
    "checkpointInRef": "checkpoints/task-9001/latest"
  }
}
```

**SliceLifecycle：**
```json
{
  "eventId": "evt-91b1aa10",
  "eventType": "SliceLifecycle",
  "schemaVersion": 1,
  "occurredAtEpochMs": 1760000004200,
  "producer": "dispatcher-core",
  "traceId": "tr-6d1f",
  "spanId": "sp-23",
  "tenantId": "tenant-a",
  "payload": {
    "taskId": "task-9001",
    "sliceId": "slice-0003",
    "dispatchId": "disp-1001",
    "action": "slice-end",
    "status": "SUSPENDED",
    "taskStatus": "QUEUED",
    "nodeId": "node-a",
    "quantumMs": 4000,
    "reason": "quantum_exhausted"
  }
}
```

### B. 数据库迁移职责

物理 DDL 不在设计文档内重复维护。生产结构和执行顺序以 [数据库迁移说明](ospf-remote-solver-dispatcher/deploy/sql/README.md) 及同目录迁移脚本为唯一权威来源：

| 版本 | 职责 |
|------|------|
| V1 | task、slice、cost ledger 核心表和队列索引 |
| V2 | node、budget、distributed lock 基础设施表 |
| V3 | scheduler 配置快照与审计表 |
| V4 | task/cost ledger 的多租户字段与索引 |
| V5 | CP 模型能力、payload format、结果报告和模型类型索引 |
| V6 | CP 恢复所需的内联 `SolverConfig` 持久化 |
| V7 | payload、result、snapshot、checkpoint `ObjectRef.etag` 持久化 |

迁移要求：
1. V1-V7 必须按版本顺序执行，并记录到 `remote_solver_migration_history`。
2. 新增持久化字段必须同时覆盖写入、读取、重启重建和兼容迁移测试。
3. 设计评审关注逻辑模型与恢复不变量；字段类型、默认值、索引名和数据库兼容语法由迁移脚本及其 README 维护。
4. 数据库回放测试是 CP2 生产验收的必要条件，迁移脚本可重复执行不等于已经证明完整重启恢复能力。

### C. 默认参数基线

**时间片与复杂任务判定：**
| 参数 | 默认值 |
|------|--------|
| `simpleTaskQuantumMs` | 2000 |
| `complexTaskQuantumMs` | 4000 |
| `complexTaskQuantumMinMs` | 2000 |
| `complexTaskQuantumMaxMs` | 10000 |
| `complexTaskVariableThreshold` | 5000 |
| `complexTaskConstraintThreshold` | 5000 |

**调度权重：**
| 参数 | 默认值 |
|------|--------|
| `costWeight` (w_c) | 0.6 |
| `deadlineRiskWeight` (w_t) | 0.3 |
| `queueDelayWeight` (w_q) | 0.1 |
| `complexUrgencyWeight` (p_1) | 0.4 |
| `complexWaitingAgeWeight` (p_2) | 0.3 |
| `complexProgressNeedWeight` (p_3) | 0.2 |
| `complexCostSensitivityWeight` (p_4) | 0.1 |

**超时参数：**
| 参数 | 默认值 |
|------|--------|
| `nodeHeartbeatTimeoutMs` | 30000 |
| `sliceTimeoutGraceMs` | 200 |
| `dispatchLockTtlMs` | 15000 |
| `maxSchedulingBatch` | 64 |

### D. 生产配置模板

```properties
# Event
event.adapter=kafka
event.kafka.bootstrap-servers=127.0.0.1:9092
event.mirror.enabled=true
event.mirror.fail-open=true

# Database (Ktorm/PostgreSQL)
distributed-lock.adapter=ktorm
distributed-lock.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
task-state.adapter=ktorm
task-state.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
node-state.adapter=ktorm
node-state.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
budget.adapter=ktorm
budget.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
cost-ledger.adapter=ktorm
cost-ledger.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
scheduler.audit.adapter=ktorm
scheduler.audit.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver

# Solver Execution
solver-execution.adapter=ospf
solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
solver-execution.ospf.bridge.arg.command=/opt/solver/bin/ospf-worker

# Storage
storage.adapter=s3
storage.s3.bucket=remote-solver
storage.s3.endpoint=http://127.0.0.1:9000
storage.s3.access-key-id=minioadmin
storage.s3.secret-access-key=minioadmin

# Metrics
metrics.adapter=prometheus
metrics.canonical.enabled=true

# API
api.http.port=18080
api.tenant.auth.enabled=true
api.monitor.auth.enabled=true

# Scheduler
scheduler.performance-learning.enabled=true
scheduler.performance-learning.rate=0.2
scheduler.hot-reload.enabled=true
checkpoint.retention.max-per-task=20

# Monitor Alert
monitor.alert.enabled=true
monitor.alert.check-interval-ms=5000
monitor.alert.route.webhook.enabled=true
monitor.alert.route.webhook.url=http://alertmanager:9093/api/v1/alerts
```

### E. 验收标准

**阶段 A（原型增强版）：**
1. 完成任务提交、排队、调度、执行、结果回写的完整闭环。
2. 简单任务按评分函数选出最低分节点。
3. 复杂任务可进入轮转队列并至少执行两轮切片。
4. 切片后可保存快照，下一轮可基于快照恢复执行。
5. LINEAR、QUADRATIC、CP 任务只分配给声明相应 `supportedModelTypes` 的节点。

**阶段 B（生产可用版）：**
1. 关键实体具备幂等保障。
2. 节点故障时，未完成切片可被回收并重调度。
3. 成本账本与预算控制可阻止超预算扩张。
4. 故障注入测试可稳定通过。
5. 数据库重启重建保留 payload、result、snapshot、checkpoint 的 path/version/etag 和内联配置。
6. 损坏、错租户、错 task/attempt 或状态冲突的严格 artifact 在持久化前被拒绝。

**阶段 C（持续优化版）：**
1. `performanceScore` 可根据历史回放自动更新。
2. 同类任务可自动给出候选"最优节点族"。
3. 成本与时延双目标优化有可量化收益。
4. CP protocol v2 的 capability 探测、跨仓库 fixture、HTTP/object-storage/dispatcher/calculator 闭环和 rebuild 恢复均有独立验收证据。

### F. 压测场景

1. **小任务洪峰**：1000 个简单任务在 1 分钟内提交，验证吞吐与排队稳定性。
2. **混合负载**：简单任务与复杂任务按 7:3 混合，验证轮转公平性。
3. **故障恢复**：运行中随机下线 20% 节点，验证切片回收与恢复时延。
4. **成本守护**：注入预算上限，验证超预算后降级策略是否生效。
5. **CP artifact 压力**：混合提交 CP v2 结果和 portable checkpoint，验证摘要校验、对象存储吞吐及恢复开销。

### G. 测试矩阵

**单元测试：**
- `SchedulerEngine`：评分函数与节点选择边界
- `RemoteSolverService`：状态流转、超时、停止策略
- `ModelData` / `NormalizedModelType`：模型类型推断和能力匹配
- `RemoteResultValidator` / `PortableCheckpointCodec`：严格 schema、归属、状态和完整性不变量
- `OspfCpSnapshotExecutor`：snapshot 规范化、稳定身份重建、结果与 checkpoint 复验

**契约测试：**
- 全 Port 的 Contract Test 持续保留
- 新增 Adapter 时必须复用同一套契约测试
- 事件适配器需覆盖"重复投递、延迟重试、消费者组负载均衡"
- external worker/bridge 需覆盖退出码、缺失文件、路径伪造、摘要错误和错 task/attempt
- 跨仓库 CP fixture 必须由双方共享 DTO 直接解码，不允许测试专用私有 DTO

**端到端回归：**
- `submit -> dispatch -> slice -> checkpoint -> resume -> complete` 主链路
- 预算超限触发降级与失败路径
- 节点掉线恢复路径（带 checkpoint 与不带 checkpoint）
- 灰度双写路径（主写成功/镜像失败 fail-open）
- `/api/v1/capabilities -> CP submit -> dispatch -> external worker -> artifact upload -> strict result -> rebuild resume` 主链路
- V1-V7 数据库迁移后的真实重启/回放路径，验证 ObjectRef ETag 和内联配置不丢失
