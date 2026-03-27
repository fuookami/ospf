# Remote Solver

中文文档: [README_ch.md](README_ch.md)

## Overview

`remote-solver` is an event-driven heterogeneous scheduling framework for solver workloads.
It focuses on cost-aware dispatching, checkpoint-based resume, and round-robin slicing for complex tasks.

## Module Structure

The project is organized into three modules:

```
ospf-remote-solver/
├── ospf-remote-solver-protocol/     # Shared protocol models and ports
├── ospf-remote-solver-dispatcher/   # Scheduler module (API + scheduling + events)
│   ├── HTTP API endpoints           # Task submission, control, monitoring
│   ├── Scheduler engine             # Cost-aware task dispatching
│   ├── Event publishing             # Kafka/in-memory adapters
│   ├── Database adapters            # Ktorm/PostgreSQL persistence
│   └── All tests                    # Contract and integration tests
└── ospf-remote-solver-calculator/   # Solver execution module
    ├── OspfExternalProcessBridge    # External process invocation
    ├── OspfSolverExecutionPort      # OSPF solver adapter
    └── InMemorySolverExecutionPort  # In-memory test adapter
```

**Dependency relationships:**
- `dispatcher` → `protocol` + `calculator`
- `calculator` → `protocol`
- Kotlin remote clients are provided by `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote`.
- A Rust remote client implementation is available in `ospf-rust-framework/src/solver/remote`.

**Usage:**
- Applications needing only the client: use `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote`
- Full scheduler deployment: depend on `ospf-remote-solver-dispatcher`
- Custom solver execution: depend on `ospf-remote-solver-calculator`

## Infrastructure Adapters

The framework uses a port/adapter architecture, allowing each infrastructure component to be swapped via configuration.

### Quick Reference

| Extension Point (Port) | Available Adapters | Default | Production |
|------------------------|-------------------|---------|------------|
| `EventPort` | `inmemory`, `kafka` | `inmemory` | `kafka` |
| `TaskStatePort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `NodeStatePort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `BudgetPort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `CostLedgerPort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `DistributedLockPort` | `inmemory`, `ktorm` | `inmemory` | `ktorm` |
| `ObjectStoragePort` | `inmemory`, `localfs`, `s3` | `inmemory` | `s3` |
| `CheckpointPort` | tied to `ObjectStoragePort` | `inmemory` | `s3` |
| `SchedulerConfigAuditPort` | `inmemory`, `localfs`, `ktorm` | `inmemory` | `ktorm` |
| `MetricsPort` | `inmemory`, `prometheus` | `inmemory` | `prometheus` |
| `SolverExecutionPort` | `inmemory`, `ospf` | `inmemory` | `ospf` |

### Configuration Keys

Each extension point is configured via `<port-name>.adapter=<key>`:

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

### Available Adapters by Category

#### Event Publishing

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `EventPort` | `inmemory` | `InMemoryEventPort` | In-process event delivery (default, no external dependencies) |
| | `kafka` | `KafkaEventPort` | Kafka-based event publishing (production) |

**Configuration:**
```properties
event.adapter=inmemory|kafka
event.kafka.bootstrap-servers=host:9092
```

#### Task State Persistence

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `TaskStatePort` | `inmemory` | `InMemoryTaskStatePort` | ConcurrentHashMap (default, process-restart loses data) |
| | `ktorm` | `KtormTaskStatePort` | PostgreSQL persistence (production) |

**Configuration:**
```properties
task-state.adapter=inmemory|ktorm
task-state.ktorm.url=jdbc:postgresql://host:5432/db
task-state.ktorm.username=user
task-state.ktorm.password=pass
```

#### Node State Persistence

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `NodeStatePort` | `inmemory` | `InMemoryNodeStatePort` | ConcurrentHashMap (default) |
| | `ktorm` | `KtormNodeStatePort` | PostgreSQL persistence (production) |

**Configuration:**
```properties
node-state.adapter=inmemory|ktorm
node-state.ktorm.url=jdbc:postgresql://host:5432/db
```

#### Budget Management

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `BudgetPort` | `inmemory` | `InMemoryBudgetPort` | ConcurrentHashMap (default) |
| | `ktorm` | `KtormBudgetPort` | PostgreSQL persistence (production) |

**Configuration:**
```properties
budget.adapter=inmemory|ktorm
budget.ktorm.url=jdbc:postgresql://host:5432/db
```

#### Cost Ledger

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `CostLedgerPort` | `inmemory` | `InMemoryCostLedgerPort` | In-memory list (default) |
| | `ktorm` | `KtormCostLedgerPort` | PostgreSQL persistence (production) |

**Configuration:**
```properties
cost-ledger.adapter=inmemory|ktorm
cost-ledger.ktorm.url=jdbc:postgresql://host:5432/db
```

#### Distributed Lock

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `DistributedLockPort` | `inmemory` | `InMemoryDistributedLockPort` | Local lock only (default, no cross-node coordination) |
| | `ktorm` | `KtormDistributedLockPort` | Database-based distributed lock (production) |

**Configuration:**
```properties
distributed-lock.adapter=inmemory|ktorm
distributed-lock.ktorm.url=jdbc:postgresql://host:5432/db
```

#### Object Storage

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `ObjectStoragePort` | `inmemory` | `InMemoryObjectStoragePort` | ConcurrentHashMap (default, process-restart loses data) |
| | `localfs` | `LocalFsObjectStoragePort` | Local filesystem storage |
| | `s3` | `S3ObjectStoragePort` | S3/MinIO object storage (production) |

