# OSPF 与 LLM 融合：技术路径

本页把[业务路径](./integrating-llms)中的生产计划例子拆成可以实现和测试的技术组件。基准模型仍为两个非负整数产量：最大化 $30x_A+50x_B$，满足 $2x_A+3x_B\le120$、$x_A+2x_B\le70$。A 至少 20 件对应新增 $x_A\ge20$；增加工时是修改独立试算副本的参数，不覆盖正式方案。

本页区分三种内容：**OSPF API 建模片段**、**示例应用自定义类型**和**尚待实现的运行时协议**。它不是已发布的完整集成 SDK，不包含可直接启动的 LLM 服务、持久化注册表或生产发布系统。业务含义和数值验收以业务页为准；本页回答组件怎么连接、对象何时创建、错误在哪里返回。

## 1. 组件与依赖方向

| 组件 | 输入 → 输出 | 禁止承担的职责 |
|---|---|---|
| Catalog / Projection | 目录版本、认证身份 → 授权字段目录 | 不创建求解器变量，不相信请求里的角色 |
| LLM Adapter | 授权上下文、请求 → 未可信候选 DTO | 不调用仓储写入，不发布能力 |
| Decoder / Validator | DTO、目录、策略 → 已验证 IR 或错误 | 不把合法 JSON 当成业务正确 |
| Compiler | 已验证 IR、符号描述 → 可绑定计划 | 不保留上一轮变量，不私自近似未知公式 |
| math 符号运算 AST | 数学表达式节点、符号绑定 → 求值或数学表示转换 | 不替代字段权限、业务版本和求解器能力检查 |
| OSPF Adapter | 冻结输入、计划、本轮变量 → 求解报告 | 不决定业务审批与模板提升 |
| Result / Evidence Mapper | 报告、字段绑定 → 证据与比较 | 不重写后端原始状态或伪造 IIS |
| Runtime / Registry | 形状、依赖、运行记录 → 计划选择与生命周期 | 不绕过通用校验，不缓存最优解冒充计划 |

请求处理路径不依赖某个 LLM 厂商；LLM、求解器和注册表均通过应用边界替换。只有已授权的应用服务可以触发执行。

```text
请求 → 授权上下文 → LLM 候选 → 严格解析 → 类型化 IR → 语义校验
                                                        ↓
                           规范化/参数提取 → 兼容计划查找
                                  ├─ 命中：绑定专用计划
                                  └─ 未命中：通用编译
                                                        ↓
                             新模型/只读快照 → 执行 → 证据 → 解释

运行记录 → 提升策略 → 异步构建 → 影子验证 → 原子发布
依赖变化 → 失效 → 通用回退或明确错误 → 新版本重建
```

## 2. 字段如何连接数值计算与符号计算

目录保存的是稳定语义，不是某个运行中的对象地址：

| 字段 | 数值入口 | 本轮符号绑定 | 能力 |
|---|---|---|---|
| `production[A]` | 从已解结果读取 A 产量 | 本轮变量 $x_A$ | 查询、最低产量约束 |
| `material.used` | 用冻结系数求 $2x_A+3x_B$ | 本轮表达式 $2x_A+3x_B$ | 查询、资源约束 |
| `processing.used` | 用冻结系数求 $x_A+2x_B$ | 本轮表达式 $x_A+2x_B$ | 查询 |
| `profit` | 求 $30x_A+50x_B$ | 本轮目标表达式 | 查询 |

每次建模创建新的 `ModelBindingContext`，维护“字段 ID + 产品维度 → 本轮变量或表达式”的映射。它仅在该模型生命周期内有效。约束模板保存字段 ID、参数槽和关系操作符，执行时通过上下文重新绑定。

数值入口和符号入口消费同一份不可变系数定义；用已知产量代入符号表达式，与独立数值计算对照测试。数据中的收益、产能与当前解不能被混入形状模板。目录能力、单位、维度、业务时段和权限分别校验；可见不等于可用于约束。

