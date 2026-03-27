# Remote Solver 事件驱动异构调度系统设计

## 一、概述

### 1.1 业务目标

在多供应商、多性能、多计费模式的求解器资源池中，以**最低成本**完成全部算法任务，并满足可接受的时效性要求。

### 1.2 技术目标

1. 复用现有原型的事件驱动架构思想（Client/Dispatcher/Solver）。
2. 支持异构节点调度（高性能高价 + 低性能低价）。
3. 支持复杂非实时任务的时间片轮转与断点续算。
4. 对不支持中断的求解器，利用"状态保存 + 下一次 warm start"实现近似可抢占。
5. 对复杂实时任务，按策略降级处理为复杂非实时任务。

### 1.3 非目标（明确边界）

1. 不直接保证复杂实时任务的硬实时 SLA。
2. 不在第一阶段实现跨数据中心强一致多活。
3. 不强绑定具体求解器 SDK，实现统一抽象后由插件接入。

### 1.4 核心约束

**任务约束：**
1. 每个任务有业务优先级、是否实时、模型复杂度、预计求解规模。
2. 任务必须在可接受时间内给出可用结果（最优或可行解）。
3. 复杂任务可能需要多轮执行和多次状态恢复。

**资源约束：**
1. 节点能力差异：CPU/GPU、并行数、求解器类型、许可证上限。
2. 计费差异：按秒/按分钟、最小计费粒度、并发附加费、许可证占用费。
3. 部分求解器不支持"立即中断"，但支持"状态导出和恢复"。

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
3. 对不支持中断的求解器，使用"受控自然返回 + 状态保存"替代抢占。
4. 复杂实时任务当前不处理，按复杂非实时处理是合理工程折中。

---

## 二、架构设计

### 2.1 总体架构

采用事件驱动架构，组件包括：

| 组件 | 职责 |
|------|------|
| `Client Gateway` | 接收任务、查询状态、发起停止 |
| `Dispatcher Core` | 调度中心，包含准入、排队、分片、分配 |
| `Scheduler Engine` | 成本感知 + SLA 感知决策器 |
| `Checkpoint Manager` | 状态快照存储和恢复编排 |
| `Solver Agent` | 节点侧执行代理，对接具体求解器 |
| `State Store` | 任务状态、调度状态、节点状态 |
| `Event Bus` | 系统事件总线（主题订阅 + 定向消息） |
| `Object Store` | 模型文件、配置、快照、结果对象 |

### 2.2 事件主题

1. `SolvingRequest` - 任务请求
2. `SolvingControl` - 控制指令（accept/confirm/stop）
3. `TaskDispatch` - 任务分发
4. `SolverHeartBeat` - 求解器心跳
5. `SliceLifecycle` - 切片生命周期（slice-start/slice-end/slice-suspend/slice-resume）
6. `TaskResult` - 任务结果
7. `CostAndCapabilityUpdate` - 成本与能力更新
8. `MonitorAlert` - 监控告警

### 2.3 关键设计点

1. 所有状态变化都通过事件驱动，便于审计和回放。
2. 关键实体（任务、切片、节点）都具备幂等 ID。
3. 分发确认采用双向握手，避免多 Dispatcher 重复受理。

### 2.4 技术栈抽象层（Port/Adapter）

**设计原则：**
1. 核心调度逻辑只依赖抽象接口（Port），不依赖具体中间件。
2. 每种技术栈通过适配器（Adapter）接入，支持替换和并行灰度。
3. 先实现 `InMemory Adapter` 跑通，再替换为生产适配器。
4. 所有 Port 都要有契约测试（Contract Test）。

**核心 Port 列表：**

