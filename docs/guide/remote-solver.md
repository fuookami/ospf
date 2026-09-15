# Remote Solving

Remote solving separates model construction from execution. Applications submit versioned payloads; the scheduler assigns work according to capabilities, capacity, deadlines, and budgets; execution bridges invoke actual backends and return structured reports and artifacts. Deployment may be on private servers or in a cloud; this is not a particular managed cloud product.

This page describes the [remote-solver framework](https://github.com/fuookami/ospf/tree/main/framework/remote-solver). Protocol and configuration follow the deployed version. Interface support, simulated workflows, and actual solver execution are distinct: a scheduling smoke test is not mathematical solver acceptance.

## 1. When to use it

- Application hosts should not install solvers or carry long-running computation.
- Tasks share nodes with different performance, pricing, licenses, and concurrency.
- Queuing, budgets, stop/resume, execution records, and artifacts need central management.
- Complex non-real-time tasks benefit from slices and portable recovery points.

Remote execution does not change the mathematics, improve optimality automatically, or guarantee hard-real-time SLAs. Actual backends still determine executability. The current design handles complex real-time tasks using complex non-real-time scheduling policies.

## 2. Modules and trust boundaries

| Component | Responsibility |
|---|---|
| Kotlin / Rust clients | Capability negotiation, submission, status/results, controls, result mapping |
| protocol | Shared wire format, task/slice identities, object references, execution ports |
| dispatcher | HTTP API, admission, queues, node selection, budgets, events, state |
| calculator | Payload reconstruction, OSPF execution adaptation, checkpoint orchestration |
| external worker | Independent-process execution through the configured backend, result files |
| State and object stores | Task/slice facts, and model/result/checkpoint contents respectively |

```text
Application model → Kotlin/Rust client → HTTP dispatcher
                                             │ admission, capabilities, budgets
                                             ↓
                                       calculator / bridge
                                             ↓
                                     external worker → OSPF backend
                                             ↓
                     validate files → upload artifacts → persist references
                                             ↓
                                        client result mapping
```

Dependencies are `dispatcher → calculator → protocol`, with dispatcher also depending directly on protocol. Clients live in the language-specific framework components. State stores are authoritative for current state; events support dispatch and audit, not a claim of complete event sourcing.

A bridge command is an execution boundary, not an arbitrary remote-shell protocol. Operators provide executable commands, accessible paths, and the process/network topology. Registering a node name does not provision a machine automatically.

## 3. Payloads and capability negotiation

Read `GET /api/v1/capabilities` before large uploads or submission. Verify protocol, model types, and checkpoint capabilities. Reported types aggregate online-node capabilities; they do not imply every backend supports every model.

| Model | Payload | Boundary |
|---|---|---|
| Linear | `SerializedLinearModel` or controlled object reference | Preserve domains, coefficients, stable identities |
| Quadratic | `SerializedQuadraticModel` or controlled object reference | Match quadratic and solver capabilities |
| CP | `ModelData.rawBytes`, `format=ospf-cp-snapshot-json`, target `cp` | Require protocol `2.0` and CP; the current real worker path uses SCIP CP |

`SolvePayload` also includes configuration or references, snapshot references, task metadata, and extensions. Prefer `taskMeta.solverType` for the required solver. Model type, solver compatibility, free slots, and budget are separate admission conditions.

Submission `payloadRef` identifies a **complete serialized SolvePayload**, not a bare LP file, CP snapshot, or client-local path. Its model data may reference another object. Uploads and references must use the deployment's object store and tenant rules; this page does not invent a generic HTTP upload endpoint.

`ObjectRef(path,version,etag)` retains object identity and integrity metadata. Recovery and database reconstruction must not keep only the path. Stable model, variable, constraint, and objective identities must not be replaced with display names or registration order.

## 4. Submission through results

1. Freeze model, configuration, identities, and task requirements; store the complete payload.
2. Dispatcher resolves and validates references, schemas, tenant, model, and configuration.
3. Queue the task, select a compatible node, and confirm dispatch.
4. Calculator reconstructs the OSPF model or invokes a controlled worker.
5. Validate slice results/checkpoints, upload trusted contents, and persist references.
6. Clients read reports, retrieve result artifacts as needed, and map stable identities.

### 4.1 HTTP example

This PowerShell example illustrates request ordering only. Start the service and first store a valid SolvePayload at the reference corresponding to `models/production-payload.json` using the deployment's storage adapter. No such ready-made object is assumed.

```powershell
$baseUrl = 'http://127.0.0.1:18080'
$headers = @{ 'X-Tenant-Id' = 'demo' }
$capabilities = Invoke-RestMethod "$baseUrl/api/v1/capabilities" -Headers $headers
$capabilities

# 先确认所需能力再提交 / Confirm required capabilities before submitting.
$body = @{
    tenantId = 'demo'
    payloadRef = 'models/production-payload.json'
    complexity = 'SIMPLE'
    timeSensitivity = 'NON_REALTIME'
    priority = 1
} | ConvertTo-Json
$reply = Invoke-RestMethod "$baseUrl/api/v1/tasks" -Method Post `
    -Headers $headers -ContentType 'application/json' -Body $body
if ($reply.code -ne 'OK') { throw $reply.message }
$taskId = $reply.data.taskId
Invoke-RestMethod "$baseUrl/api/v1/tasks/$taskId" -Headers $headers
```

This queries status once. Actual clients need bounded polling, timeout, and cancellation handling. Accepted submission does not mean completed optimization. Supply required authentication credentials when enabled; `X-Tenant-Id` alone is not authentication.

| Operation | HTTP endpoint |
|---|---|
| Capabilities | `GET /api/v1/capabilities` |
| Submit/query | `POST /api/v1/tasks`, `GET /api/v1/tasks/{taskId}` |
| Stop/resume | `POST /api/v1/tasks/{taskId}/stop`, `POST /api/v1/tasks/{taskId}/resume` |
| Timeline | `GET /api/v1/tasks/{taskId}/timeline?limit=200` |
| Health | `GET /health`, `GET /health/ready`, `GET /health/live` |

A stop body is `{"reason":"manual-stop"}`; a resume body is `{}`. Permissions and state transitions govern acceptance; arbitrary terminal states cannot simply resume.

### 4.2 Client entry points

Kotlin provides `RemoteSolverClient` and model-facing `RemoteLinearSolver`/`RemoteQuadraticSolver` wrappers. Rust provides a client implementation under `ospf-rust-framework/solver/remote`. Wire semantics are shared, but constructors and error types need not match.

- [Kotlin remote client sources](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote)
- [Rust remote client sources](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-framework/src/solver/remote)

Applications do not need the entire dispatcher dependency. `RemoteSolverBootstrapFactory` and `runtime.apiFacade` embed server services; they are not HTTP clients for cross-machine communication.

## 5. Scheduling, budgets, and task states

Simple tasks use available compatible nodes with cost/deadline tradeoffs. Complex tasks rotate through slices with execution and recovery records. This scheduling policy is not a proof of globally optimal infrastructure cost.

```text
QUEUED → ACCEPTED → DISPATCHING → RUNNING → COMPLETED
                                      └→ SUSPENDED → ACCEPTED
QUEUED / ACCEPTED → WAITING_FOR_BUDGET → QUEUED
Controllable state → STOPPING → STOPPED
Scheduling or execution failure → FAILED
```

Normal HTTP admission creates QUEUED tasks; CREATED is a compatibility/transient state. Resume supports `STOPPED → QUEUED|SUSPENDED`; invalid transitions return an error.

Slices record runtime, node price, billing granularity, and license charges. Insufficient budget can cause waiting or policy-driven downgrade, never relaxation of business constraints. Task time limits, slice quantum, dispatcher timeouts, and heartbeat timeouts have different responsibilities.

## 6. Checkpoints and recovery

A **portable checkpoint** retains portable model/input information, configuration identity, incumbent, and integrity metadata. Subsequent execution rebuilds and attempts a warm start; it does not promise a native search tree, thread state, or process handles. The service distinguishes `supportsPortableCheckpoint=true` from `supportsNativeCheckpoint=false`.

Backends without immediate interruption return through time limits or controlled phases; stop is not an immediate process-kill guarantee. CP recovery is rebuild-based and does not claim OR-Tools, native optional intervals, or native search-tree recovery.

When a node is lost, reclaim capacity and record slice failure. Resume from a trusted checkpoint if available; otherwise rerun stable inputs or fail according to policy. Dispatcher restart recovery requires persisted tasks/slices; default in-memory mode cannot survive process restart.

Before recovery, verify tenant, task, historical attempt, model/configuration fingerprints, schema, and digest. Reject damaged or mismatched artifacts instead of falling back to unvalidated files.

## 7. Interpreting results correctly

Task `COMPLETED` is an execution lifecycle state, not an optimality proof. Read independently:

- `problemStatus`: mathematical conclusion;
- `terminationReason`: why execution stopped;
- `solutionPresence`: availability of a usable solution;
- `proofStatus`: proof status;
- provenance, fingerprints, run/attempt identity, and result references.

Timeout with an incumbent differs from timeout without one; no incumbent is not proven infeasibility. Compatibility `feasible/optimal` booleans are insufficient. CP v2 preserves `objectiveValueInt64` and exact integer assignments keyed by stable IDs; intervals include start/size/end/present. Avoid lossy floating-point round trips.

Strict results pair `resultRef` with `artifactDigest` and validate run/attempt, fingerprints, and schema. Exit codes, stdout, and local filenames alone are not trusted results. Bridge validation and successful upload precede publishing remote references.

## 8. Deployment and configuration

### 8.1 Local integration versus actual solving

Defaults largely use `inmemory`, including simulated execution. They support single-process integration and lose data on exit. Setting `solver-execution.adapter=ospf` alone is not a replacement for configuring an actual execution bridge.

The built-in worker uses snapshot reconstruction and SCIP CP with `--model-format ospf-cp-snapshot-json`. Without that format it retains a non-CP compatibility progress protocol, **not an actual linear/quadratic solver result**. Connect an actual backend for those tasks; illustrative objective output is not acceptance evidence.

### 8.2 Bring-up steps

1. Match JDK, build environment, native libraries, and licenses to the current POM and solver dependencies; merely finding Java is insufficient.
2. Select state, event, and object adapters. Separate processes must not use mutually invisible in-memory object stores.
3. Back up JDBC databases, then apply V1→V7 in order. CP2 recovery needs V5 capabilities/reports, V6 inline configuration, and V7 ETags.
4. Configure an actual bridge command, worker file access, and truthful node capabilities. Do not infer model support from a solver name.
5. Start API/scheduler, register nodes and heartbeats, and validate an actual model end to end.

From the server's `ospf-remote-solver-dispatcher` directory, use the following script entry after adapting configuration to the environment:

```powershell
.\deploy\scripts\start-api.ps1 `
    -ConfigPath deploy/config/scheduler-ktorm.properties `
    -Host 127.0.0.1 -Port 18080
```

Configuration sources include `--config`, `REMOTE_SOLVER_CONFIG`, and default `deploy/config/scheduler.properties`. Typical production choices follow; connection details, credentials, and commands remain required:

| Port | Production choice | Purpose |
|---|---|---|
| event | kafka | Cross-process events |
| task-state / node-state / budget / cost-ledger / distributed-lock | ktorm | Persistence and coordination |
| storage | s3 | S3/MinIO model, result, checkpoint artifacts |
| scheduler.audit | ktorm | Configuration history |
| metrics | prometheus | Metric scraping |
| solver-execution | ospf + actual bridge | Real solver integration |

Enable authentication, tenant scoping, and secure transport. Monitoring user/role headers must come from a trusted authentication boundary, not be freely forged by public callers. Keep secrets out of source and examples. Hot reload is allowlisted and changes/rollback retain actors and versions.

## 9. Monitoring, troubleshooting, and acceptance

Use `/api/v1/monitor/overview`, `/monitor`, and `/metrics` with the Prometheus adapter. Observe queues, lost nodes, failures, slice costs, checkpoint overhead, and integrity errors. Timeline replay is audit support, not sufficient mathematical-input reconstruction by itself.

| Symptom | Check first |
|---|---|
| No compatible node | Model type, solverType, online capabilities, free slots |
| Reference cannot be read | Complete payload, tenant prefix, object version, ETag |
| WAITING_FOR_BUDGET | Budget scope, reservations, ledger |
| SOLVER_EXECUTION_FAILED | Bridge command, native dependencies, license, actual output files |
| Recovery failure | Original model/configuration, digest, historical attempt |
| No optimal solution | Solution presence, termination reason, proof status separately |

Before production, test known optima, infeasibility, timeouts with/without solutions, stop/resume, lost nodes, actual database restart recovery, cross-tenant rejection, duplicate requests/slices, corrupted artifacts, and insufficient budget. Clients and server must decode shared protocol fixtures; CP exact integers and stable identities must survive cross-language exchange.

This documentation change did not start databases, Kafka, object storage, or real solvers, nor run environment-level failure drills. Design SLOs are targets, not measured guarantees.

## 10. Sources and further reading

- [Server README](https://github.com/fuookami/ospf/blob/main/framework/remote-solver/README.md)
- [Architecture design](https://github.com/fuookami/ospf/blob/main/framework/remote-solver/design.md)
- [CP protocol field table](https://github.com/fuookami/ospf/blob/main/framework/remote-solver/docs/remote-cp-protocol.md)
- [Database migrations](https://github.com/fuookami/ospf/tree/main/framework/remote-solver/ospf-remote-solver-dispatcher/deploy/sql)
- [Deployment scripts and configuration](https://github.com/fuookami/ospf/tree/main/framework/remote-solver/ospf-remote-solver-dispatcher/deploy)