### 2.1 math 组件提供的符号运算 AST

math 组件的符号运算 AST 为业务 IR 提供结构化数学表达，应用不需要为 LLM 再维护一套通用数学表达式引擎。AST 的节点、求值与部分求值见[符号表达式与符号运算](./symbolic-expressions)，从数学表示到后端的转换见[类编译器架构与模型转换](./compiler-architecture)。

本例仍要保留三层边界：业务 IR 记录“A 产品当天至少生产 20 件”的对象、单位、时间和授权；数学表达记录 $x_A\ge20$；core 模型绑定本轮整数变量和约束。AST 节点不证明字段访问权限，也不自动证明目标后端支持表达中的运算。

对材料关系 $2x_A+3x_B\le120$，应用先检查单位和字段来源。绑定方案 $(30,20)$ 得到材料用量 120 kg；保留未知量则进入系数为 $[2,3]$、右端为 120 的线性约束。简单模板可以直接构造多项式，不必先生成文本，也不要求所有查询和流程都通过数学 AST。

### 2.2 AST 与运行时专门化的关系

可复用计划保存表达结构和稳定字段描述，执行时重新绑定参数与本轮变量。通用符号运算的缓存规则由表达层处理，运行时还要加入业务目录、授权范围、时间窗口和发布版本等依赖。

相同表达哈希不等于相同场景或授权；缓存求值计划不是缓存最优解。允许表示 $x_Ax_B$ 不意味着可以将它放入本例的线性模型，必须明确选择适合的模型路径或者拒绝，不能静默删去乘积项。

## 3. LLM 输出不是内部领域对象

适配器输入包含目录投影、基线引用、上下文哈希、可用操作和预算。输出先经过大小限制和 JSON 解析，再检查 schema 版本、未知属性、枚举、整数溢出、必填字段与数组长度。数字不能经浮点截断变为件数；不允许从类名、URL、SQL 或脚本构造操作。

应用将边界 DTO 转为以下闭集类型。这里的字符串 ID 是缩小篇幅的示例，生产实现应分别封装 RunId、BaselineId 等类型。代码定义类型，不构成完整的 JSON 解码器；调用者需复制防御性集合或使用不可变集合，防止验证后被修改。

::: code-group

```kotlin [Kotlin]
enum class Product { A, B }
enum class Metric { MaterialUsed, HoursUsed, Profit }
enum class UnitCode { Piece, Hour }
data class Literal(val value: Long, val unit: UnitCode)

sealed interface QueryIR {
    data class ReadRun(val runId: String, val fields: List<Metric>) : QueryIR
}
sealed interface ConstraintIR {
    data class Minimum(val product: Product, val quantity: Literal) : ConstraintIR
}
sealed interface WorkflowIR {
    data class CompareHours(
        val baselineId: String,
        val deltas: List<Literal>
    ) : WorkflowIR
}

enum class ParameterType { ProductId, NonnegativePieces }
data class MinimumPlan(
    val compilerVersion: String,
    val parameters: List<ParameterType> =
        listOf(ParameterType.ProductId, ParameterType.NonnegativePieces)
)
// Application plan data only: no OSPF variable object is cached.
```

```rust [Rust]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Product { A, B }
#[derive(Clone, Copy, Debug)]
enum Metric { MaterialUsed, HoursUsed, Profit }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitCode { Piece, Hour }
#[derive(Clone, Copy, Debug)]
struct Literal { value: i64, unit: UnitCode }

enum QueryIR {
    ReadRun { run_id: String, fields: Vec<Metric> },
}
enum ConstraintIR {
    Minimum { product: Product, quantity: Literal },
}
enum WorkflowIR {
    CompareHours { baseline_id: String, deltas: Vec<Literal> },
}

enum ParameterType { ProductId, NonnegativePieces }
struct MinimumPlan {
    compiler_version: String,
    parameters: Vec<ParameterType>,
}
// Application plan data only: no OSPF variable object is cached.
```

