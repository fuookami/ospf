# OSPF Rust Framework

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-framework` 是 OSPF Rust 领域框架共享 framework 层。它提供高层 solver abstraction、pipeline modeling、shadow-price 工具、dynamic model lifecycle helper、persistence expression contract、remote solver client infrastructure、network helper 和 running heartbeat 数据结构。

## 作用范围

本 crate 覆盖：

1. column generation、Benders decomposition、linear/quadratic solving 和 solver combinator 的 solver abstraction。
2. constraint、column generation、heuristic analysis、shadow price 和 dynamic model lifecycle 的 pipeline modeling。
3. persistence expression repository contract 和 feature-gated backend translator。
4. remote solver domain model、port、client、HTTP transport、object storage 和 model serialization。
5. network 工具、running heartbeat 结构和 framework solve options。

明确非目标：

1. cutting plan、packing layer 或 scheduling task 等领域专用建模。
2. 底层 variable/token/symbol/model standard-form 所有权；这些属于 `ospf-rust-core`。
3. solver 安装和许可证管理，除 backend setup notes 外不在本 crate 处理。

## 模块结构

| Rust 模块 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| `model` | `framework/model` | `Pipeline`、`CGPipeline`、`HAPipeline`、shadow-price map、dynamic column state 和 dynamic model lifecycle。 |
| `solver` | `framework/solver` | column generation、Benders、组合求解器、framework solve options、dual solution 和 backend extension wrapper。 |
| `solver::remote` | `framework/solver.remote` | remote solver client、domain model、port、adapter、HTTP task client 和 model serialization。 |
| `persistence` | `framework/persistence` | persistence DTO、repository contract、expression schema、关系查询计划、sort/update descriptor 和 backend feature boundary。 |
| `network` | `framework/network` | HTTP/network helper contract。 |
| `running_heart_beat` | `framework heartbeat` | progress、running 和 finish heartbeat 数据结构。 |

## 架构概览

framework crate 位于 `ospf-rust-core` 与领域 framework crate 之间：

1. `core` 拥有建模原语和 solver-facing standard form。
2. `framework` 把这些原语组合成可复用求解编排、pipeline、shadow-price 和 lifecycle 抽象。
3. CSP1D、BPP3D、Gantt Scheduling 等领域 crate 在 `framework` 之上装配 context / aggregation / model component / pipeline 结构。

application 代码通常应使用统一的 `solve(...)` 和 `solve_with_options(...)` 入口。领域专用约束和目标应通过 pipeline 或 context extension 注册，而不是在 application 层直接拼模型。

## 核心概念

### 共享列投影

`solver::ColumnProjection<V>` 使用 Core 的完整线性快照，不在领域 context 旁边维护第二份
行或目标定义。`ColumnDecision` 显式将原变量映射到块，未映射变量保留为全局变量；
`ProjectedColumn` 必须提供该块全部决策值。`add_columns` / `remove_columns` 管理列池，
`master` 用相同源行和目标重建 LP / final MILP，Phase I 使用人工变量而不是大 M。
LP 列变量只有非负界（不添加干扰行对偶定价的冗余上界）；最终整数模型使用二元选列。

`ColumnMasterRegistration` 私有保存冻结列、实际行身份与结构版本。f64 的受控入口
`solve_master_lp` 将本次实际编译模型交给后端，验证完整最优 LP 报告的证书、指纹和行列
映射，并按编译时保存的源行身份及乘数合并等式拆行和翻号对偶。不提供公开裸价格绑定。
每次成功求解签发新 dual epoch；同一模型结构不变而重求，也会拒绝旧请求。失败求解
清除旧价格。原数值域快照不意味着可以转移 f64 证明：目前精确定价求解门禁仅限同域
f64，其他数值域可以建模和 seed，但未获得受控证据时不能声明定价收敛。
`pricing_request` 从同源行生成 c-πa，
`pricing_model` 仅复制完全属于当前块的局部行；跨块及全局行通过对偶参与目标。
无对偶 seed 使用 `None`，不能据此声明定价收敛。通过 `add_priced_batch` 在所有块
使用同一轮对偶完成定价后一次加列；不应逐块加列后继续使用旧请求。
所有候选列先按冻结版本完整校验，再一次提交列池与状态；后续块或最后一列失败不残留
前面的列，也不使失败前合法注册失效。

源模型、列池或实际主问题修改都会在 `require_current` / `require_pricing_current`
和定价消费边界被拒绝。新增局部行或收紧范围使旧列不可行时，主问题重建显式报错，
要求删列或重新定价。独立补全的资源从列的行/目标贡献中排除，改为全局变量计一次，
整数补全模式恢复资源的原整数类型。
`extract_solution` 要求无人工变量的整数主问题，按 VariableId 合并并复核完整主问题和
原始展开模型；领域仍需独立验证。不实现联合 CG/Benders、割有效性证明、原生增量矩阵
或 Branch-and-Price，也不把根 LP 收敛当作整数最优证书。

1. `Pipeline<M>` 为模型注册并执行一个约束或目标族。
2. `CGPipeline<Args, Model, Map>` 在 pipeline 基础上增加 shadow-price refresh 和 reduced-cost extraction。
3. `ShadowPriceKey`、`ShadowPrice` 和 `ShadowPriceMap` 为 column generation 提供稳定 dual mapping。
4. `DynamicModelLifecycle` 跟踪 warm start、hidden/fixed/removed column、solver solution 和 column range restoration。
5. `FrameworkSolveOptions` 集中 solve name、logging、solution amount、conversion policy、callback 和算法调参选项。

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `Pipeline`、`PipelineList` | constraint/objective pipeline execution。 | stable within migration |
| `CGPipeline`、`BasicShadowPriceMap`、`ShadowPriceMap` | column-generation shadow-price lifecycle。 | stable within migration |
| `DynamicModelLifecycle`、`DynamicColumnContext`、`ColumnState` | dynamic column lifecycle 和 warm-start state。 | migration |
| `ColumnGenerationSolver` | framework column-generation solver trait。 | migration |
| `LinearBendersDecompositionSolver`、`QuadraticBendersDecompositionSolver` | Benders decomposition solver trait。 | migration |
| `LogicBasedBendersEngine` | 线性 master 与 CP 子问题的 Logic-Based Benders 编排。 | migration |
| `SerialCombinatorial*`、`ParallelCombinatorial*` | solver combinator。 | migration |
| `FrameworkSolveOptions` | 统一 solve options。 | migration |
| `RemoteSolverClient`、`RemoteLinearSolver`、`RemoteQuadraticSolver` | feature-gated remote solver 编排。 | migration |
| `ExpressionRepository`、`RepositoryQuery`、`SortBy`、`UpdateAssignments` | persistence expression contract。 | migration |
| `RelationalQueryPlan`、`JoinSpec`、`ProjectionSpec` | 与数据库无关的关系查询计划、校验和规范化审计摘要。 | migration |
| `DiagnosticPersistenceFieldResolver`、`PersistenceFieldResolution` | 保留字段缺失、歧义和非法配置诊断。 | migration |
| `SqlxRelationalQueryCompiler` | 将白名单关系计划编译为参数化 SQLx SQL，并提供执行统计。 | feature-gated |

## 建模扩展点

framework-level 扩展应表达为：

1. `Pipeline`：普通约束/目标注册。
2. `CGPipeline`：shadow-price-aware 约束和 reduced-cost extraction。
3. `HAPipeline`：heuristic analysis 和 validation。
4. `DynamicModelLifecycle`：column hiding、fixing、removal、restoration 和 warm start。
5. solver wrapper 或 combinator：backend selection 和 fallback。

领域 crate 应暴露自身 context / aggregation / pipeline 扩展点，并使用 framework 抽象，而不是把领域逻辑加到 application solver 中。

## Logic-Based Benders

`LogicBasedBendersEngine` 通过 `LogicBasedBendersMaster` 和
`ConstraintProgrammingSubproblemSolver` 组合线性 master 与 constraint-programming 子问题。
master 返回带稳定变量 ID 的已验证 `SolveReport<f64>`；子问题接收对应的 `MasterAssignment`，并返回
`ConstraintProgrammingSubproblemResult`。

当结果必须携带全局证明时使用 `LogicBasedBendersMode::Exact`。Exact 模式要求由 snapshot 复验的
CP 终态、全局有效且已验证的 cut、一致的 objective evaluator，以及已验证的 master 最优解。
`LogicBasedBendersMode::Heuristic` 可以在证明条件不完整时保留 incumbent，但会清除最优性证明，
并在 report diagnostics 中记录降级原因。

`DefaultFeasibilityCutOracle` 会生成 assignment no-good cut：二值绑定使用
`binary_assignment_no_good`，有限整数绑定使用 `bounded_integer_assignment_no_good`；后者返回包含
完整辅助变量声明的 cut family。实现 `LogicBasedBendersCutOracle` 可以追加 conflict 或 optimality
cut，并通过 `SolveReport.trace.iteration_snapshots` 检查逐轮诊断。启用可选 `serde` feature 后，
还可以使用 portable checkpoint capture/resume API。

## 泛型数值边界

framework Meta 入口接受 `MetaModel<V>`，并通过 `SolveValueConversionPolicy` 转换到后端数值域：

- `Strict`：拒绝有损转换。
- `AllowRounding`：允许受控舍入到 `f64`。

当前值类型支持 `f64`、通过 `big-rational` 启用的 `BigRational`，以及通过 `big-decimal` 启用的 `BigDecimal`。转换失败必须显式暴露，不应静默降精度。

## 求解生命周期

常见 framework 使用方式：

1. domain context 注册 `MetaModel`。
2. application service 选择 solver 或 solver combinator。
3. `FrameworkSolveOptions` 携带模型外求解选项。
4. solver trait 调用 core model lowering 和 backend adapter。
5. LP dual、Benders sub-result 或 column-generation shadow price 通过 framework result type 返回。
6. dynamic lifecycle state 可在支持时写回 `MetaModel`，供 final MILP 求解消费。

## 使用方式

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};

fn run_column_generation<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let _result = solver.solve(meta_model)?;

    let options = FrameworkSolveOptions::new()
        .with_name("cg_case")
        .with_log_model(true);
    let _result_with_options = solver.solve_with_options(meta_model, options)?;
    Ok(())
}
```

