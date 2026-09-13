# 复杂示例 4：上下文模型索引

[English](/examples/framework-example4/domain-models)

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
| task | 航班任务、航段、航班周期、飞机和恢复数据；供生成与编译使用 | [任务](domain-task/domain-model)；`Application` 不注册独立优化模型 |
| rule | 航段连接、限制、流控制以及可行性/时间/成本计算器 | [规则](domain-rule/domain-model)；由航班串生成使用的谓词 |
| crew | 机组成员、等级、排班和转场时间数据 | [机组](domain-crew/domain-model) |
| cargo | 货物数据和容量/中断占位模型 | [货物](domain-cargo/domain-model) |
| passenger | 旅客数量、取消、舱等/航班变更及其目标和约束管线 | [旅客](domain-passenger/domain-model) |
| bunch_generation | 根据任务/规则/机组/货物数据和影子价格生成可行航班串/列 | [航班串生成](domain-bunch_generation/domain-model)；生成服务，不是独立主问题模型 |
| bunch_compilation | 主模型列选择、任务/流/机队/航段/容量表达式和增量列 | [航班串编译](domain-bunch_compilation/domain-model) |
| bunch_selection | 分支定价策略、约化成本回调和编排 | [航班串选择](domain-bunch_selection/domain-model)；算法服务，不新增独立变量族 |

## 3. 公共符号与主问题契约

公共符号（`B_k`、`T`、`A`、`L`）和生成列关联系数在对应上下文页面中定义。编译页是注册
变量、松弛变量、约束和目标的来源；本索引不重复维护这些公式。

## 4. 上下文边界与证据

1. `BunchGenerationContext.generateFlightTaskBunch` 是定价回调：根据影子价格返回生成航班串，
   它不是求解器约束。
2. `BunchCompilationContext.register` 创建线性元模型符号并返回生效管线。没有被该列表返回的模型类，不是生效断言。
3. `PassengerContext.register` 是独立注册路径；只有调用者显式调用它，才能把旅客模型加入编译模型。
4. `BranchAndPriceAlgorithm` 负责策略、界、约化成本和生成列的编排，不增加独立领域变量族。

父页面介绍通用量可执行样例，并提供 Kotlin/Rust 源码入口。上下文级术语和公式可在本索引与拆分页中审阅，
实现行证据见下列源码链接。

5. [Kotlin Demo4 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo4)
6. [Rust Demo4 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo4.rs)
