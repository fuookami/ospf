# 使用领域驱动设计（DDD）架构

OSPF 中的领域驱动设计不是把一个大模型机械地拆成多个文件，而是把**领域知识、数学表达和软件职责**按有界上下文组织起来。每个上下文拥有自己的变量、具名中间值、目标与约束，并通过稳定的语义接口参与应用编排。

这种方法适合需要长期演进、由多个业务主题共同构成，或需要在单体 MILP、列生成、Benders 等求解路径之间复用领域知识的优化系统。一次性的教学模型仍可直接使用求解器或 OSPF Core 的脚本式接口。

## 1. 从业务问题到上下文

先用业务语言识别相对独立的决策主题，再决定边界；不要先按公式类型或求解器对象分包。

以服务器放置问题为例，可以识别两个有依赖但近似正交的上下文：

| 上下文 | 回答的问题 | 对外发布的语义 |
|---|---|---|
| Route | 使用哪些服务器、部署到哪些节点 | 节点是否部署服务器、服务器是否启用 |
| Bandwidth | 流量如何经过网络并满足需求 | 链路使用量、入流、出流、净流出 |

Bandwidth 可以依赖 Route 发布的“节点是否部署服务器”，但不应读取 Route 内部变量的数组下标或求解器句柄。新增多活、安全或可靠性规则时，可以增加上下文并复用原有接口，而不必重写两个既有上下文。

一个好的上下文边界通常同时满足：

- 名称来自业务统一语言，而不是 `constraint_1`、`x_group` 之类的技术名称；
- 对一组决策变量和派生量具有清晰所有权；
- 能列出自己的输入、输出、目标和约束；
- 对外依赖通过领域对象或具名中间值表达；
- 可以被独立初始化、注册和测试。

## 2. 分层和职责

典型应用分为三层：

| 层 | 主要职责 | 不应承担 |
|---|---|---|
| Domain | 实体、值对象、聚合、变量、中间值、业务规则与 Pipeline | 求解器选择、传输协议 |
| Application | 选择业务场景，编排 `init → register → construct → solve → analyze` | 重新实现领域公式 |
| Infrastructure | 求解器适配、持久化、DTO、远程调用和运行环境 | 决定领域边界 |

领域层中的常见构件如下：

| 构件 | 含义 |
|---|---|
| Context | 一个有界上下文的公开入口和生命周期 |
| Aggregation | 持有上下文共享的变量、中间值及领域对象 |
| Model | 表达实体、值对象及局部数学结构 |
| Service | 无法自然归属单个实体的领域规则或算法 |
| Pipeline | 可命名、可组合、可延迟注册的约束单元 |
| PipelineListGenerator | 按业务模式或求解路径组装 Pipeline |

## 3. 变量和中间值的所有权

在传统求解器代码中，变量通常由某个 solver model 创建并持有。这样做会把领域知识绑定到一次具体求解，跨上下文复用、构造多个模型或切换求解路径时都需要重新建立映射。

OSPF 的 DDD 模式把变量与中间值提升为领域对象的属性，再把它们注册到一个或多个元模型中：

~~~kotlin
class RouteAggregation(
    val nodes: List<Node>,
    val services: List<Service>
) {
    lateinit var assignment: BinVariable2
    lateinit var nodeAssigned: LinearIntermediateSymbols1<Flt64>
    lateinit var serviceAssigned: LinearIntermediateSymbols1<Flt64>

    fun register(model: AbstractLinearMetaModel<Flt64>): Try {
        // 创建或复用领域变量和中间值，再注册到本次求解模型。
        return model.add(assignment)
            .andThen { model.add(nodeAssigned) }
            .andThen { model.add(serviceAssigned) }
    }
}
~~~

这里的核心不是具体类型名，而是所有权方向：

$$
\text{Domain object} \longrightarrow
\{\text{variable},\ \text{intermediate value}\}
\longrightarrow \text{meta model}.
$$

元模型是编译和求解载体，不是领域语义的唯一拥有者。

## 4. 中间值是上下文接口

中间值是被命名、可复用的符号表达式。它应当：

1. 与展开后的匿名多项式或函数表达式语义等价；
2. 在表达式文法中像变量一样被引用；
3. 在一次领域模型生命周期内具有稳定名称和全局可见性；
4. 隐藏其内部是常量、变量还是复合表达式。

例如 Route 上下文可以发布：

$$
\operatorname{NodeAssigned}_i
= \sum_{s \in S} x_{is},
\qquad
\operatorname{ServiceAssigned}_s
= \sum_{i \in N} x_{is}.
$$

Bandwidth 上下文只使用 `NodeAssigned` 的含义来约束流量：

$$
\operatorname{OutFlow}_i
\le
\operatorname{MaxOutBandwidth}_i
\operatorname{NodeAssigned}_i.
$$

依赖关系因此变成：

$$
\text{Route context}
\xrightarrow{\text{NodeAssigned}}
\text{Bandwidth context}.
$$

