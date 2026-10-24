# 第一次二维排样：从矩形开始

本课直接写业务示例，不以运行仓库测试作为前置步骤。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

把 R1（4 × 3 m，6 件）和 R2（2 × 2 m，8 件）排入 20 × 12 m 的标准板，允许旋转，不加刀缝。每件 1 kg，板材承重 10000 kg。

净面积为 104 m²，小于一张板的 240 m²；但面积足够不证明几何可放置。物品目录、需求和板型必须使用一致的 ID，不能按尺寸把不同订单合并。

## 2. 构造输入并调用框架

Kotlin 示例完整构造物品、板型和请求，并调用公开应用服务。Rust 示例是明确的接入边界：调用者传入领域输入与 RMP/final 两个真实适配器；它不是开箱即用的独立程序。当前 Rust 服务接口固定为 `f64 + Meter`，物品重量与长度还共用单位类型，业务接入必须核对这一边界；不能照抄测试里用米表示重量的简化。`StandardRmpSolver`/`StandardFinalSolver` 只是闭包适配器，不是自动连接 SCIP 的实现。

下面是示例函数的完整源码（含导入），不是测试代码。Kotlin 的 `runExample` 由应用注入真实 `ColumnGenerationSolver`；独立程序的启动方式见下一节。

::: code-group

<<< ../../../frameworks/_examples/bpp2d.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp2d.rs [Rust：接入边界]

:::

## 3. 在自己的程序中启动

将 Kotlin 示例放在应用源码中，保留其 package。下面这个独立入口调用本课函数；需添加领域 starter、Gurobi 插件和 coroutines，配置匹配的原生库与许可证。这里使用的是生产插件，不是测试中的 delegating wrapper。

```kotlin
import docs.examples.bpp2d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Rust 请把 `solve_layout` 作为应用适配层：先完成输入归一与 RMP/final 实现，再传入它们。当前教程没有提供可独立启动的 Rust BPP2D 原生适配器；不要用 `None, None` 或手填价格/使用量冒充求解。

示例在 CLI 边界用异常或错误返回结束本次调用；服务端应在请求边界转换错误，并保留诊断。正常无 incumbent 要保留报告，不伪造成技术异常或空的成功方案。

## 4. 读懂并验收结果

先确认 LP 和最终整数阶段实际执行，再逐 ID 核对 R1=6、R2=8。随后检查每件在板内、两件不内部重叠、旋转许可和间距。示例中的件数检查只是第一道交付门槛，不能替代几何复核，也不能据此宣称全局最优。

## 5. 修改一个业务条件

只把 R1 的 `allowRotate` 改为 false，保留需求，再比较候选与坐标。下一次再增加正刀缝；不要同时改板型、朝向和间距，否则很难解释变化。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时再查[仓库验证参考](../verification)，测试命令不是本课的业务入口。
