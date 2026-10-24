# 第一次卷材分切：需求与方案使用量

本课直接写业务示例，不以运行仓库测试作为前置步骤。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

生产宽 0.8 m 的产品，需求 3 卷。原料幅宽范围 0.5–1.5 m，先不配置机器。这里的 3 是卷数，不是长度 3 m，也不是方案使用次数。Rust 示例另设原料可用批次为 3。

使用 `ProductDemand.legacyRoll` / `legacy_roll` 显式声明卷数。每个方案有自己的产品贡献，实际产量是“贡献 × 使用量”的汇总；生成了三个候选不代表已经生产三卷。

## 2. 构造输入并调用框架

Kotlin 使用实际 solver 的普通入口，并直接返回结构化 `Csp1dSolution`，不是 `Ret`。Rust 默认入口无需原生求解器，但内部使用启发式，不能从方法名 `Milp` 推断调用了原生 MILP。需要原生主问题时再配置对应 solver 接口。

下面是示例函数的完整源码（含导入），不是测试代码。Kotlin 的 `runExample` 由应用注入真实 `ColumnGenerationSolver`；独立程序的启动方式见下一节。

::: code-group

<<< ../../../frameworks/_examples/csp1d.kt [Kotlin]

<<< ../../../frameworks/_examples/csp1d.rs [Rust]

:::

## 3. 在自己的程序中启动

将 Kotlin 示例放在应用源码中，保留其 package。下面这个独立入口调用本课函数；需添加领域 starter、Gurobi 插件和 coroutines，配置匹配的原生库与许可证。这里使用的是生产插件，不是测试中的 delegating wrapper。

```kotlin
import docs.examples.csp1d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/csp1d.rs`，在 `src/main.rs` 使用以下入口。依赖与 toolchain 见环境页；同步示例不启用 `async`。

```rust
mod csp1d;

fn main() {
    csp1d::run_example();
}
```

示例在 CLI 边界用异常或错误返回结束本次调用；服务端应在请求边界转换错误，并保留诊断。正常无 incumbent 要保留报告，不伪造成技术异常或空的成功方案。

## 4. 读懂并验收结果

先读取 `status`、失败信息与 `unmetDemands` / `unmet_demands`，再读取选中方案及使用量。按卷数口径重算 P1 的产出，复核每个方案幅宽与原料批次；候选池非空不等于覆盖需求，部分结果也不能直接作为完整交付。

## 5. 修改一个业务条件

先把需求从 3 卷改成 4 卷，重新检查可用批次与方案贡献。再尝试重量需求：不能只把标签从“卷”改成“kg”，还需给出单位重量、卷长等转换依据，并同步修改验收口径。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时再查[仓库验证参考](../verification)，测试命令不是本课的业务入口。
