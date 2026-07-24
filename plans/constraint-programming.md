# 约束规划与 Logic-Based Benders 支持计划

## 1. 文档状态

| 项目 | 内容 |
| --- | --- |
| 状态 | Implemented with declared capability boundaries |
| 日期 | 2026-08-01 |
| 范围 | `ospf-kotlin-core`、`ospf-kotlin-core-plugin`、`ospf-kotlin-framework`、`ospf-kotlin-example` |
| 求解实现 | core 提供通用 CP 抽象；SCIP 提供首个原生实现；Gurobi 支持可精确降为 MIP 的子集 |
| 首个 Benders 形态 | 线性整数主问题 + CP 子问题 |
| 兼容策略 | 保留现有线性/二次模型、经典 Benders API 和现有 IIS 计算作为降级分支，新增平行能力 |

本文档定义 OSPF 对约束规划（Constraint Programming，CP）的目标架构和实施顺序，覆盖两种使用方式：

1. 在 core 中直接构造并求解 CP 模型。
2. 在 framework 中把 CP 模型作为 Logic-Based Benders 子问题。

## 2. 背景与现状

当前 OSPF 的高层建模以 `MetaModel` 为轴心，主要覆盖线性和二次模型：

- `LinearMetaModel` 负责线性约束和线性目标。
- `QuadraticMetaModel` 负责二次约束和二次目标。
- `LinearMechanismModel` / `QuadraticMechanismModel` 负责展开。
- `LinearTriadModel` / `QuadraticTetradModel` 负责向求解器插件传输稀疏模型。

当前 Benders 接口位于 framework，分为：

- `LinearBendersDecompositionSolver`
- `QuadraticBendersDecompositionSolver`

两者均以对偶解或 Farkas 证书生成 cut。这一机制不适用于一般 CP 子问题，因为 CP 求解器通常不提供能直接构造经典 Benders cut 的 LP 对偶信息。

因此，本计划采用 Logic-Based Benders：CP 后端提供子问题结论和可验证证据，领域 cut oracle 负责把证据转换为主问题 cut。

### 当前实现状态（2026-08-01）

- CP0～CP4 的 core AST、snapshot、fake solver、SCIP solver、精确 MIP lowerer、结构化不可行诊断、Logic-Based Benders、示例 pipeline 和测试已经落地。
- CP5 已提供整数 master no-good 编码、optional interval/variable duration 的精确 MIP-backed 支持、portable snapshot serialization 和 checkpoint 重建评估。
- optional interval 与 variable duration 尚未声明为 SCIP native 能力；SCIP native 路径仍限制为固定 duration interval。Circuit、Automaton、Reservoir 已有 AST、求值和 snapshot 支持，但当前 SCIP/MIP 编译器对其返回结构化 Unsupported。
- native IIS/Farkas 通过 solver capability 和 analyzer SPI 接入：Gurobi 提供 LP/MIP/QP 原生 IIS，Gurobi 与 SCIP 提供连续 LP Farkas；未声明能力的 backend 使用带来源和证据等级的 legacy fallback。
- 全量验收使用 `mvn clean compile test-compile -T 0.75C`、`mvn test -T 0.75C` 和插件 `mvn verify`；Surefire 报告汇总为 0 failures、0 errors、6 skipped（Maven 模块汇总会重复计数，不以日志中的总测试数作为唯一统计口径），Failsafe 集成测试为 9 tests、0 failures。

当前 `core/solver/iis` 通过弹性模型和删除过滤定位不可行元素。删除过滤保留的是必须继续放开才能恢复可行性的 slack，语义更接近最小修复集（Minimal Correction Set，MCS），不能普遍保证返回的原始约束与变量界自身仍然不可行，也不能普遍保证 IIS 的不可约性。该实现继续保留，但在统一报告中只能按实际保证标记为 `ElasticFilter` / `DeletionFilter` 的启发式或未知证据。

## 3. 核心架构决策

### 3.1 CP 是独立的一等模型族

新增 `ConstraintProgrammingModel`，不把 CP 全局约束伪装成线性或二次多项式，也不要求所有 CP 模型先线性化。

`ConstraintProgrammingModel` 不直接继承现有 `MetaModel`。原因是 `MetaModel` 的约束集合和目标集合当前绑定数学多项式语义，强行继承会导致空约束列表、旁路目标和不完整导出等错误契约。

CP 模型应复用以下公共能力：

- OSPF 变量及其稳定标识。
- 变量注册和解回填。
- `ObjectCategory`。
- 约束分组。
- `Try` / `Ret<T>` 错误传播。
- 求解进度和统一报告。

新增可选接口 `ConstraintGroupRegistry`，由 `MetaModel` 和 `ConstraintProgrammingModel` 实现。framework 的 `Pipeline.register` 通过该接口注册约束组，不再只识别 `MetaModel`。

### 3.2 CP AST 使用精确整数语义

首版 CP 模型采用整数和布尔值域。整数系数、上下界、interval 时间点和容量统一使用 `Int64` 或可无损转换为 `Int64` 的值。

禁止在 CP AST 内隐式使用浮点近似。对于小数业务量，调用方必须声明缩放规则：

```text
业务值 --显式 scale--> CP 整数值 --求解--> CP 整数解 --显式 unscale--> 业务值
```

以下情况必须返回失败：

- 非整数值未提供缩放策略。
- 缩放后超过 `Int64` 范围。
- 线性表达式累加可能溢出。
- 不同物理单位在未换算时参与同一 CP 表达式。

### 3.3 复用现有标量变量，新增 CP 结构变量

尽量复用现有 `BinVar`、`IntVar`、`UIntVar` 作为标量决策变量，使同一领域变量可以同时注册到 MILP 主问题和 CP 子问题。

新增的 CP 结构不伪装成标量变量：

- `BooleanLiteral`：变量和正负极性的组合。
- `IntegerDomain`：连续区间或离散值集合。
- `IntervalVariable`：start、size、end 和可选 presence literal。
- 后续可扩展 sequence、automaton state 等结构。

### 3.4 core 提供一次性求解和会话式求解

直接 CP 建模使用一次性入口；Benders 反复求解同一静态子问题时使用会话入口：

