# 第一次三维装箱：从选择到实际放置

本课通过业务示例学习输入、求解和结果解析。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

先装一件长方体，不同时引入圆柱、支撑规则和多箱型。Kotlin 输入为 1 × 1 × 1 m 纸箱、3 × 3 × 3 m 箱体；Rust CSV 为 2 × 3 × 4 m 货物、10 × 10 × 10 m 箱体。它们演示同一流程，不比较目标数值。

候选层不是最终装载方案。Kotlin 显式提供有真实放置的初始候选层及一个空箱，并关闭新增层生成，仅学习已有层如何经过优化进入实际装载。Rust 用公开 CSV 加载器加载并转换数据，再接两个真实模型后端。

## 2. 构造输入并调用框架

::: info Kotlin 与 Rust 的入口差异
Kotlin 的物料、包装属性、朝向和坐标均在示例中定义；`runExample` 使用注入的真实求解器。Rust 示例需要领域包的 `serde`功能开关，且本课使用同步 API，不启用 `async`。分别为受限主问题与最终整数求解阶段配置后端，保留各阶段的求解报告。
:::

示例包含导入和错误处理。Kotlin 的 `runExample` 接收应用提供的 `ColumnGenerationSolver`，下一节给出启动入口。

::: code-group

<<< ../../../frameworks/_examples/bpp3d.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp3d.rs [Rust]

:::

## 3. 在自己的程序中启动

保留包声明，将 Kotlin 示例放入应用源码目录。下面的独立入口调用本课函数；需添加对应领域的集成依赖、Gurobi 插件和协程依赖，配置匹配的原生库与许可证。

```kotlin
import docs.examples.bpp3d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/bpp3d.rs`，在 `src/main.rs` 使用以下入口。依赖与 工具链 见环境页；同步示例不启用 `async`。

```rust
mod bpp3d;
use ospf_rust_core::solver::solvers::scip::SCIPConfig;
use ospf_rust_framework::solver::{ScipColumnGenerationSolver, meta_model_backend::ColumnGenerationSolverMetaModelBackend};

fn main() -> Result<(), String> {
    let config = SCIPConfig::new().with_output(false).with_time_limit(30.0);
    let rmp = ColumnGenerationSolverMetaModelBackend::new(
        ScipColumnGenerationSolver::with_config(config.clone()),
    ).with_lp_subproblem_defaults();
    let final_backend = ColumnGenerationSolverMetaModelBackend::new(
        ScipColumnGenerationSolver::with_config(config),
    );
    bpp3d::run_example(rmp, final_backend)
}
```

示例在命令行程序中通过抛出异常或返回错误结束失败调用。接入服务端时，将错误转换为结构化响应，并保留求解报告和诊断信息。求解器正常结束但未找到可行解时，按报告返回相应状态。

## 4. 读懂并验收结果

| 读取内容 | Kotlin | Rust |
| --- | --- | --- |
| 启用箱体 | `packingSnapshot.packingResult.aggregation.bins → name / type / items` | `final_execution.packed_bins → name / bin_type / items` |
| 位置、朝向、装载次序 | `placement.x / y / z / orientation`、`loadingOrder` | `item.position / orientation / loading_order` |
| 件数与目标 | `bins.flatMap { it.items }`、`result.finalObjective` | `packed_bins.iter().map(\|b\| b.items.len())`、`final_execution.objective` |
| 诊断 | `result.finalSolved / finalInfo` | `final_execution.final_solved / diagnostics`、`result.info` |

::: info 结果语义
本入口没有候选集合 `complete`，也不输出 BPP1D 的一维空闲区间。位置给出局部三维坐标；剩余体积可以由箱体积减实占体积计算，但不代表可放入任意同体积物品的自由空间。Kotlin 的 `Item` 是接口，读取业务 ID 时用 `ActualItem` 或应用维护的映射。
:::

区分候选层数、最终选层和实际装箱。Kotlin 读取 `packingSnapshot`，Rust 同时读取最终层与 `final_final_diagnostics` 等诊断；有候选层或展示数据，不等于货物已全部合法装入。按 ID 复核精确件数、朝向、边界、重叠和承重。初始候选数量有限时，结果的证明范围也仅限于这些候选。

## 5. 修改一个业务条件

把一件改成两件时，连同可用空箱及候选所包含的物品件数一起调整；不要只把需求数字翻倍。随后再增加第二箱型，最后才调整目标。此例的空新增层生成器不是通用装载搜索器。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时查[仓库验证参考](../verification)。