| Port | 职责 | 开发实现 | 生产实现 |
|------|------|----------|----------|
| `EventPort` | 事件发布、订阅、确认、重试 | InMemory Channel | Kafka / Pulsar |
| `TaskStatePort` | 任务/切片/调度状态读写 | InMemory Map | PostgreSQL |
| `NodeStatePort` | 节点能力、心跳、容量状态 | InMemory Map | Redis + PostgreSQL |
| `ObjectStoragePort` | 模型、快照、结果对象存取 | Local FS | MinIO / S3 |
| `CheckpointPort` | 快照元数据和版本管理 | Local FS + JSON | MinIO + PostgreSQL |
| `DistributedLockPort` | 分布式锁与租约 | JVM Mutex | Redis RedLock / PG advisory lock |
| `BudgetPort` | 预算控制与成本扣减 | InMemory Counter | PostgreSQL + Redis |
| `SolverExecutionPort` | 求解器启动、恢复、暂停、结果 | Mock Executor | OSPF + Gurobi/SCIP |
| `MetricsPort` | 指标上报 | 日志打印 | Prometheus |
| `TracingPort` | 链路追踪 | No-op | OpenTelemetry |
| `AuthPort` | 认证与鉴权 | InMemory | JWT / OIDC |

**Kotlin 接口草案：**

```kotlin
interface EventPort {
    suspend fun publish(topic: EventTopicName, key: String, payload: ByteArray)
    suspend fun subscribe(topic: EventTopicName, consumerGroup: ConsumerGroupId, handler: suspend (EventRecord) -> Unit)
    suspend fun ack(record: EventRecord)
    suspend fun nack(record: EventRecord, retryAt: Instant? = null)
}

interface TaskStatePort {
    suspend fun getTask(taskId: TaskId): TaskState?
    suspend fun upsertTask(task: TaskState)
    suspend fun compareAndSet(taskId: TaskId, from: TaskStatus, to: TaskStatus): Boolean
    suspend fun appendSlice(slice: SliceState)
}

interface ObjectStoragePort {
    suspend fun put(path: ObjectPath, bytes: ByteArray, metadata: Map<String, String> = emptyMap()): ObjectRef
    suspend fun get(ref: ObjectRef): ByteArray
    suspend fun delete(ref: ObjectRef): Boolean
}

interface SolverExecutionPort {
    suspend fun start(payload: SolvePayload, taskId: TaskId, sliceId: SliceId, nodeId: NodeId, tenantId: TenantId): ExecutionHandle
    suspend fun resume(payload: SolvePayload, checkpoint: ObjectRef, taskId: TaskId, sliceId: SliceId, nodeId: NodeId, tenantId: TenantId): ExecutionHandle
    suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult
    suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef?
    suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult?
    suspend fun stop(handle: ExecutionHandle): Boolean
}
```

客户端协议、状态模型与交互流程仍作为跨语言客户端实现依据保留在本文档和 protocol 模块中。Kotlin 侧 `RemoteSolverClient`、`RemoteLinearSolver`、`RemoteQuadraticSolver` 等实现由 `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote` 提供；其它语言客户端应按本文档的 HTTP/API、任务状态与对象存储协议实现。

**适配器切换方式：**
1. 启动配置选择实现：`event.adapter=kafka|inmemory`、`storage.adapter=minio|local`。
2. 所有实现通过依赖注入装配，业务层不感知具体技术栈。
3. 支持双写/双读灰度（例如 `EventPort` 先 in-memory + Kafka 并行）。

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

### 3.2 节点能力画像

每个节点维护：
1. `solverType`：gurobi/scip/heuristic/...
2. `performanceScore`：历史吞吐与收敛效率估计。
3. `pricePerSecond`：单位时间成本。
4. `minBillingUnit`：最小计费粒度。
5. `supportsInterrupt`：是否支持硬中断。
6. `supportsCheckpoint`：是否支持导出状态。
7. `supportsWarmStart`：是否支持恢复启动。
8. `parallelUnits`：可并发槽位。

### 3.3 任务状态机

```
Created -> Accepted -> Queued -> Dispatching -> Running -> Suspended -> Running -> Completed
```

异常分支：
1. 任意状态可进入 `Stopping -> Stopped`。
2. 任意运行态可进入 `Failed`（节点故障、恢复失败、超时失败）。

### 3.4 切片状态机

```
SlicePlanned -> SliceRunning -> SliceCheckpointing -> SliceSuspended
```