**Configuration:**
```properties
storage.adapter=inmemory|localfs|s3
storage.localfs.root=./data/storage
storage.s3.bucket=my-bucket
storage.s3.endpoint=http://minio:9000
storage.s3.access-key-id=minioadmin
storage.s3.secret-access-key=minioadmin
```

#### Checkpoint Storage

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `CheckpointPort` | `inmemory` | `InMemoryCheckpointPort` | In-memory (default, tied to ObjectStoragePort) |
| | `localfs` | `LocalFsCheckpointPort` | Local filesystem (tied to ObjectStoragePort) |
| | `s3` | `S3CheckpointPort` | S3/MinIO storage (production, tied to ObjectStoragePort) |

**Note:** CheckpointPort uses the same `storage.adapter` setting as ObjectStoragePort.

#### Scheduler Config Audit

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `SchedulerConfigAuditPort` | `inmemory` | `InMemorySchedulerConfigAuditPort` | In-memory list (default) |
| | `localfs` | `LocalFsSchedulerConfigAuditPort` | Local filesystem JSON files |
| | `ktorm` | `KtormSchedulerConfigAuditPort` | PostgreSQL persistence (production) |

**Configuration:**
```properties
scheduler.audit.adapter=inmemory|localfs|ktorm
scheduler.audit.localfs.path=./data/audit
scheduler.audit.ktorm.url=jdbc:postgresql://host:5432/db
```

#### Metrics Export

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `MetricsPort` | `inmemory` | `InMemoryMetricsPort` | In-memory counters (default, for testing) |
| | `prometheus` | `PrometheusMetricsPort` | Prometheus exposition format (production) |

**Configuration:**
```properties
metrics.adapter=inmemory|prometheus
metrics.canonical.enabled=true
```

When `metrics.adapter=prometheus`, metrics are exposed at `GET /metrics`.

#### Solver Execution

| Port | Adapter Key | Implementation | Description |
|------|-------------|----------------|-------------|
| `SolverExecutionPort` | `inmemory` | `InMemorySolverExecutionPort` | Simulates solver without actual execution (default, testing only) |
| | `ospf` | `OspfSolverExecutionPort` | Calls OSPF solver via bridge (production) |

**Configuration:**
```properties
solver-execution.adapter=inmemory|ospf
solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
solver-execution.ospf.bridge.arg.command=/path/to/solver-runner
```

**Important:** `InMemorySolverExecutionPort` does NOT call real solvers - it only simulates progress from 0.0 to 1.0. For production, always use `ospf` adapter with a proper bridge.

### Production Configuration Example

```properties
# Event - Kafka for cross-node coordination
event.adapter=kafka
event.kafka.bootstrap-servers=kafka:9092

# Database - PostgreSQL for persistence
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

# Object Storage - S3/MinIO
storage.adapter=s3
storage.s3.bucket=remote-solver
storage.s3.endpoint=http://minio:9000
storage.s3.access-key-id=minioadmin
storage.s3.secret-access-key=minioadmin

# Scheduler Audit - Database
scheduler.audit.adapter=ktorm
scheduler.audit.ktorm.url=jdbc:postgresql://postgres:5432/remote_solver

# Metrics - Prometheus
metrics.adapter=prometheus

# Solver Execution - OSPF bridge
solver-execution.adapter=ospf
solver-execution.ospf.bridge.arg.command=/opt/solver/bin/run-solver.sh
```

### Development/Testing Configuration Example

