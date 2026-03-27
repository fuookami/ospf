# Remote Solver

English document: [README.md](README.md)

## 概述

`remote-solver` 是一个事件驱动的异构求解调度框架，目标是在满足时效约束的前提下最小化总体成本。
核心能力包括成本感知调度、复杂任务时间片轮转、checkpoint 持久化与 warm start 续算。

## 模块结构

项目拆分为三个模块：

```
ospf-remote-solver/
├── ospf-remote-solver-protocol/     # 共享协议模型与端口
├── ospf-remote-solver-dispatcher/   # 调度器模块（API + 调度 + 事件）
│   ├── HTTP API 端点               # 任务提交、控制、监控
│   ├── 调度引擎                     # 成本感知任务分发
│   ├── 事件发布                     # Kafka/in-memory 适配器
│   ├── 数据库适配器                 # Ktorm/PostgreSQL 持久化
│   └── 所有测试                     # 契约与集成测试
└── ospf-remote-solver-calculator/   # 求解执行模块
    ├── OspfExternalProcessBridge    # 外部进程调用
    ├── OspfSolverExecutionPort      # OSPF 求解适配器
    └── InMemorySolverExecutionPort  # 内存测试适配器
```

**依赖关系：**
- `dispatcher` → `protocol` + `calculator`
- `calculator` → `protocol`
- Kotlin 远程客户端由 `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote` 提供
- Rust 远程客户端已在 `ospf-rust-framework/src/solver/remote` 实现

**使用方式：**
- 仅需客户端的应用：使用 `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote`
- 完整调度器部署：依赖 `ospf-remote-solver-dispatcher`
- 自定义求解执行：依赖 `ospf-remote-solver-calculator`

## 基础设施适配器

框架采用端口/适配器架构，每个基础设施组件均可通过配置切换实现。

### 快速参考

| 扩展点 (Port) | 可用适配器 | 默认值 | 生产推荐 |
|---------------|-----------|--------|----------|
| `EventPort` | `inmemory`, `kafka` | `inmemory` | `kafka` |
| `TaskStatePort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `NodeStatePort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `BudgetPort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `CostLedgerPort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `DistributedLockPort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `ObjectStoragePort` | `inmemory`, `localfs`, `s3` | `inmemory` | `s3` |
| `CheckpointPort` | 随 `ObjectStoragePort` 联动 | `inmemory` | `s3` |
| `SchedulerConfigAuditPort` | `inmemory`, `localfs`, `ktorm` | `inmemory` | `ktorm` |
| `MetricsPort` | `inmemory`, `prometheus` | `inmemory` | `prometheus` |
| `SolverExecutionPort` | `inmemory`, `ospf` | `inmemory` | `ospf` |

### 配置键说明

每个扩展点通过 `<端口名>.adapter=<适配器标识>` 配置：

```properties
event.adapter=inmemory|kafka
task-state.adapter=inmemory|ktorm
node-state.adapter=inmemory|ktorm
budget.adapter=inmemory|ktorm
cost-ledger.adapter=inmemory|ktorm
distributed-lock.adapter=inmemory|ktorm
storage.adapter=inmemory|localfs|s3
scheduler.audit.adapter=inmemory|localfs|ktorm
metrics.adapter=inmemory|prometheus
solver-execution.adapter=inmemory|ospf
```

### 按类别划分的可用适配器

#### 事件发布

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `EventPort` | `inmemory` | `InMemoryEventPort` | 进程内事件投递（默认，无外部依赖） |
| | `kafka` | `KafkaEventPort` | Kafka 事件发布（生产环境） |

**配置示例：**
```properties
event.adapter=inmemory|kafka
event.kafka.bootstrap-servers=host:9092
```

#### 任务状态持久化

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `TaskStatePort` | `inmemory` | `InMemoryTaskStatePort` | ConcurrentHashMap（默认，进程重启丢失） |
| | `ktorm` | `KtormTaskStatePort` | PostgreSQL 持久化（生产环境） |

**配置示例：**
```properties
task-state.adapter=inmemory|ktorm
task-state.ktorm.url=jdbc:postgresql://host:5432/db
task-state.ktorm.username=user
task-state.ktorm.password=pass
```

#### 节点状态持久化

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `NodeStatePort` | `inmemory` | `InMemoryNodeStatePort` | ConcurrentHashMap（默认） |
| | `ktorm` | `KtormNodeStatePort` | PostgreSQL 持久化（生产环境） |

**配置示例：**
```properties
node-state.adapter=inmemory|ktorm
node-state.ktorm.url=jdbc:postgresql://host:5432/db
```

#### 预算管理

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `BudgetPort` | `inmemory` | `InMemoryBudgetPort` | ConcurrentHashMap（默认） |
| | `ktorm` | `KtormBudgetPort` | PostgreSQL 持久化（生产环境） |

**配置示例：**
```properties
budget.adapter=inmemory|ktorm
budget.ktorm.url=jdbc:postgresql://host:5432/db
```

#### 成本账本

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `CostLedgerPort` | `inmemory` | `InMemoryCostLedgerPort` | 内存列表（默认） |
| | `ktorm` | `KtormCostLedgerPort` | PostgreSQL 持久化（生产环境） |

**配置示例：**
```properties
cost-ledger.adapter=inmemory|ktorm
cost-ledger.ktorm.url=jdbc:postgresql://host:5432/db
```

#### 分布式锁

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `DistributedLockPort` | `inmemory` | `InMemoryDistributedLockPort` | 本地锁（默认，无跨节点协调） |
| | `ktorm` | `KtormDistributedLockPort` | 数据库分布式锁（生产环境） |

**配置示例：**
```properties
distributed-lock.adapter=inmemory|ktorm
distributed-lock.ktorm.url=jdbc:postgresql://host:5432/db
```

#### 对象存储

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `ObjectStoragePort` | `inmemory` | `InMemoryObjectStoragePort` | ConcurrentHashMap（默认，进程重启丢失） |
| | `localfs` | `LocalFsObjectStoragePort` | 本地文件系统存储 |
| | `s3` | `S3ObjectStoragePort` | S3/MinIO 对象存储（生产环境） |

**配置示例：**
```properties
storage.adapter=inmemory|localfs|s3
storage.localfs.root=./data/storage
storage.s3.bucket=my-bucket
storage.s3.endpoint=http://minio:9000
storage.s3.access-key-id=minioadmin
storage.s3.secret-access-key=minioadmin
```

#### 检查点存储

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `CheckpointPort` | `inmemory` | `InMemoryCheckpointPort` | 内存（默认，与 ObjectStoragePort 联动） |
| | `localfs` | `LocalFsCheckpointPort` | 本地文件系统（与 ObjectStoragePort 联动） |
| | `s3` | `S3CheckpointPort` | S3/MinIO 存储（生产环境，与 ObjectStoragePort 联动） |

**注意：** CheckpointPort 使用与 ObjectStoragePort 相同的 `storage.adapter` 配置。

#### 调度器配置审计

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `SchedulerConfigAuditPort` | `inmemory` | `InMemorySchedulerConfigAuditPort` | 内存列表（默认） |
| | `localfs` | `LocalFsSchedulerConfigAuditPort` | 本地文件系统 JSON 文件 |
| | `ktorm` | `KtormSchedulerConfigAuditPort` | PostgreSQL 持久化（生产环境） |

**配置示例：**
```properties
scheduler.audit.adapter=inmemory|localfs|ktorm
scheduler.audit.localfs.path=./data/audit
scheduler.audit.ktorm.url=jdbc:postgresql://host:5432/db
```

#### 指标导出

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `MetricsPort` | `inmemory` | `InMemoryMetricsPort` | 内存计数器（默认，测试用） |
| | `prometheus` | `PrometheusMetricsPort` | Prometheus 格式导出（生产环境） |

**配置示例：**
```properties
metrics.adapter=inmemory|prometheus
metrics.canonical.enabled=true
```

当 `metrics.adapter=prometheus` 时，指标通过 `GET /metrics` 端点暴露。

#### 求解执行

| 端口 | 适配器标识 | 实现类 | 说明 |
|------|------------|--------|------|
| `SolverExecutionPort` | `inmemory` | `InMemorySolverExecutionPort` | 模拟求解，不调用真实求解器（默认，仅测试用） |
| | `ospf` | `OspfSolverExecutionPort` | 通过桥接调用 OSPF 求解器（生产环境） |

**配置示例：**
```properties
solver-execution.adapter=inmemory|ospf
solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
solver-execution.ospf.bridge.arg.command=/path/to/solver-runner
```

**重要：** `InMemorySolverExecutionPort` 不调用真实求解器，仅模拟进度从 0.0 到 1.0。生产环境请务必使用 `ospf` 适配器并配置正确的桥接命令。

### 生产环境配置示例

```properties
# 事件 - Kafka 跨节点协调
event.adapter=kafka
event.kafka.bootstrap-servers=kafka:9092