```kotlin
interface ConstraintProgrammingSolver {
    val descriptor: SolverDescriptor

    suspend fun solve(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions = ConstraintProgrammingSolveOptions()
    ): Ret<ConstraintProgrammingSolverOutput>

    fun createSession(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions = ConstraintProgrammingSolveOptions()
    ): Ret<ConstraintProgrammingSession>
}

interface ConstraintProgrammingSession : AutoCloseable {
    suspend fun solve(
        assumptions: List<BooleanLiteral> = emptyList(),
        hints: ConstraintProgrammingSolution? = null
    ): Ret<ConstraintProgrammingSolverOutput>
}
```

`ConstraintProgrammingSession` 是生命周期与可复用编译计划的边界，不承诺所有插件都能原地增量求解。不得把具体求解器变量泄露到 domain 或 application；插件无法增量修改模型时可以重建内部模型，但必须保持相同的公共契约。

### 3.5 求解器实现策略

OSPF 不依赖、不引入 OR-Tools。core 只定义 CP 模型、snapshot、solver SPI、能力声明和统一输出，不依赖任何具体求解器。

首版生产实现按以下顺序提供：

- 现有 SCIP 插件新增 `ScipConstraintProgrammingSolver`，优先使用 SCIP constraint handler、domain propagation 和 conflict analysis 能力。
- core 新增精确 `ConstraintProgrammingToLinearModelLowerer`，把能力子集等价转换为 `LinearMetaModel`。
- `MipBackedConstraintProgrammingSolver` 接收现有 `LinearSolver`；Gurobi 通过该入口求解可精确降维的 CP 子集。
- fake solver 和小规模穷举 oracle 只用于 core 契约与编译正确性测试，不作为生产 CP 引擎。

SCIP 的原生约束处理与 MIP 精确降维是不同实现机制，必须通过 capability 明确区分。不得把近似线性化、弱化约束或仅必要条件声明为已支持能力。

首版状态映射冻结如下：

| 求解器终态 | 统一输出 | 证明语义 |
| --- | --- | --- |
| 已证明最优 | `SolverStatus.Optimal` | `SolutionPresence.Optimal` + `ProofStatus.Verified` |
| 未证明最优但存在 incumbent | `SolverStatus.Feasible` | `SolutionPresence.Incumbent` + `ProofStatus.None` |
| 已证明不可行 | `SolverStatus.Infeasible` | `SolutionPresence.None` + `ProofStatus.Verified` |
| 达到限制且不存在 incumbent | `ConstraintProgrammingUnknownOutput` | `SolutionPresence.None` + `ProofStatus.None` |
| 模型校验失败 | 结构化 `Ret` 失败 | 不进入搜索，不产生 Benders 证书 |

`Exact` Benders 只接受求解器证明的最优或不可行子问题，以及声明全局有效性的 cut；可行但未证明最优、未知、取消和超时只能进入 `Heuristic` 或失败路径。精确 MIP 降维只有在证明与原 CP 约束等价时才能产生 `Verified` 终态。

### 3.6 framework 使用 Logic-Based Benders

首版只支持：

```text
LinearMetaModel master + ConstraintProgrammingModel subproblem
```

主问题继续由现有线性求解器求解，CP 子问题默认由 SCIP 插件求解；能力子集也可以使用注入了 Gurobi 等线性求解器的 `MipBackedConstraintProgrammingSolver`。具体求解器不生成领域 cut，framework 新增 `LogicBasedBendersEngine` 负责迭代，`BendersCutOracle` 负责生成 cut。

现有 `LinearBendersDecompositionSolver` 和 `QuadraticBendersDecompositionSolver` 保持不变，避免破坏插件和调用方兼容性。

### 3.7 精确模式和启发式模式必须分离

新增模式：

```kotlin
enum class BendersProofMode {
    Exact,
    Heuristic
}
```

`Exact` 模式必须满足：

- 每轮主问题得到可接受的最优证明。
- 用于收敛判断的 CP 子问题得到最优或不可行证明。
- 所有加入主问题的 cut 都声明全局有效性。
- 最终上下界满足配置的收敛容差。
- 超时、取消或 `Unknown` 子问题不能被当作精确证书。

`Heuristic` 模式可以接受可行但未证明最优的 CP 解，但最终报告不得声明全局最优。

### 3.8 CP conflict 复用于结构化不可行诊断

CP assumptions 和 conflict core 除用于 Logic-Based Benders 外，也作为 OSPF 通用结构化不可行诊断的一条精确路径。它替换的是当前通用 IIS 算法的首选实现，不替换 backend 原生 IIS/Farkas，也不要求把任意 LP、QP 或 QCP 转换为 CP。

默认诊断策略按模型类型和 capability 选择：

1. backend 能直接对原模型提供 native IIS 时优先使用 native IIS。
2. backend 能提供 Farkas 证书时，作为独立的精确不可行证明返回，不冒充 IIS；Farkas 不适用于只有整数语义才不可行而连续松弛可行的模型。
3. CP 模型或可精确编译为 CP/MIP assumption 模型的离散模型，使用 assumption conflict；后端 conflict core 不可用时，以全部活动 assumptions 作为可靠起点。
4. 不满足精确编译条件、包含 CP MVP 不支持的连续/二次语义或诊断达到限制时，降级到现有弹性/删除过滤实现。
5. 所有诊断失败只进入 `SolveDiagnostics` 的缺失/失败原因，不得覆盖已经确定的 `ProblemStatus.Infeasible`。

core 只定义 `InfeasibilityAnalyzer<M>`、策略编排和证据合同，不直接创建 SCIP、Gurobi 或其他插件实例。solver plugin 通过 capability 和 analyzer SPI 提供 native IIS、Farkas 或 conflict 能力；旧 `computeIIS(...)` 保留为兼容 facade，内部委托新编排器，并在无法使用新策略时调用现有实现。

CP conflict core 默认只表示“这些 assumptions 足以导致不可行”，不自动表示 IIS。只有满足以下条件时才能标记为不可约：

- 每个原始约束、变量下界、变量上界和 sparse domain 限制分别由稳定 activation ID 控制。
- 变量在 solver 中使用不额外制造不可行性的安全基础值域；如果关闭某个 bound/domain assumption 后该限制仍隐含在基础值域中，或无法构造安全有限值域，则该路径必须返回 `Unsupported`。
- CP 编译与原模型语义双向等价，不是松弛、必要条件或近似缩放。
- 删除一个成员后的每次子求解都得到已证明终态。
- 仅在剩余集合仍被证明不可行时删除成员；`Unknown`、超时或取消时保留成员并降低 minimality 等级。
- 最终成员只通过稳定 origin ID 回映射，求解器内部辅助约束不得泄露为公共证据成员。