相同模式也适用于线性/二次 MetaModel solver extension 和线性/二次 Benders solver 入口。

## Feature Flags

- `async`：启用 solver operation 的 async/await 支持。
- `nightly`：透传 `ospf-rust-core` 的 nightly callable 支持。
- `big-rational`：启用 `BigRational` `MetaModel` value type 支持。
- `big-decimal`：启用 `BigDecimal` `MetaModel` value type 支持。
- `gurobi`、`gurobi10`、`gurobi11`、`gurobi12`：通过 core 启用 Gurobi backend。
- `scip`、`scip-bundled`、`scip-from-source`、`scip-quadratic`：通过 core 启用 SCIP backend。
- `persistence*`：启用 persistence contract 和 backend-specific translator。
- `remote-solver`：启用 async remote solver domain model、port、client orchestration、local object storage 和 model serializer。
- `remote-solver-http-reqwest`：启用基于 reqwest 的 HTTP transport。

## Remote Solver 边界

共享的 report、proof、取消合同见 [core 统一求解合同](../ospf-rust-core/README_ch.md#统一求解合同)。

Remote checkpoint recovery 保留旧的 `CheckpointResumeExpectation` 和 `validate_resume` 兼容投影。
精确恢复必须使用 `CheckpointResumeExpectationWithAttempt`、`load_checkpoint_artifact_from` 和
`validate_resume_from`，这些入口会校验源 attempt、调用方期望的 parent、solver provenance
与取消链。携带 checkpoint 的版本化报告还必须匹配 model/configuration/solver fingerprint
和 provenance，不能只匹配 run 与 attempt。Backend 证据遵循
[native 矩阵](../ospf-rust-core/README_ch.md#solver-原生验收矩阵)，Kotlin source 覆盖见
[traceability 表](../ospf-rust-core/README_ch.md#source-追踪)。

remote solver 支持遵循 Cargo feature，不按 Maven-style backend module 拆分。公共 feature 提供 `RemoteSolverClient`、`RemoteLinearSolver`、`RemoteQuadraticSolver`、async `SolverExecutionPort` 与 `ObjectStoragePort`、`RemoteSolverHttpClient`、`LocalFileObjectStoragePort` 和 `OspfRemoteModelSerializer`。

在 current-thread Tokio runtime 内，建议使用 `solve_remote(...)` / `solve_remote_with_options(...)`，因为同步 trait 入口会主动拒绝阻塞该 runtime flavor。需要读取终态语义的新代码应使用 `solve_remote_report(...)` 或 core 的 `solve_linear_report` / `solve_quadratic_report` 入口。版本化 remote report 保留 core `SolveReport`、run/attempt identity 和 artifact digest；旧 `SolveResult` 与 `SerializedSolution` 仍支持兼容读取，但未知 report schema version 会被拒绝。

## Persistence 边界

persistence backend 遵循 Cargo feature。公共层包含 `ExpressionRepository`、`RepositoryQuery`、`SortBy`、`UpdateAssignments`、`PredicateSchema`、`FieldPath` 和 request/response DTO/record 类型。

`RelationalQueryPlan` 是数据源、别名、Join、谓词、投影、分组、排序、分页和可选根键的数据库无关计划边界。计划会在边界递归快照拥有型表达式树和动态符号元数据，在编译前校验 Join 关联，并提供 `canonical()` 与 SHA-256 `canonical_hash()` 审计摘要。canonical 会规范化嵌套布尔表达式和成员候选集合，同时保留字面量类型形状而省略字面量原值。计划有意不包含权限、预算、物理表名、数据库连接和行映射类型。

仓储合同与后端无关：`find` 使用默认 `RepositoryQuery` 选项委托给 `find_with_options`，`count`、`update` 和 `delete` 返回匹配或受影响行数 `u64`，`exists` 委托给 `count`。`eq`、`and_scope`、`in_values`、`is_null` 等 Boolean DSL helper 生成 `BooleanExpression<ExpressionValue>`，可直接传给仓储方法，也可通过 `RelationalQueryPlan::with_predicate` 进入关系计划编译，或交给兼容的 SQLx statement builder。`SortBy` 与 `UpdateAssignments::{set, set_null, set_expr}` 分别提供可组合的排序/分页和更新描述。

当适配器需要区分字段缺失、映射歧义和非法配置时，应实现 `DiagnosticPersistenceFieldResolver`；`PredicateSchema<String>` 已为注册的字符串字段映射提供该诊断行为。

后端边界：

1. SQLx 通过 `SqlxRelationalQueryCompiler` 构建参数化 SQL 语句，支持 `Inner`、`Left` 和相关 `Exists` Join；隐式投影必须使用显式字段白名单，一对多关系的 `COUNT(DISTINCT root_key)` 保持根粒度。`compile_count` 保留兼容的 `SqlxSql` 结果，`compile_count_typed` 则保留参数类型及顺序，供审计或适配器进行类型感知绑定。
2. SeaORM 把表达式翻译成 SeaQuery/SeaORM 类型，并提供 async repository adapter。
3. Rbatis 在 Rbatis-facing 名称下复用参数化 SQL builder。
4. Diesel 和 Toasty 通过 planning helper 保持 typed ORM 边界。
5. Cornucopia 绑定生成查询函数名。
6. MongoDB 和 Redis 提供 JSON/document 与命令 helper，不要求具体 client 类型。

SQLx 编译器返回 `SqlxCompiledQuery`，其中包含 SQL 模板、参数值、参数类型摘要、方言和复制后的计划。`execute_with` 有意接收外部执行器，因为连接所有权和行映射属于应用或 SQLx adapter；该方法只补充返回行数、耗时和截断统计。需要结构化执行分类的 adapter 应使用 `execute_with_classifier`，显式把执行器错误映射为 `Timeout`、`UnsupportedDialect` 或 `Database`；通用层不会根据错误文本推断分类。未知数据源、未知或歧义字段、非法 Join、不支持的谓词和错误白名单均以结构化 `RelationalQueryFailure` 返回。

## Solver Backend 说明

### 固定连续追索

`LinearBendersPartition::build` 划分完整展开的 `MetaModel<f64>`，不平行手写另一套模型。
全部整数／辅助整数留在 master；零成本连续变量默认为可行性追索；显式目标追索需要由
真实变量边界推导有限 theta 下界。追索矩阵不可变，RHS 对 master 决策为仿射关系。追索
边界全部作为显式行，子问题变量自由，切割因此包含完整边界贡献。

`FixedRecourseAlgorithm` 控制当前 master／sub 求解，只从实际 LP 独立验证的原始／最优
对偶证据或 Farkas 证据私有生成割。缺少数值证据时可在同一总时限内求显式证书模型，
已有但无效的证据不被静默替换。实际报告映射／指纹、完整源可行性、源版本、目标常数
及有证据界均被检查。结果保留实际 Core 报告，区分证书失败、数学终态和执行终态，
不隐式跨数值域转换证明。应用层回退必须采用全新模型且仅用剩余预算；共享 BPP 应用
已提供此边界。
切割系数／常数精确累加，舍入误差按真实 master 边界补偿，RHS 只向外舍入。无法约束
的舍入误差方向以 Unsupported 停止，不授权不安全切割，并保留已有已验证源方案。
theta box 下界、固定 anchor 后的 RHS 和源目标也使用精确二进制 f64 有理运算，下界向下
舍入、候选解目标向上舍入。RHS 无法精确表示为有限 f64 时，在求解不同系统前以
Unsupported 停止。

framework backend adapter 只有在 `ospf-rust-framework` 上启用对应 feature 时才会编译，并且该 feature 会透传到 `ospf-rust-core`。

framework 扩展入口：

1. `src/solver/gurobi_extension.rs`
2. `src/solver/scip_extension.rs`

core backend 准备说明：

1. Gurobi: [`../ospf-rust-core/src/solver/solvers/gurobi/README_ch.md`](../ospf-rust-core/src/solver/solvers/gurobi/README_ch.md)
2. SCIP: [`../ospf-rust-core/src/solver/solvers/scip/README_ch.md`](../ospf-rust-core/src/solver/solvers/scip/README_ch.md)

对于 LP 和可线性化子问题，framework 优先使用后端返回且一致的 row dual/Farkas certificate。对于真实二次子问题，后端对完整 dual/Farkas certificate 的支持有限，因此可能返回结果但无法生成完整 quadratic Benders cut。

## 本地验证

```powershell
cargo check -p ospf-rust-framework
cargo test -p ospf-rust-framework --no-run
cargo test -p ospf-rust-framework --lib
cargo check -p ospf-rust-framework --features async
cargo test -p ospf-rust-framework --features remote-solver remote
cargo check -p ospf-rust-framework --features "remote-solver remote-solver-http-reqwest"
```

backend feature 示例：

```powershell
cargo test -p ospf-rust-framework --features gurobi10
cargo test -p ospf-rust-framework --features scip
cargo test -p ospf-rust-framework --features "scip-bundled async"
```

## 依赖

- `ospf-rust-core`
- `ospf-rust-base`
- `parking_lot`
- `log`
- `async-trait` 可选
- `tokio`、`serde`、`serde_json`、`reqwest` 和 `sea-orm` 按 feature 可选；`sha2` 用于查询审计摘要并为必需依赖

## 相关模块

- [根 README](../README_ch.md)
- [Core README](../ospf-rust-core/README_ch.md)
- [Kotlin framework README](../../ospf-kotlin/ospf-kotlin-framework/README_ch.md)

## 许可证

本项目与主 OSPF Rust 项目使用相同许可证。