# 数据库 - PostgreSQL 持久化
task-state.adapter=ktorm
task-state.ktorm.url=jdbc:postgresql://postgres:5432/remote_solver
task-state.ktorm.username=remote_solver
task-state.ktorm.password=secret

node-state.adapter=ktorm
node-state.ktorm.url=jdbc:postgresql://postgres:5432/remote_solver

budget.adapter=ktorm
budget.ktorm.url=jdbc:postgresql://postgres:5432/remote_solver

cost-ledger.adapter=ktorm
cost-ledger.ktorm.url=jdbc:postgresql://postgres:5432/remote_solver

distributed-lock.adapter=ktorm
distributed-lock.ktorm.url=jdbc:postgresql://postgres:5432/remote_solver

# 对象存储 - S3/MinIO
storage.adapter=s3
storage.s3.bucket=remote-solver
storage.s3.endpoint=http://minio:9000
storage.s3.access-key-id=minioadmin
storage.s3.secret-access-key=minioadmin

# 调度器审计 - 数据库
scheduler.audit.adapter=ktorm
scheduler.audit.ktorm.url=jdbc:postgresql://postgres:5432/remote_solver

# 指标 - Prometheus
metrics.adapter=prometheus

# 求解执行 - OSPF 桥接
solver-execution.adapter=ospf
solver-execution.ospf.bridge.arg.command=/opt/solver/bin/run-solver.sh
```

### 开发/测试配置示例

```properties
# 全部使用内存实现（以下均为默认值）
event.adapter=inmemory
task-state.adapter=inmemory
node-state.adapter=inmemory
budget.adapter=inmemory
cost-ledger.adapter=inmemory
distributed-lock.adapter=inmemory
storage.adapter=inmemory
scheduler.audit.adapter=inmemory
metrics.adapter=inmemory
solver-execution.adapter=inmemory
```

### InMemory 适配器限制说明

InMemory 适配器设计用于本地开发和测试，无需外部依赖。请注意以下限制：

| 适配器 | 限制 |
|--------|------|
| `InMemoryEventPort` | 仅进程内事件投递，无跨节点广播 |
| `InMemoryTaskStatePort` | 进程重启数据丢失 |
| `InMemoryNodeStatePort` | 进程重启数据丢失 |
| `InMemoryBudgetPort` | 进程重启数据丢失 |
| `InMemoryCostLedgerPort` | 进程重启数据丢失 |
| `InMemoryDistributedLockPort` | 仅本地锁，无跨节点协调 |
| `InMemoryObjectStoragePort` | 进程重启数据丢失 |
| `InMemoryCheckpointPort` | 进程重启数据丢失 |
| `InMemorySchedulerConfigAuditPort` | 进程重启数据丢失 |
| `InMemoryMetricsPort` | 无外部指标采集 |
| `InMemorySolverExecutionPort` | **不调用真实求解器**，仅模拟进度 0.0→1.0 |

**重要提示：** 生产环境部署，特别是 `SolverExecutionPort`，请务必使用生产适配器（`ospf`）并配置正确的求解器集成。

### 固定实现的基础设施组件

以下组件无适配器选择，为固定实现：

| 端口 | 实现类 | 说明 |
|------|--------|------|
| `ClockPort` | `SystemClockPort` | 系统时钟 |
| `IdGeneratorPort` | `UUIDIdGeneratorPort` | UUID 生成 |
| `TracingPort` | `InMemoryTracingPort` | 内存链路追踪（生产环境可考虑集成 OpenTelemetry） |

## 当前范围

1. 已完成领域模型与 Port 抽象定义。
2. 已提供 InMemory 适配器，可本地端到端联调。
3. 调度器支持成本感知的节点选择。
4. 复杂任务支持切片执行、快照保存与恢复。
5. 已集成预算控制与成本账本。
6. 事件发布支持可选的镜像双写能力（灰度用途）。
7. 支持按任务配置 checkpoint 保留上限。

## 运行配置项

1. `event.adapter=inmemory|kafka`
2. `event.mirror.enabled=true|false`（默认 `false`）
3. `event.mirror.fail-open=true|false`（默认 `true`）
4. `event.kafka.bootstrap-servers=<host:port[,host:port...]>`（当 event 适配器为 `kafka` 时必填）
5. `event.kafka.client-id=<client-id>`（默认 `remote-solver-event-port`）
6. `event.kafka.consumer-poll-interval-ms=<正整数>`（默认 `200`）
7. `event.kafka.query-poll-timeout-ms=<正整数>`（默认 `1500`）
8. `event.kafka.query-max-poll-rounds=<正整数>`（默认 `8`）
9. `event.kafka.query-topics=<csv-topics>`（默认 `SolvingRequest,SolvingControl,TaskDispatch,SolverHeartBeat,SliceLifecycle,TaskResult,CostAndCapabilityUpdate,MonitorAlert`）
10. `distributed-lock.adapter=inmemory|ktorm`
11. `distributed-lock.ktorm.url=<jdbc-url>`（当锁适配器为 `ktorm` 时必填）
12. `distributed-lock.ktorm.username=<username>`（可选）
13. `distributed-lock.ktorm.password=<password>`（可选）
14. `distributed-lock.ktorm.table=<table-name>`（默认 `remote_solver_lock`）
15. `node-state.adapter=inmemory|ktorm`
16. `node-state.ktorm.url=<jdbc-url>`（当 node-state 适配器为 `ktorm` 时必填）
17. `node-state.ktorm.username=<username>`（可选）
18. `node-state.ktorm.password=<password>`（可选）
19. `node-state.ktorm.table=<table-name>`（默认 `remote_solver_node_state`）
20. `budget.adapter=inmemory|ktorm`
20. `budget.ktorm.url=<jdbc-url>`（当 budget 适配器为 `ktorm` 时必填）
21. `budget.ktorm.username=<username>`（可选）
22. `budget.ktorm.password=<password>`（可选）
23. `budget.ktorm.table=<table-name>`（默认 `remote_solver_budget`）
24. `task-state.adapter=inmemory|ktorm`
25. `task-state.ktorm.url=<jdbc-url>`（当 task-state 适配器为 `ktorm` 时必填）
26. `task-state.ktorm.username=<username>`（可选）
27. `task-state.ktorm.password=<password>`（可选）
28. `task-state.ktorm.task-table=<table-name>`（默认 `remote_solver_task_state`）
29. `task-state.ktorm.slice-table=<table-name>`（默认 `remote_solver_slice_state`）
30. `cost-ledger.adapter=inmemory|ktorm`
31. `cost-ledger.ktorm.url=<jdbc-url>`（当 cost-ledger 适配器为 `ktorm` 时必填）
32. `cost-ledger.ktorm.username=<username>`（可选）
33. `cost-ledger.ktorm.password=<password>`（可选）
34. `cost-ledger.ktorm.table=<table-name>`（默认 `remote_solver_cost_ledger`）
35. `solver-execution.adapter=inmemory|ospf`
36. `solver-execution.ospf.simulated-total-runtime-ms=<正整数>`（默认 `12000`）
37. `solver-execution.ospf.bridge-class=<fqcn>`（可选，类需实现 `OspfExecutionBridge`）
38. `solver-execution.ospf.bridge.arg.<key>=<value>`（可选，桥接类自定义参数）
39. 若未设置 `bridge-class` 但配置了 `solver-execution.ospf.bridge.arg.command`，
    调度器会自动使用 `OspfExternalProcessBridge`。
40. 内置桥接示例：
   `solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge`
   `solver-execution.ospf.bridge.arg.command=<your-runner-command>`
41. `storage.adapter=inmemory|localfs|s3`
42. `storage.localfs.root=<path>`
43. `storage.s3.bucket=<bucket-name>`（当 storage 适配器为 `s3` 时必填）
44. `storage.s3.region=<region>`（默认 `us-east-1`）
45. `storage.s3.endpoint=<endpoint-url>`（可选，MinIO 推荐填写）
46. `storage.s3.access-key-id=<access-key>`（当 storage 适配器为 `s3` 时必填）
47. `storage.s3.secret-access-key=<secret-key>`（当 storage 适配器为 `s3` 时必填）
48. `storage.s3.path-style-access=true|false`（默认 `true`）
49. `storage.s3.object-prefix=<prefix>`（默认 `objects`）
50. `storage.s3.checkpoint-prefix=<prefix>`（默认 `checkpoints`）
51. `checkpoint.retention.max-per-task=<非负整数>`（`0` 表示不限制）
52. `scheduler.performance-learning.enabled=true|false`（默认 `true`）
53. `scheduler.performance-learning.rate=<[0,1] 区间 double>`（默认 `0.2`）
54. `scheduler.performance-learning.score-min=<非负 double>`（默认 `0.1`）
55. `scheduler.performance-learning.score-max=<非负 double>`（默认 `10.0`，且必须 >= `score-min`）
56. `api.http.port=<1..65535>`（默认 `18080`，供 `RemoteSolverApiMain` 使用）
57. `api.tenant.auth.enabled=true|false`（默认 `false`；开启后 API 必须携带 `X-Tenant-Id` 请求头）
58. `scheduler.config.version=<string>`（默认 `v0`）
59. `scheduler.hot-reload.enabled=true|false`（默认 `false`）
60. `scheduler.audit.adapter=inmemory|localfs|ktorm`（默认 `inmemory`）
61. `scheduler.audit.localfs.path=<path>`（默认 `target/remote-solver-audit`，仅 `localfs` 适配器生效）
62. `scheduler.audit.ktorm.url=<jdbc-url>`（当 scheduler audit 适配器为 `ktorm` 时必填）
63. `scheduler.audit.ktorm.username=<username>`（可选）
64. `scheduler.audit.ktorm.password=<password>`（可选）
65. `scheduler.audit.ktorm.audit-table=<table-name>`（默认 `remote_solver_scheduler_audit`）
66. `scheduler.audit.ktorm.snapshot-table=<table-name>`（默认 `remote_solver_scheduler_snapshot`）
67. `metrics.canonical.enabled=true|false`（默认 `true`）
68. `metrics.adapter=inmemory|prometheus`（默认 `inmemory`；当适配器支持 scrape 时开启 `/metrics`）
69. `api.monitor.auth.enabled=true|false`（默认 `true`；保护 `/api/v1/monitor/*` 与 `/monitor`）
70. `api.monitor.auth.roles=<csv-roles>`（默认 `admin,monitor_read`）
71. `api.auth.login-url=<path-or-url>`（默认 `/login`；未认证访问 `/monitor` 时重定向到该地址）
72. `monitor.alert.enabled=true|false`（默认 `false`）
73. `monitor.alert.check-interval-ms=<非负 long>`（默认 `5000`）
74. `monitor.alert.overview-limit=<正整数>`（默认 `300`）
75. `monitor.alert.cooldown-ms=<非负 long>`（默认 `60000`）
76. `monitor.alert.escalate-after-consecutive=<正整数>`（默认 `3`）
77. `monitor.alert.stale-nodes-threshold=<非负整数>`（默认 `1`，`0` 表示关闭该规则）
78. `monitor.alert.offline-nodes-threshold=<非负整数>`（默认 `1`，`0` 表示关闭该规则）
79. `monitor.alert.failed-tasks-threshold=<非负整数>`（默认 `5`，`0` 表示关闭该规则）
80. `monitor.alert.failed-ratio-threshold=<[0,1] 区间 double>`（默认 `0.3`）
81. `monitor.alert.queue-depth-threshold=<非负整数>`（默认 `100`，`0` 表示关闭该规则）
82. `monitor.alert.route.event.enabled=true|false`（默认 `true`，发布到主题 `MonitorAlert`）
83. `monitor.alert.route.webhook.enabled=true|false`（默认 `false`）
84. `monitor.alert.route.webhook.url=<http-url>`（启用 webhook 路由时必填）
85. `monitor.alert.route.webhook.timeout-ms=<正整数 long>`（默认 `3000`）

## 部署说明

### 拓扑

1. 调度器节点（运行 `RemoteSolverService` 与所选适配器）。
2. 求解节点（运行 OSPF worker 进程或桥接命令目标）。
3. 共享基础设施（按适配器选择）：Kafka、JDBC 数据库、对象存储（LocalFS 或 S3/MinIO）。
4. 监控栈（可选）：Prometheus、Grafana、AlertManager。

---

## 调度服务部署

### 前置条件

1. Java 11+ 运行时
2. 数据库（推荐 PostgreSQL）用于生产适配器
3. Kafka 集群用于事件驱动模式
4. 对象存储（S3/MinIO 或 LocalFS）用于模型/快照持久化

### 步骤 1：数据库迁移

首次使用 JDBC 部署前执行迁移脚本：

```bash
# PostgreSQL 示例
./deploy/scripts/apply-migrations.sh "postgresql://127.0.0.1:5432/remote_solver"
```

Windows:
```powershell
.\deploy\scripts\apply-migrations.ps1 -DbUrl "postgresql://127.0.0.1:5432/remote_solver"
```

迁移脚本：
- `deploy/sql/V1__remote_solver_core.sql` - 核心表（lock, node_state, budget, task_state, slice_state, cost_ledger）
- `deploy/sql/V2__remote_solver_infra.sql` - 基础设施表
- `deploy/sql/V3__remote_solver_scheduler_audit.sql` - 调度审计表
- `deploy/sql/V4__remote_solver_multi_tenant.sql` - 多租户支持（tenant_id 列）

### 步骤 2：配置文件

使用生产模板 `deploy/config/scheduler-ktorm.properties`：

```properties
# 事件适配器
event.adapter=kafka
event.kafka.bootstrap-servers=127.0.0.1:9092

# 数据库适配器（Ktorm/PostgreSQL）
distributed-lock.adapter=ktorm
distributed-lock.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
node-state.adapter=ktorm
node-state.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
budget.adapter=ktorm
budget.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
task-state.adapter=ktorm
task-state.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
cost-ledger.adapter=ktorm
cost-ledger.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver

# 调度审计持久化
scheduler.audit.adapter=ktorm
scheduler.audit.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver

# 求解器执行
solver-execution.adapter=ospf
solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
solver-execution.ospf.bridge.arg.command=<求解器启动命令>

# 对象存储
storage.adapter=s3
storage.s3.bucket=remote-solver
storage.s3.endpoint=http://127.0.0.1:9000
storage.s3.access-key-id=minioadmin
storage.s3.secret-access-key=minioadmin

# 指标
metrics.adapter=prometheus
metrics.canonical.enabled=true

# API
api.http.port=18080
api.tenant.auth.enabled=true
api.monitor.auth.enabled=true
api.monitor.auth.roles=admin,monitor_read

# 监控告警
monitor.alert.enabled=true
monitor.alert.check-interval-ms=5000
monitor.alert.route.webhook.enabled=true
monitor.alert.route.webhook.url=http://alertmanager:9093/api/v1/alerts
```

### 步骤 3：启动调度服务

仅启动调度器（无 HTTP API）：

```bash
./deploy/scripts/start-scheduler.sh deploy/config/scheduler-ktorm.properties
```

Windows:
```powershell
.\deploy\scripts\start-scheduler.ps1 -ConfigPath deploy/config/scheduler-ktorm.properties
```

启动 API 服务（内含调度循环 + HTTP 端点）：

```bash
./deploy/scripts/start-api.sh deploy/config/scheduler-ktorm.properties 0.0.0.0 18080
```

Windows:
```powershell
.\deploy\scripts\start-api.ps1 -ConfigPath deploy/config/scheduler-ktorm.properties -Host 0.0.0.0 -Port 18080
```

### 步骤 4：验证调度器

运行冒烟测试：

```bash
./deploy/scripts/smoke-up.sh deploy/config/scheduler-ktorm.properties
```

手动验证：

```bash
# 提交测试任务
TASK_ID=$(curl -s -X POST "http://127.0.0.1:18080/api/v1/tasks" \
  -H "Content-Type: application/json" \
  -d '{"payloadRef":"models/demo","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}' \
  | sed -n 's/.*"taskId":"\([^"]*\)".*/\1/p')