这里的不可约是 inclusion-irreducible，不保证成员数量最少。对包含连续变量、非精确缩放或首版 CP AST 不支持的二次约束，不得启用 CP 精确诊断路径。

## 4. core 模型设计

### 4.1 拟议目录

```text
ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/
  model/constraint_programming/
    ConstraintProgrammingModel.kt
    ConstraintProgrammingModelSnapshot.kt
    ConstraintProgrammingExpression.kt
    ConstraintProgrammingConstraint.kt
    IntegerDomain.kt
    BooleanLiteral.kt
    IntervalVariable.kt
    ConstraintGroupRegistry.kt
  solver/constraint_programming/
    ConstraintProgrammingSolver.kt
    ConstraintProgrammingSession.kt
    ConstraintProgrammingSolveOptions.kt
    ConstraintProgrammingSolverOutput.kt
    ConstraintProgrammingFeature.kt
```

### 4.2 模型生命周期

`ConstraintProgrammingModel` 覆盖以下生命周期：

1. 注册标量变量和结构变量。
2. 注册具名表达式。
3. 注册约束和约束组。
4. 注册目标。
5. 校验模型。
6. 生成不可变 snapshot。
7. 由具体 solver adapter 或精确 lowerer 编译 snapshot。
8. 回填求解结果。

snapshot 必须满足：

- 变量和约束顺序确定。
- 每个元素都有稳定 ID。
- 不包含引擎内部可变状态。
- 可以重复编译。
- 编译后修改原模型不会影响已有 snapshot。

### 4.3 首版变量与表达式

| 类型 | 首版 | 说明 |
| --- | --- | --- |
| Boolean variable | 是 | 复用 `BinVar` |
| Integer variable | 是 | 复用 `IntVar` / `UIntVar` |
| Sparse integer domain | 是 | 支持区间集合和值集合 |
| Boolean literal | 是 | 支持正变量和 negated literal |
| Integer linear expression | 是 | 精确整数系数 |
| Interval variable | 是 | 固定或变量 duration |
| Optional interval | 是 | presence literal 控制 |
| Sequence variable | 否 | 后续按 solver capability 扩展 |
| Floating variable | 否 | 不属于 OSPF CP MVP |

### 4.4 首版约束

| 约束 | 首版 | 备注 |
| --- | --- | --- |
| 整数等式/不等式 | 是 | `==`、`<=`、`>=` |
| BoolAnd / BoolOr / BoolXor | 是 | 支持 literal |
| Implication | 是 | 支持 enforcement literal |
| Reified constraint | 是 | 明确单向和双向语义 |
| AllDifferent | 是 | 整数表达式集合 |
| Element | 是 | 常量或变量数组 |
| Allowed assignments | 是 | table constraint |
| Forbidden assignments | 是 | table constraint |
| NoOverlap | 是 | interval 集合 |
| Cumulative | 是 | interval、demand、capacity |
| Circuit | 后续 | 第二批全局约束 |
| Automaton | 后续 | 第二批全局约束 |
| Reservoir | 后续 | 第二批全局约束 |

### 4.5 求解器输出

CP 输出不强制复用 `FeasibleSolverOutput`，建议定义：

```kotlin
sealed interface ConstraintProgrammingSolverOutput : SolverOutput {
    val report: SolveReport?
}

data class ConstraintProgrammingFeasibleOutput(
    val solution: ConstraintProgrammingSolution,
    val objective: Flt64?,
    val bestBound: Flt64?,
    val status: SolverStatus,
    val proofStatus: ProofStatus,
    override val report: SolveReport?
) : ConstraintProgrammingSolverOutput

data class ConstraintProgrammingInfeasibleOutput(
    val conflict: ConstraintProgrammingConflict?,
    val proofStatus: ProofStatus,
    override val report: SolveReport?
) : ConstraintProgrammingSolverOutput

data class ConstraintProgrammingUnknownOutput(
    val terminationReason: TerminationReason,
    override val report: SolveReport?
) : ConstraintProgrammingSolverOutput
```

`ConstraintProgrammingSolution` 以稳定变量 ID 保存精确整数值，并提供针对 `BinVar`、`IntVar`、`UIntVar` 和 interval 的类型化读取方法。

### 4.6 conflict 与统一不可行证据

`ConstraintProgrammingConflict` 不建立与 `SolveReport.InfeasibilityEvidence` 平行且无法互转的第二套诊断模型。CP 输出中的 conflict 必须能无损转换为统一证据，并保留以下信息：

- `source`：`NativeIIS`、`ConstraintConflict`、`Farkas`、`ElasticFilter`、`DeletionFilter` 或 `None`。
- 原始约束、变量下界、变量上界、sparse domain 和 assumption 的稳定 ID。
- validity：`Verified`、`Heuristic` 或 `Unknown`，表示是否已经证明当前成员集合不可行。
- minimality：`Irreducible`、`NotChecked` 或 `Partial`。
- completeness、耗时、求解次数、终止原因和缺失/失败原因。
- backend 原生证书或内部 conflict 的可选引用，但不暴露 JNI/native 对象。

现有 `variableBoundIds: Set<VariableId>` 无法区分同一变量的上下界，实施前改为包含 `VariableId + BoundSide` 的结构化成员；sparse domain 使用独立 `VariableDomainRef`，不伪装成连续上下界。现有 `EvidenceExactness` 中 `Exact` 与 `Irreducible` 不是互斥概念，实施前拆分 validity 与 minimality；例如，一个未完成缩减但已证明不可行的 conflict 应表达为 `Verified + Partial`，不能被迫在 `Exact` 与 `Irreducible` 之间二选一。

旧 `LinearInfeasibleSolverOutput.iis`、`QuadraticInfeasibleSolverOutput.iis` 和 `SolverOutputWithIIS` 在兼容期继续存在。新报告以 `InfeasibilityEvidence` 为主；只有证据成员可稳定映射回原始模型时，兼容 facade 才物化旧 IIS model view。弹性/MCS 结果不得通过新报告伪装成 native 或 verified IIS。

## 5. 求解器能力与配置

### 5.1 能力声明

扩展 `SolverModelType`：

```kotlin
enum class SolverModelType {
    LP,
    MIP,
    QP,
    QCP,
    CP
}
```

避免继续向 `SolverCapabilities` 添加大量平铺布尔字段，新增：