或：
```
SliceRunning -> SliceCompleted`（任务整体结束）
```

### 3.5 持久化模型

**task_state 核心字段：**
- `task_id`（PK）
- `request_id`（UK）
- `status`, `priority`, `complexity`, `time_sensitivity`
- `deadline_epoch_ms`
- `current_node_id`（可空）
- `last_checkpoint_ref`（可空）
- `created_at/updated_at`

索引建议：
- `(status, priority desc, created_at asc)`：队列出队
- `(request_id)`：幂等受理

**slice_state 核心字段：**
- `slice_id`（PK）
- `task_id`, `dispatch_id`, `status`, `node_id`
- `quantum_ms`, `elapsed_ms`
- `checkpoint_in_ref` / `checkpoint_out_ref`
- `started_at/ended_at`

约束：`unique(task_id, dispatch_id, slice_id)`

**cost_ledger 核心字段：**
- `record_id`（PK）
- `task_id`, `budget_scope`, `node_id`
- `runtime_ms`, `billed_cost`, `license_cost`, `total_cost`
- `recorded_at`

---

## 四、调度设计

### 4.1 总体策略

采用"两层调度 + 一层执行编排"：
1. 第一层：任务分类与准入（简单/复杂，实时/非实时）。
2. 第二层：节点选择（成本、性能、能力约束）。
3. 第三层：复杂任务时间片轮转执行。

### 4.2 简单任务调度

**流程：**
1. 过滤可执行节点（求解器兼容 + 空闲槽位 > 0）。
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
2. 载入最近快照（若存在）。
3. 下发本切片约束（timeLimit/iterationLimit/objectiveGapStep）。
4. 切片结束后导出状态快照并持久化。
5. 任务回到轮转队列，等待下一轮。

### 4.4 不支持中断的求解器处理

1. 不发硬中断指令。
2. 通过配置 `timeLimit` 或阶段性迭代阈值，让求解自然返回。
3. 在返回点保存状态，作为下一轮 warm start 输入。

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
1. 业务层、调度层、`remote-solver-adapter-ospf` 之外的模块，**禁止直接调用** gurobi/scip 原生 API。
2. 所有求解请求必须先转换为中间模型，再通过 OSPF 提供的统一求解接口执行。
3. 厂商差异只能在 OSPF 适配层内处理，不允许向上泄漏。

**任务载荷抽象：**
- `modelRef`：模型对象引用
- `configRef`：求解参数引用
- `snapshotRef`：快照引用（可空）
- `taskMeta`：目标类型、时间限制、解数量等

**中间模型层：**
- `NormalizedModel`：统一表达变量、约束、目标、初值、求解配置
- `NormalizedLinearModel`：线性模型子类型
- `NormalizedQuadraticModel`：二次模型子类型
- `ModelTranslator`：将上层任务输入转换为 `NormalizedModel`

**Remote Solver 客户端入口：**
- `RemoteSolverClient`：面向远程调度器的基础客户端，负责提交任务、等待切片、导出 checkpoint、获取最终结果。
- `RemoteLinearSolver : LinearSolver`：线性模型远程求解封装，负责将线性模型转换为远程求解 payload，并把远程结果映射回求解器结果。
- `RemoteQuadraticSolver : QuadraticSolver`：二次模型远程求解封装，负责将二次模型转换为远程求解 payload，并把远程结果映射回求解器结果。

职责：远程任务提交、状态轮询或切片轮询、checkpoint 管理、结果映射，不直接触达 gurobi/scip API。客户端必须按服务端 protocol 中的 `TaskId`、`SliceId`、`NodeId`、`TenantId`、`ObjectRef`、`SolvePayload`、`SliceResult`、`SolveResult` 等协议模型交互。

客户端侧协议设计继续保留，用于 Kotlin 以外语言实现远程求解客户端。Kotlin 客户端封装由 `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote` 提供；Rust 客户端已在 `ospf-rust-framework/src/solver/remote` 实现。本工程保留 dispatcher、calculator、protocol 等服务端职责。

### 5.2 事件契约

**统一事件包络：**
- `eventId`：事件唯一 ID（幂等主键）
- `eventType`：业务事件类型
- `schemaVersion`：事件结构版本
- `occurredAt`：事件生成时间戳
- `producer`：生产方标识
- `traceId` / `spanId`：可观测链路字段
- `tenantId`：租户标识
- `payload`：业务数据对象

**关键事件结构：**
- `SolvingRequestCreated`：requestId, taskId, complexity, timeSensitivity, payloadRef, budgetScope
- `TaskDispatched`：dispatchId, taskId, nodeId, sliceId, quantumMs, checkpointInRef
- `SliceFinished`：sliceId, taskId, nodeId, elapsedMs, checkpointOutRef, interimResultRef, completed
- `TaskTerminalized`：taskId, terminalStatus, finalResultRef, reasonCode
- `NodeHeartbeatUpdated`：nodeId, availableUnits, performanceScore, timestamp

**版本演进规则：**
1. `schemaVersion` 仅允许递增，不回退。
2. 新增字段必须可选，禁止删除必填字段。
3. 消费端按"忽略未知字段"策略实现向前兼容。

### 5.3 API 设计

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
| 参数层 | `INVALID_ARGUMENT`, `MISSING_REQUIRED_FIELD` |
| 状态层 | `INVALID_TASK_STATE_TRANSITION` |
| 资源层 | `NO_ELIGIBLE_NODE`, `NODE_OFFLINE` |
| 预算层 | `TASK_BUDGET_EXCEEDED`, `BUDGET_SCOPE_EXCEEDED` |
| 执行层 | `SOLVER_EXECUTION_FAILED`, `CHECKPOINT_EXPORT_FAILED` |
| 系统层 | `EVENT_PUBLISH_FAILED`, `STORAGE_IO_FAILED` |

**返回规范：**
1. 对外 API 返回稳定 `code + message + traceId`。
2. 内部异常统一映射领域错误码，禁止透传底层异常类名。
3. `TaskFailed` 事件中必须包含 `reasonCode`。

---

## 六、可靠性设计

### 6.1 幂等机制

1. `requestId`、`dispatchId`、`sliceId` 全局唯一。
2. 相同 `sliceId` 重复投递只执行一次。
3. 结果回写使用 compare-and-set 防止覆盖。

### 6.2 故障恢复

1. Dispatcher 重启后从事件日志恢复队列。
2. Solver 节点心跳超时后回收其未完成切片。
3. 快照存在时优先恢复；无快照则从最近稳定点重跑。

### 6.3 超时策略

| 超时类型 | 说明 |
|----------|------|
| 任务总超时 | hard timeout，任务级别最大运行时间 |
| 切片超时 | slice timeout，单次时间片最大运行时间 |
| 心跳超时 | node offline，节点无响应判定 |

### 6.4 灰度发布策略

**适配器灰度：**
1. 先 `inmemory` 单机回归。
2. 开启 `event.mirror.enabled=true` 做 in-memory + Kafka 双写观测。
3. 仅当消费一致性与延迟达标后，切主为 `event.adapter=kafka`。

**风险开关：**
- `event.mirror.fail-open=true`：镜像失败不阻断主链路
- `scheduler.performance-learning.enabled=true`：可快速回退到固定评分
- `checkpoint.retention.max-per-task`：防止对象存储无限增长

---

## 七、运营与治理

### 7.1 可观测性

**关键指标：**
1. 任务成功率、SLA 达成率。
2. 平均成本/任务、P95 成本。
3. 节点利用率与排队时长。
4. checkpoint 开销占比。
5. 单位成本目标改善率。

**告警规则：**
1. 成本突增告警。
2. 轮转队列积压告警。
3. 快照失败率告警。
4. 单节点异常失败率告警。

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
3. 事件包络包含 `tenantId` 字段，跨租户消费默认拒绝。

**多租户调度策略：**
1. 基线配额：每租户保底并发槽位。
2. 弹性借用：空闲配额可借用，超出时按优先级回收。
3. 预算隔离：`budgetScope` 默认包含租户前缀，防止串账。

**审计要求：**
1. 所有 stop/resume 操作记录操作者、来源、原因。
2. 关键配置变更与预算变更进入审计日志，不可静默覆盖。

---

## 附录

### A. 事件 JSON 示例

**TaskDispatched：**
```json
{
  "eventId": "evt-8b93f9d2",
  "eventType": "TaskDispatched",
  "schemaVersion": "v1",
  "occurredAt": 1760000000000,
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

**SliceFinished：**
```json
{
  "eventId": "evt-91b1aa10",
  "eventType": "SliceFinished",
  "schemaVersion": "v1",
  "occurredAt": 1760000004200,
  "producer": "solver-worker",
  "traceId": "tr-6d1f",
  "spanId": "sp-23",
  "tenantId": "tenant-a",
  "payload": {
    "taskId": "task-9001",
    "sliceId": "slice-0003",
    "nodeId": "node-a",
    "elapsedMs": 4012,
    "completed": false,
    "checkpointOutRef": "checkpoints/task-9001/0003",
    "interimResultRef": "results/task-9001/interim/0003"
  }
}
```

### B. 数据库 DDL（PostgreSQL）

```sql
create table if not exists task_state (
    task_id varchar(128) primary key,
    request_id varchar(128) not null unique,
    status varchar(32) not null,
    priority int not null,
    complexity varchar(16) not null,
    time_sensitivity varchar(16) not null,
    deadline_epoch_ms bigint,
    current_node_id varchar(128),
    last_checkpoint_ref text,
    created_at timestamp not null,
    updated_at timestamp not null
);

create index if not exists idx_task_queue on task_state(status, priority desc, created_at asc);

create table if not exists slice_state (
    slice_id varchar(128) primary key,
    task_id varchar(128) not null,
    dispatch_id varchar(128) not null,
    status varchar(32) not null,
    node_id varchar(128) not null,
    quantum_ms bigint not null,
    elapsed_ms bigint,
    checkpoint_in_ref text,
    checkpoint_out_ref text,
    started_at timestamp,
    ended_at timestamp,
    unique(task_id, dispatch_id, slice_id)
);

create index if not exists idx_slice_latest on slice_state(task_id, started_at desc);

create table if not exists cost_ledger (
    record_id varchar(128) primary key,
    task_id varchar(128) not null,
    budget_scope varchar(128) not null,
    node_id varchar(128) not null,
    runtime_ms bigint not null,
    billed_cost numeric(18, 6) not null,
    license_cost numeric(18, 6) not null,
    total_cost numeric(18, 6) not null,
    recorded_at timestamp not null
);

create index if not exists idx_cost_task on cost_ledger(task_id, recorded_at asc);
create index if not exists idx_cost_budget on cost_ledger(budget_scope, recorded_at asc);
```

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

**阶段 B（生产可用版）：**
1. 关键实体具备幂等保障。
2. 节点故障时，未完成切片可被回收并重调度。
3. 成本账本与预算控制可阻止超预算扩张。
4. 故障注入测试可稳定通过。

**阶段 C（持续优化版）：**
1. `performanceScore` 可根据历史回放自动更新。
2. 同类任务可自动给出候选"最优节点族"。
3. 成本与时延双目标优化有可量化收益。

### F. 压测场景

1. **小任务洪峰**：1000 个简单任务在 1 分钟内提交，验证吞吐与排队稳定性。
2. **混合负载**：简单任务与复杂任务按 7:3 混合，验证轮转公平性。
3. **故障恢复**：运行中随机下线 20% 节点，验证切片回收与恢复时延。
4. **成本守护**：注入预算上限，验证超预算后降级策略是否生效。

### G. 测试矩阵

**单元测试：**
- `SchedulerEngine`：评分函数与节点选择边界
- `RemoteSolverService`：状态流转、超时、停止策略
- `ModelTranslator`：中间模型转换合法性与兼容性

**契约测试：**
- 全 Port 的 Contract Test 持续保留
- 新增 Adapter 时必须复用同一套契约测试
- 事件适配器需覆盖"重复投递、延迟重试、消费者组负载均衡"

**端到端回归：**
- `submit -> dispatch -> slice -> checkpoint -> resume -> complete` 主链路
- 预算超限触发降级与失败路径
- 节点掉线恢复路径（带 checkpoint 与不带 checkpoint）
- 灰度双写路径（主写成功/镜像失败 fail-open）