# 查看任务状态
curl -s "http://127.0.0.1:18080/api/v1/tasks/$TASK_ID"
```

---

## 求解节点部署

### 前置条件

1. Java 11+ 运行时
2. 求解引擎（Gurobi、SCIP 或其他 OSPF 兼容求解器）
3. 与调度器和对象存储的网络连通

### 步骤 1：Worker 配置

求解节点由调度器通过桥接命令调用。内置 worker 入口：

`fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverWorkerMain`

Worker 参数：
- `--model <path>` - 模型引用路径
- `--task <taskId>` - 任务标识
- `--slice <sliceId>` - 切片标识
- `--node <nodeId>` - 节点标识
- `--quantum_ms <ms>` - 本次切片时间配额
- `--checkpoint-in <path>` - 可选快照用于 warm-start

额外参数：
- `--state-dir <dir>` - 本地状态目录（默认：`target/remote-solver-worker-state`）
- `--total-runtime-ms <ms>` - 模拟运行时间（默认：`12000`）

### 步骤 2：注册节点

启动 worker 进程后，向调度器注册节点能力：

```kotlin
val node = SolverNode(
    nodeId = "solver-node-1",
    solverType = SolverType.LINEAR_PROGRAMMING,
    capabilities = setOf(
        NodeCapability.INTERRUPTIBLE,
        NodeCapability.CHECKPOINT,
        NodeCapability.WARM_START
    ),
    parallelUnits = 4,
    costPerTask = 0.5
)
runtime.service.registerNode(node)
```

### 步骤 3：验证 Worker

手工启动 worker（协议自检）：

```bash
./deploy/scripts/start-worker.sh --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

