# 第一次板材套裁：需求、图案与工艺

本课直接写业务示例，不以运行仓库测试作为前置步骤。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

把矩形产品从定尺板材中裁出。Kotlin 为 100 × 60 m 板、40 × 30 m 产品、需求 3 件；Rust 为 10 × 8 m 板、2 × 2 m 产品、需求 20 件、原料可用 100 张。先用 Freeform、零刀缝。尺寸是便于手算的教学量值，不是生产规格。

两种输入都能构造一张板的可行排布。Kotlin 显式声明产品、件数需求和工艺；Rust 用 `Csp2dProblem::single` 装配单产品问题。它们不是同一个数值基准。

## 2. 构造输入并调用框架

Kotlin 的 `Csp2dMilp` 接入真实 solver。Rust 默认 `Csp2dSolvers::new().solve_milp` 使用启发式主问题，不需要原生后端；方法名不是原生执行或最优证明。需要真实优化时使用 `Csp2dMasterSolver` 接口。

下面是示例函数的完整源码（含导入），不是测试代码。Kotlin 的 `runExample` 由应用注入真实 `ColumnGenerationSolver`；独立程序的启动方式见下一节。

::: code-group

<<< ../../../frameworks/_examples/csp2d.kt [Kotlin]

<<< ../../../frameworks/_examples/csp2d.rs [Rust]

:::

## 3. 在自己的程序中启动

将 Kotlin 示例放在应用源码中，保留其 package。下面这个独立入口调用本课函数；需添加领域 starter、Gurobi 插件和 coroutines，配置匹配的原生库与许可证。这里使用的是生产插件，不是测试中的 delegating wrapper。

```kotlin
import docs.examples.csp2d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/csp2d.rs`，在 `src/main.rs` 使用以下入口。依赖与 toolchain 见环境页；同步示例不启用 `async`。

```rust
mod csp2d;

fn main() -> ospf_rust_base::error::Ret<()> {
    csp2d::run_example()
}
```

示例在 CLI 边界用异常或错误返回结束本次调用；服务端应在请求边界转换错误，并保留诊断。正常无 incumbent 要保留报告，不伪造成技术异常或空的成功方案。

## 4. 读懂并验收结果

分四层验收：需求覆盖、原料用量、几何合法、工艺合法。图案数量不是板材张数，应汇总使用量。Freeform 不要求切分树；断头台需要有效切分树及级数、区域和刀缝检查。默认 Rust 场景的手算参照为覆盖 20 件、用一张板、净面积余量为零；实际输出仍应复核。

## 5. 修改一个业务条件

先只增加正刀缝，原先刚好铺满的图案可能失效。再改两级断头台并重新生成图案，而不是给原坐标附加工艺标签。最后再切换卷材，并把主要用量从张数改为消耗长度。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时再查[仓库验证参考](../verification)，测试命令不是本课的业务入口。