:::


三个 IR 分开：查询读取已完成运行，约束限定未来可行解，流程组合已注册操作。不能把 `ReadRun` 转成求解器约束，或用 `Minimum` 对历史记录做过滤来假装完成建模。

验证层额外生成不可由 DTO 直接构造的 `ValidatedRequest`，绑定请求内容哈希、目录/策略版本、基线、已授权身份和确认记录。其构造入口只向验证器开放。执行前仍检查版本与权限是否过期，不能因持有该对象而永久信任。

## 4. 校验、规范化与编译

### 4.1 最低产量的编译规则

对 `Minimum(A, Literal(20, Piece))`：

1. 名称解析：A 是登记产品，目标字段是计划产量。
2. 类型和单位：数量为非负整数件，未超过服务声明的数值上限。
3. 范围检查：规则对应冻结业务日和基线，允许加入主整数线性模型。
4. 授权与确认：操作者允许提出该约束，业务含义已确认。
5. 规范化：生成 `GE(Field(production.quantity,A),Integer(20,piece))`。
6. 绑定：在本轮上下文解析 A，生成系数为 1、右端为 20 的约束行。

在原生接口使用浮点系数时，整数先通过精确可表示范围检查，再转换；不能将任意 64 位整数无条件转为浮点。

| 错误代码（示例应用） | 例子 | 处理 |
|---|---|---|
| UNKNOWN_FIELD | 未登记的 C 产品 | 拒绝或澄清，不按名称相似自动绑定 |
| UNIT_MISMATCH | 20 kg 被用作件数 | 要求明确换算或修正 |
| STALE_CONTEXT | 基线或目录已更新 | 重新获取上下文并验证 |
| UNSUPPORTED_EXPRESSION | 未登记良率函数 | 返回能力缺口，不静默线性化 |
| FORBIDDEN_OPERATION | 要求绕过审批下发 | 拒绝，不交给模型重试 |
| BUDGET_EXCEEDED | 候选数量或耗时超限 | 停止并转人工 |

技术性重试只产生新的运行尝试；业务约束修改产生新候选。每次纠正保留父引用、原因与预算消耗。重试不能自动扩大权限。

### 4.2 到 OSPF 的模型边界

下列原生 API 片段从业务页移到此处。Kotlin 错误检查仍由外围应用补齐；不能把注释视为已经实现的错误传播。两端编译与真实求解器集成测试仍是后续验收项。


下面使用两端各自的核心建模 API，是建模片段，不是完整 LLM 客户端或可独立运行的求解程序。Kotlin 片段展示 S1，错误传播由外围应用补齐：每次注册和添加约束后立即检查返回结果，失败则停止。Rust 使用 `?` 传播建模错误。S2 应以 80 小时重建模型，不能同时保留旧的 70 小时约束。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.mechanism.LinearMetaModel
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val model = LinearMetaModel<Flt64>(
    name = "production",
    objectCategory = ObjectCategory.Maximum
)
val xA = IntVar("xA")
val xB = IntVar("xB")
val registration = model.add(listOf(xA, xB))
// Check registration before proceeding; propagate any error.

fun p(vararg terms: LinearMonomial<Flt64>) =
    LinearPolynomial<Flt64>(terms.toList(), Flt64.zero)
fun constant(value: Double) =
    LinearPolynomial<Flt64>(emptyList(), Flt64(value))

val material = p(
    LinearMonomial(Flt64(2.0), xA),
    LinearMonomial(Flt64(3.0), xB)
)
val hours = p(
    LinearMonomial(Flt64.one, xA),
    LinearMonomial(Flt64(2.0), xB)
)
val profit = p(
    LinearMonomial(Flt64(30.0), xA),
    LinearMonomial(Flt64(50.0), xB)
)