Windows:
```powershell
.\deploy\scripts\start-worker.ps1 --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

期望 stdout 输出：
```text
completed=true
feasible=true
objective=1234.56
gap=0.01
elapsedMs=8000
message=solved
checkpointPath=/data/checkpoints/t1/s1.cp
resultPath=/data/results/t1/s1.json
```

---

## 监控服务部署

### 前置条件

1. Prometheus 服务端（指标抓取）
2. Grafana 服务端（可视化）
3. AlertManager（告警路由）

### 步骤 1：Prometheus 配置

添加抓取配置：

```yaml
scrape_configs:
  - job_name: 'remote-solver'
    scrape_interval: 15s
    static_configs:
      - targets: ['scheduler-host:18080']
    metrics_path: /metrics
```

告警规则文件：
`deploy/observability/prometheus/alerts-remote-solver.yml`

### 步骤 2：AlertManager 配置

AlertManager 路由配置：
`deploy/observability/alertmanager/alertmanager.yml`

主要接收器：
- `critical-alerts` - 高严重度告警（节点离线、预算超限）
- `warning-alerts` - 中严重度告警（节点过期、队列深度）
- `node-alerts` - 节点相关告警
- `task-alerts` - 任务失败告警

抑制规则：
- 高严重度告警抑制同资源的低严重度告警
- 节点离线抑制节点恢复告警

### 步骤 3：Grafana 看板

导入看板模板：
`deploy/observability/grafana/remote-solver-overview.json`

导入脚本：
```bash
./deploy/scripts/import-observability.sh http://grafana:3000 <api-key> ./target/observability/prometheus-rules
```

Windows:
```powershell
.\deploy\scripts\import-observability.ps1 -GrafanaUrl http://grafana:3000 -GrafanaApiKey <api-key> -PromRulesTargetDir .\target\observability\prometheus-rules
```

### 步骤 4：监控访问

看板地址：`http://scheduler-host:18080/monitor`

