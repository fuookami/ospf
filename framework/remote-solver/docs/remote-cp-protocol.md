# Remote CP Protocol Field Table

本文档冻结远程 CP 请求、结果和 artifact 的线格式。客户端仓库中的
`SolvePayload`、`SolveResult` 和 `SerializedSolution` 必须与本表保持同名字段；服务端不定义私有 CP DTO。

## Version

| Field | JSON type | Required | Default | Since |
| --- | --- | --- | --- | --- |
| `schemaVersion` | string | result/artifact required for CP | `"2.0"` | 2.0 |
| `protocolVersions` | string array | capability response required | `[]` | 1.0 |
| `supportedModelTypes` | string array | capability response required | `[]` | 1.0 |

CP 客户端提交前必须探测 `/api/v1/capabilities`，并同时看到协议 `2.0` 和模型类型 `CP`。
旧服务端没有该端点或没有 CP 能力时，客户端在上传 payload 前返回结构化 `INVALID_ARGUMENT`，不会把 CP 当作线性模型提交。

## Request

| JSON field | JSON type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `payloadRef` | string | yes | - | Object path. Server applies tenant scope exactly once. |
| `requestId` | string or null | no | `null` | Idempotency key. |
| `tenantId` | string or null | no | `null` | Must match `X-Tenant-Id` when tenant auth is enabled. |
| `complexity` | enum string or null | no | `null` | `SIMPLE` or `COMPLEX`. |
| `timeSensitivity` | enum string or null | no | `null` | `REALTIME` or `NON_REALTIME`. |
| `priority` | integer | no | `0` | Scheduler priority. |

`payloadRef` 指向序列化 `SolvePayload` artifact，而不是 snapshot 本身。`SolvePayload.modelData.format`
为 `ospf-cp-snapshot-json` 且 `targetType` 为 CP 时才进入 CP calculator。

## SolvePayload

| JSON field | JSON type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `modelData` | object | yes | - | CP uses `rawBytes` plus `format`. |
| `modelData.rawBytes` | integer array | CP | `null` | Kotlin `ByteArray` JSON representation. |
| `modelData.format` | string | CP | `null` | Must equal `ospf-cp-snapshot-json`. |
| `config` | object or null | no | `null` | Inline solver configuration. |
| `taskMeta` | object | no | `{}` | Scheduler fallback limits only; not part of the slice quantum. |
| `extension` | object | no | `{}` | String-to-string extension fields. |

`Int64` 变量、目标和 interval 值使用 JSON number，并必须在 `Long.MIN_VALUE..Long.MAX_VALUE` 范围内。
`Duration` 使用以毫秒为单位的 JSON integer；`Instant` 使用 epoch milliseconds；`ObjectRef` 使用
`path`、可选 `version` 和可选 `etag` 字段。

canonical CP v2 结果 fixture 位于两个仓库的测试资源中，SHA-256 为
`EBAD2593BDD9279BFB1D536F1174A328AB774C93B64102234AB52599B26C8691`；两侧测试必须直接解码该文件。

## External worker

When `OspfExternalProcessBridge` is selected, the command receives the model path and
`--model-format ospf-cp-snapshot-json`, task/slice/node identifiers, `--quantum-ms`, tenant,
optional inline `--config-json`, task-level limits and an optional `--checkpoint-in` path. The
worker must write a `SerializedSolution` JSON file and may write a portable checkpoint file; it
reports their paths with `resultPath=` and `checkpointPath=`. The bridge uploads file contents to
tenant-scoped object storage and never treats a local path string as an artifact.

The repository worker keeps the legacy key/value progress mode only when the CP model format is
absent. That compatibility mode is not a solver result and must not be used for CP tasks.

## SolveResult and SerializedSolution

| JSON field | JSON type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `problemStatus` | enum string or null | CP | `null` | `FEASIBLE`, `INFEASIBLE`, `UNBOUNDED`, `INFEASIBLE_OR_UNBOUNDED`, `UNKNOWN`. |
| `terminationReason` | enum string or null | CP | `null` | `COMPLETED`, `TIME_LIMIT`, `SOLUTION_LIMIT`, `CANCELLED`, `BACKEND_FAILURE`, etc. |
| `solutionPresence` | enum string or null | CP | `null` | `NONE`, `INCUMBENT`, `OPTIMAL`. |
| `proofStatus` | enum string or null | CP | `NONE` | `NONE`, `CLAIMED`, `VERIFIED`. |
| `objectiveValueInt64` | integer or null | CP objective | `null` | Exact objective; never convert through floating point. |
| `variableValuesById` | object<string, integer> | CP solution | `{}` | Stable variable ID to exact Int64 value. |
| `intervalValues` | object<string, object> | CP solution | `{}` | `start`, `size`, `end` are Int64 and `present` is boolean. |
| `provenance` | object<string, string> | no | `{}` | Redacted execution source. |
| `fingerprints` | object<string, string> | CP | `{}` | Model/configuration/solver identities. |
| `fingerprintSchemas` | object<string, string> | CP | `{}` | Schema version per fingerprint key. |
| `statistics` | object<string, string> | no | `{}` | Decimal values are encoded as strings to preserve fractional bounds/gaps. |
| `diagnostics` | object<string, string> | no | `{}` | Structured evidence encoded by the shared report mapping. |
| `runId` | string or null | CP | `null` | Must equal task identity for strict schema 2.0. |
| `attemptId` | string or null | CP | `null` | Must equal slice identity for strict schema 2.0. |
| `artifactDigest` | string or null | CP artifact | `null` | Required for a result referenced by `resultRef`. |

旧线性/二次 DTO 可以继续读取 `feasible`、`optimal` 和浮点 `objectiveValue`。CP 客户端不会从这些兼容字段
推断更强的状态，且 CP 精确目标只消费 `objectiveValueInt64`。

## Compatibility

- 服务端和客户端都必须拒绝未知的未来主版本。
- 缺失摘要、run/attempt、fingerprint schema 或状态冲突的 CP artifact 不得被静默接受。
- 旧协议 fixture 与跨仓库双向 fixture 已有直接解码测试；完整 HTTP/object-storage/dispatcher/calculator
  fixture 仍属于 RS-CP-7 的未完成验收项，本表不是对该集成证据的替代。
