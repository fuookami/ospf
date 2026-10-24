# 第一次活动排程：模式、依赖与资源

本课通过业务示例学习输入、求解和结果解析。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

排活动 a、b，a 完成后 b 才能开始。a 可选 fast（2 h、machine、费用 3）或 slow（3 h、alternate、费用 1）；b 为 2 h、machine、费用 0。每个模式只允许在 0 h 或 2 h 开始，资源容量均为 1，活动必须执行。

slow 最早在 3 h 完成，b 没有 3 h 或更晚的起点，因此不能贪心选便宜模式。可行选择为 a/fast 在 0 h 开始、b 在 2 h 开始，总费用 3。两种示例使用相同相对时间；Kotlin 的 t0 是固定业务日期，Rust 用 UNIX_EPOCH 作为教学原点。

## 2. 构造输入并调用框架

::: info Kotlin 与 Rust 的入口差异
使用有限候选活动的单次 MILP 入口，不需要先实现任务束列生成。资源容量/占用与费用在本例统一为无量纲教学数值；生产数据应声明对应业务单位。Kotlin 返回外层 `Ret` 加排程报告，Rust 返回报告并保留可能不存在的业务解。
:::

示例包含导入和错误处理。Kotlin 的 `runExample` 接收应用提供的 `ColumnGenerationSolver`，下一节给出启动入口。

::: code-group

<<< ../../../frameworks/_examples/gantt.kt [Kotlin]

<<< ../../../frameworks/_examples/gantt.rs [Rust]

:::

## 3. 在自己的程序中启动

保留包声明，将 Kotlin 示例放入应用源码目录。下面的独立入口调用本课函数；需添加对应领域的集成依赖、Gurobi 插件和协程依赖，配置匹配的原生库与许可证。

```kotlin
import docs.examples.gantt.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/gantt.rs`，在 `src/main.rs` 使用以下入口。依赖与 工具链 见环境页；同步示例不启用 `async`。

```rust
mod gantt;
use ospf_rust_framework::solver::ScipColumnGenerationSolver;

fn main() -> Result<(), String> {
    gantt::run_example(ScipColumnGenerationSolver::new())
}
```

示例在命令行程序中通过抛出异常或返回错误结束失败调用。接入服务端时，将错误转换为结构化响应，并保留求解报告和诊断信息。求解器正常结束但未找到可行解时，按报告返回相应状态。

## 4. 读懂并验收结果

| 读取内容 | Kotlin | Rust |
| --- | --- | --- |
| 任务模式与时间 | `schedule.placements → key.task / key.mode / time.start / time.end` | `schedule.placements → key.task / key.mode / time.start / time.end` |
| 实际资源占用 | `placement.reservations → resource / time / quantity` | `placement.reservations → resource / time / quantity` |
| 费用与保留事实 | `cost / cancelled / executions / materialInventory` | `cost / cancelled / executions / material_inventory` |
| 求解状态与候选域 | `outcome.report`、`schedule.candidateScope` | `report`、`schedule.candidate_scope` |

::: info 结果语义
`candidateScope` / `candidate_scope` 是声明的允许选择集合，不是 `complete` 或全局最优标记。`schedule` 可能为空，此时保留求解报告；有排程时仍需检查取消、依赖与资源全过程。
:::

外层成功不代表一定有排程。先读取报告，再检查是否有业务解；有解时核对 a 的模式、两活动的起止、依赖、资源占用和费用。允许起点是业务可行域，不是求解器的搜索提示；增加时限不会自动生成 3 h 起点。

## 5. 修改一个业务条件

只把 b 的允许起点增加 3 h，此时可以考察 a/slow 在 0 h、b 在 3 h 的费用 1 方案。随后再给 machine 加停机日历，检查结果如何变化，不要一次加入冻结和滚动重排。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时查[仓库验证参考](../verification)。