访问所需请求头：
- `X-User-Id: <用户标识>`
- `X-User-Roles: admin` 或 `X-User-Roles: monitor_read`

监控 API：
```bash
curl -s "http://127.0.0.1:18080/api/v1/monitor/overview?limit=300" \
  -H "X-User-Id: ops-user" \
  -H "X-User-Roles: monitor_read"
```

### 步骤 5：告警冒烟测试

验证告警配置：
```bash
./deploy/scripts/validate-alert-linkage.sh
```

Windows:
```powershell
.\deploy\scripts\validate-alert-linkage.ps1
```

验证监控告警联动：
```bash
./deploy/scripts/validate-monitor-alert-linkage.sh
```

Windows:
```powershell
.\deploy\scripts\validate-monitor-alert-linkage.ps1 -ApiPort 18091 -WebhookPort 19091
```

---

## 快速参考：部署脚本一览

| 脚本 | 用途 |
|------|------|
| `start-scheduler.sh/ps1` | 启动调度器服务 |
| `start-api.sh/ps1` | 启动 API 服务（含调度器） |
| `start-worker.sh/ps1` | 启动求解器 Worker |
| `apply-migrations.sh/ps1` | 执行数据库迁移 |
| `smoke-up.sh/ps1` | 端到端冒烟测试 |
| `load-test.sh/ps1` | 混合任务压测 |
| `monitor-smoke.sh/ps1` | 监控 API/看板冒烟测试 |
| `monitor-alert-smoke.sh/ps1` | 监控告警联动冒烟测试 |
| `validate-alert-linkage.sh/ps1` | 验证 AlertManager 配置 |
| `validate-monitor-alert-linkage.sh/ps1` | 验证监控告警集成 |
| `import-observability.sh/ps1` | 导入 Prometheus/Grafana 资产 |
| `scheduler-audit-dump.sh/ps1` | 导出调度审计日志 |

---

### 调度器配置

默认配置文件位置：
`deploy/config/scheduler.properties`
Ktorm 生产模板：
`deploy/config/scheduler-ktorm.properties`

数据库基线迁移脚本：
1. `deploy/sql/V1__remote_solver_core.sql`
2. `deploy/sql/V2__remote_solver_infra.sql`
3. `deploy/sql/V3__remote_solver_scheduler_audit.sql`
4. `deploy/sql/V4__remote_solver_multi_tenant.sql`

首次使用 JDBC 部署前建议先执行（PostgreSQL 示例）：
```bash
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V1__remote_solver_core.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V2__remote_solver_infra.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V3__remote_solver_scheduler_audit.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V4__remote_solver_multi_tenant.sql
```

