# 停机日历与滚动重排：保留事实，再优化未来

完成[第一次活动排程](./first-schedule)后，先增加停机与分时容量，再处理实际执行记录和基准排程。基准排程是用于比较调整幅度的原计划；已经执行的活动则是必须保留的事实，不能仅靠高额罚费鼓励模型保留。

## 1. 用一个活动识别“只检查起点”的错误

活动持续 2 h，全程占设备容量 2；允许起点为 0、2、4 h。设备正常容量为 2，在 `[1,2)` 停机，在 `[3,4)` 容量降为 1。

| 开始 | 占用区间 | 是否可行 | 原因 |
| --- | --- | --- | --- |
| 0 h | [0,2) | 否 | 跨越停机段 |
| 2 h | [2,4) | 否 | 后半段容量不足 |
| 4 h | [4,6) | 是 | 全程容量充分 |

沿用第一课的 `at`、`q` 和资源 ID，把停机与容量覆盖显式传入 `ActivityResource`。注意左闭右开区间：4 h 开始不占用 `[3,4)`。

## 2. 把容量日历接到资源，不是活动标签

### 构造与表格一致的活动和日历

完整函数定义 `at` 为统一时间原点后的小时、`q` 为无量纲容量；资源、占用量和允许起点一起构造，因此可直接验证 0、2、4 h 三个选择。

::: code-group

<<< ../../../frameworks/_examples/gantt-business.kt#input [Kotlin]

<<< ../../../frameworks/_examples/gantt-business.rs#input [Rust]

:::

| 业务含义 | Kotlin | Rust |
| --- | --- | --- |
| 停机区间 | `ActivityResource.unavailableTimes` | `ActivityResource.unavailable_times` |
| 分时容量 | `ActivityResource.capacities` | `ActivityResource.capacities` |
| 相对阶段占用 | `ActivityResourceUse` | `ActivityResourceUse` |
| 实际已执行状态 | `ActivityExecution` | `ActivityExecution` |
| 基准排程与冻结 | `ActivityBaseline` | `ActivityBaseline` |

设备和技术员可能只在活动的部分阶段占用。不要为了方便，把所有资源都锁定整个活动；也不能只检查开始时设备空闲。输入中的占用阶段必须与真实业务一致。

在第一课 `runExample` 中构造如下资源，并作为 `problem.resources` 的 machine 配置；`at`、`q`、`machine` 沿用该函数：

```kotlin
val calendarResource = ActivityResource(
    id = machine, capacity = q(2.0),
    unavailableTimes = listOf(TimeRange(at(1), at(2))),
    capacities = listOf(ActivityCapacity(TimeRange(at(3), at(4)), q(1.0)))
)
```

## 3. 在滚动时刻划清三类数据

以现在为 0 h：一个运行中的活动在 -1 h 开始、2 h 完成，在未来窗口占设备 `[0,2)`；一个已完成活动在 -3 至 -2 h 执行；其余活动仍待决策。

- **实际执行记录**：保留原始开始/完成与状态；只把剩余资源占用放入未来窗口，不能重新排成 0 h 开始的完整任务。
- **冻结选择**：已经承诺的开始/模式属于硬限制；新停机与其冲突时，应报告冲突/不可行，而不是为了低费用偷偷移动。
- **可调整基准排程**：允许改动，但按实际配置计调整代价；基准排程不是强制实际执行记录，也不会自动生成新允许起点。

把实际执行记录传入 `executions`，把承诺的选择与可调整基准传入 `baselines`；依赖引用必须保留相同任务 ID。不要把已完成活动重新加入待排活动列表，也不要把运行中活动的完整资源占用再次计入未来窗口。

### 保持入口，区分允许起点与搜索上限 {#entry-budget}

将上述问题传回第一课的活动服务即可，无需切换列生成入口。`starts` 定义全部允许起点，不是搜索次数上限。求解时间和节点数上限在实际后端适配器中配置；达到时限时，应同时记录是否已找到可行解。

::: code-group

<<< ../../../frameworks/_examples/gantt-business.kt#entry [Kotlin]

<<< ../../../frameworks/_examples/gantt-business.rs#entry [Rust]

:::

## 4. 做两次受控变更

### 最小扩展：晚起点加收费用

本例另加“4 h 及以后开始收费 2”的偏好。Kotlin 用 `additionalCost` 注入 `ActivitySchedulingProblem`；Rust 的闭包实现 `ActivityCostPolicy`，在创建上下文对象时传入。容量仍是硬约束，附加费用只影响合法选择的排序。

::: code-group

<<< ../../../frameworks/_examples/gantt-business.kt#rule [Kotlin]

<<< ../../../frameworks/_examples/gantt-business.rs#rule [Rust]

:::

1. 在上表中删掉 4 h 起点。若活动必须执行且不能取消，当前允许起点中没有可行选择；框架不会自动增加 6 h 起点，也不会自动把活动改成可中断执行。
2. 恢复起点，先添加一个冻结选择，再让它与停机重叠。确认正常不可行与输入技术错误分开处理，并明确由谁授权解除冻结。

新增起点、换资源、允许取消是三种不同业务决定，要分别配置和验收。取消活动还需检查前后依赖与物料事件，不能仅从画面删除条形。

## 5. 重排交付验收

保留已完成活动的执行记录、运行中活动的原始时间和剩余占用；冻结选择不变；可调整活动只在允许候选中选择；全程资源、依赖和库存符合要求；调整费用可重算。达到时限后返回的可行解仍需标明时限状态。

滚动过程中使用输入版本或请求号避免旧结果覆盖新状态。更多建模约定见[滚动优化](/zh-cn/guide/rolling-optimization)，应用边界见[接入指南](../integration)。

源码回归命令统一放在[仓库验证参考](../verification)；先完成业务输入、调用与结果复核，再按需验证。


## 完整应用源码

片段来自同一个源码文件；保留导入声明和辅助函数后，从应用调用 `runBusiness` / `run_business`。构造函数也可独立用于输入校验。后端依赖沿用第一课；这些入口不创建隐式求解后端。

:::: details 展开完整 Kotlin / Rust 源码

::: code-group

<<< ../../../frameworks/_examples/gantt-business.kt [Kotlin]

<<< ../../../frameworks/_examples/gantt-business.rs [Rust]

:::

::::