```properties
# All in-memory (these are the defaults)
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

### InMemory Adapter Limitations

InMemory adapters are designed for local development and testing with zero external dependencies. Be aware of these limitations:

| Adapter | Limitation |
|---------|------------|
| `InMemoryEventPort` | Events are delivered in-process only; no cross-node broadcast |
| `InMemoryTaskStatePort` | Data lost on process restart |
| `InMemoryNodeStatePort` | Data lost on process restart |
| `InMemoryBudgetPort` | Data lost on process restart |
| `InMemoryCostLedgerPort` | Data lost on process restart |
| `InMemoryDistributedLockPort` | Local lock only; no cross-node coordination |
| `InMemoryObjectStoragePort` | Data lost on process restart |
| `InMemoryCheckpointPort` | Data lost on process restart |
| `InMemorySchedulerConfigAuditPort` | Data lost on process restart |
| `InMemoryMetricsPort` | No external metrics scraping |
| `InMemorySolverExecutionPort` | **Does NOT call real solvers**; only simulates progress 0.0→1.0 |

**Critical:** For production deployments, especially `SolverExecutionPort`, always use the production adapter (`ospf`) with proper solver integration.

### Fixed Infrastructure Components

These components have no adapter selection (fixed implementation):

| Port | Implementation | Description |
|------|----------------|-------------|
| `ClockPort` | `SystemClockPort` | System wall clock |
| `IdGeneratorPort` | `UUIDIdGeneratorPort` | UUID-based ID generation |
| `TracingPort` | `InMemoryTracingPort` | In-memory span tracking (consider OpenTelemetry for production) |

## Current Scope

1. Domain model and port interfaces are defined.
2. In-memory adapters are available for end-to-end local runs.
3. Scheduler supports cost-aware node selection.
4. Complex tasks support slice execution with checkpoint and warm-start resume.
5. Budget and cost ledger tracking are included.
6. Event publishing supports optional mirror-write mode for gray rollout.
7. Checkpoint retention can be capped per task.

## Runtime Properties

1. `event.adapter=inmemory|kafka`
2. `event.mirror.enabled=true|false` (default `false`)
3. `event.mirror.fail-open=true|false` (default `true`)
4. `event.kafka.bootstrap-servers=<host:port[,host:port...]>` (required when event adapter is `kafka`)
5. `event.kafka.client-id=<client-id>` (default `remote-solver-event-port`)
6. `event.kafka.consumer-poll-interval-ms=<positive-long>` (default `200`)
7. `event.kafka.query-poll-timeout-ms=<positive-long>` (default `1500`)
8. `event.kafka.query-max-poll-rounds=<positive-int>` (default `8`)
9. `event.kafka.query-topics=<csv-topics>` (default `SolvingRequest,SolvingControl,TaskDispatch,SolverHeartBeat,SliceLifecycle,TaskResult,CostAndCapabilityUpdate,MonitorAlert`)
10. `distributed-lock.adapter=inmemory|ktorm`
11. `distributed-lock.ktorm.url=<jdbc-url>` (required when lock adapter is `ktorm`)
12. `distributed-lock.ktorm.username=<username>` (optional)
13. `distributed-lock.ktorm.password=<password>` (optional)
14. `distributed-lock.ktorm.table=<table-name>` (default `remote_solver_lock`)
15. `node-state.adapter=inmemory|ktorm`
16. `node-state.ktorm.url=<jdbc-url>` (required when node-state adapter is `ktorm`)
17. `node-state.ktorm.username=<username>` (optional)
18. `node-state.ktorm.password=<password>` (optional)
19. `node-state.ktorm.table=<table-name>` (default `remote_solver_node_state`)
20. `budget.adapter=inmemory|ktorm`
20. `budget.ktorm.url=<jdbc-url>` (required when budget adapter is `ktorm`)
21. `budget.ktorm.username=<username>` (optional)
22. `budget.ktorm.password=<password>` (optional)
23. `budget.ktorm.table=<table-name>` (default `remote_solver_budget`)
24. `task-state.adapter=inmemory|ktorm`
25. `task-state.ktorm.url=<jdbc-url>` (required when task-state adapter is `ktorm`)
26. `task-state.ktorm.username=<username>` (optional)
27. `task-state.ktorm.password=<password>` (optional)
28. `task-state.ktorm.task-table=<table-name>` (default `remote_solver_task_state`)
29. `task-state.ktorm.slice-table=<table-name>` (default `remote_solver_slice_state`)
30. `cost-ledger.adapter=inmemory|ktorm`
31. `cost-ledger.ktorm.url=<jdbc-url>` (required when cost-ledger adapter is `ktorm`)
32. `cost-ledger.ktorm.username=<username>` (optional)
33. `cost-ledger.ktorm.password=<password>` (optional)
34. `cost-ledger.ktorm.table=<table-name>` (default `remote_solver_cost_ledger`)
35. `solver-execution.adapter=inmemory|ospf`
36. `solver-execution.ospf.simulated-total-runtime-ms=<positive-long>` (default `12000`)
37. `solver-execution.ospf.bridge-class=<fqcn>` (optional, class must implement `OspfExecutionBridge`)
38. `solver-execution.ospf.bridge.arg.<key>=<value>` (optional bridge custom args)
39. If `bridge-class` is empty but `solver-execution.ospf.bridge.arg.command` is provided,
    scheduler auto-uses `OspfExternalProcessBridge`.
40. Built-in bridge example:
   `solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge`
   `solver-execution.ospf.bridge.arg.command=<your-runner-command>`
41. `storage.adapter=inmemory|localfs|s3`
42. `storage.localfs.root=<path>`
43. `storage.s3.bucket=<bucket-name>` (required when storage adapter is `s3`)
44. `storage.s3.region=<region>` (default `us-east-1`)
45. `storage.s3.endpoint=<endpoint-url>` (optional, recommended for MinIO)
46. `storage.s3.access-key-id=<access-key>` (required when storage adapter is `s3`)
47. `storage.s3.secret-access-key=<secret-key>` (required when storage adapter is `s3`)
48. `storage.s3.path-style-access=true|false` (default `true`)
49. `storage.s3.object-prefix=<prefix>` (default `objects`)
50. `storage.s3.checkpoint-prefix=<prefix>` (default `checkpoints`)
51. `checkpoint.retention.max-per-task=<non-negative integer>` (`0` means unlimited)
52. `scheduler.performance-learning.enabled=true|false` (default `true`)
53. `scheduler.performance-learning.rate=<double in [0,1]>` (default `0.2`)
54. `scheduler.performance-learning.score-min=<non-negative double>` (default `0.1`)
55. `scheduler.performance-learning.score-max=<non-negative double>` (default `10.0`, and must be >= `score-min`)
56. `api.http.port=<1..65535>` (default `18080`, used by `RemoteSolverApiMain`)
57. `api.tenant.auth.enabled=true|false` (default `false`; when enabled, API requires `X-Tenant-Id` header)
58. `scheduler.config.version=<string>` (default `v0`)
59. `scheduler.hot-reload.enabled=true|false` (default `false`)
60. `scheduler.audit.adapter=inmemory|localfs|ktorm` (default `inmemory`)
61. `scheduler.audit.localfs.path=<path>` (default `target/remote-solver-audit`, only for `localfs`)
62. `scheduler.audit.ktorm.url=<jdbc-url>` (required when scheduler audit adapter is `ktorm`)
63. `scheduler.audit.ktorm.username=<username>` (optional)
64. `scheduler.audit.ktorm.password=<password>` (optional)
65. `scheduler.audit.ktorm.audit-table=<table-name>` (default `remote_solver_scheduler_audit`)
66. `scheduler.audit.ktorm.snapshot-table=<table-name>` (default `remote_solver_scheduler_snapshot`)
67. `metrics.canonical.enabled=true|false` (default `true`)
68. `metrics.adapter=inmemory|prometheus` (default `inmemory`; `/metrics` is enabled when adapter supports scrape)
69. `api.monitor.auth.enabled=true|false` (default `true`; protects `/api/v1/monitor/*` and `/monitor`)
70. `api.monitor.auth.roles=<csv-roles>` (default `admin,monitor_read`)
71. `api.auth.login-url=<path-or-url>` (default `/login`; used by unauthenticated `/monitor` redirect)
72. `monitor.alert.enabled=true|false` (default `false`)
73. `monitor.alert.check-interval-ms=<non-negative long>` (default `5000`)
74. `monitor.alert.overview-limit=<positive int>` (default `300`)
75. `monitor.alert.cooldown-ms=<non-negative long>` (default `60000`)
76. `monitor.alert.escalate-after-consecutive=<positive int>` (default `3`)
77. `monitor.alert.stale-nodes-threshold=<non-negative int>` (default `1`, `0` means disabled)
78. `monitor.alert.offline-nodes-threshold=<non-negative int>` (default `1`, `0` means disabled)
79. `monitor.alert.failed-tasks-threshold=<non-negative int>` (default `5`, `0` means disabled)
80. `monitor.alert.failed-ratio-threshold=<double in [0,1]>` (default `0.3`)
81. `monitor.alert.queue-depth-threshold=<non-negative int>` (default `100`, `0` means disabled)
82. `monitor.alert.route.event.enabled=true|false` (default `true`, publish to topic `MonitorAlert`)
83. `monitor.alert.route.webhook.enabled=true|false` (default `false`)
84. `monitor.alert.route.webhook.url=<http-url>` (required when webhook route is enabled)
85. `monitor.alert.route.webhook.timeout-ms=<positive long>` (default `3000`)

## Deployment

### Topology

1. Scheduler node (runs `RemoteSolverService` and all selected adapters).
2. Solver node(s) (run OSPF worker process / bridge target command).
3. Shared infrastructure (optional by adapter): Kafka, JDBC database, Object Storage (LocalFS or S3/MinIO).
4. Monitoring stack (optional): Prometheus, Grafana, AlertManager.

---

## Scheduler Service Deployment

### Prerequisites

1. Java 11+ runtime
2. Database (PostgreSQL recommended) for production adapters
3. Kafka cluster for event-driven mode
4. Object storage (S3/MinIO or LocalFS) for model/checkpoint persistence

### Step 1: Database Migration

Before first JDBC deployment, run migration scripts:

```bash
# PostgreSQL example
./deploy/scripts/apply-migrations.sh "postgresql://127.0.0.1:5432/remote_solver"
```

Windows:
```powershell
.\deploy\scripts\apply-migrations.ps1 -DbUrl "postgresql://127.0.0.1:5432/remote_solver"
```

Migration scripts:
- `deploy/sql/V1__remote_solver_core.sql` - Core tables (lock, node_state, budget, task_state, slice_state, cost_ledger)
- `deploy/sql/V2__remote_solver_infra.sql` - Infrastructure tables
- `deploy/sql/V3__remote_solver_scheduler_audit.sql` - Scheduler audit tables
- `deploy/sql/V4__remote_solver_multi_tenant.sql` - Multi-tenant support (tenant_id columns)

### Step 2: Configuration

Use template `deploy/config/scheduler-ktorm.properties` for production:

```properties
# Event adapter
event.adapter=kafka
event.kafka.bootstrap-servers=127.0.0.1:9092

# Database adapters (Ktorm/PostgreSQL)
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

# Scheduler audit persistence
scheduler.audit.adapter=ktorm
scheduler.audit.ktorm.url=jdbc:postgresql://127.0.0.1:5432/remote_solver

# Solver execution
solver-execution.adapter=ospf
solver-execution.ospf.bridge-class=fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
solver-execution.ospf.bridge.arg.command=<solver-runner-command>

# Object storage
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
api.monitor.auth.roles=admin,monitor_read

# Monitoring alert
monitor.alert.enabled=true
monitor.alert.check-interval-ms=5000
monitor.alert.route.webhook.enabled=true
monitor.alert.route.webhook.url=http://alertmanager:9093/api/v1/alerts
```

### Step 3: Start Scheduler Service

Start scheduler-only mode (no HTTP API):

```bash
./deploy/scripts/start-scheduler.sh deploy/config/scheduler-ktorm.properties
```

Windows:
```powershell
.\deploy\scripts\start-scheduler.ps1 -ConfigPath deploy/config/scheduler-ktorm.properties
```

Start API service (includes scheduler loop + HTTP endpoints):

```bash
./deploy/scripts/start-api.sh deploy/config/scheduler-ktorm.properties 0.0.0.0 18080
```

Windows:
```powershell
.\deploy\scripts\start-api.ps1 -ConfigPath deploy/config/scheduler-ktorm.properties -Host 0.0.0.0 -Port 18080
```

### Step 4: Verify Scheduler

Run smoke test:

```bash
./deploy/scripts/smoke-up.sh deploy/config/scheduler-ktorm.properties
```

Or manual verification:

```bash
# Submit a test task
TASK_ID=$(curl -s -X POST "http://127.0.0.1:18080/api/v1/tasks" \
  -H "Content-Type: application/json" \
  -d '{"payloadRef":"models/demo","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}' \
  | sed -n 's/.*"taskId":"\([^"]*\)".*/\1/p')

