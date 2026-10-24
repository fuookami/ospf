# 第一次板材套裁：需求、图案与工艺

本课通过业务示例学习输入、求解和结果解析。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

把矩形产品从定尺板材中裁出。Kotlin 为 100 × 60 m 板、40 × 30 m 产品、需求 3 件；Rust 为 10 × 8 m 板、2 × 2 m 产品、需求 20 件、原料可用 100 张。先用自由排样、零刀缝。尺寸是便于手算的教学量值，不是生产规格。

两种输入都能构造一张板的可行排布。Kotlin 显式声明产品、件数需求和工艺；Rust 用 `Csp2dProblem::single` 构造单产品问题。它们不是同一个数值基准。

## 2. 构造输入并调用框架

::: info Kotlin 与 Rust 的入口差异
Kotlin 的 `Csp2dMilp` 接入真实求解器。Rust 默认 `Csp2dSolvers::new().solve_milp` 使用启发式主问题，不需要原生后端；方法名不是原生执行或最优证明。需要真实优化时使用 `Csp2dMasterSolver` 接口。
:::

示例包含导入和错误处理。Kotlin 的 `runExample` 接收应用提供的 `ColumnGenerationSolver`，下一节给出启动入口。

::: code-group

<<< ../../../frameworks/_examples/csp2d.kt [Kotlin]

<<< ../../../frameworks/_examples/csp2d.rs [Rust]

:::

## 3. 在自己的程序中启动

保留包声明，将 Kotlin 示例放入应用源码目录。下面的独立入口调用本课函数；需添加对应领域的集成依赖、Gurobi 插件和协程依赖，配置匹配的原生库与许可证。

```kotlin
import docs.examples.csp2d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/csp2d.rs`，在 `src/main.rs` 使用以下入口。依赖与 工具链 见环境页；同步示例不启用 `async`。

```rust
mod csp2d;

fn main() -> ospf_rust_base::error::Ret<()> {
    csp2d::run_example()
}
```

示例在命令行程序中通过抛出异常或返回错误结束失败调用。接入服务端时，将错误转换为结构化响应，并保留求解报告和诊断信息。求解器正常结束但未找到可行解时，按报告返回相应状态。

## 4. 读懂并验收结果

| 读取内容 | Kotlin | Rust |
| --- | --- | --- |
| 图案、原料、使用次数 | `solution.usages → plan.pattern / plan.material / usage` | `solution.usage → (plan.pattern, count)` |
| 放置与切分树 | `pattern.placements`、`pattern.cutTree` | `pattern.placements`、`pattern.cut_tree` |
| 产出与原料消耗 | `coverage / sheetCount / consumedLength / wasteArea` | `coverage / material_usage / sheet_assignment / total_waste_area()` |
| 覆盖与终止 | `solution.covers(problem)`、`iterations` | `fully_covered / termination / failure_message` |

::: info 结果语义
`covers(problem)` / `fully_covered` 只回答需求是否覆盖，不表示候选图案集合完整或全局最优。每个图案的坐标只描述单次执行，单次产出、余料和卷材段长都要乘使用次数。自由排样的切分树允许为空。
:::

分四层验收：需求覆盖、原料用量、几何合法、工艺合法。图案数量不是板材张数，应汇总使用量。自由排样不要求切分树；断头台切割需要有效切分树及级数、区域和刀缝检查。默认 Rust 场景的手算参照为覆盖 20 件、用一张板、净面积余量为零；实际输出仍应复核。

## 5. 修改一个业务条件

先只增加正刀缝，原先刚好铺满的图案可能失效。再改两级断头台切割并重新生成图案，而不是给原坐标附加工艺标签。最后再切换卷材，并把主要用量从张数改为消耗长度。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时查[仓库验证参考](../verification)。