```kotlin
enum class ConstraintProgrammingFeature {
    BooleanLogic,
    Reification,
    SparseDomain,
    AllDifferent,
    Element,
    Table,
    Interval,
    OptionalInterval,
    NoOverlap,
    Cumulative,
    Assumption,
    ConflictCore,
    SolutionHint,
    IncrementalSolve
}

enum class ConstraintProgrammingSupportLevel {
    Native,
    ExactLowering,
    Unsupported
}
```

`SolverCapabilities` 增加带默认值的 `constraintProgrammingFeatures: Map<ConstraintProgrammingFeature, ConstraintProgrammingSupportLevel>`，以保持现有构造调用兼容。调用前必须检查能力；`Unsupported` 返回结构化失败，禁止静默降级。

### 5.2 配置

`ConstraintProgrammingSolveOptions` 首版包含：

- time limit
- random seed
- deterministic mode
- solution limit
- relative / absolute objective gap
- log switch
- progress context
- cancellation hook
- whether to collect conflict core
- solver-specific backend configuration

配置无效时返回 `Failed(ErrorCode.IllegalArgument, ...)`，错误消息遵守中英双语格式。

## 6. Logic-Based Benders 契约

### 6.1 变量绑定

新增 `BendersVariableBinding`，显式描述主问题变量如何生成 CP assumption：

```kotlin
interface BendersVariableBinding {
    fun bind(
        masterSolution: ConstraintProgrammingValueSource,
        subproblem: ConstraintProgrammingModel
    ): Ret<BendersSubproblemAssignment>
}
```

绑定必须基于稳定变量 ID 或明确的领域 key，不允许只依赖对象引用相等。

`BendersSubproblemAssignment` 至少包含：

- 主问题变量及其值。
- CP 变量及其固定值。
- 本轮加入的 assumption literals。
- assumption literal 到 master assignment 的反向映射。

### 6.2 子问题结果

```kotlin
sealed interface LogicBasedBendersSubproblemResult {
    val assignment: BendersSubproblemAssignment
}

data class FeasibleSubproblemResult(
    override val assignment: BendersSubproblemAssignment,
    val output: ConstraintProgrammingFeasibleOutput
) : LogicBasedBendersSubproblemResult

data class InfeasibleSubproblemResult(
    override val assignment: BendersSubproblemAssignment,
    val conflict: ConstraintProgrammingConflict?
) : LogicBasedBendersSubproblemResult

data class UnknownSubproblemResult(
    override val assignment: BendersSubproblemAssignment,
    val terminationReason: TerminationReason
) : LogicBasedBendersSubproblemResult
```

### 6.3 cut 生成

```kotlin
interface BendersCutOracle {
    fun feasibilityCuts(
        result: InfeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>>

    fun optimalityCuts(
        result: FeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>>
}
```

首版 master cut 类型：

```kotlin
data class BendersMasterCut(
    val inequality: LinearInequality<Flt64>,
    val kind: BendersCutKind,
    val validity: BendersCutValidity,
    val proofStatus: ProofStatus,
    val source: String
)
```

`BendersCutValidity` 至少区分：

- `Global`：对整个 master 可行域有效。
- `Assignment`：只对当前 assignment 有效，需要 point-cut 编码。
- `Heuristic`：没有全局有效性证明，只能在启发式模式使用。

Exact 模式只允许加入 `Global` cut，或者由 framework 通过已验证的 point-cut 编码转换为全局有效线性约束。

### 6.4 默认 feasibility cut

对于二进制 master assignment，可以生成 no-good cut。

给定当前赋值集合：

```text
S1 = {i | x_i = 1}
S0 = {i | x_i = 0}
```

默认 no-good cut：

```text
sum(1 - x_i, i in S1) + sum(x_i, i in S0) >= 1
```

如果 CP 后端返回 assumption core，只使用 core 中的变量生成 cut，从而得到更强的冲突 cut。

一般整数变量的“不得等于当前向量”是析取约束，不能直接表示为单条线性不等式。`IntegerNoGoodEncoding` 已提供经过范围门禁的辅助二进制编码，`LogicBasedBendersEngine` 可以注册其多约束 cut；engine 仍不默认替调用方生成一般整数 no-good，调用方必须显式选择该编码或提供业务 cut oracle。

### 6.5 默认 optimality cut 的限制

CP 最优值 `Q(x*)` 本身只描述当前 assignment，不能自动推导对所有 `x` 有效的经典 Benders optimality cut。

因此首版规定：

- framework 不根据 `Q(x*)` 自动伪造全局 optimality cut。
- Exact 模式下，优化型 CP 子问题必须提供业务 optimality cut oracle，或提供已验证的 point-cut 上下界编码。
- Exact 模式下，主问题目标含 first-stage cost、`theta` 或其他项时必须提供 `completeObjectiveEvaluator`，显式计算当前 assignment 的完整 incumbent 目标；未提供时只能返回结构化契约缺失，不能把 `Q(x*)` 直接与主问题目标比较。
- feasibility-only CP 子问题可以只使用 no-good/conflict cut。
- Heuristic 模式可以使用带明确标记的启发式 cut，但最终结果不得标记为全局最优。

### 6.6 Benders 迭代生命周期

```text
1. register master model
2. register static CP subproblem
3. compile CP session
4. solve master
5. validate master terminal state
6. bind master solution to CP assumptions
7. solve CP subproblem
8. validate CP terminal state and proof
9. generate cuts through cut oracle
10. validate, deduplicate and register cuts
11. update bounds, gap, trace and progress
12. converge, stop, or continue
13. extract final solution and certificates
```

每轮 trace 至少记录：

- master objective and bound
- subproblem objective and bound
- master/subproblem termination reason
- cut count by kind
- conflict core size
- elapsed time
- current proof mode
- convergence gap

## 7. framework Context 与 Pipeline

### 7.1 拟议接口

```kotlin
interface ConstraintProgrammingPipeline : Pipeline<ConstraintProgrammingModel>

interface BendersSubproblemPipeline {
    fun register(
        model: ConstraintProgrammingModel,
        binding: BendersVariableBinding
    ): Try

    fun extractSolution(
        solution: ConstraintProgrammingSolution
    ): Try = ok
}
```

Context、Aggregation 和 ModelComponent 的职责继续遵循 framework 架构规范：

- Context 对 application 暴露注册入口。
- Aggregation 组织变量、表达式和 CP 结构。
- ModelComponent 保存领域变量及结果提取引用。
- Pipeline 负责单一 CP 约束族或目标族。
- Application 只负责模型创建、上下文组合、求解器选择和结果组装。