# Check task status
curl -s "http://127.0.0.1:18080/api/v1/tasks/$TASK_ID"
```

---

## Solver Node Deployment

### Prerequisites

1. Java 11+ runtime
2. Solver engine (Gurobi, SCIP, or other OSPF-compatible solver)
3. Network access to scheduler and object storage

### Step 1: Worker Configuration

Solver nodes are invoked by scheduler through bridge command. Built-in worker entrypoint:

`fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverWorkerMain`

Worker parameters:
- `--model <path>` - Model reference path
- `--task <taskId>` - Task identifier
- `--slice <sliceId>` - Slice identifier
- `--node <nodeId>` - Node identifier
- `--quantum_ms <ms>` - Time quantum for this slice
- `--checkpoint-in <path>` - Optional checkpoint for warm-start

Additional options:
- `--state-dir <dir>` - Local state directory (default: `target/remote-solver-worker-state`)
- `--total-runtime-ms <ms>` - Simulated runtime (default: `12000`)

### Step 2: Register Node

After starting worker process, register node capabilities with scheduler:

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

### Step 3: Verify Worker

Manual worker invocation (for protocol check):

```bash
./deploy/scripts/start-worker.sh --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

Windows:
```powershell
.\deploy\scripts\start-worker.ps1 --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

Expected stdout output:
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

## Monitor Service Deployment

### Prerequisites

1. Prometheus server (for metrics scraping)
2. Grafana server (for visualization)
3. AlertManager (for alert routing)

### Step 1: Prometheus Configuration

Add scraper config to Prometheus:

```yaml
scrape_configs:
  - job_name: 'remote-solver'
    scrape_interval: 15s
    static_configs:
      - targets: ['scheduler-host:18080']
    metrics_path: /metrics
