# 多容器类型、库存与费用：容器数最少不一定最便宜

完成[第一次一维装箱](./first-packing)后，把同一批物品交给两种容器。本课从 CSV 加载到目标比较，再加入禁配规则和生成预算。

## 1. 先准备能手算的输入

物品表：

```csv
id,length,weight,amount
a,6,1,2
b,4,1,2
```

容器表：

```csv
id,length,max_weight,cost,inventory
small,10,100,3,2
large,20,100,10,1
```

长度单位为 m、重量单位为 kg，间距先为零。物品净长度为 20 m；一个 large 或两个 small 都能装完，且不超重。

| 目标 | 一个最优选择 | 容器数 | 总费用 |
| --- | --- | ---: | ---: |
| 最少容器数 | 1 个 large | 1 | 10 |
| 最小费用 | 2 个 small，各装 a+b | 2 | 6 |
| 最少消耗长度 | 所启用容器的容量总长度均为 20 m：$1 \times 20\text{ m}$ 或 $2 \times 10\text{ m}$，均满载、零尾部余量 | 不唯一 | 不由此目标决定 |

这些手算值可作为小实例的业务对照；程序返回的证明范围由候选完整性和求解报告共同决定。

## 2. 从 CSV 生成输入并切换目标

将上面的两个 CSV 文本分别传入 `itemsCsv` / `binsCsv`（Rust 为 `ITEMS_CSV` / `BINS_CSV`）。数值解析函数 `parseNumber` / `parse_number` 和导入均在本页末尾的完整源码中。

::: code-group

<<< ../../../frameworks/_examples/bpp1d-business.kt#csv [Kotlin]

<<< ../../../frameworks/_examples/bpp1d-business.rs#csv [Rust]

:::

::: tip 显式单位与库存空值
CSV 不携带单位，加载时绑定 `Meter`、`Kilogram` 并指定间距。本例使用 `Flt64` / `f64` 解析；高精度数值应直接使用对应数值类型的解析器。空的 `inventory` 单元格表示不限库存，`0` 表示不可用。
:::

::: info 加载器返回值不同
Kotlin 的 `load` 返回 `Ret<PackingProblem<Flt64>>`，示例用 `getOrThrow()` 在命令行程序中处理错误，然后通过 `copy(objective = ...)` 切换目标。Rust 的 `load_packing_csv` 直接返回校验后的 `PackingContext`；切换目标时克隆 `context.problem()`，修改 `objective`，再用 `PackingContext::new` 校验并构造新上下文。完整源码演示了逐个目标求解。
:::

沿用第一课的后端，分别以 `BinCount`、`Cost`、`ConsumedLength` 求解，并按第一课的方法解析实际放置结果。

## 3. 一次只改一个条件

以下均以最小费用为目标；第 4 步恢复原始库存。

| 步骤 | 数据变化 | 结果与业务解释 |
| --- | --- | --- |
| 1 | small 库存从 2 改为 1，间距仍为零 | 一个 large 装完，最小费用 10 |
| 2 | 恢复 small 库存为 2，large 库存改为零 | 两个 small 各装 a+b，最小费用 6 |
| 3 | 在第 2 步基础上将 small 库存改为 1 | 总容量不足，无完整可行方案 |
| 4 | 恢复原始库存，只将间距改为 1 m | large 装 a+a+b，small 装 b，总费用 $10 + 3 = 13$ |

第 4 步可以逐项检查：四件全放 large 需 $20 + 3 \times 1 = 23\text{ m} > 20\text{ m}$；a+b 放 small 需 $6 + 1 + 4 = 11\text{ m} > 10\text{ m}$；a+a+b 放 large 需 $6 + 1 + 6 + 1 + 4 = 18\text{ m}$，余下一件 b（4 m）放 small 可行。

## 4. 复核结果并加入禁配规则

交付前，从实际放置结果独立复核：

1. **物品件数守恒**：按 ID 汇总 `placements`，a、b 都恰好 2 件。
2. **容器库存不超限**：按容器类型汇总 `plans` 的数量，对照非空库存上限。
3. **单容器物理规则满足**：逐容器检查起止坐标、总长度、承重与相邻间距。
4. **独立重算总费用**：每个启用容器累加一次对应容器类型的 `cost`，与 KPI 及所选目标对照。

### 单容器禁配：PatternPolicy