### 7.2 建议注册入口

领域 Context 可以按能力提供：

```kotlin
fun registerForConstraintProgramming(
    model: ConstraintProgrammingModel
): Try

fun registerForBendersSubproblem(
    model: ConstraintProgrammingModel,
    binding: BendersVariableBinding
): Try
```

不要求所有 Context 同时支持 MILP 和 CP。需要双建模的领域能力应共享领域数据和稳定变量 key，但可以使用不同的 Pipeline 实现。

## 8. SCIP 与 MIP 降维实现计划

### 8.1 目录结构

```text
ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/solver/constraint_programming/
  ConstraintProgrammingSolver.kt
  ConstraintProgrammingSession.kt
  ConstraintProgrammingSolveOptions.kt
  ConstraintProgrammingSolverOutput.kt
  lowering/
    ConstraintProgrammingToLinearModelLowerer.kt
    ConstraintProgrammingLoweringPolicy.kt
    MipBackedConstraintProgrammingSolver.kt

ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip/src/main/
  fuookami/ospf/kotlin/core/solver/scip/
    ScipConstraintProgrammingSolver.kt
    ScipConstraintProgrammingSession.kt
    ScipConstraintProgrammingCompiler.kt
    ScipConstraintProgrammingStatusMapper.kt
```

Gurobi 不新增独立 CP AST 编译器。调用方通过：

```kotlin
MipBackedConstraintProgrammingSolver(
    linearSolver = GurobiLinearSolver(),
    loweringPolicy = ConstraintProgrammingLoweringPolicy.Strict
)
```

求解可精确降为 MIP 的 CP 子集。其他现有 `LinearSolver` 插件也可以复用该入口。

### 8.2 首版能力矩阵

| CP 能力 | SCIP 插件 | MIP-backed/Gurobi | 首版策略 |
| --- | --- | --- | --- |
| 整数线性约束 | native linear handler | exact linear model | 支持 |
| Bool AND/OR/XOR | native constraint handler | exact binary lowering | 支持 |
| implication/reification | indicator/superindicator | indicator 或 exact big-M | 仅在有限界可证明时支持 |
| sparse integer domain | bound-disjunction 或枚举 | one-hot/disjunction | 有规模门禁 |
| all-different | exact decomposition | exact decomposition | 有规模门禁 |
| element | exact one-hot decomposition | exact one-hot decomposition | 有限 index/domain 才支持 |
| allowed/forbidden table | tuple selection/no-good decomposition | 同左 | 有规模门禁 |
| mandatory fixed-duration interval | start/end 等式 | exact linear model | 支持 |
| no-overlap | cumulative capacity 1 或 pairwise disjunction | pairwise order binaries | 支持固定 duration |
| cumulative | native cumulative handler | time-indexed lowering | SCIP native；MIP 首版默认不支持 |
| optional/variable-duration interval | 当前 JSCIP API 不完整 | 需额外 exact formulation | 不进入首版 |
| assumptions | 每轮加入固定等式并重建模型 | 每轮加入固定等式并重建模型 | 功能支持，非增量 |
| conflict core | 全部活动 assumptions；可选删除缩减 | 同左 | 有效但可能较弱 |
| solution hint | 按插件现有 warm-start 能力 | MIP start | capability 控制 |

所有 exact decomposition 必须满足双向等价，并用小规模穷举 oracle 对照原 CP 语义。超过 tuple 数、domain 大小、时间范围或辅助变量阈值时返回 `Unsupported`，不得生成不可控的大模型。

### 8.3 SCIP session 与 conflict 策略

当前 `jscip:1.0.0` 已暴露 cumulative、AND、OR、XOR、indicator、superindicator、bound-disjunction、set packing/covering/partitioning 和 SOS 等约束创建接口，但未暴露完整的增量 bound 修改、probing 和 conflict graph 提取接口。

首版采用以下正确性优先策略：

1. `ScipConstraintProgrammingSession` 缓存 OSPF snapshot 和编译计划，不缓存可变 SCIP 搜索状态。
2. 每轮 assumptions 作为固定等式编译到新的 SCIP 模型。
3. SCIP 证明不可行时，全部活动 assumptions 构成有效 conflict core。
4. 配置开启 core shrinking 时，通过删除一个 assumption 后重新求解，得到 irreducible 但不保证最小的 core。
5. 只有所有缩减子求解都得到已证明终态时，缩减后的 core 才可进入 Exact Benders。

后续若升级 JSCIP binding 暴露 probing、bound change 和 conflict analysis，再增加真正的增量 session；不得为了性能提前泄露 JNI 类型到 core SPI。

### 8.4 状态与资源边界

SCIP adapter 和 MIP-backed solver 必须：

- 在编译前检查 feature support level。
- 捕获可捕获的 Java/native binding 异常并转换为 `Ret`。
- 提取 objective、best bound、assignment、终止原因和统计信息。
- 在 session 关闭或单次求解完成后释放 solver model、constraint 和 variable 引用。
- CP activation path 保持稳定 origin ID，禁止用求解器内部序号作为公共身份；线性/二次 native provider 在 `OSPF-SOL-013` 完成前仅提供 origin-backed 或明确标注为 model-local 的诊断 ID。

### 8.5 assumption conflict IIS 算法与降级链

结构化 IIS 路径复用 CP session 的 assumption 能力，但以原模型元素而不是 Benders 主问题赋值作为 assumption 来源：

1. 为每个原始约束创建独立 activation literal，并以单向 enforcement 保持“激活时原约束完整生效”。
2. 为每个有限变量下界、上界和 sparse domain 限制分别创建 activation literal，禁止只使用 `VariableId` 合并两个边界或把离散 domain 冒充连续区间。
3. 编译诊断模型时把 solver 变量建立在经过证明的安全基础值域上，原 bound/domain 只通过 activation 约束生效；无法安全移除限制时返回 `Unsupported`。
4. 使用全部 activation literals 求解；只有后端证明不可行，才产生 verified conflict seed。
5. 后端支持可靠 conflict core 时先投影到原始 activation ID；否则以全部活动 assumptions 为 seed。
6. 对 seed 执行 deletion-based shrinking：移除候选后若仍证明不可行则永久移除；若证明可行则保留；若 `Unknown`、超时或取消则保留并把 minimality 标记为 `Partial`。
7. 最终集合始终重新验证不可行；只有所有必要性检查均得到证明，才标记为 `Irreducible`。

