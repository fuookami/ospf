# 第一次一维装箱：物品、间距与库存

本课直接写业务示例，不以运行仓库测试作为前置步骤。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

有 3 件长条物品，每件长 40 cm、重 2 kg。small 容器长 1 m、承重 5 kg，large 长 2 m、承重 10 kg；各有一个。相邻物品留 20 cm，目标是最少容器数。

两种语言使用同一个业务问题。Kotlin 保留 cm/m 混合输入，由 adapter 转换；Rust 显式统一为 m/kg。注意 `amount` 是必须装入的精确件数，`inventory = 0` 与不限制库存不同。

## 2. 构造输入并调用框架

Kotlin 调用 `Bpp1dMilp.solve`。Rust 先创建经过校验的 `PackingContext`，再把实际模型、剩余时限与取消句柄交给注入的 `LinearSolver`。下面没有测试 solver 或预设解。

下面是示例函数的完整源码（含导入），不是测试代码。Kotlin 的 `runExample` 由应用注入真实 `ColumnGenerationSolver`；独立程序的启动方式见下一节。

::: code-group

<<< ../../../frameworks/_examples/bpp1d.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp1d.rs [Rust]

:::

## 3. 在自己的程序中启动

将 Kotlin 示例放在应用源码中，保留其 package。下面这个独立入口调用本课函数；需添加领域 starter、Gurobi 插件和 coroutines，配置匹配的原生库与许可证。这里使用的是生产插件，不是测试中的 delegating wrapper。

```kotlin
import docs.examples.bpp1d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/bpp1d.rs`，在 `src/main.rs` 使用以下入口。依赖与 toolchain 见环境页；同步示例不启用 `async`。

```rust
mod bpp1d;
use ospf_rust_core::solver::solvers::scip::SCIPSolver;

fn main() -> Result<(), String> {
    bpp1d::run_example(&SCIPSolver::new())
}
```

示例在 CLI 边界用异常或错误返回结束本次调用；服务端应在请求边界转换错误，并保留诊断。正常无 incumbent 要保留报告，不伪造成技术异常或空的成功方案。

## 4. 读懂并验收结果

正确的可行方案只使用一个 large：物品起点为 0、60、120 cm，最后一件终点为 160 cm，总重 6 kg。没有第三个尾间距。读取状态/报告后，按 ID 统计件数，并复核尺寸、承重和库存；不要只看目标值为 1。

## 5. 修改一个业务条件

把 large 的库存改为零。只剩一个 small，无法装入全部 3 件；应用应展示无完整可行方案，而不是复用上次结果。再恢复库存，将目标改为 `Cost`，观察费用目标与箱数目标的区别。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时再查[仓库验证参考](../verification)，测试命令不是本课的业务入口。
