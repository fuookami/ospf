# Remote Solver Metrics Spec

This document defines canonical metrics, labels, and alert semantics for operations dashboards.

## Scope

1. Scheduler runtime health
2. Task execution outcomes and failure reasons
3. Cost and queue pressure
4. Mirror-write and retry reliability signals

## Naming Convention

1. Counter: suffix `_total`
2. Gauge: no suffix (or `_ratio`, `_value`)
3. Duration histogram: suffix `_ms_bucket/_ms_sum/_ms_count`
4. Metric prefix: `remote_solver_`

## Canonical Metrics

| Metric | Type | Labels | Description | Current Source |
|---|---|---|---|---|
| `remote_solver_task_failed_total` | Counter | `reason` | Failed tasks by reason code category | `task.failed` |
| `remote_solver_task_stopped_total` | Counter | none | User/system stopped tasks | `task.stopped` |
| `remote_solver_task_recovered_total` | Counter | `reason` | Recovered tasks after fault handling | `task.recovered` |
| `remote_solver_slice_runtime_ms_*` | Histogram | none | Slice runtime distribution | `slice.runtime.ms` |
| `remote_solver_slice_cost` | Gauge | `node_id` | Latest slice cost by node | `slice.cost` |
| `remote_solver_node_performance_score` | Gauge | `node_id` | Learned performance score by node | `node.performance.score` |

## Recommended Derived Metrics

1. Failure rate (5m)
   - `sum(rate(remote_solver_task_failed_total[5m])) / max(sum(rate(remote_solver_task_terminal_total[5m])), 1)`
2. Budget burn trend (15m)
   - `sum(rate(remote_solver_slice_cost[15m]))`
3. Slice timeout ratio (5m)
   - `sum(rate(remote_solver_task_failed_total{reason="slice_timeout"}[5m])) / max(sum(rate(remote_solver_task_failed_total[5m])), 1)`

## Required Labels and Values

1. `reason` values (recommended):
   - `hard_timeout`
   - `slice_timeout`
   - `budget`
   - `solver_execution`
   - `checkpoint_export`
   - `node_timeout`
2. `node_id` must be stable and unique per solver node.

## Alert Mapping

1. High failure rate -> check `reason` split first.
2. Cost spike -> inspect `remote_solver_slice_cost` by `node_id`.
3. Node instability -> inspect `remote_solver_task_recovered_total{reason="node_timeout"}`.

## Notes

1. Runtime bootstrap enables canonical metric mapping by default (`metrics.canonical.enabled=true`).
2. If canonical mapping is disabled, raw keys (`task.failed`, `slice.runtime.ms`, etc.) may appear.
3. Production telemetry adapters should export canonical names above.