诊断编排不采用单一固定 solver，而是根据原求解器和模型 capability 选择策略：

```text
NativeIISAnalyzer
  -> FarkasAnalyzer（适用时）
  -> ConstraintConflictAnalyzer（仅精确 CP/离散编译范围）
  -> LegacyElasticInfeasibilityAnalyzer
  -> Unavailable evidence
```

`LegacyElasticInfeasibilityAnalyzer` 直接复用当前 `core/solver/iis` 代码，首轮迁移不删除其算法。其输出保留 slack 值和修复候选等解释信息，但来源必须是 `ElasticFilter` / `DeletionFilter`，默认 validity 为 `Heuristic` 或 `Unknown`，除非另有独立验证证明返回成员本身不可行。

诊断策略自身返回 `Ret<InfeasibilityEvidence>`，但报告编排必须把诊断 `Failed` / `Fatal` 转换为 `SolveDiagnostics` 中的结构化问题；它不能把原始求解已经证明的 `Infeasible` 改写成一次求解调用失败。

## 9. 分阶段实施任务

### Phase CP0：求解器边界与 SCIP 探针

- [x] `OSPF-CP-001` 冻结 OSPF 不依赖、不引入 OR-Tools 的边界。
- [x] `OSPF-CP-002` 冻结 core 通用 CP SPI、SCIP 原生实现和 MIP 精确降维三层结构。
- [x] `OSPF-CP-003` 建立最小 JSCIP 探针，验证布尔约束、indicator 和 cumulative 可创建并求解。
- [x] `OSPF-CP-004` 冻结整数缩放和溢出策略。
- [x] `OSPF-CP-005` 冻结 CP 求解器终态到 `SolveReport` 的映射。

验收：不增加新求解器依赖；现有 JSCIP 能求解最小布尔与 cumulative 模型；所有未决 API 能力被标为 `Native`、`ExactLowering` 或 `Unsupported`。

### Phase CP1：core CP 模型与 SPI

- [x] `OSPF-CP-101` 新增 `ConstraintGroupRegistry` 并兼容现有 `MetaModel`。
- [x] `OSPF-CP-102` 新增整数 domain、boolean literal 和 CP expression。
- [x] `OSPF-CP-103` 新增首版 CP constraints。
- [x] `OSPF-CP-104` 新增 interval 和 scheduling constraints。
- [x] `OSPF-CP-105` 新增 `ConstraintProgrammingModel` 和 immutable snapshot。
- [x] `OSPF-CP-106` 新增 solver、session、options 和 output SPI。
- [x] `OSPF-CP-107` 扩展 solver capabilities 和 model type。
- [x] `OSPF-CP-108` 新增 fake solver 测试夹具。

验收：core CP AST 不依赖 SCIP、Gurobi 或其他求解器类型；fake solver 能验证完整建模和结果回填路径。

### Phase CP2A：SCIP CP/CIP 实现

- [x] `OSPF-CP-201` 新增 `ScipConstraintProgrammingSolver`、session 和 compiler。
- [x] `OSPF-CP-202` 实现变量、domain、整数表达式和 objective 编译。
- [x] `OSPF-CP-203` 映射 AND、OR、XOR、indicator、superindicator 和 bound-disjunction。
- [x] `OSPF-CP-204` 映射 mandatory fixed-duration interval、no-overlap 和 cumulative。
- [x] `OSPF-CP-205` 实现 all-different、element 和 table 的受限精确分解。
- [x] `OSPF-CP-206` 实现状态、solution、objective bound、统计和错误映射。
- [x] `OSPF-CP-207` 实现基于模型重建的 assumptions session 和全 assumptions conflict core。
- [x] `OSPF-CP-208` 实现可选 deletion-based core shrinking 和证明门禁。
- [x] `OSPF-CP-209` 实现配置、取消、进度、资源释放和 capability 声明。

验收：Sudoku、固定 duration Job Shop 和 cumulative 调度由 SCIP 插件求解；限制终态不会被映射为已证明最优或不可行。

### Phase CP2B：精确 MIP 降维与 Gurobi

- [x] `OSPF-CP-221` 新增 `ConstraintProgrammingToLinearModelLowerer` 和严格 lowering policy。
- [x] `OSPF-CP-222` 实现整数线性、布尔逻辑、indicator 和 reification 的精确降维。
- [x] `OSPF-CP-223` 实现受规模限制的 sparse domain、all-different、element 和 table 降维。
- [x] `OSPF-CP-224` 实现固定 duration no-overlap 的 pairwise order formulation。
- [x] `OSPF-CP-225` 新增 `MipBackedConstraintProgrammingSolver`，复用现有 `LinearSolver` 状态、hint 和进度能力。
- [x] `OSPF-CP-226` 使用 fake linear solver 验证编译契约，并使用 Gurobi 插件完成定向集成测试。

验收：同一受支持 CP 模型通过 SCIP native 和 Gurobi MIP-backed 两条路径得到一致的可行性与目标值；不支持能力在建模编译阶段失败。

### Phase CP2C：结构化不可行证据与 IIS 迁移

- [x] `OSPF-CP-241` 扩展 `InfeasibilityEvidenceSource`，新增 `ConstraintConflict`，并把证据 validity 与 minimality 拆为正交字段。
- [x] `OSPF-CP-242` 新增可区分上下界的 `VariableBoundRef`、`VariableDomainRef` 和统一 `InfeasibilityMember`，贯穿 snapshot、编译映射和报告。
- [x] `OSPF-CP-243` 新增 backend-neutral `InfeasibilityAnalyzer<M>`、策略 capability 和诊断编排器。
- [x] `OSPF-CP-244` 实现约束/变量界/domain activation、安全基础值域门禁、全 assumptions conflict seed、origin 投影和最终不可行复验。
- [x] `OSPF-CP-245` 实现 deletion-based conflict shrinking，以及 `Verified + Irreducible/Partial/NotChecked` 证明门禁。
- [x] `OSPF-CP-246` 接入 backend native IIS/Farkas，并按模型类型与 capability 优先于通用降级算法。
- [x] `OSPF-CP-247` 将当前弹性/删除过滤封装为 `LegacyElasticInfeasibilityAnalyzer`，保留算法但按实际保证标记为启发式、未知或 MCS 证据。
- [x] `OSPF-CP-248` 将旧 `computeIIS(...)` 和 IIS output 改为兼容 facade；诊断失败写入 `SolveDiagnostics`，不得覆盖已证明的 `Infeasible`。
- [x] `OSPF-CP-249` 增加 native IIS、CP conflict、Farkas、legacy fallback 和诊断失败的策略矩阵测试。