也可直接使用迁移脚本：
```bash
./deploy/scripts/apply-migrations.sh "postgresql://127.0.0.1:5432/remote_solver"
```

Windows：
```powershell
.\deploy\scripts\apply-migrations.ps1 -DbUrl "postgresql://127.0.0.1:5432/remote_solver"
```

最小单机联调配置：

```properties
event.adapter=inmemory
distributed-lock.adapter=inmemory
node-state.adapter=inmemory
budget.adapter=inmemory
task-state.adapter=inmemory
cost-ledger.adapter=inmemory
solver-execution.adapter=ospf
solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
solver-execution.ospf.bridge.arg.command=<solver-runner-command>
storage.adapter=localfs
storage.localfs.root=./data/remote-solver
```

启动调度器服务：

```bash
./deploy/scripts/start-scheduler.sh deploy/config/scheduler.properties
```

Windows:

```powershell
.\deploy\scripts\start-scheduler.ps1 -ConfigPath deploy/config/scheduler.properties
```

启动 API 服务（内含调度循环）：

```bash
./deploy/scripts/start-api.sh deploy/config/scheduler.properties 0.0.0.0 18080
```

Windows：

```powershell
.\deploy\scripts\start-api.ps1 -ConfigPath deploy/config/scheduler.properties -Host 0.0.0.0 -Port 18080
```

HTTP 适配实现：Ktor（`CIO` 引擎）+ `kotlinx.serialization`。

启动参数与配置解析已统一收敛到：
`fuookami.ospf.framework.remote_solver.bootstrap.BootstrapCliSupport`。

### API 端点

1. `POST /api/v1/tasks`
2. `GET /api/v1/tasks/{taskId}`
3. `POST /api/v1/tasks/{taskId}/stop`
4. `POST /api/v1/tasks/{taskId}/resume`
5. `POST /api/v1/scheduler/config/hot-reload`
6. `POST /api/v1/scheduler/config/rollback`
7. `GET /api/v1/scheduler/config/audits?limit=<n>`
8. `GET /api/v1/tasks/{taskId}/timeline?limit=<n>`
9. `GET /api/v1/monitor/overview?limit=<n>`
10. `GET /monitor`（内置监控看板页面）
11. `GET /metrics`（当 `metrics.adapter=prometheus` 时可用）

监控访问说明：
1. `/api/v1/monitor/*` 与 `/monitor` 需要 `X-User-Id` 和 `X-User-Roles` 请求头。
2. 默认允许角色为 `admin` 和 `monitor_read`。
3. 未认证访问监控 API 返回 `401`。
4. 未认证访问监控页面返回 `302` 并重定向到 `api.auth.login-url`（默认 `/login`）。

监控告警联动说明：
1. 设置 `monitor.alert.enabled=true` 后，API 调度循环会周期性评估监控快照。
2. 异常首次命中会发送 `TRIGGERED`，持续异常会按规则发送 `ESCALATED`，恢复后发送 `RECOVERED`。
3. 事件路由会把告警发布到主题 `MonitorAlert`。
4. 可选 webhook 路由可把同样的告警载荷转发到外部告警系统。

响应包络格式：

```json
{
  "code": "OK",
  "message": "success",
  "data": {}
}
```

错误响应包络：

```json
{
  "code": "INVALID_ARGUMENT",
  "message": "payloadRef.path must not be blank",
  "data": null
}
```

提交请求体示例：

```json
{
  "tenantId": "tenant-a",
  "payloadRef": "models/demo",
  "complexity": "SIMPLE",
  "timeSensitivity": "NON_REALTIME",
  "priority": 1
}
```

租户说明：
1. 未显式传入 `tenantId` 时默认使用 `default`。
2. 当传入 `tenantId` 时，提交模型引用路径会自动规范为 `tenantId/...` 前缀。
3. `budgetScope` 会标准化为 `tenantId:<scope>`，避免跨租户串账。
4. 事件包络默认携带 `tenantId` 头字段。

停止请求体示例：

```json
{
  "reason": "manual-stop"
}
```

调度参数热更新请求体示例：

```json
{
  "operator": "ops",
  "changeSet": {
    "scheduler.simple-task-quantum-ms": "3200",
    "scheduler.complex-urgency-weight": "0.7"
  }
}
```

调度参数回滚请求体示例：

```json
{
  "operator": "ops",
  "targetVersion": "v-base"
}
```

快速 curl 流程：

```bash
TASK_ID=$(curl -s -X POST "http://127.0.0.1:18080/api/v1/tasks" \
  -H "Content-Type: application/json" \
  -d '{"payloadRef":"models/demo","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}' \
  | sed -n 's/.*"taskId":"\([^"]*\)".*/\1/p')

curl -s "http://127.0.0.1:18080/api/v1/tasks/$TASK_ID"
curl -s -X POST "http://127.0.0.1:18080/api/v1/tasks/$TASK_ID/stop" -H "Content-Type: application/json" -d '{"reason":"manual-stop"}'
curl -s -X POST "http://127.0.0.1:18080/api/v1/tasks/$TASK_ID/resume" -d '{}'
curl -s -X POST "http://127.0.0.1:18080/api/v1/scheduler/config/hot-reload" -H "Content-Type: application/json" -d '{"operator":"ops","changeSet":{"scheduler.simple-task-quantum-ms":"3200"}}'
curl -s "http://127.0.0.1:18080/api/v1/scheduler/config/audits?limit=20"
curl -s "http://127.0.0.1:18080/api/v1/tasks/$TASK_ID/timeline?limit=100"
curl -s "http://127.0.0.1:18080/api/v1/monitor/overview?limit=300" \
  -H "X-User-Id: ops-user" \
  -H "X-User-Roles: monitor_read"
curl -s "http://127.0.0.1:18080/monitor" \
  -H "X-User-Id: admin-user" \
  -H "X-User-Roles: admin"
# 未认证访问看板会重定向:
# curl -I "http://127.0.0.1:18080/monitor"
```

监控冒烟校验（API + 看板）：

```bash
./deploy/scripts/monitor-smoke.sh "http://127.0.0.1:18080" 300 "smoke-user" "monitor_read"
```

Windows:

```powershell
.\deploy\scripts\monitor-smoke.ps1 -BaseUrl "http://127.0.0.1:18080" -Limit 300 -AuthUser "smoke-user" -AuthRoles "monitor_read"
```