```

Alert rules are provided in:
`deploy/observability/prometheus/alerts-remote-solver.yml`

### Step 2: AlertManager Configuration

AlertManager routing config:
`deploy/observability/alertmanager/alertmanager.yml`

Key receivers:
- `critical-alerts` - High severity alerts (node offline, budget exceeded)
- `warning-alerts` - Medium severity (stale nodes, queue depth)
- `node-alerts` - Node-specific alerts
- `task-alerts` - Task failure alerts

Inhibit rules:
- Critical alerts suppress warning alerts for same resource
- Node offline suppresses node recovery alerts

### Step 3: Grafana Dashboard

Import dashboard template:
`deploy/observability/grafana/remote-solver-overview.json`

Import script:
```bash
./deploy/scripts/import-observability.sh http://grafana:3000 <api-key> ./target/observability/prometheus-rules
```

Windows:
```powershell
.\deploy\scripts\import-observability.ps1 -GrafanaUrl http://grafana:3000 -GrafanaApiKey <api-key> -PromRulesTargetDir .\target\observability\prometheus-rules
```

### Step 4: Monitor Access

Dashboard URL: `http://scheduler-host:18080/monitor`

Required headers for access:
- `X-User-Id: <user-id>`
- `X-User-Roles: admin` or `X-User-Roles: monitor_read`

Monitor API:
```bash
curl -s "http://127.0.0.1:18080/api/v1/monitor/overview?limit=300" \
  -H "X-User-Id: ops-user" \
  -H "X-User-Roles: monitor_read"
```

### Step 5: Alert Smoke Test

Validate alert configuration:
```bash
./deploy/scripts/validate-alert-linkage.sh
```

Windows:
```powershell
.\deploy\scripts\validate-alert-linkage.ps1
```

Validate monitor-alert linkage:
```bash
./deploy/scripts/validate-monitor-alert-linkage.sh
```

Windows:
```powershell
.\deploy\scripts\validate-monitor-alert-linkage.ps1 -ApiPort 18091 -WebhookPort 19091
```

---

## Quick Reference: Deployment Scripts

| Script | Purpose |
|--------|---------|
| `start-scheduler.sh/ps1` | Start scheduler service |
| `start-api.sh/ps1` | Start API service with scheduler |
| `start-worker.sh/ps1` | Start solver worker |
| `apply-migrations.sh/ps1` | Apply database migrations |
| `smoke-up.sh/ps1` | End-to-end smoke test |
| `load-test.sh/ps1` | Load test with mixed tasks |
| `monitor-smoke.sh/ps1` | Monitor API/dashboard smoke test |
| `monitor-alert-smoke.sh/ps1` | Monitor alert linkage smoke test |
| `validate-alert-linkage.sh/ps1` | Validate AlertManager config |
| `validate-monitor-alert-linkage.sh/ps1` | Validate monitor-alert integration |
| `import-observability.sh/ps1` | Import Prometheus/Grafana assets |
| `scheduler-audit-dump.sh/ps1` | Dump scheduler audit logs |

