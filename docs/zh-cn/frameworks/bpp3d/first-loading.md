# 第一次三维装箱：从选择到实际放置

本课直接写业务示例，不以运行仓库测试作为前置步骤。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

先装一件长方体，不同时引入圆柱、支撑规则和多箱型。Kotlin 输入为 1 × 1 × 1 m 纸箱、3 × 3 × 3 m 箱体；Rust CSV 为 2 × 3 × 4 m 货物、10 × 10 × 10 m 箱体。它们演示同一流程，不比较目标数值。

候选层不是最终装载方案。Kotlin 显式提供有真实放置的 seed 层及一个空箱，并关闭新增层生成，仅学习已有层如何经过优化进入实际装载。Rust 用公开 CSV loader 物化数据，再接两个真实模型后端。

## 2. 构造输入并调用框架

Kotlin 的物料、包装属性、朝向和坐标均在示例中定义；`runExample` 使用注入的真实 solver。Rust 示例需要领域 crate 的 `serde` feature，且本课使用同步 API，不启用 `async`。RMP 与 final 必须分别接通，不能传预设目标或空执行器。

下面是示例函数的完整源码（含导入），不是测试代码。Kotlin 的 `runExample` 由应用注入真实 `ColumnGenerationSolver`；独立程序的启动方式见下一节。

::: code-group

<<< ../../../frameworks/_examples/bpp3d.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp3d.rs [Rust]

:::

## 3. 在自己的程序中启动

将 Kotlin 示例放在应用源码中，保留其 package。下面这个独立入口调用本课函数；需添加领域 starter、Gurobi 插件和 coroutines，配置匹配的原生库与许可证。这里使用的是生产插件，不是测试中的 delegating wrapper。

```kotlin
import docs.examples.bpp3d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/bpp3d.rs`，在 `src/main.rs` 使用以下入口。依赖与 toolchain 见环境页；同步示例不启用 `async`。

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

示例在 CLI 边界用异常或错误返回结束本次调用；服务端应在请求边界转换错误，并保留诊断。正常无 incumbent 要保留报告，不伪造成技术异常或空的成功方案。

## 4. 读懂并验收结果

区分候选层数、最终选层和实际装箱。Kotlin 读取 `packingSnapshot`，Rust 同时读取最终层与 `final_final_diagnostics` 等诊断；有层或有 schema 不等于货物全部合法装入。按 ID 复核精确件数、朝向、边界、重叠和承重。有限 seed 的结果只证明所建候选问题的相应结论。

## 5. 修改一个业务条件

把一件改成两件时，连同可用空箱及候选覆盖一起调整；不要只把需求数字翻倍。随后再增加第二箱型，最后才调整目标。此例的空新增层生成器不是通用装载搜索器。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时再查[仓库验证参考](../verification)，测试命令不是本课的业务入口。
