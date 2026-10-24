# 第一次二维排样：从矩形开始

本课通过业务示例学习输入、求解和结果解析。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

把 R1（4 × 3 m，6 件）和 R2（2 × 2 m，8 件）排入 20 × 12 m 的标准板，允许旋转，不加刀缝。每件 1 kg，板材承重 10000 kg。

净面积为 104 m²，小于一张板的 240 m²；但面积足够不证明几何可放置。物品目录、需求和板型必须使用一致的 ID，不能按尺寸把不同订单合并。

## 2. 构造输入并调用框架

::: info Kotlin 与 Rust 的入口差异
Kotlin 示例完整构造物品、板型和请求，并调用公开应用服务。Rust 的 `solve_layout` 接收业务输入，以及受限主问题和最终整数求解的两个后端适配器。当前 Rust 服务接口固定为 `f64 + Meter`，物品重量与长度还共用单位类型，接入时应显式处理这项单位限制。`StandardRmpSolver`/`StandardFinalSolver` 只是闭包适配器，不是自动连接 SCIP 的实现。
:::

示例包含导入和错误处理。Kotlin 的 `runExample` 接收应用提供的 `ColumnGenerationSolver`，下一节给出启动入口。

::: code-group

<<< ../../../frameworks/_examples/bpp2d.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp2d.rs [Rust：接入边界]

:::

## 3. 在自己的程序中启动

保留包声明，将 Kotlin 示例放入应用源码目录。下面的独立入口调用本课函数；需添加对应领域的集成依赖、Gurobi 插件和协程依赖，配置匹配的原生库与许可证。

```kotlin
import docs.examples.bpp2d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Rust 调用方应先统一输入单位，实现受限主问题和最终整数求解的后端适配器，再将它们传入 `solve_layout`。这两个适配器需要应用自行实现。

示例在命令行程序中通过抛出异常或返回错误结束失败调用。接入服务端时，将错误转换为结构化响应，并保留求解报告和诊断信息。求解器正常结束但未找到可行解时，按报告返回相应状态。

## 4. 读懂并验收结果

| 读取内容 | Kotlin | Rust |
| --- | --- | --- |
| 启用板材与物品 | `packing.aggregation.bins → name / type / items` | `packing.sheets → name / sheet_type / items` |
| 坐标与旋转 | `placement.x / y / maxX() / maxY() / rotated` | `placement.x() / y() / max_x() / max_y() / rotated` |
| 余料与 KPI | `packing.remnants`、`response.kpi` | `sheet.remnants`、`packing.item_amounts() / occupied_area() / remnant_area()` |
| 终止范围 | `response.result.finalSolved / terminatedByIterationLimit / terminatedByTimeLimit` | `result.final_solved / terminated_by_iteration_limit / terminated_by_time_limit` |

::: info 结果语义
本入口没有 `complete`。`finalSolved` 表示执行了最终求解，预算终止标记说明搜索范围；几何方案与精确需求件数仍要独立复核。余料是按板名关联的空闲矩形区域，不是已经入库的库存。
:::

先确认 LP 和最终整数阶段实际执行，再逐 ID 核对 R1=6、R2=8。随后检查每件在板内、两件不内部重叠、旋转许可和间距。示例中的件数检查只是第一道结果检查，不能替代几何复核，也不能据此宣称全局最优。

## 5. 修改一个业务条件

只把 R1 的 `allowRotate` 改为 false，保留需求，再比较候选与坐标。下一次再增加正刀缝；不要同时改板型、朝向和间距，否则很难解释变化。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时查[仓库验证参考](../verification)。