---

### Scheduler Configuration

Default config location:
`deploy/config/scheduler.properties`
Ktorm production template:
`deploy/config/scheduler-ktorm.properties`

Database baseline migration scripts:
1. `deploy/sql/V1__remote_solver_core.sql`
2. `deploy/sql/V2__remote_solver_infra.sql`
3. `deploy/sql/V3__remote_solver_scheduler_audit.sql`
4. `deploy/sql/V4__remote_solver_multi_tenant.sql`

Run before first JDBC deployment (example for PostgreSQL):
```bash
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V1__remote_solver_core.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V2__remote_solver_infra.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V3__remote_solver_scheduler_audit.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V4__remote_solver_multi_tenant.sql
```

Or use migration script:
```bash
./deploy/scripts/apply-migrations.sh "postgresql://127.0.0.1:5432/remote_solver"
```

Windows:
```powershell
.\deploy\scripts\apply-migrations.ps1 -DbUrl "postgresql://127.0.0.1:5432/remote_solver"
```

Minimum scheduler config (single-machine local run):

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

Start scheduler service:

```bash
./deploy/scripts/start-scheduler.sh deploy/config/scheduler.properties
```

Windows:

```powershell
.\deploy\scripts\start-scheduler.ps1 -ConfigPath deploy/config/scheduler.properties
```

Start API service (includes scheduler loop):

```bash
./deploy/scripts/start-api.sh deploy/config/scheduler.properties 0.0.0.0 18080
```

Windows:

```powershell
.\deploy\scripts\start-api.ps1 -ConfigPath deploy/config/scheduler.properties -Host 0.0.0.0 -Port 18080
```

HTTP adapter implementation: Ktor (`CIO` engine) + `kotlinx.serialization`.

Bootstrap CLI argument/config parsing is centralized in:
`fuookami.ospf.framework.remote_solver.bootstrap.BootstrapCliSupport`.

### API Endpoints

1. `POST /api/v1/tasks`
2. `GET /api/v1/tasks/{taskId}`
3. `POST /api/v1/tasks/{taskId}/stop`
4. `POST /api/v1/tasks/{taskId}/resume`
5. `POST /api/v1/scheduler/config/hot-reload`
6. `POST /api/v1/scheduler/config/rollback`
7. `GET /api/v1/scheduler/config/audits?limit=<n>`
8. `GET /api/v1/tasks/{taskId}/timeline?limit=<n>`
9. `GET /api/v1/monitor/overview?limit=<n>`
10. `GET /monitor` (built-in monitoring dashboard page)
11. `GET /metrics` (enabled when `metrics.adapter=prometheus`)

Monitor access notes:
1. `/api/v1/monitor/*` and `/monitor` require `X-User-Id` and role header `X-User-Roles`.
2. Allowed roles are `admin` and `monitor_read` by default.
3. Unauthenticated monitor API requests return `401`.
4. Unauthenticated dashboard requests return `302` redirect to `api.auth.login-url` (default `/login`).

Monitor alert linkage notes:
1. Set `monitor.alert.enabled=true` to evaluate monitor snapshots periodically in API loop.
2. Active anomalies emit `TRIGGERED`, consecutive unresolved anomalies emit `ESCALATED`, and recovered anomalies emit `RECOVERED`.
3. Event routing publishes alert notifications to topic `MonitorAlert`.
4. Optional webhook routing can forward the same alert payload to external alert systems.

Response envelope:

```json
{
  "code": "OK",
  "message": "success",
  "data": {}
}
```

Error response envelope:

```json
{
  "code": "INVALID_ARGUMENT",
  "message": "payloadRef.path must not be blank",
  "data": null
}
```

Submit request body example:

```json
{
  "tenantId": "tenant-a",
  "payloadRef": "models/demo",
  "complexity": "SIMPLE",
  "timeSensitivity": "NON_REALTIME",
  "priority": 1
}
```

Tenant notes:
1. `tenantId` defaults to `default` when omitted.
2. API submit refs are auto-scoped to tenant path prefix (`tenantId/...`) when `tenantId` is provided.
3. `budgetScope` is normalized to `tenantId:<scope>` to prevent cross-tenant ledger collisions.
4. Event envelope includes `tenantId` header by default.

Stop request body example:

```json
{
  "reason": "manual-stop"
}
```

Scheduler hot-reload request body example:

```json
{
  "operator": "ops",
  "changeSet": {
    "scheduler.simple-task-quantum-ms": "3200",
    "scheduler.complex-urgency-weight": "0.7"
  }
}
```

Scheduler rollback request body example:

```json
{
  "operator": "ops",
  "targetVersion": "v-base"
}
```