// Check each result before continuing to the next operation.
val nonnegativeA = model.addConstraint(
    LinearInequality(p(LinearMonomial(Flt64.one, xA)),
        constant(0.0), Comparison.GE), name = "xA_nonnegative"
)
val nonnegativeB = model.addConstraint(
    LinearInequality(p(LinearMonomial(Flt64.one, xB)),
        constant(0.0), Comparison.GE), name = "xB_nonnegative"
)
val materialRow = model.addConstraint(
    LinearInequality(material, constant(120.0), Comparison.LE),
    name = "material.capacity"
)
val hoursRow = model.addConstraint(
    LinearInequality(hours, constant(70.0), Comparison.LE),
    name = "processing.capacity"
)
val objective = model.maximize(profit, name = "profit")

// S1 only: after candidate validation and confirmation.
val minimumA = model.addConstraint(
    LinearInequality(p(LinearMonomial(Flt64.one, xA)),
        constant(20.0), Comparison.GE), name = "minimum.A"
)
```

```rust [Rust]
use ospf_rust_core::error::Result;
use ospf_rust_core::model::{
    LinearExpressionBuilder, MetaModel, ObjectiveCategory,
};
use ospf_rust_core::variable::{IntegerVariableItem, VariableRange};

// Inputs here have already passed application validation.
fn build_model(
    hours: f64,
    min_a: Option<u32>,
    min_b: Option<u32>,
) -> Result<MetaModel<f64>> {
    let mut model = MetaModel::<f64>::new("production");
    // Bounds follow from the fixed 120 kg material capacity.
    let a = model.register_variable(IntegerVariableItem::auto_with_range(
        "xA", VariableRange::bounded(0.0, 60.0),
    ))?;
    let b = model.register_variable(IntegerVariableItem::auto_with_range(
        "xB", VariableRange::bounded(0.0, 40.0),
    ))?;
    model.add_linear_constraint_input(
        LinearExpressionBuilder::new().term(a, 2.0).term(b, 3.0)
            .le(120.0, "material.capacity"),
    )?;
    model.add_linear_constraint_input(
        LinearExpressionBuilder::new().term(a, 1.0).term(b, 2.0)
            .le(hours, "processing.capacity"),
    )?;
    if let Some(q) = min_a {
        model.add_linear_constraint_input(
            LinearExpressionBuilder::new().term(a, 1.0)
                .ge(f64::from(q), "minimum.A"),
        )?;
    }
    if let Some(q) = min_b {
        model.add_linear_constraint_input(
            LinearExpressionBuilder::new().term(b, 1.0)
                .ge(f64::from(q), "minimum.B"),
        )?;
    }
    model.set_linear_objective_input(
        LinearExpressionBuilder::new().term(a, 30.0).term(b, 50.0)
            .maximize("profit").category(ObjectiveCategory::Maximum),
    );
    Ok(model)
}
// S0: build_model(70.0, None, None)
// S1: build_model(70.0, Some(20), None)
// S2: build_model(80.0, Some(20), None)
// S3: build_model(70.0, Some(20), Some(30))
```

:::

Kotlin 的非负约束行表达变量取值域；Rust 使用非负范围，并加入由材料容量推导出的冗余上界。两者可行域一致，均未引入辅助变量。

API 依据：[Kotlin MetaModel](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MetaModel.kt)、[Rust MetaModel](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/model/meta_model.rs)。这些链接指向 API 实现，不表示已有独立发布的本教程示例源码。

Kotlin 的求解流程需要将元模型转为机制模型、再转为求解模型并调用求解器，不能虚构 `model.solve()` 捷径。Rust 提供 `meta_model.solve_report(&solver)`。应用必须配置实际后端并传播报告；SCIP/Gurobi 需要相应运行依赖及适用的许可证。以上片段已按源码 API 核对，但本次文档变更尚未对这些片段进行 OSPF 编译和真实求解器运行验证。

## 5. 执行、结果与证据

构建 S1 时添加最低产量行；构建 S2 时从同一冻结输入复制参数，将工时右端替换为 80，重新创建变量和模型。不要向已有模型追加一个 80 小时约束而保留原来的 70 小时约束。

执行服务接收模型构建结果，配置明确的求解器、时间上限与取消信号，保存实际报告和失败。结果映射从变量绑定表恢复 A/B 产量，再按同一模型输入求中间值。约束与字段的稳定 ID 使 `material.capacity` 可以回到业务说明。

报告中的问题状态、终止原因、解存在性分别保存。只有存在可用解时才计算方案 KPI；无解报告不是零产量方案。诊断项要保留来源、支持能力和缺失原因。应用自行计算的本例矛盾证明与求解器提供的 IIS 分别标注。

解释适配器只接收授权证据，例如运行 ID、资源 lhs/rhs/slack、目标分项与基线差异。自然语言不作为状态字段；provider 超时不丢弃已完成的求解结果，用户仍可读取结构化表格。

## 6. 从 IR 产生可缓存计划

### 6.1 参数提取与依赖键

`Minimum(A,20)` 与 `Minimum(A,25)` 分别生成不同具体请求，参数化为同一形状：

```text
shape = Minimum(product: ProductId, quantity: NonnegativeInteger[piece])
parameters(request1) = [A, 20]
parameters(request2) = [A, 25]
plan = resolve product field → emit GE coefficient 1, RHS parameter[1]
```

形状编码固定操作版本、类型、单位、时间语义和算法放置位置。请求哈希另包含实际参数与输入引用。相同形状不能跨不兼容目录或权限规则直接复用。

注册表选择键为 `tenantScope + shapeHash + dependencyKey`。依赖至少包括目录语义、权限策略、模型语义、编译器与适配器版本；数据库查询还包括数据源 schema 版本。认证身份来自会话，不能来自形状中的用户字符串。

计划保存参数槽、预解析描述和绑定步骤，不保存变量索引、求解器句柄或旧结果。绑定顺序属于版本化计划，不能依赖映射遍历顺序；重新注册变量后只使用当前上下文返回的索引。

### 6.2 两条执行路径

下面是语言无关的算法伪代码，所列方法是示例端口，不是 OSPF API：

```text
execute(candidate, authenticatedSession):
    validated = validate(candidate, authenticatedSession, frozenCatalog)
    shape, parameters = canonicalizeAndExtract(validated)
    key = (authorizedScope, shape.hash, dependencyVersions)
    plan = registry.findActive(key)
    if plan exists and verifyDigestAndDependencies(plan):
        executable = plan
        path = Specialized
    else:
        executable = genericCompiler.compile(validated)
        path = Generic
    recheckAuthorizationAndVersions(validated, executable)
    context = createFreshExecutionContext(validated.baseline)
    result = bindAndExecute(executable, parameters, context)
    appendRun(path, key, planVersionIfUsed, inputRefs, result)
    return result