监控告警联动冒烟校验（队列深度异常 -> webhook 触发/升级）：

```bash
./deploy/scripts/monitor-alert-smoke.sh "http://127.0.0.1:18091" "http://127.0.0.1:19091/events" 2 60
```

Windows:

```powershell
.\deploy\scripts\monitor-alert-smoke.ps1 -BaseUrl "http://127.0.0.1:18091" -WebhookEventsUrl "http://127.0.0.1:19091/events" -TaskCount 2 -PollTimeoutSeconds 60
```

监控告警联动校验（自动启动 webhook mock + API 并执行冒烟）：

```bash
./deploy/scripts/validate-monitor-alert-linkage.sh
```

Windows:

```powershell
.\deploy\scripts\validate-monitor-alert-linkage.ps1 -ApiPort 18091 -WebhookPort 19091
```

配置加载优先级：
1. 命令行参数 `--config <path>`
2. 环境变量 `REMOTE_SOLVER_CONFIG=<path>`
3. 默认路径 `deploy/config/scheduler.properties`

接近生产的示例配置：

```properties
event.adapter=kafka
event.kafka.bootstrap-servers=127.0.0.1:9092
distributed-lock.adapter=ktorm
distributed-lock.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
node-state.adapter=ktorm
node-state.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
budget.adapter=ktorm
budget.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
task-state.adapter=ktorm
task-state.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
cost-ledger.adapter=ktorm
cost-ledger.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver
solver-execution.adapter=ospf
solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
solver-execution.ospf.bridge.arg.command=<solver-runner-command>
storage.adapter=s3
storage.s3.bucket=remote-solver
storage.s3.endpoint=http://127.0.0.1:9000
storage.s3.access-key-id=minioadmin
storage.s3.secret-access-key=minioadmin
```

### 求解节点配置

求解节点服务由调度器通过 `solver-execution.ospf.bridge.arg.command` 拉起执行。
仓库内置了 worker 入口：
`fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverWorkerMain`

手工启动 worker（用于协议自检）：

```bash
./deploy/scripts/start-worker.sh --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

Windows:

```powershell
.\deploy\scripts\start-worker.ps1 --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

桥接命令需要支持以下参数：

1. `--model <path>`
2. `--task <taskId>`
3. `--slice <sliceId>`
4. `--node <nodeId>`
5. `--quantum-ms <quantum>`
6. 可选 `--checkpoint-in <path>`

本仓库 worker 额外支持参数：
1. `--state-dir <dir>`（默认 `target/remote-solver-worker-state`）
2. `--total-runtime-ms <long>`（默认 `12000`）

标准输出需返回 key-value：

```text
completed=true|false
feasible=true|false
objective=<double>
gap=<double>
elapsedMs=<long>
message=<string>
checkpointPath=<path>
resultPath=<path>
```

### 启动检查清单

1. 启动共享依赖（按需：Kafka/JDBC/S3）。
2. 通过 `RemoteSolverBootstrapFactory.create(...)` 加载配置并启动调度器。
3. 在每个求解节点启动 worker 命令。
4. 通过 `registerNode(...)` 注册节点能力（`solverType`、中断/快照/warm-start、`parallelUnits`）。
5. 提交一个冒烟任务，确认状态流转 `QUEUED -> RUNNING -> COMPLETED`。

可运行内置 smoke 流程：

```bash
mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverSmokeMain \
  -Dexec.args="--config deploy/config/scheduler.properties"
```

一条命令完成 smoke（启动调度器、执行 smoke、自动停止调度器）：

```bash
./deploy/scripts/smoke-up.sh deploy/config/scheduler.properties
```

Windows:

```powershell
.\deploy\scripts\smoke-up.ps1 -ConfigPath deploy/config/scheduler.properties
```

可运行压测流程（简单/复杂混合任务）：

```bash
./deploy/scripts/load-test.sh deploy/config/scheduler.properties 200 0.7 0.2 3000 8 0.5
```

参数说明：
1. `configPath`（默认 `deploy/config/scheduler.properties`）
2. `totalTasks`（默认 `100`）
3. `simpleRatio`（默认 `0.7`）
4. `realtimeRatio`（默认 `0.2`）
5. `maxRounds`（默认 `2000`）
6. `nodes`（默认 `6`）
7. `highTierRatio`（默认 `0.4`）

Windows:

```powershell
.\deploy\scripts\load-test.ps1 -ConfigPath deploy/config/scheduler.properties -TotalTasks 200 -SimpleRatio 0.7 -RealtimeRatio 0.2 -MaxRounds 3000 -Nodes 8 -HighTierRatio 0.5
```

从本地文件导出调度参数热更新审计：

```bash
./deploy/scripts/scheduler-audit-dump.sh deploy/config/scheduler.properties 50
```

Windows:

```powershell
.\deploy\scripts\scheduler-audit-dump.ps1 -ConfigPath deploy/config/scheduler.properties -Limit 50
```

导入可观测性资产（Prometheus 告警规则 + Grafana 看板）：

```bash
./deploy/scripts/import-observability.sh http://127.0.0.1:3000 <grafana-api-key> ./target/observability/prometheus-rules
```

Windows：

```powershell
.\deploy\scripts\import-observability.ps1 -GrafanaUrl http://127.0.0.1:3000 -GrafanaApiKey <grafana-api-key> -PromRulesTargetDir .\target\observability\prometheus-rules
```

Linux 校验辅助脚本：

```bash
./deploy/scripts/validate-observability-import.sh
```

## 求解器兼容性

调度前会校验任务与节点的 `solverType` 兼容性。任务侧可通过以下任一字段声明：

1. `SolvePayload.taskMeta.solverType`（推荐）
2. `SolvePayload.extension["solverType"]`
3. `SolvePayload.taskMeta.metadata["solverType"]`

## 事件包络头（Event Envelope Headers）

`EventPort.publish(...)` 现在会对事件包络头执行强校验与标准化，包含：

1. `schemaVersion`
2. `eventType`
3. `producer`
4. `occurredAtEpochMs`
5. 可选 `traceId`
6. 可选 `spanId`

校验规则：
1. `schemaVersion` 必须在当前支持范围内（当前仅支持 `1`）。
2. 若传入 `occurredAtEpochMs`，必须是正整数。
3. 缺失必填包络头时会自动补齐：
   `eventType=<topic>`、`producer=<adapter-default>`、`occurredAtEpochMs=<publish-time>`。