Quick curl flow:

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
# Unauthenticated dashboard call redirects:
# curl -I "http://127.0.0.1:18080/monitor"
```

Monitoring smoke check (API + dashboard):

```bash
./deploy/scripts/monitor-smoke.sh "http://127.0.0.1:18080" 300 "smoke-user" "monitor_read"
```

Windows:

```powershell
.\deploy\scripts\monitor-smoke.ps1 -BaseUrl "http://127.0.0.1:18080" -Limit 300 -AuthUser "smoke-user" -AuthRoles "monitor_read"
```

Monitor alert linkage smoke check (queue-depth anomaly -> webhook trigger/escalate):

```bash
./deploy/scripts/monitor-alert-smoke.sh "http://127.0.0.1:18091" "http://127.0.0.1:19091/events" 2 60
```

Windows:

```powershell
.\deploy\scripts\monitor-alert-smoke.ps1 -BaseUrl "http://127.0.0.1:18091" -WebhookEventsUrl "http://127.0.0.1:19091/events" -TaskCount 2 -PollTimeoutSeconds 60
```

Monitor alert linkage validation (starts mock webhook + API and runs smoke):

```bash
./deploy/scripts/validate-monitor-alert-linkage.sh
```

Windows:

```powershell
.\deploy\scripts\validate-monitor-alert-linkage.ps1 -ApiPort 18091 -WebhookPort 19091
```

Alternative config injection:
1. CLI argument `--config <path>`
2. Environment variable `REMOTE_SOLVER_CONFIG=<path>`
3. Fallback path `deploy/config/scheduler.properties`

Production-like scheduler config (example):

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

### Solver Node Configuration

Solver node service is invoked by scheduler through `solver-execution.ospf.bridge.arg.command`.
This repository provides a worker entrypoint:
`fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverWorkerMain`

Manual worker startup (for protocol check only):

```bash
./deploy/scripts/start-worker.sh --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

Windows:

```powershell
.\deploy\scripts\start-worker.ps1 --model demo --task t1 --slice s1 --node n1 --quantum-ms 1000
```

Bridge command must accept:

1. `--model <path>`
2. `--task <taskId>`
3. `--slice <sliceId>`
4. `--node <nodeId>`
5. `--quantum-ms <quantum>`
6. optional `--checkpoint-in <path>`

Worker optional args in this repo:
1. `--state-dir <dir>` (default `target/remote-solver-worker-state`)
2. `--total-runtime-ms <long>` (default `12000`)

Expected stdout key-value format:

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

### Bring-up Checklist

1. Start shared dependencies (Kafka/JDBC/S3 if selected).
2. Start scheduler process with `RemoteSolverBootstrapFactory.create(...)` properties.
3. Start solver worker command on each solver node.
4. Register solver nodes through `registerNode(...)` with correct `solverType`, capability flags, and `parallelUnits`.
5. Submit a smoke task and verify task status transitions (`QUEUED -> RUNNING -> COMPLETED`).

Run built-in smoke flow:

```bash
mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverSmokeMain \
  -Dexec.args="--config deploy/config/scheduler.properties"
```

One-command smoke (start scheduler, run smoke, stop scheduler):

```bash
./deploy/scripts/smoke-up.sh deploy/config/scheduler.properties
```

Windows:

```powershell
.\deploy\scripts\smoke-up.ps1 -ConfigPath deploy/config/scheduler.properties
```

Run load test flow (mixed simple/complex tasks):

```bash
./deploy/scripts/load-test.sh deploy/config/scheduler.properties 200 0.7 0.2 3000 8 0.5
```

Arguments:
1. `configPath` (default `deploy/config/scheduler.properties`)
2. `totalTasks` (default `100`)
3. `simpleRatio` (default `0.7`)
4. `realtimeRatio` (default `0.2`)
5. `maxRounds` (default `2000`)
6. `nodes` (default `6`)
7. `highTierRatio` (default `0.4`)

Windows:

```powershell
.\deploy\scripts\load-test.ps1 -ConfigPath deploy/config/scheduler.properties -TotalTasks 200 -SimpleRatio 0.7 -RealtimeRatio 0.2 -MaxRounds 3000 -Nodes 8 -HighTierRatio 0.5
```

Dump scheduler hot-reload audits from local file:

```bash
./deploy/scripts/scheduler-audit-dump.sh deploy/config/scheduler.properties 50
```

Windows:

```powershell
.\deploy\scripts\scheduler-audit-dump.ps1 -ConfigPath deploy/config/scheduler.properties -Limit 50
```

Import observability assets (Prometheus alerts + Grafana dashboard):

```bash
./deploy/scripts/import-observability.sh http://127.0.0.1:3000 <grafana-api-key> ./target/observability/prometheus-rules
```

Windows:

```powershell
.\deploy\scripts\import-observability.ps1 -GrafanaUrl http://127.0.0.1:3000 -GrafanaApiKey <grafana-api-key> -PromRulesTargetDir .\target\observability\prometheus-rules
```

Linux validation helper:

```bash
./deploy/scripts/validate-observability-import.sh
```

## Solver Compatibility

Task-to-node solver compatibility is enforced before dispatch.
The required solver type can be provided in either:

1. `SolvePayload.taskMeta.solverType` (recommended)
2. `SolvePayload.extension["solverType"]`
3. `SolvePayload.taskMeta.metadata["solverType"]`

## Event Envelope Headers

`EventPort.publish(...)` now enforces and normalizes event envelope headers:

1. `schemaVersion`
2. `eventType`
3. `producer`
4. `occurredAtEpochMs`
5. optional `traceId`
6. optional `spanId`

Validation rules:
1. `schemaVersion` must be within supported range (`1` currently).
2. `occurredAtEpochMs` must be a positive integer when provided.
3. Missing required envelope headers are auto-filled:
   `eventType=<topic>`, `producer=<adapter-default>`, `occurredAtEpochMs=<publish-time>`.

## Error Code Mapping

`RemoteSolverException.code` is now aligned to normalized failure reasons.