```

注册表失效并不等于候选合法。通用编译也拒绝未知字段或不支持能力，回退不放宽检查。专用绑定尚未开始时可以安全切换通用路径；执行已经开始或是否完成不明时，先确定运行状态，不能盲目重复执行。只读求解重试也需保留新 runId 与父运行，正式副作用使用独立幂等协议。

专门化能减少重复解析、字段解析和编排，不自动减少整数规划搜索量。先分别记录 LLM、解析、校验、建模、求解耗时，再决定应优化哪一段。

## 7. 查询与流程如何使用同一运行时

### 7.1 查询计划

`ReadRun(runId,fields)` 绑定的是来源快照，不创建 OSPF 模型。编译固定字段访问器和结果映射，执行时检查目标运行归属、字段权限和预算。不能将 planId 当作缓存结果键。

首版不需要 Join 或 SQL。增加数据库实现时，字段到列的映射由适配器登记；只下推可证明等价的表达式，其余保留残余求值或拒绝。空值、行粒度、排序、分页与授权语义都参与等价性验收。

### 7.2 流程计划

`CompareHours(base,deltas)` 展开为已登记的 ReadBaseline、CreateHoursCandidates、SolveCandidates、CompareResults 四节点。编译器验证图无环、端口类型、小时单位、候选数量、节点版本和结果依赖。

执行上下文固定一个基线；每个增量独立创建候选，而不是在前一候选上累加工时。可并行运行求解节点，但按候选 ID 汇总，保留超时、取消和无解状态。比较节点不能将缺失报告转换为 0 元收益。

沙箱使用真实纯计算与求解端口；不注册正式发布、生产下发或外部写入端口。端口缺失时拒绝执行，不能临时反射查找同名处理器。

## 8. 异步构建、发布与恢复

注册表建议拆分不可变 `PlanVersion`、追加式 `PlanEvent` 和可原子更新的 `ActivePointer`。版本记录包括参数 schema、规范 IR、依赖、摘要、验证引用、创建者和回退引用。活动指针只是索引，不替代历史事实。

1. 运行指标在租户范围按形状聚合；满足策略后创建幂等构建任务。
2. 构建任务以 `shape + dependencies` 去重，不在请求线程同步编译。
3. 候选计划用固定样本与边界反例验证；影子结果不影响通用响应。
4. 发布时再次检查依赖，以预期版本/CAS 原子切换活动指针并记录事件。
5. 失败版本保留诊断，不能留下部分 ACTIVE 状态；旧兼容版本或通用路径继续服务。
6. 目录、策略或 schema 变化按依赖索引失效计划；重复事件幂等。
7. 重启重新读取活动指针，校验依赖与摘要后装载，不能无条件恢复内存缓存。
8. 历史回放选择精确版本与原始快照；当前执行只能使用当前兼容版本。

多实例发布需要事务或等价的一致性协议；文件缓存不能充当发布锁。计划撤销和回滚追加事实，不覆盖历史记录。用户显式调用能力 ID 时也需经过同一授权入口。

## 9. 验证分层与交付边界

| 层 | 应有的测试 | 不能用什么替代 |
|---|---|---|
| Decoder | 未知字段、整数溢出、嵌套深度、错误单位 | 仅测试合法 JSON |
| Validator | 权限、时段、过期上下文、确认记录 | LLM 声称请求合法 |
| Compiler | IR 对应系数、关系、右端和变量域 | 仅检查模型对象创建成功 |
| Binding | 同一模板绑定两个新模型，变量互不串用 | 复用第一次的索引 |
| Solver adapter | S0/S1=1900、S2=1930、S3 不可行及终止状态 | 用枚举器冒充真实后端测试 |
| Runtime | 命中/未命中、撤销、重启、并发发布、回退 | 只测试内存 map 查询 |
| Equivalence | 通用/专用的数值、权限、预算与错误一致 | 只比结果哈希或只比耗时 |
| LLM adapter | 候选生成、澄清、非法输出、超时和成本 | 确定性 fixture 替代真实模型评测 |

建议实现顺序是：固定候选解码 → 验证 → IR 编译 → 真实 OSPF 求解 → 证据回传 → 通用运行记录 → 参数化计划 → 注册表与影子验证 → 发布、失效与恢复。每一步保持可独立回放，不把一次请求的哈希生成称为仿真完成。

本页类型与算法说明的是实现契约；目前提供的是文档及 API 片段，尚未交付完整运行时实现或真实求解集成测试。下一步工程实现应在对应 Kotlin/Rust 示例仓库中提供独立文件和测试，再把实际可运行入口链接到本页，不能预先链接不存在的示例。

回到[业务路径](./integrating-llms)查看完整模型、方案比较与能力形成的九步验收轨迹。
