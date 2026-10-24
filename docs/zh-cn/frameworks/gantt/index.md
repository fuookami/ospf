# 时间与资源排程 Gantt

为活动选择执行方式和开始时间，同时安排设备、人员等资源，并满足依赖、工作日历与物料事件。第一课使用有限候选活动 MILP，不需要把活动先变成路径。

## 最少需要理解的对象

| 业务概念 | API | 输入或输出 |
| --- | --- | --- |
| 活动 | `Activity` | 业务工作单元；取消需显式声明代价 |
| 执行模式 | `ActivityMode` | 时长、允许起点、费用和资源占用段 |
| 资源 | `ActivityResource` | 容量、不可用区间与时变容量 |
| 依赖 | `ActivityDependency` | 活动端点关系及时间界限 |
| 问题与结果 | `ActivitySchedulingProblem` / `ActivitySchedule` | 候选、排程、占用、取消与库存 |

允许起点是业务域，不是搜索预算：只给 08:00 和 10:00，不会暗中允许 09:00 开始。资源占用采用半开区间，起点可用不代表执行全程可用。

## 从哪个入口调用？

两种语言都有 `ActivitySchedulingApplicationService`。独立服务与嵌入式 context 共享领域注册。复用 `ColumnGenerationSolver` 的 MILP 能力，不代表这个活动入口执行列生成。

APS / MPS / LSP 是业务层级标记，不是本课的独立求解入口。任务束、产能和分支定价是后续求解路径。

## 能力边界

- 不承诺任意连续时间排程、自动抢占/分批或完整行业法规。
- 工作日历不可用区间不会自动转化为“暂停后继续执行”。
- 联合资源和物料库存是不同契约；多人技能组容量不替代具名人员互斥。
- 普通 Gantt 不强制依赖 Network；跨地点行驶子问题才需要评估组合。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次活动排程](./first-schedule)，检查模式、依赖和资源占用。
3. 加入停机、基线调整与物料事件，区分已执行事实和待优化决策。
4. 阅读[滚动优化](/zh-cn/guide/rolling-optimization)，再看[航班恢复综合案例](/zh-cn/examples/framework-example4)。

## 接口参考

- [Kotlin 活动契约](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-domain-activity-context/README_ch.md)
- [Rust 活动契约](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-gantt-scheduling/src/domain/activity/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[停机日历与滚动重排](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
