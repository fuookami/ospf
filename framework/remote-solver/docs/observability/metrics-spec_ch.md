# Remote Solver 指标规范

本文档定义了运维监控看板的规范指标、标签和告警语义。

## 适用范围

1. 调度器运行时健康状态
2. 任务执行结果与失败原因
3. 成本与队列压力
4. 镜像双写与重试可靠性信号

## 命名规范

1. Counter（计数器）：后缀 `_total`
2. Gauge（仪表）：无后缀（或 `_ratio`、`_value`）
3. Histogram（直方图）：后缀 `_ms_bucket/_ms_sum/_ms_count`
4. 指标前缀：`remote_solver_`

## 规范指标表

| 指标名 | 类型 | 标签 | 说明 | 原始来源 |
|---|---|---|---|---|
| `remote_solver_task_failed_total` | Counter | `reason` | 按原因分类的失败任务数 | `task.failed` |
| `remote_solver_task_stopped_total` | Counter | 无 | 用户/系统停止的任务数 | `task.stopped` |
| `remote_solver_task_recovered_total` | Counter | `reason` | 故障处理后恢复的任务数 | `task.recovered` |
| `remote_solver_slice_runtime_ms_*` | Histogram | 无 | 切片运行时间分布 | `slice.runtime.ms` |
| `remote_solver_slice_cost` | Gauge | `node_id` | 各节点最新切片成本 | `slice.cost` |
| `remote_solver_node_performance_score` | Gauge | `node_id` | 学习得到的各节点性能评分 | `node.performance.score` |

## 推荐衍生指标

1. 失败率（5分钟窗口）
   - `sum(rate(remote_solver_task_failed_total[5m])) / max(sum(rate(remote_solver_task_terminal_total[5m])), 1)`
2. 预算消耗趋势（15分钟窗口）
   - `sum(rate(remote_solver_slice_cost[15m]))`
3. 切片超时比例（5分钟窗口）
   - `sum(rate(remote_solver_task_failed_total{reason="slice_timeout"}[5m])) / max(sum(rate(remote_solver_task_failed_total[5m])), 1)`

## 必要标签与取值

1. `reason` 标签值（推荐）：
   - `hard_timeout` - 硬超时
   - `slice_timeout` - 切片超时
   - `budget` - 预算超限
   - `solver_execution` - 求解器执行失败
   - `checkpoint_export` - 快照导出失败
   - `node_timeout` - 节点超时
2. `node_id` 必须稳定且每个求解节点唯一。

## 告警映射

1. 高失败率 → 先检查 `reason` 标签分布。
2. 成本突增 → 按 `node_id` 检查 `remote_solver_slice_cost`。
3. 节点不稳定 → 检查 `remote_solver_task_recovered_total{reason="node_timeout"}`。

## 说明

1. 运行时启动默认开启规范指标映射（`metrics.canonical.enabled=true`）。
2. 若关闭规范映射，原始键名（如 `task.failed`、`slice.runtime.ms`）可能直接出现。
3. 生产遥测适配器应导出上述规范名称。