验收：CP activation path 的矛盾约束和矛盾变量界能返回稳定 origin ID 的 verified infeasible subset；线性/二次 native path 在 `OSPF-SOL-013` 完成前对无 origin 元素返回明确的 model-local ID，不宣称跨重建稳定；完成全部缩减证明时标记为不可约；任一缩减求解为 `Unknown` 时不冒充 IIS；连续/二次不支持模型自动走 native 或 legacy 分支；所有诊断失败仍保留原始 `ProblemStatus.Infeasible`。

CP2C-246/248 的 native provider 接入不等同于完成 `OSPF-SOL-013`：中间模型目前没有贯穿规范化、重建和远程序列化的一等约束/变量身份。provider 使用 origin-backed ID 时可回查原模型；无 origin 时必须保留 model-local 标记，待身份契约单独完成后再升级为跨重建稳定 ID。

### Phase CP3：framework Logic-Based Benders

- [x] `OSPF-CP-301` 新增 master/subproblem variable binding。
- [x] `OSPF-CP-302` 新增 CP subproblem result 和 proof contract。
- [x] `OSPF-CP-303` 新增 master cut、validity 和 cut oracle。
- [x] `OSPF-CP-304` 实现 binary no-good cut。
- [x] `OSPF-CP-305` 实现全赋值 no-good cut 和缩减 assumption-core conflict cut。
- [x] `OSPF-CP-306` 实现 cut validation、deduplication 和 naming。
- [x] `OSPF-CP-307` 实现 `LogicBasedBendersEngine`。
- [x] `OSPF-CP-308` 实现 Exact/Heuristic 终止契约。
- [x] `OSPF-CP-309` 实现 iteration trace、progress 和 fallback hook。

验收：一个二进制 master + SCIP CP feasibility subproblem 的用例能通过冲突 cut 收敛，且错误终态不能被误报为最优。

### Phase CP4：framework 集成与示例

- [x] `OSPF-CP-401` 新增 `ConstraintProgrammingPipeline`。
- [x] `OSPF-CP-402` 新增 `BendersSubproblemPipeline`。
- [x] `OSPF-CP-403` 提供最小直接 CP core demo。
- [x] `OSPF-CP-404` 提供最小 framework CP Context/Aggregation/Pipeline demo。
- [x] `OSPF-CP-405` 提供 Logic-Based Benders demo。
- [x] `OSPF-CP-406` 更新中英文 README 和架构文档。

验收：下游能够只新增 Context/Pipeline 和 cut oracle，不修改 Benders engine 主循环。

### Phase CP5：增强与迁移

- [ ] `OSPF-CP-501` 增加更多全局约束（AST、求值和 snapshot 已覆盖 Circuit/Automaton/Reservoir，生产 backend 编译仍待实现）。
- [ ] `OSPF-CP-502` 增加 remote model serialization（目前提供 portable CP snapshot codec，尚未接入 remote execution protocol）。
- [x] `OSPF-CP-503` 评估 checkpoint/resume 能力（已提供 portable snapshot checkpoint 和 capability assessment；native resume 仍由 backend 声明）。
- [ ] `OSPF-CP-504` 评估升级 JSCIP binding 以支持增量 bound、probing 和 conflict graph。
- [ ] `OSPF-CP-505` 评估 demo2 或甘特排程框架迁移。
- [x] `OSPF-CP-506` 增加一般整数 master no-good encoding。
- [x] `OSPF-CP-507` 增加 optional interval 和 variable duration 的精确支持（MIP-backed 路径；SCIP native 仍为 fixed-duration）。

## 10. 测试矩阵

| 层级 | 测试内容 | 必须覆盖 |
| --- | --- | --- |
| core unit | domain、literal、表达式、约束、snapshot | 空模型、重复 ID、非法 domain、溢出 |
| core contract | solver/session/output | Feasible、Optimal、Infeasible、Unknown |
| SCIP compiler unit | native handler mapping 和 decomposition | 每种首版约束、ID、资源释放 |
| SCIP integration | native CP/CIP solve | Sudoku、Job Shop、Cumulative、终态映射 |
| lowerer property | 穷举 oracle 对照 | 原 CP 与 LinearMetaModel 可行集、目标值一致 |
| MIP-backed contract | fake/Gurobi linear solver | capability、unsupported、hint、状态传播 |
| cross-solver differential | SCIP 与 Gurobi 对照 | 受支持随机小模型可行性和最优值一致 |
| infeasibility contract | 统一证据、成员 ID、validity、minimality | native IIS、CP conflict、Farkas、legacy、unavailable |
| conflict shrinking | assumption seed 与删除缩减 | 约束、上下界、sparse domain、不可约、partial、Unknown |
| IIS compatibility | 旧 `computeIIS` 和旧 output facade | 可物化 IIS、MCS 不冒充 IIS、诊断失败不覆盖结论 |
| framework unit | binding 和 cut oracle | 二进制 no-good、conflict core、cut 去重 |
| Benders contract | 终态和证明 | master 非最优、SP 可行非最优、SP unknown |
| framework integration | Context/Pipeline | direct CP 和 Benders SP 两条注册路径 |
| end-to-end | Logic-Based Benders | 收敛、停滞、超时、取消、fallback |

测试必须验证以下负路径：

- 非整数 master 值无法绑定到整数 CP 变量。
- conflict core 含未知 assumption ID。
- conflict core 含求解器内部辅助约束但缺少稳定 origin ID。
- 同一变量上下界被错误合并为一个证据成员。
- 关闭 bound/domain assumption 后限制仍隐含在 solver 变量基础值域中。
- 无法构造安全基础值域却继续生成 verified conflict。
- 缩减子求解为 `Unknown`、超时或取消却被标记为 `Irreducible`。
- 弹性/MCS 结果被标记为 native、verified 或 irreducible IIS。
- IIS 诊断失败覆盖原始 `Infeasible` 结论。
- Exact 模式收到 heuristic cut。
- Exact 模式收到未证明最优的 CP 子问题结果。
- solver adapter 不支持已注册的 CP feature。
- exact lowering 缺少有限上下界或超过配置规模门禁。
- 非等价或只单向蕴含的 formulation 被错误声明为 `ExactLowering`。
- solver session 已关闭后继续求解。
- callback 或 progress reporter 返回失败。

