# 远端求解

远端求解将模型构建与实际执行分离：应用提交版本化模型载荷，调度服务根据节点能力、容量、时限与预算分配任务，由执行桥接器调用实际求解后端，再返回结构化报告和结果对象。它可以部署在自有服务器或云环境，不专指某种托管云产品。

本页介绍仓库中的 [remote-solver 框架](https://github.com/fuookami/ospf/tree/main/framework/remote-solver)。配置与协议以部署版本为准；本文区分接口能力、模拟链路与真实后端，不把调度 smoke test 当作数学求解验收。

## 1. 何时使用

- 应用机器不适合安装求解器或承担长时间计算。
- 多个任务共享有不同性能、价格、许可证和并发容量的节点。
- 需要集中管理排队、预算、停止/恢复、执行记录与结果对象。
- 复杂非实时任务希望通过时间片执行与可移植恢复点改善资源共享。

远端执行不改变原问题的数学定义，也不自动提高最优性或保证硬实时 SLA。模型可执行性仍取决于实际后端能力。复杂实时任务在当前设计中按复杂非实时策略处理。

## 2. 模块与信任边界

| 组件 | 职责 |
|---|---|
| Kotlin / Rust 客户端 | 能力协商、载荷提交、状态与结果查询、控制和结果映射 |
| protocol | 共享线格式、任务/切片身份、对象引用和执行端口 |
| dispatcher | HTTP API、准入、排队、节点选择、预算、事件与状态管理 |
| calculator | 载荷重建、OSPF 执行适配、checkpoint 编排 |
| external worker | 在独立进程中执行所配置的求解路径，写出结果文件 |
| 状态存储与对象存储 | 分别保存任务/切片事实，以及模型、结果和恢复点内容 |

```text
应用模型 → Kotlin/Rust 客户端 → HTTP dispatcher
                                    │ 准入、能力、预算、调度
                                    ↓
                              calculator / bridge
                                    ↓
                             external worker → OSPF 后端
                                    ↓
               校验结果文件 → 上传对象存储 → 持久化结果引用
                                    ↓
                               客户端结果映射
```

服务端模块依赖为 `dispatcher → calculator → protocol`，dispatcher 也直接依赖 protocol。客户端位于各语言的 framework 组件，不在本服务端工程内维护。状态存储是当前状态的权威来源，事件用于分发、审计和时间线，不等于系统已经完全采用事件溯源。

bridge 启动的命令是执行边界，不是任意远端 shell 协议。部署方负责命令可执行、路径可访问和所需网络/进程拓扑；不能只登记一个节点名就认为机器已自动安装和启动。

## 3. 模型载荷与能力协商

先读取 `GET /api/v1/capabilities`，确认所需协议、模型类型和 checkpoint 能力，再上传大对象或提交任务。支持的类型是在线节点能力的聚合，不是“所有后端均支持全部模型”。

| 模型 | 载荷 | 注意事项 |
|---|---|---|
| 线性 | `SerializedLinearModel` 或受控对象引用 | 保留变量域、系数及稳定身份 |
| 二次 | `SerializedQuadraticModel` 或受控对象引用 | 需匹配二次模型和求解器能力 |
| CP | `ModelData.rawBytes`，`format=ospf-cp-snapshot-json`，目标 `cp` | 先确认 protocol `2.0` 与 CP 能力；当前真实 worker 路径使用 SCIP CP |

`SolvePayload` 还包含配置或配置引用、snapshot 引用、任务元数据与扩展信息。推荐用 `taskMeta.solverType` 声明要求的求解器。模型类型匹配、求解器兼容、空闲槽位和预算是分别检查的条件。

提交请求中的 `payloadRef` 指向**完整序列化 SolvePayload**，不是裸 LP 文件、CP snapshot 或客户端本地路径。模型数据可由该载荷进一步引用对象。上传与引用必须使用同一部署对象存储和租户规则；本页不虚构通用上传 HTTP 端点。

`ObjectRef(path,version,etag)` 标识对象及其版本/完整性信息。恢复和数据库重建不能只保留 path。模型、变量、约束、目标的稳定 identity 也不能改用显示名称或注册顺序。

## 4. 从提交到结果

1. 客户端冻结模型、配置、身份与任务要求，存储完整载荷。
2. dispatcher 解析引用并校验 schema、租户、模型与配置。
3. 任务入队；调度器选择兼容节点并完成分发确认。
4. calculator 重建 OSPF 模型或调用受控 worker。
5. 切片结束后校验结果/checkpoint，上传可信内容并持久化引用。
6. 客户端读取任务报告，按需取回结果 artifact，映射回模型身份。

### 4.1 HTTP 示例

以下 PowerShell 示例只演示请求顺序。执行前必须已启动服务，并通过部署提供的对象存储适配器写入 `models/production-payload.json` 对应的合法 SolvePayload；它不是开箱即用的示例对象。

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

这里只查询一次状态。实际客户端需有限轮询、超时和取消处理，不能把提交成功当成求解完成。部署启用身份认证时，应补充其要求的认证凭证；`X-Tenant-Id` 本身不是身份认证。

| 操作 | HTTP 接口 |
|---|---|
| 能力 | `GET /api/v1/capabilities` |
| 提交/查询 | `POST /api/v1/tasks`、`GET /api/v1/tasks/{taskId}` |
| 停止/恢复 | `POST /api/v1/tasks/{taskId}/stop`、`POST /api/v1/tasks/{taskId}/resume` |
| 时间线 | `GET /api/v1/tasks/{taskId}/timeline?limit=200` |
| 健康 | `GET /health`、`GET /health/ready`、`GET /health/live` |

停止请求体示例是 `{"reason":"manual-stop"}`，恢复请求体是 `{}`。是否允许操作取决于任务状态和权限，不能对任意终态直接恢复。

### 4.2 客户端入口

Kotlin 的 `RemoteSolverClient` 负责基础远端流程，`RemoteLinearSolver`、`RemoteQuadraticSolver` 提供建模侧封装。Rust 在 `ospf-rust-framework` 的 `solver/remote` 下提供客户端实现。它们共享协议语义，但不假设构造参数或错误类型完全相同。

- [Kotlin 远端客户端源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework/src/main/fuookami/ospf/kotlin/framework/solver/remote)
- [Rust 远端客户端源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-framework/src/solver/remote)

业务应用不需要依赖整个 dispatcher。服务端 `RemoteSolverBootstrapFactory` 和 `runtime.apiFacade` 是嵌入服务的入口，不是 HTTP 客户端，不能拿它们代替跨机器通信。

## 5. 调度、预算与任务状态

简单任务优先分配空闲且兼容的节点，在成本与预计时限间权衡；复杂任务按时间片轮转，保留执行与恢复记录。该选择是调度策略，不是对全局最小费用的数学最优性证明。

正常任务大致经历：

```text
QUEUED → ACCEPTED → DISPATCHING → RUNNING → COMPLETED
                                      └→ SUSPENDED → ACCEPTED
QUEUED / ACCEPTED → WAITING_FOR_BUDGET → QUEUED
可控制状态 → STOPPING → STOPPED
调度或执行失败 → FAILED
```

HTTP 正常受理直接建立 QUEUED 任务；CREATED 是兼容/瞬时状态。停止后恢复支持 `STOPPED → QUEUED|SUSPENDED`，无合法恢复条件则返回状态错误。

每个切片记录实际耗时、节点价格、计费粒度与许可证费用。预算不足可能等待或按策略降级，不能因此放宽业务数学约束。任务级时间限制、切片 quantum、dispatcher 超时和心跳超时有不同作用，不能用一个参数代替全部。

## 6. checkpoint 与故障恢复

**Portable checkpoint** 保存可移植模型/输入、配置身份、incumbent 与完整性信息。下一轮重建模型并尝试 warm start，不承诺保留原生搜索树、线程状态或进程句柄。当前服务区分 `supportsPortableCheckpoint=true` 与 `supportsNativeCheckpoint=false`。

不支持立即中断的后端通过时间限制或阶段性返回交还控制；stop 不是保证立即杀死进程。CP 的恢复属于 rebuild-based 路径，不宣称 OR-Tools、原生 optional interval 或原生搜索树恢复。

节点失联时回收容量、记录切片失败；有可信 checkpoint 则恢复，无 checkpoint 则按策略从稳定输入重跑或失败。dispatcher 重启恢复依赖持久化任务/切片数据，默认内存模式不能提供跨进程重启恢复。

恢复前核对租户、task、历史 attempt、模型/配置指纹、schema 和摘要。损坏或身份不匹配的恢复点必须拒绝，不退回未经校验的文件。

## 7. 正确解释结果

任务 `COMPLETED` 表示执行生命周期完成，不自动等于“已证明最优”。分别读取：

- `problemStatus`：数学问题结论；
- `terminationReason`：停止原因；
- `solutionPresence`：是否存在可用解；
- `proofStatus`：证明状态；
- provenance、指纹、run/attempt 身份，以及结果引用。

超时有可行解与超时无解不同，超时无解不等于不可行。兼容字段 `feasible/optimal` 不足以表达全部信息。CP v2 使用 `objectiveValueInt64` 和按稳定变量 ID 保存的精确整数值；interval 包含 start/size/end/present，不应经浮点往返损失精度。

严格结果的 `resultRef` 与 `artifactDigest` 成对出现，并核对 run/attempt、指纹和 schema。worker 的退出码、stdout 或本地文件名不能独立作为可信结果；bridge 读取、校验、上传成功后才形成远端对象引用。

## 8. 部署与配置

### 8.1 开发联调与真实求解

默认端口多为 `inmemory`，包括模拟执行器；用于单进程功能联调，数据随进程退出丢失。仅设置 `solver-execution.adapter=ospf` 也不能替代正确配置的真实执行 bridge。

内置 worker 带 `--model-format ospf-cp-snapshot-json` 时走 CP snapshot 重建和 SCIP CP 路径。未提供该格式时保留非 CP 的兼容进度协议，**不是实际线性/二次求解结果**。线性/二次任务需接通对应真实执行后端；示意 objective 输出不能用于验收。

### 8.2 启动步骤

1. 按当前工程 POM 和求解器依赖准备匹配的 JDK、构建环境、原生库与许可证，不仅检查 Java 命令是否存在。
2. 选择状态、事件、对象存储适配器；跨进程部署不要混用互不可见的内存对象存储。
3. JDBC 部署备份数据库后按 V1→V7 顺序迁移；CP2 恢复依赖 V5 能力/报告、V6 内联配置、V7 ETag 持久化。
4. 配置实际 bridge 命令、worker 文件访问和节点能力；不按求解器名称猜测支持的模型类型。
5. 启动 API 与调度循环，完成节点注册、心跳和端到端真实模型验收。

在服务端的 `ospf-remote-solver-dispatcher` 目录下，可参考以下脚本入口；请先根据环境编辑配置，不要直接把示例连接信息用于生产：

```powershell
.\deploy\scripts\start-api.ps1 `
    -ConfigPath deploy/config/scheduler-ktorm.properties `
    -Host 127.0.0.1 -Port 18080
```

配置来源支持 `--config`、`REMOTE_SOLVER_CONFIG` 与默认 `deploy/config/scheduler.properties`。生产常用选择如下，具体连接、凭证和命令必须补齐：

| 端口 | 生产选择 | 说明 |
|---|---|---|
| event | kafka | 跨进程消息 |
| task-state / node-state / budget / cost-ledger / distributed-lock | ktorm | 持久化与并发协调 |
| storage | s3 | S3/MinIO 模型、结果与 checkpoint |
| scheduler.audit | ktorm | 配置审计 |
| metrics | prometheus | 指标抓取 |
| solver-execution | ospf + 实际 bridge | 真实求解接入 |

生产启用认证、租户作用域与安全传输；监控接口的用户/角色头应由可信认证入口提供，不能允许公网调用者自行伪造。密钥不写入源码或示例文档。配置热更新仅限白名单，变更和回滚保留操作者及版本。

## 9. 监控、排错与验收

`/api/v1/monitor/overview`、`/monitor` 提供监控视图；配置 Prometheus 适配器后使用 `/metrics`。重点观察排队、节点失联、任务失败、切片成本、checkpoint 开销和协议完整性错误。时间线回放用于审计，不保证仅凭事件就能重建数学输入。

| 问题 | 优先检查 |
|---|---|
| 无兼容节点 | 模型类型、solverType、在线能力与空闲槽位 |
| 引用读取失败 | 完整 SolvePayload 是否存在、租户前缀、对象版本与 ETag |
| WAITING_FOR_BUDGET | 预算范围、成本预留与账本 |
| SOLVER_EXECUTION_FAILED | bridge 命令、原生依赖、许可证、实际输出文件 |
| 恢复失败 | 原始模型/配置、checkpoint 摘要与历史 attempt |
| 查询结果没有最优解 | 分别查看解存在性、终止原因和证明状态 |

上线前至少验收：已知最优模型、不可行模型、超时有解/无解、停止/恢复、节点失联、真实数据库重启恢复、跨租户拒绝、重复请求和切片、损坏 artifact、预算不足。客户端、protocol 与服务端使用一致 fixture，CP 精确整数和稳定身份必须跨语言保持。

本次页面编写未启动数据库、Kafka、对象存储或真实求解器，也未执行环境级故障演练。设计文档中的 SLO 是目标，不应当作已测量承诺。

## 10. 源码与进一步阅读

- [服务端 README](https://github.com/fuookami/ospf/blob/main/framework/remote-solver/README.md)
- [架构设计](https://github.com/fuookami/ospf/blob/main/framework/remote-solver/design.md)
- [CP 协议字段](https://github.com/fuookami/ospf/blob/main/framework/remote-solver/docs/remote-cp-protocol.md)
- [数据库迁移](https://github.com/fuookami/ospf/tree/main/framework/remote-solver/ospf-remote-solver-dispatcher/deploy/sql)
- [部署脚本与配置](https://github.com/fuookami/ospf/tree/main/framework/remote-solver/ospf-remote-solver-dispatcher/deploy)
