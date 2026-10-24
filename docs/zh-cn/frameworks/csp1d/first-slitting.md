# 第一次卷材分切：需求与方案使用量

本课通过业务示例学习输入、求解和结果解析。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

生产宽 0.8 m 的产品，需求 3 卷。原料幅宽范围 0.5–1.5 m，先不配置机器。这里的 3 是卷数，不是长度 3 m，也不是方案使用次数。Rust 示例另设原料可用批次为 3。

使用 `ProductDemand.legacyRoll` / `legacy_roll` 显式声明卷数。每个方案有自己的产品产出，实际产量是“单次产出 × 使用次数”的汇总；生成了三个候选不代表已经生产三卷。

## 2. 构造输入并调用框架

::: info Kotlin 与 Rust 的入口差异
Kotlin 使用实际求解器的普通入口，并直接返回结构化 `Csp1dSolution`，不是 `Ret`。Rust 默认入口无需原生求解器，但内部使用启发式，不能从方法名 `Milp` 推断调用了原生 MILP。需要原生主问题时再配置对应求解器接口。
:::

示例包含导入和错误处理。Kotlin 的 `runExample` 接收应用提供的 `ColumnGenerationSolver`，下一节给出启动入口。

::: code-group

<<< ../../../frameworks/_examples/csp1d.kt [Kotlin]

<<< ../../../frameworks/_examples/csp1d.rs [Rust]

:::

## 3. 在自己的程序中启动

保留包声明，将 Kotlin 示例放入应用源码目录。下面的独立入口调用本课函数；需添加对应领域的集成依赖、Gurobi 插件和协程依赖，配置匹配的原生库与许可证。

```kotlin
import docs.examples.csp1d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/csp1d.rs`，在 `src/main.rs` 使用以下入口。依赖与 工具链 见环境页；同步示例不启用 `async`。

```rust
mod csp1d;

fn main() {
    csp1d::run_example();
}
```

示例在命令行程序中通过抛出异常或返回错误结束失败调用。接入服务端时，将错误转换为结构化响应，并保留求解报告和诊断信息。求解器正常结束但未找到可行解时，按报告返回相应状态。

## 4. 读懂并验收结果

| 读取内容 | Kotlin | Rust |
| --- | --- | --- |
| 方案和使用次数 | `solution.produce.cuttingPlans → plan / amount` | `solution.produce.cutting_plans → plan / amount` |
| 原料与余宽 | `plan.material.id / slices / restWidth` | `plan.material.id / slices / rest_width()` |
| 单次产出 | `plan.demandContributions → product.id / quantity` | `plan.demand_contributions → product.id / quantity` |
| 欠产与状态 | `produce.unmetDemands`、`status / failureMessage / kpi` | `produce.unmet_demands`、`status / failure_message / kpi` |

::: info 结果语义
按 `Σ 单次产出 × 使用次数` 汇总产出；不要把方案列表长度当产量。`restWidth` / `rest_width()` 返回余宽而非一组放置坐标。该结果没有候选集合 `complete`；部分结果应结合状态、欠产与列生成迭代记录判断。
:::

先读取 `status`、失败信息与 `unmetDemands` / `unmet_demands`，再读取选中方案及使用量。按卷数重算 P1 的产出，复核每个方案幅宽与原料批次；候选集合非空不等于覆盖需求，部分结果也不能直接作为完整交付。

## 5. 修改一个业务条件

先把需求从 3 卷改成 4 卷，重新检查可用批次与方案产出。再尝试重量需求：不能只把标签从“卷”改成“kg”，还需给出单位重量、卷长等转换依据，并同步修改验收标准。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时查[仓库验证参考](../verification)。
