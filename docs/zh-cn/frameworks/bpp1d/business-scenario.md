# 多箱型、库存与成本：箱数最少不一定最便宜

完成[第一次一维装箱](./first-packing)后，把同一批物品交给两种容器，练习把业务目标和库存传入模型。本页是两种语言可共用的**新练习输入**，用于替换第一课的输入。

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

显式声明长度单位为 m、重量单位为 kg，间距先为零。净长度为 20 m；一个 large 或两个 small 都能装完，且不超重。

| 目标 | 一个可验收的最优选择 | 箱数 | 总费用 |
| --- | --- | ---: | ---: |
| 最少箱数 | 1 个 large | 1 | 10 |
| 最小费用 | 2 个 small，各装 a+b | 2 | 6 |
| 最少消耗长度 | 两种方案均为 20 m | 不唯一 | 不由此目标决定 |

这些是完整微型问题的手算结论。若生成预算截断导致程序没看到某种组合，必须保留受限候选模型的结论范围。

## 2. 映射到真实入口

Kotlin 使用 `Bpp1dCsvDatasetLoader`，Rust 使用 `load_packing_csv`；二者均需显式配置单位与数值解析，CSV 本身不带单位。也可按第一课的工厂直接构造 `Item`、`BinType` 和 `PackingProblem`。

以下只展示**已经构造好 `problem` 之后**的目标切换，不是独立程序：

::: code-group

```kotlin [Kotlin]
val costProblem = problem.copy(objective = PackingObjective.Cost)
```

```rust [Rust]
let mut cost_problem = problem.clone();
cost_problem.objective = PackingObjective::Cost;
```

:::

枚举定义见 [Kotlin 输入模型](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/bpp1d-domain-item-context/src/main/fuookami/ospf/kotlin/framework/bpp1d/domain/item/model/PackingProblem.kt)和 [Rust 输入模型](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/src/domain/model.rs)。沿用第一课已接通的 solver 与结果处理，不要用换目标来替代后端接线。

## 3. 一次只改一个条件

1. 将 small 库存从 2 改为 1，保持零间距。最小费用变为 10，使用一个 large；不允许悄悄再采购一个 small。
2. 恢复 small 库存为 2，再把 large 库存改为零。仍可用两个 small，费用为 6。
3. 在第 2 步基础上把 small 库存改为 1。总容量不足，应无可行完整方案。
4. 恢复原始库存，只将间距改为 1 m。一个 large 装不下四件，small 也不能装 a+b；可用 large 装 a+a+b、small 装 b，费用为 13。验证这个新问题，而不是沿用旧断言。

库存空值表示不限，零表示不能使用。业务适配器不能把两者转换成相同默认值。

## 4. 交付前重算四个量

按实际放置统计 a、b 都恰好 2 件；按箱型统计库存；逐箱检查长度、承重和相邻间距；最后用实际使用箱数重算费用。即使消耗长度相同，费用和箱数也可能不同。

追加业务规则时，单组合禁配用 `PatternPolicy`；跨箱数量/预算规则考察 `PatternModelingExtension`，并检查 MILP、LP、final 与最终复核是否一致。不要只在报表里过滤掉违规箱子。

在第一课示例中直接替换物品与容器列表，保留单位、库存和目标字段；重新调用框架并独立重算结果。

完成后按[接入指南](../integration)把同一组数据、结果不变量与失败用例迁入自己的应用。

源码回归命令统一放在[仓库验证参考](../verification)；先完成业务输入、调用与结果复核，再按需验证。