## 11. 构建与验收策略

任务进行中优先执行受影响模块的增量构建，并把完整输出写入日志。例如：

```bash
mvn compile test-compile -pl ospf-kotlin-core,ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip,ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-gurobi,ospf-kotlin-framework -am -T 0.75C > cp-incremental-build.log 2>&1
mvn test -pl ospf-kotlin-core,ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip,ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-gurobi,ospf-kotlin-framework -am -T 0.75C > cp-incremental-test.log 2>&1
```

SCIP adapter 和 MIP lowerer 加入后增加对应插件的定向测试、lowering 性质测试和跨求解器测试。最终验收执行：

```bash
mvn clean compile test-compile -T 0.75C > cp-final-build.log 2>&1
mvn test -T 0.75C > cp-final-test.log 2>&1
```

任何失败都基于同一次构建的完整日志分析，不通过反复截断输出来重复运行 Maven。

## 12. 主要风险与控制措施

| 风险 | 影响 | 控制措施 |
| --- | --- | --- |
| CP 没有 LP 对偶证书 | 无法自动生成经典 Benders cut | 使用 Logic-Based Benders 和领域 cut oracle |
| 把当前 CP 最优值误当作全局 cut | 可能排除真正最优解 | cut validity + proof mode 强制校验 |
| 浮点缩放或整数溢出 | 模型语义错误 | 显式 scale、构建期范围分析、失败返回 |
| JSCIP 未暴露完整 SCIP API | 某些 native constraint、增量 session 或 conflict 无法接入 | capability 门禁、模型重建、必要时单独升级 binding |
| exact lowering 不等价 | 漏解、伪解或错误最优证明 | 双向等价审查、穷举 oracle、SCIP/Gurobi differential test |
| table/domain 降维膨胀 | 辅助变量和约束不可控 | tuple/domain/time horizon 规模门禁，超过阈值返回 unsupported |
| session 每轮重建 | Benders 子问题耗时增加 | 缓存 snapshot/编译计划，后续升级 JSCIP 增量 API |
| solver model 资源泄漏 | native 内存增长 | `AutoCloseable`、局部映射、生命周期和重复求解测试 |
| 过早迁移大型业务示例 | API 尚未稳定时产生大量返工 | 先最小 demo，再迁移 demo2/甘特排程 |
| 默认 no-good cut 太弱 | Benders 迭代次数过高 | assumption core、业务强 cut、trace 指标 |
| conflict core 过弱或缩减未证明 | Benders 迭代慢或 cut 无效 | 全 assumptions 安全回退、缩减子求解证明、Exact 门禁 |
| 把 conflict core 或 MCS 误报为 IIS | 用户修复错误模型元素，审计结果失真 | validity/minimality 正交标记、最终不可行复验、legacy 明确降级 |
| CP MVP 不覆盖连续或二次语义 | 无法替换所有线性/二次 IIS 路径 | native IIS/Farkas 优先，精确 capability 门禁，legacy fallback |
| 上下界或辅助约束无法回映射 | 结构化证据歧义或泄露 backend 细节 | `VariableBoundRef`、稳定 origin ID、无法投影时返回 partial/unavailable |
| 诊断失败覆盖求解结论 | 已证明不可行被错误报告为系统失败 | 诊断作为 `SolveDiagnostics` 附属结果，失败只记录结构化 issue |
| SCIP/Gurobi capability 语义漂移 | 同一 CP 模型跨 solver 行为不同 | 三态 capability、统一 contract test、跨求解器对照 |

## 13. MVP 非目标

以下内容不进入第一版：

- CP master problem。
- 混合 CP/MIP 单一求解模型。
- SCIP native optional interval 和 variable duration 的通用编译（MIP-backed 精确子集已支持）。
- 自动把任意 CP 全局约束降为 MIP。
- remote CP execution protocol 与 native checkpoint/resume；当前只提供 portable snapshot/checkpoint 重建。
- 未经证明的自动 optimality cut。
- Circuit、Automaton、Reservoir 的生产 backend 编译。
- OSPF 自研生产级 CP 搜索引擎。
- OR-Tools adapter 或依赖。

## 14. MVP 完成定义

满足以下条件后，约束规划 MVP 才算完成：

1. core 用户可以不依赖 framework 构建 CP 模型，并通过注入 solver plugin 直接求解。
2. OSPF 不引入 OR-Tools 依赖或 adapter。
3. SCIP 插件能求解逻辑、table、固定 duration interval 和 cumulative 模型。
4. MIP-backed solver 能通过 Gurobi 求解 capability 声明覆盖的精确降维子集。
5. framework Context/Pipeline 可以注册 CP 子问题。
6. Logic-Based Benders 能处理二进制 master 的 SCIP CP feasibility subproblem。
7. 全 assumptions core 或经证明缩减的 core 可以转换为有效 conflict cut。
8. CP conflict 能输出结构化不可行证据，并在证明充分时输出 inclusion-irreducible IIS。
9. backend native IIS/Farkas 优先；现有弹性/删除过滤作为有明确来源和证据等级的降级分支保留。
10. 诊断失败不会覆盖已证明的 `Infeasible`；CP activation 成员可通过稳定 origin ID 回查，native 线性/二次 fallback 在 `OSPF-SOL-013` 完成前明确标记为 model-local。
11. Exact 模式不会接受未证明有效的 cut、非等价 lowering 或未证明终态。
12. 所有失败路径使用 `Try` / `Ret<T>`，求解器异常不会穿透 adapter 边界。
13. 中英文 README、最小 demo、能力矩阵和扩展点测试同步完成。
14. 全量编译与全量测试通过。

## 15. 推荐实施顺序

严格按以下顺序推进：

```text
CP0 JSCIP 布尔/indicator/cumulative 探针
  -> CP1 core AST、SPI 和 fake solver
  -> CP2A SCIP native compiler 和 direct solve
  -> CP2A assumptions/session/conflict core
  -> CP2B exact MIP lowerer 和 Gurobi 对照
  -> CP2C 结构化不可行证据、IIS shrinking 和 legacy fallback
  -> CP3 Benders binding 和 conflict cut
  -> CP3 完整迭代器与证明门禁
  -> CP4 Context/Pipeline 和示例
  -> CP5 增强与业务迁移
```

第一项实现工作是 `OSPF-CP-003` 最小 JSCIP 能力探针。探针验证当前 binding 与 native library 可用后，再开始批量新增 core 公共 API。
