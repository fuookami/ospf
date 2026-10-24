# 第一次网络流优化：供需、容量与费用

本课通过业务示例学习输入、求解和结果解析。先按[环境页](../setup)准备依赖和后端。

## 1. 业务场景

source 供应 2 单位货物，sink 需求 2 单位；一条 delivery 弧连接两者，容量为 0–3，单位费用为 3。先用同一无量纲单位，完整跑通供需、容量、真实调用与结果读取。

供给写为 +2，需求写为 -2，总平衡为零。手算流量为 2、费用为 6。节点 ID、弧 ID 与商品 ID 分开；后续增加平行服务时，不能仅靠起终点识别弧。

## 2. 构造输入并调用框架

::: info Kotlin 与 Rust 的入口差异
Kotlin 的节点、弧、容量和商品构造器会返回 `Ret`，示例显式处理失败；再调用 `NetworkFlowApplicationService`。Rust 用 `FlowProblem::new` 与 `FlowApplicationService`，注入真正的 `LinearSolver`。应用可从返回报告读取求解状态，再解析业务流量。
:::

示例包含导入和错误处理。Kotlin 的 `runExample` 接收应用提供的 `ColumnGenerationSolver`，下一节给出启动入口。

::: code-group

<<< ../../../frameworks/_examples/network.kt [Kotlin]

<<< ../../../frameworks/_examples/network.rs [Rust]

:::

## 3. 在自己的程序中启动

保留包声明，将 Kotlin 示例放入应用源码目录。下面的独立入口调用本课函数；需添加对应领域的集成依赖、Gurobi 插件和协程依赖，配置匹配的原生库与许可证。

```kotlin
import docs.examples.network.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

将 Rust 示例函数放入 `src/network.rs`，在 `src/main.rs` 使用以下入口。依赖与 工具链 见环境页；同步示例不启用 `async`。

```rust
mod network;
use ospf_rust_core::solver::solvers::scip::SCIPSolver;

fn main() -> Result<(), String> {
    network::run_example(SCIPSolver::new())
}
```

示例在命令行程序中通过抛出异常或返回错误结束失败调用。接入服务端时，将错误转换为结构化响应，并保留求解报告和诊断信息。求解器正常结束但未找到可行解时，按报告返回相应状态。

## 4. 读懂并验收结果

| 读取内容 | Kotlin | Rust |
| --- | --- | --- |
| 实际商品流 | `solution.flows: FlowKey(commodity, arc) → Quantity` | `flow.flows: (CommodityId, NetworkArcId) → Quantity` |
| 共享资源占用 | `capacityUsage` | `resources` |
| 未使用供应与缺货 | `unusedSupply / unmetDemand` | `spills / shortages` |
| 费用分解与状态 | `cost / commodityCosts / relaxationCost`、`outcome.report` | `operating_cost / spill_cost / shortage_cost / total_cost`、`report` |

::: info 结果语义
`solution` 可能为空；正常无解或没已找到可行解时按求解报告返回业务状态。该接口没有候选集合 `complete`。有界缺货可以是合法提取结果，但是否允许交付必须由业务策略判断。
:::

先检查报告及可选业务解，随后核对 source 出流为 2、sink 入流为 2、delivery 在 0–3 之间，以及费用 2 × 3 = 6。不要把求解器步数当物理量，也不要把外层成功当成已经满足需求。

## 5. 修改一个业务条件

把容量上界从 3 改为 1。硬需求下应无可行完整流，不能自动丢掉 1 单位订单。再恢复容量，增加第二条费用不同且 ID 不同的服务弧，观察选择变化。缺货只有显式启用且有界、计费时才允许。

下一课：[业务场景进阶](./business-scenario)。上线前完成[业务接入检查](../integration)。需要复核源码时查[仓库验证参考](../verification)。
