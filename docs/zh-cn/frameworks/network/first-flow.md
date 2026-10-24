# 第一次网络流优化：供需、容量与费用

本课直接写业务示例，不以运行仓库测试作为前置步骤。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

source 供应 2 单位货物，sink 需求 2 单位；一条 delivery 弧连接两者，容量为 0–3，单位费用为 3。先用同一无量纲单位，完整跑通供需、容量、真实调用与结果读取。

供给写为 +2，需求写为 -2，总平衡为零。手算流量为 2、费用为 6。节点 ID、弧 ID 与商品 ID 分开；后续增加平行服务时，不能仅靠起终点识别弧。

## 2. 构造输入并调用框架

Kotlin 的节点、弧、容量和商品构造器会返回 `Ret`，示例显式处理失败；再调用 `NetworkFlowApplicationService`。Rust 用 `FlowProblem::new` 与 `FlowApplicationService`，注入真正的 `LinearSolver`。没有手填见证向量，也不从测试上下文复制建模过程。

下面是示例函数的完整源码（含导入），不是测试代码。Kotlin 的 `runExample` 由应用注入真实 `ColumnGenerationSolver`；独立程序的启动方式见下一节。

::: code-group

<<< ../../../frameworks/_examples/network.kt [Kotlin]

<<< ../../../frameworks/_examples/network.rs [Rust]

:::

## 3. 在自己的程序中启动

将 Kotlin 示例放在应用源码中，保留其 package。下面这个独立入口调用本课函数；需添加领域 starter、Gurobi 插件和 coroutines，配置匹配的原生库与许可证。这里使用的是生产插件，不是测试中的 delegating wrapper。

```kotlin
import docs.examples.network.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/network.rs`，在 `src/main.rs` 使用以下入口。依赖与 toolchain 见环境页；同步示例不启用 `async`。

```rust
mod network;
use ospf_rust_core::solver::solvers::scip::SCIPSolver;

fn main() -> Result<(), String> {
    network::run_example(SCIPSolver::new())
}
```

示例在 CLI 边界用异常或错误返回结束本次调用；服务端应在请求边界转换错误，并保留诊断。正常无 incumbent 要保留报告，不伪造成技术异常或空的成功方案。

## 4. 读懂并验收结果

先检查报告及可选业务解，随后核对 source 出流为 2、sink 入流为 2、delivery 在 0–3 之间，以及费用 2 × 3 = 6。不要把 solver 步数当物理量，也不要把外层成功当成已经满足需求。

## 5. 修改一个业务条件

把容量上界从 3 改为 1。硬需求下应无可行完整流，不能自动丢掉 1 单位订单。再恢复容量，增加第二条费用不同且 ID 不同的服务弧，观察选择变化。缺货只有显式启用且有界、计费时才允许。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时再查[仓库验证参考](../verification)，测试命令不是本课的业务入口。