恢复本课的零间距和原始库存，新增“a 与 b 不能放在同一个容器”。策略读取候选组合里的物品 ID；Kotlin 将策略传给候选生成器，再注入求解入口：

::: code-group

<<< ../../../frameworks/_examples/bpp1d-business.kt#policy [Kotlin]

<<< ../../../frameworks/_examples/bpp1d-business.rs#policy [Rust：实现策略]

:::

Rust 在调用时显式传入策略与无附加模型扩展：

<<< ../../../frameworks/_examples/bpp1d-business.rs#policy-injection [Rust：注入策略]

这里的 `problem` / `context` 来自第 2 节。`backend(solver)` 将模型、剩余时限与取消句柄转发给第一课接通的后端。禁配后可选 large 装 a+a、small 装 b+b，最小费用为 13。结果件数和库存的复核方式不变，再增加一项“每个容器没有同时出现 a 与 b”。

::: info 自定义规则的报告范围
策略同时用于候选枚举和初始方案生成，完整性是相对于该策略允许的组合而言。Rust 的 `solve_with_extension` 对自定义规则保守地不设置 `globally_optimal`；结合当前模型的 `report`、候选完整性和业务复核解释结果。
:::

### 跨容器约束：PatternModelingExtension

例如“最多启用两个容器”是跨容器规则，应将候选组合使用次数之和限制为 2。下面给出普通 MILP 的最小实现和注入位置，独立于前面的禁配练习：

::: code-group

<<< ../../../frameworks/_examples/bpp1d-business.kt#extension [Kotlin]

<<< ../../../frameworks/_examples/bpp1d-business.rs#extension [Rust]

:::

::: code-group

<<< ../../../frameworks/_examples/bpp1d-business.kt#extension-injection [Kotlin：传入 SolveConfig]

<<< ../../../frameworks/_examples/bpp1d-business.rs#extension-injection [Rust：传入 solve_with_extension]

:::

::: warning 将模型扩展用于列生成前
Kotlin 的扩展在不同阶段会调用以下方法：`register(context): Try`、`refresh(context): Try`、`reducedCost(pattern, context, dual): Flt64`、`extractSolution(context, values): Ret<Map<String, String>>`。新增候选后需要更新附加表达式；搜索新候选时，还需考虑附加约束对目标改进的影响。本例的 Kotlin `BinLimit` 只支持普通 MILP，并显式检查阶段。

Rust 在候选集合变化时重建完整模型；`register(model, packing, aggregation, stage)` 接收当前模型和列集合，可选的 `extract_solution(values)` 提取业务指标。框架从完整矩阵计算定价，两种语言的扩展调用顺序和职责不同，应分别实现。完整方法定义和约束标识方式见概览页的[接口参考](./#接口参考)。
:::

## 5. 切换到列生成并设置预算 {#column-generation}

使用第 2 节的输入与同一个后端，替换入口并显式配置预算：

::: code-group

<<< ../../../frameworks/_examples/bpp1d-business.kt#cg [Kotlin]

<<< ../../../frameworks/_examples/bpp1d-business.rs#cg [Rust]

:::

`maxPatterns` / `max_patterns` 限制生成候选数，`maxNodes` / `max_nodes` 限制枚举搜索节点；`iterationLimit` / `iteration_limit` 限制定价轮数，`columnsPerIteration` / `columns_per_iteration` 限制每轮激活列数。结果中读取候选完整性、`columnCount` / `column_count` 与 `pricingIterations` / `pricing_iterations`。这些预算不同于求解器的时限；达到生成上限时，当前有限候选集合的最优结论仍需保留其范围。

## 6. 在自己的应用中运行

Kotlin 将完整源码放入应用，调用 `runBusinessExample(solver)`；Rust 放入 `src/bpp1d_business.rs`，调用 `bpp1d_business::run_business_example(&solver)`。启动入口沿用第一课，分别替换函数导入或模块名；示例依次运行目标比较、禁配、列生成与跨容器限制。

:::: details 完整源码（含 CSV、解析函数、导入与后端转发）

::: code-group

<<< ../../../frameworks/_examples/bpp1d-business.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp1d-business.rs [Rust]

:::

::::

完成后按[接入指南](../integration)替换业务数据、添加结果检查并处理失败；需要验证源码时查[仓库验证参考](../verification)。
