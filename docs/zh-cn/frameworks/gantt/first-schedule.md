# 第一次活动排程：模式、依赖与资源

本课直接写业务示例，不以运行仓库测试作为前置步骤。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

排活动 a、b，a 完成后 b 才能开始。a 可选 fast（2 h、machine、费用 3）或 slow（3 h、alternate、费用 1）；b 为 2 h、machine、费用 0。每个模式只允许在 0 h 或 2 h 开始，资源容量均为 1，活动必须执行。

slow 最早在 3 h 完成，b 没有 3 h 或更晚的起点，因此不能贪心选便宜模式。可行选择为 a/fast 在 0 h 开始、b 在 2 h 开始，总费用 3。两种示例使用相同相对时间；Kotlin 的 t0 是固定业务日期，Rust 用 UNIX_EPOCH 作为教学原点。

## 2. 构造输入并调用框架

使用有限候选活动的单次 MILP 入口，不需要先实现任务束列生成。资源容量/占用与费用在本例统一为无量纲教学数值；生产数据应声明对应业务单位。Kotlin 返回外层 `Ret` 加排程报告，Rust 返回报告并保留可能不存在的业务解。

下面是示例函数的完整源码（含导入），不是测试代码。Kotlin 的 `runExample` 由应用注入真实 `ColumnGenerationSolver`；独立程序的启动方式见下一节。

::: code-group

<<< ../../../frameworks/_examples/gantt.kt [Kotlin]

<<< ../../../frameworks/_examples/gantt.rs [Rust]

:::

## 3. 在自己的程序中启动

将 Kotlin 示例放在应用源码中，保留其 package。下面这个独立入口调用本课函数；需添加领域 starter、Gurobi 插件和 coroutines，配置匹配的原生库与许可证。这里使用的是生产插件，不是测试中的 delegating wrapper。

```kotlin
import docs.examples.gantt.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/gantt.rs`，在 `src/main.rs` 使用以下入口。依赖与 toolchain 见环境页；同步示例不启用 `async`。

```rust
mod gantt;
use ospf_rust_framework::solver::ScipColumnGenerationSolver;

fn main() -> Result<(), String> {
    gantt::run_example(ScipColumnGenerationSolver::new())
}
```

示例在 CLI 边界用异常或错误返回结束本次调用；服务端应在请求边界转换错误，并保留诊断。正常无 incumbent 要保留报告，不伪造成技术异常或空的成功方案。

## 4. 读懂并验收结果

外层成功不代表一定有排程。先读取报告，再检查是否有业务解；有解时核对 a 的模式、两活动的起止、依赖、资源占用和费用。允许起点是业务可行域，不是 solver 的搜索提示；增加时限不会自动生成 3 h 起点。

## 5. 修改一个业务条件

只把 b 的允许起点增加 3 h，此时可以考察 a/slow 在 0 h、b 在 3 h 的费用 1 方案。随后再给 machine 加停机日历，检查结果如何变化，不要一次加入冻结和滚动重排。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时再查[仓库验证参考](../verification)，测试命令不是本课的业务入口。