Common runtime failure codes:
1. `TASK_FAILED_BUDGET_EXCEEDED`
2. `TASK_FAILED_HARD_TIMEOUT`
3. `TASK_FAILED_SLICE_TIMEOUT`
4. `SOLVER_EXECUTION_FAILED`

Failed tasks also persist `latestResult.extension["reasonCode"]` for downstream diagnosis.

## Remote Solver Entry

Use bootstrap runtime to get the service-side API facade:

```kotlin
val runtime = RemoteSolverBootstrapFactory.create()
val api = runtime.apiFacade
```

Client-side remote solver wrappers now live in `ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote`. Use that package from Kotlin applications that submit work to a deployed dispatcher. The Rust implementation has also been completed under `ospf-rust-framework/src/solver/remote`.

```kotlin
// Client applications should depend on ospf-kotlin-framework and use its solver remote client package.
```

The `remote-solver` project keeps server-side scheduling, execution, protocol, adapter responsibilities, and the protocol design needed by non-Kotlin clients.

For service-level one-shot workflow, use `submitAndAwait`:

```kotlin
val task = runtime.service.submitAndAwait(
    payload = SolvePayload(modelRef = ObjectRef(path = "models/lp")),
    complexity = TaskComplexity.SIMPLE,
    timeSensitivity = TimeSensitivity.NON_REALTIME,
    maxRounds = 20
)
```

Set `throwIfNotTerminal = true` if you want an exception when task is still non-terminal after `maxRounds`.
The exception type is `RemoteSolverException` with code `TASK_NOT_TERMINAL_WITHIN_MAX_ROUNDS`.
If `throwIfFailed = true`, failed tasks also throw `RemoteSolverException` (for example `TASK_FAILED_BUDGET_EXCEEDED`).

Task control APIs:

```kotlin
val stopped = runtime.service.stopTask(taskId = "task-1", reason = "manual-stop")
val resumed = runtime.service.resumeTask(taskId = "task-1")
```

`resumeTask` supports `STOPPED -> QUEUED|SUSPENDED` and rejects invalid transitions with
`RemoteSolverException(code = INVALID_TASK_STATE_TRANSITION)`.

For API-layer integration (without binding a specific HTTP framework), use `RemoteSolverApiFacade`:

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

Task timeline replay via API facade:

```kotlin
val report = api.replayTaskTimeline(taskId = submit.taskId, limitEvents = 200)
println("events=${report.events.size}, gaps=${report.gaps.size}")
```

Task timeline replay via CLI:

```bash
mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverReplayMain \
  -Dexec.args="--config deploy/config/scheduler.properties --task <taskId> --format text --limit 200"
```

## Run Tests

```bash
mvn test
```

## Observability

Metrics spec:
1. `docs/observability/metrics-spec.md` (English)
2. `docs/observability/metrics-spec_ch.md` (中文)

Prometheus alert rules:
1. `deploy/observability/prometheus/alerts-remote-solver.yml`

Grafana dashboard template:
1. `deploy/observability/grafana/remote-solver-overview.json`

AlertManager routing config:
1. `deploy/observability/alertmanager/alertmanager.yml`

Notes:
1. `metrics.canonical.enabled=true` wraps metrics with canonical mapping (`remote_solver_*`).
2. Production telemetry adapters should export canonical names in `metrics-spec.md`.
3. `metrics.adapter=prometheus` exposes Prometheus text format on `GET /metrics`.

## Database Schema

Database migration scripts (PostgreSQL):

| Script | Tables | Description |
|--------|--------|-------------|
| `deploy/sql/V1__remote_solver_core.sql` | `remote_solver_lock`, `remote_solver_node_state`, `remote_solver_budget`, `remote_solver_task_state`, `remote_solver_slice_state`, `remote_solver_cost_ledger` | Core domain tables |
| `deploy/sql/V2__remote_solver_infra.sql` | Infrastructure extension tables | Infrastructure metadata |
| `deploy/sql/V3__remote_solver_scheduler_audit.sql` | `remote_solver_scheduler_audit`, `remote_solver_scheduler_snapshot` | Scheduler hot-reload audit trail |
| `deploy/sql/V4__remote_solver_multi_tenant.sql` | `tenant_id` column on `remote_solver_task_state`, `remote_solver_cost_ledger` | Multi-tenant isolation |

Apply migrations:

```bash
./deploy/scripts/apply-migrations.sh "postgresql://127.0.0.1:5432/remote_solver"
```

Windows:

```powershell
.\deploy\scripts\apply-migrations.ps1 -DbUrl "postgresql://127.0.0.1:5432/remote_solver"
```

Manual execution:

```bash
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V1__remote_solver_core.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V2__remote_solver_infra.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V3__remote_solver_scheduler_audit.sql
psql "postgresql://127.0.0.1:5432/remote_solver" -f deploy/sql/V4__remote_solver_multi_tenant.sql
```

## Acceptance Scenarios

The following design scenarios are mapped to executable tests:

1. Mixed load `7:3` (simple:complex), with complex-task checkpoint persistence:
   `RemoteSolverServiceAcceptanceTest.mixedLoadSevenToThreeShouldCompleteAndPersistComplexSnapshots`
2. Small burst queue drain without failures:
   `RemoteSolverServiceAcceptanceTest.smallBurstShouldDrainQueueWithoutFailures`

Run only acceptance tests:

```bash
mvn -Dtest=RemoteSolverServiceAcceptanceTest test
```

## Design

See [design.md](design.md) for architecture and staged roadmap.