## 错误码映射

`RemoteSolverException.code` 已与标准失败原因对齐。

常见运行时失败码：
1. `TASK_FAILED_BUDGET_EXCEEDED`
2. `TASK_FAILED_HARD_TIMEOUT`
3. `TASK_FAILED_SLICE_TIMEOUT`
4. `SOLVER_EXECUTION_FAILED`

任务失败时也会在 `latestResult.extension["reasonCode"]` 中持久化失败码，便于下游排障。

## Remote Solver 入口

可通过 bootstrap runtime 获取服务端 API 门面：

```kotlin
val runtime = RemoteSolverBootstrapFactory.create()
val api = runtime.apiFacade
```

客户端侧远程求解封装已迁移到 `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote`。提交任务到 dispatcher 的 Kotlin 应用应依赖并使用该包。Rust 版本也已在 `ospf-rust-framework/src/solver/remote` 实现。

```kotlin
// 客户端应用应依赖 ospf-kotlin-framework，并使用其中的 solver remote client 包。
```

`remote-solver` 工程保留服务端调度、执行、协议、适配器职责，以及其它语言客户端实现所需的协议设计。

如果希望使用服务层的一次性流程，可使用 `submitAndAwait`：

```kotlin
val task = runtime.service.submitAndAwait(
    payload = SolvePayload(modelRef = ObjectRef(path = "models/lp")),
    complexity = TaskComplexity.SIMPLE,
    timeSensitivity = TimeSensitivity.NON_REALTIME,
    maxRounds = 20
)
```

若希望在超过 `maxRounds` 仍未进入终态时直接抛错，可设置 `throwIfNotTerminal = true`。
异常类型为 `RemoteSolverException`，错误码为 `TASK_NOT_TERMINAL_WITHIN_MAX_ROUNDS`。
若设置 `throwIfFailed = true`，任务失败时也会抛 `RemoteSolverException`（例如 `TASK_FAILED_BUDGET_EXCEEDED`）。

任务控制接口：

```kotlin
val stopped = runtime.service.stopTask(taskId = "task-1", reason = "manual-stop")
val resumed = runtime.service.resumeTask(taskId = "task-1")
```

`resumeTask` 支持 `STOPPED -> QUEUED|SUSPENDED`，非法状态转换会抛出
`RemoteSolverException(code = INVALID_TASK_STATE_TRANSITION)`。

如果希望先在代码层实现 API 语义（不绑定具体 HTTP 框架），可使用 `RemoteSolverApiFacade`：

```kotlin
val api = RemoteSolverApiFacade(runtime.service)
val submit = api.submit(
    TaskSubmitRequest(
        payloadRef = ObjectRef(path = "models/lp"),
        complexity = TaskComplexity.SIMPLE,
        timeSensitivity = TimeSensitivity.NON_REALTIME
    )
)
val view = api.get(submit.taskId)
```

通过 API Facade 获取任务时序回放：

```kotlin
val report = api.replayTaskTimeline(taskId = submit.taskId, limitEvents = 200)
println("events=${report.events.size}, gaps=${report.gaps.size}")
```

通过 CLI 获取任务时序回放：

```bash
mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverReplayMain \
  -Dexec.args="--config deploy/config/scheduler.properties --task <taskId> --format text --limit 200"
```

## 运行测试

```bash
mvn test
```

## 可观测性

指标规范：
1. `docs/observability/metrics-spec.md`（英文）
2. `docs/observability/metrics-spec_ch.md`（中文）

Prometheus 告警规则：
1. `deploy/observability/prometheus/alerts-remote-solver.yml`

Grafana 看板模板：
1. `deploy/observability/grafana/remote-solver-overview.json`

AlertManager 路由配置：
1. `deploy/observability/alertmanager/alertmanager.yml`

说明：
1. 当 `metrics.canonical.enabled=true` 时，运行时会将指标自动映射为 `remote_solver_*` 规范名。
2. 生产 telemetry 适配器应按 `metrics-spec.md` 导出统一指标名。
3. 配置 `metrics.adapter=prometheus` 后，可通过 `GET /metrics` 抓取 Prometheus 文本格式指标。

## 数据库 Schema

数据库迁移脚本（PostgreSQL）：

| 脚本 | 表名 | 说明 |
|------|------|------|
| `deploy/sql/V1__remote_solver_core.sql` | `remote_solver_lock`, `remote_solver_node_state`, `remote_solver_budget`, `remote_solver_task_state`, `remote_solver_slice_state`, `remote_solver_cost_ledger` | 核心领域表 |
| `deploy/sql/V2__remote_solver_infra.sql` | 基础设施扩展表 | 基础设施元数据 |
| `deploy/sql/V3__remote_solver_scheduler_audit.sql` | `remote_solver_scheduler_audit`, `remote_solver_scheduler_snapshot` | 调度参数热更新审计轨迹 |
| `deploy/sql/V4__remote_solver_multi_tenant.sql` | `remote_solver_task_state`, `remote_solver_cost_ledger` 增加 `tenant_id` 列 | 多租户隔离 |

执行迁移：

```bash
./deploy/scripts/apply-migrations.sh "postgresql://127.0.0.1:5432/remote_solver"
```

Windows：

```powershell
.\deploy\scripts\apply-migrations.ps1 -DbUrl "postgresql://127.0.0.1:5432/remote_solver"
```

手工执行：

```bash
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V1__remote_solver_core.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V2__remote_solver_infra.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V3__remote_solver_scheduler_audit.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V4__remote_solver_multi_tenant.sql
```

## 验收场景

以下设计场景已映射为可执行测试：

1. 混合负载 `7:3`（简单任务:复杂任务），并验证复杂任务快照持久化：
   `RemoteSolverServiceAcceptanceTest.mixedLoadSevenToThreeShouldCompleteAndPersistComplexSnapshots`
2. 小洪峰场景下队列可排空且无失败任务：
   `RemoteSolverServiceAcceptanceTest.smallBurstShouldDrainQueueWithoutFailures`

仅运行验收测试：

```bash
mvn -Dtest=RemoteSolverServiceAcceptanceTest test
```

## 设计文档

详细设计见 [design.md](design.md)。
