# 时间与资源排程（Gantt）：适用问题与学习路线

为活动选择执行方式和开始时间，同时安排设备、人员等资源，并满足依赖、工作日历与物料事件。第一课使用有限候选活动 MILP，不需要把活动先变成路径。

## 最少需要理解的对象

| 业务概念 | API | 角色 | 核心字段 |
| --- | --- | --- | --- |
| 活动 | `Activity` | 输入 | 业务工作单元；取消需显式声明代价 |
| 执行模式 | `ActivityMode` | 输入 | 时长、允许起点、费用和资源占用段 |
| 资源 | `ActivityResource` | 输入 | 容量、不可用区间与分时容量 |
| 依赖 | `ActivityDependency` | 输入 | 活动端点关系及时间界限 |
| 排程问题 | `ActivitySchedulingProblem` | 输入 | 活动、资源、依赖、实际执行记录与基准排程 |
| 排程结果 | `ActivitySchedule` | 输出 | 任务模式与时间、资源占用、取消、费用与实际执行记录 |

允许起点定义了业务允许的开始时间，不是搜索次数上限：只给 08:00 和 10:00，就不会允许 09:00 开始。资源占用采用左闭右开区间 `[start, end)`，即包含开始时刻、不包含结束时刻；起点可用不代表执行全程可用。

::: tip 取消与资源阶段的空值
`cancellationCost = null` / `cancellation_cost = None` 表示任务必须执行；0 表示允许免费取消。`ActivityResourceUse.until` 为空表示占用持续到模式结束，不是零时长。
:::

## 从哪个入口调用？

::: info Kotlin 与 Rust 的入口差异
两种语言都有 `ActivitySchedulingApplicationService`。独立服务和嵌入联合模型的上下文对象使用相同的领域建模步骤。虽然接口名为 `ColumnGenerationSolver`，本课只调用它的整数求解能力，不执行列生成。
:::

排产层级的划分不会改变本课活动服务的调用方式。批量任务与列生成排程可在熟悉基本活动模型后继续学习，算法名称见[术语说明](../#terminology)。

有限起点活动排程无需切换列生成入口。允许起点是业务可选集合，不能当作最大节点数使用；求解时间/节点限制属于具体后端配置。先用少量起点复核日历，再按业务时间粒度扩展，[第二课](./business-scenario#entry-budget)保留相同服务入口。

## 能力边界

- 不承诺任意连续时间排程、自动抢占/分批或完整行业法规。
- 工作日历不可用区间不会自动转化为“暂停后继续执行”。
- 资源容量和物料库存需要分别建模；同一技能组的总人数限制，不能替代“同一名人员不能同时执行两个活动”的约束。
- 普通 Gantt 不强制依赖 Network；跨地点行驶子问题才需要评估组合。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次活动排程](./first-schedule)，检查模式、依赖和资源占用。
3. 加入停机、基准排程调整与物料事件，区分已实际执行记录和待优化决策。
4. 阅读[滚动优化](/zh-cn/guide/rolling-optimization)，再看[航班恢复综合案例](/zh-cn/examples/framework-example4)。

## 接口参考

- [Kotlin 活动模型与接口](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-domain-activity-context/README_ch.md)
- [Rust 活动模型与接口](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-gantt-scheduling/src/domain/activity/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[第一次活动排程](./first-schedule)。完成后再学[停机日历与滚动重排](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