同一个中间值还可以有多态实现。例如“预计业载”在全配载模式中来自装载决策，在预配载模式中来自计划或估计量，在建议打板模式中来自推荐重量变量。下游适航或重心上下文只依赖“预计业载”的语义，不需要知道当前实现。

## 5. 用 Pipeline 表达约束

直接调用 `addConstraint` 会产生难以检索和组合的副作用。Pipeline 把约束提升为可测试的一等对象：

~~~kotlin
class NodeAssignmentLimit(
    private val aggregation: RouteAggregation
) : LinearPipeline<Flt64> {
    override fun invoke(model: AbstractLinearMetaModel<Flt64>): Try {
        for (node in aggregation.nodes.indices) {
            model.addConstraint(
                aggregation.nodeAssigned[node] leq 1,
                name = "node_assignment_$node"
            )
        }
        return ok
    }
}
~~~

`PipelineListGenerator` 决定某种场景包含哪些规则；Context 负责注册；Application 只决定采用哪种场景。这样，新增约束通常落在对应上下文，而不会扩散到主求解流程。

## 6. 应用生命周期

推荐把一次求解显式分成六个阶段：

| 阶段 | 输入与输出 | 失败时应报告 |
|---|---|---|
| `init` | DTO → 领域对象与上下文 | 数据、单位、索引或前置条件错误 |
| `register` | 上下文 → 变量、中间值、Pipeline | 重复注册、缺少依赖或模式不兼容 |
| `construct` | 符号模型 → 求解器可执行模型 | 表达式或能力不受后端支持 |
| `solve` | 可执行模型 → 状态、目标值、变量值 | 不可行、无界、超时或求解器故障 |
| `analyze` | 原始数值 → 领域解 | 解缺失、数值容差或映射错误 |
| `diagnose` | 日志与指标 → 可解释诊断 | 规则名称、上下文和求解轨迹 |

~~~kotlin
suspend fun invoke(request: RequestDTO): Ret<ResponseDTO> {
    routeContext.init(request)
    bandwidthContext.init(request, routeContext.aggregation)

    routeContext.register(metaModel)
    bandwidthContext.register(metaModel)

    metaModel.construct()
    val solution = solver.solve(metaModel)

    return analyze(solution)
}
~~~

应用层可以组合不同上下文，但不应越过 Context 去修改其内部变量或拼接其约束。

## 7. 一个上下文的文档契约

每个上下文的数学模型文档至少应包含以下章节，这也是复杂示例子页面采用的格式：

1. **上下文目标与边界**：解决什么问题，不解决什么问题；
2. **输入与假设**：集合、索引、常量、单位和前置条件；
3. **决策变量**：符号、定义域、维度、业务含义及所有者；
4. **中间值**：公式、含义、发布者和使用者；
5. **目标函数**：优先级、方向、量纲和业务解释；
6. **约束**：名称、公式、适用条件和边界行为；
7. **上下文协作**：输入/输出协议及依赖方向；
8. **构建与求解**：注册顺序、求解路径和降级策略；
9. **解分析**：如何恢复领域结果；
10. **验证**：单元测试、集成测试、基准实例和追溯关系。

文档中的每个变量、中间值、目标和约束都应能追溯到实现；实现中的公开数学构件也应在文档中找到对应项。

## 8. 测试边界

DDD 并不会自动保证模型正确。建议分层验证：

- **Model/值对象测试**：单位换算、索引和派生属性；
- **Intermediate value 测试**：具名表达式与手工展开式等价；
- **Pipeline 测试**：约束数量、系数、名称、边界值和启用条件；
- **Context 测试**：初始化、注册幂等性及缺失依赖；
- **Application 测试**：上下文组合、求解路径和解分析；
- **基准测试**：与已知最优值、下界或人工可验证实例比较；
- **后端契约测试**：不同求解器在状态、容差和结果映射上保持一致。

## 9. 常见误区

- **按变量、约束、目标分上下文**：这是技术分类，不是领域边界。
- **跨上下文直接访问内部数组**：会把调用方绑定到实现；应发布具名中间值或领域协议。
- **Context 自己选择求解器**：基础设施决策泄漏到了领域层。
- **Application 重新写公式**：领域规则出现两个事实来源。
- **只拆目录，不拆所有权**：变量和规则仍由一个全局对象控制，无法独立测试或复用。
- **把中间值当缓存**：它首先是数学语义接口；性能优化不能改变定义。

## 10. 延伸阅读

- [DDD 运筹组件模式与传统模式对比](https://github.com/fuookami/ospf-kotlin/blob/main/docs/ddd.md)
- [复杂示例 1：服务器放置问题](/zh-cn/examples/framework-example1)
- [使用 DDD 架构实现列生成](/zh-cn/guide/use-ddd-architecture-with-column-generation)
- [使用 DDD 架构实现 Benders 分解](/zh-cn/guide/use-ddd-architecture-with-benders)
- [数学模型的演绎逻辑表达](/zh-cn/guide/deductive-logic-expression)
