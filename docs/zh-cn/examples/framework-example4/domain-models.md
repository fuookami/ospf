# Framework 示例 4：上下文领域模型索引

[English](../../examples/framework-example4/domain-models.md)

Demo4 是架构示例而不是完整的 Kotlin 应用：`Application` 为空，只有
`Demo4GenericQuantitySample` 可执行。因此本索引把上下文契约和应用编排分开。
下面的本地模型页是维护性的领域模型文字；数据型和编排型上下文明确记录其不拥有
独立求解变量，而不是虚构一个全局模型。

## 1. 上下文图

```text
task + rule + crew + cargo → bunch_generation → bunch_compilation
                                      ↘ bunch_selection（分支定价策略）
passenger ───────────────────────────→ bunch_compilation（可选管线）
```

箭头表示数据和服务依赖，不表示 `Application` 会在同一个模型中注册所有上下文。

## 2. 上下文模型页面

| 上下文 | 模型边界 | 本地模型 |
| --- | --- | --- |
| task | 航班任务、航段、航班周期、飞机和恢复数据；供生成与编译使用 | [task](domain-task/domain-model)；`Application` 不注册独立优化模型 |
| rule | 航段连接、限制、流控制以及可行性/时间/成本计算器 | [rule](domain-rule/domain-model)；由 bunch generation 使用的谓词 |
| crew | 机组成员、等级、排班和转场时间数据 | [crew](domain-crew/domain-model) |
| cargo | 货物数据和容量/中断占位模型 | [cargo](domain-cargo/domain-model) |
| passenger | 旅客数量、取消、舱等/航班变更及其目标和约束管线 | [passenger](domain-passenger/domain-model) |
| bunch_generation | 根据任务/规则/机组/货物数据和影子价格生成可行 bunch/列 | [bunch generation](domain-bunch_generation/domain-model)；生成服务，不是独立主问题模型 |
| bunch_compilation | 主模型列选择、任务/流/机队/航段/容量表达式和增量列 | [bunch compilation](domain-bunch_compilation/domain-model) |
| bunch_selection | 分支定价策略、约简成本回调和编排 | [bunch selection](domain-bunch_selection/domain-model)；算法服务，不新增独立变量族 |

## 3. 公共符号与主问题契约

令 `B_k` 为第 `k` 次迭代可用的生成 bunch 集合，`T` 为航班任务集合，`A` 为
飞机/执行器集合，`L` 为航段连接集合。对于选中的 bunch `b`，`cover(t,b)`、
`use(a,b)` 和 `link(l,b)` 是由源码提供的关联系数。编译上下文可以注册：

$$
x_b\in\{0,1\}\ (b\in B_k),\qquad
y_t\in\{0,1\}\ (t\in T),\qquad
z_a\in\{0,1\}\ (a\in A)。
$$

两个显式实现的限制族可以用非负松弛表示：

$$
slack^{link}_l\ge 1 \quad(l\in L^{selected}),
\qquad
slack^{fleet}_c\ge amount(c) \quad(c\in C^{checkpoint})。
$$

对应编译管线会最小化带系数的松弛。系数及生成列的形状在注册时由聚合根提供，
并非由本索引固定。

## 4. 上下文边界与证据

1. `BunchGenerationContext.generateFlightTaskBunch` 是定价回调：根据影子价格返回生成 bunch，
   它不是求解器约束。
2. `BunchCompilationContext.register` 创建线性元模型符号并返回生效管线。没有被该列表返回的模型类，不是生效断言。
3. `PassengerContext.register` 是独立注册路径；只有调用者显式调用它，才能把旅客模型加入编译模型。
4. `BranchAndPriceAlgorithm` 负责策略、界、约简成本和生成列的编排，不增加独立领域变量族。

父页面记录通用量可执行样例和 Kotlin/Rust 实现状态。上下文级术语和公式可在本索引与拆分页中审阅，
实现行证据见父页面的源码链接。
