# 第一次一维装箱：物品、间距与库存

从一批长条物品开始，完成输入、求解和业务结果解析。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

有 3 件长条物品，每件长 40 cm、重 2 kg。small 容器长 1 m、承重 5 kg、费用为 1，large 长 2 m、承重 10 kg、费用为 3；各有一个。相邻物品留 20 cm，目标是最少容器数。

三件合放需要 $3 \times 40\text{ cm} + 2 \times 20\text{ cm} = 160\text{ cm} = 1.6\text{ m}$，总重 $6\text{ kg}$。small 的长度与承重都不足；large 能一个容器装完。

::: tip 数量、单位与零库存
`amount` 是必须装入的精确件数。Kotlin 保留 cm/m 混合输入，由适配器转换；Rust 显式统一为 m/kg。库存空值 `null` / `None` 表示不限，零值表示该容器类型不能使用。
:::

## 2. 构造输入并调用框架

Kotlin 调用 `Bpp1dMilp.solve`。Rust 先创建经过校验的 `PackingContext`，再把模型、剩余时限与取消句柄交给注入的 `LinearSolver`。

示例包含导入和错误处理。`runExample` / `run_example` 接收应用提供的求解器，下一节给出启动入口。

::: code-group

<<< ../../../frameworks/_examples/bpp1d.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp1d.rs [Rust]

:::

## 3. 在自己的程序中启动

保留包声明，将 Kotlin 示例放入应用源码目录。下面的独立入口调用本课函数；需添加对应领域的集成依赖、Gurobi 插件和协程依赖，配置匹配的原生库与许可证。

```kotlin
import docs.examples.bpp1d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/bpp1d.rs`，在 `src/main.rs` 使用以下入口。依赖与 工具链 见环境页；同步示例不启用 `async`。

```rust
mod bpp1d;
use ospf_rust_core::solver::solvers::scip::SCIPSolver;

fn main() -> Result<(), String> {
    bpp1d::run_example(&SCIPSolver::new())
}
```

示例在命令行程序中通过抛出异常或返回错误结束失败调用。接入服务端时，将错误转换为结构化响应，并保留求解报告和诊断信息。求解器正常结束但未找到可行解时，按报告返回相应状态。

## 4. 读懂并验收结果

`solve` 后的关键拆解如下，已包含在第 2 节源码中。Kotlin 位于 `Ok` 分支；Rust 位于 `Some(packing)` 分支，并保留原 `context` 供查容器类型和计算 KPI。

::: code-group

<<< ../../../frameworks/_examples/bpp1d.kt#result [Kotlin]

<<< ../../../frameworks/_examples/bpp1d.rs#result [Rust]

:::

本例的一个容器方案使用 large。将坐标统一成 m 后，应能读出：

| 业务字段 | 应读到的值 |
| --- | --- |
| 物品区间 | a：`[0, 0.4)`、`[0.6, 1.0)`、`[1.2, 1.6)` |
| 物品间隙 | `[0.4, 0.6)`、`[1.0, 1.2)` |
| 尾部余量 | `[1.6, 2.0)`，长 0.4 m |
| 核心 KPI | 容器数 1、件数 3、消耗容器长度 2 m、费用 3、利用率 0.6 |

::: info 空闲区间与完整性
Kotlin 的 `remnants` 只给出尾部余量；源码用 `placements.zipWithNext()` 读取内部间隙。Rust 的 `free_intervals` 同时给出内部间隙与尾部余量。间隙是业务要求预留的空间，不能当作可随意插入物品的余量。`candidateComplete` / `candidate_complete` 说明候选集合是否穷尽；最优性还要结合状态或报告读取。
:::

源码最后按 `itemId` / `item_id` 汇总放置件数，并与输入 `amount` 逐项比较。接入自己的返回数据对象时，同样保留容器类型 ID、物品区间、KPI 与求解状态，便于下游执行和追溯。

## 5. 修改一个业务条件

把 large 的库存改为零。只剩一个 small，无法装入全部 3 件；应用应展示无完整可行方案，而不是复用上次结果。再恢复库存，将目标改为 `Cost`，观察费用目标与箱数目标的区别。

下一课：[多容器类型、库存与费用](./business-scenario)。上线前完成[业务接入检查](../integration)；需要复核源码时查[仓库验证参考](../verification)。
