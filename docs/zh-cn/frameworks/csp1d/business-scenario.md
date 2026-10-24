# 需求计量与业务规则：同一个数不代表同一种订单

完成[第一次卷材分切](./first-slitting)后，先让应用明确“客户要什么”，再增加设备与兼容规则。需求标签、物理单位、方案单次产出和方案使用量必须连成一条链。

## 1. 先设计一个独立于算法的验收题

假设一个已校验切割方案 P 每执行一次产出两卷 A，每卷 10 kg。下面先用手算比较不同计量方式；构造输入时，需要显式提供单位和换算数据：

| 订单 | P 单次产出 | 需要的使用次数 | 产出 |
| --- | --- | ---: | --- |
| A，6 卷 | 2 卷 | 3 | 6 卷 |
| A，60 kg | 20 kg | 3 | 60 kg |
| A，30 kg | 20 kg | 不能精确满足 | 需明确是否允许超产/其他方案 |

最后一行不能悄悄使用 1.5 次不可拆分的方案。若当前配置允许超产，可以使用 2 次并报告 40 kg；若要求精确产量且无其他方案，则不可行。允许范围必须进入模型，不是改一下订单标签。

## 2. 先用框架的需求类型表达订单

### 先构造可调用的卷数算例

下面落实表中“6 卷”这一行：A 幅宽 0.8 m，原料最大幅宽 1.6 m，M1 有 3 批；M2 是待隔离原料。每批可排两条 A。重量两行是独立的计量练习，要先补产品单件重量，不能直接把下面的 6 改成 60。

::: code-group

<<< ../../../frameworks/_examples/csp1d-business.kt#input [Kotlin]

<<< ../../../frameworks/_examples/csp1d-business.rs#input [Rust]

:::

| 需求 | Kotlin 入口 | Rust 入口 |
| --- | --- | --- |
| 兼容接口：卷数 | `ProductDemand.legacyRoll` | `ProductDemand::legacy_roll` |
| 兼容接口：张数 | `ProductDemand.legacySheet` | `ProductDemand::legacy_sheet` |
| 兼容接口：重量 | `ProductDemand.legacyWeight` | `ProductDemand::legacy_weight` |

::: tip 单位约束
这些辅助函数表达需求计量，不会替你补齐产品重量、长度或单位重量信息。采用物理量构造时，离散/连续属性还由实际单位决定，不能只看 `mode`。
:::

调用第一课的输入与求解入口，按当前需求单位读取单次产出。兼容输入可能保留旧模式标签，不能据此忽略真实数量单位或生产密度。

## 3. 明确方案产出，再调用求解

1. 按稳定产品 ID 建立需求，保留产品的宽度、可选长度和重量来源。
2. 生成方案时检查 `CuttingPlanDemandContribution` 的 `quantity` 与订单单位兼容。卷数订单即使产品有重量，仍应按卷数计量。
3. 使用各语言提供的产出计算接口，并检查所需的幅宽、长度、面密度或单件重量；切条数量不能直接作为 kg 数值。
4. 按 `Σ 单次产出 × 使用次数` 重算每个产品的产出，分别输出超产、欠产与允许界限。

同一产品的两种计量要求是否需要同时约束，是业务决定；不能让适配器看到相同 ID 就覆盖其中一张订单。

### 切换入口与方案预算 {#entry-budget}

保留问题与规则，仅从 `Csp1dMilp` 切到 `Csp1dColumnGeneration`。初始方案上限、每轮新增方案上限与迭代上限都放在求解配置中；同时读取迭代记录的终止原因和最终状态。

::: code-group

<<< ../../../frameworks/_examples/csp1d-business.kt#entry [Kotlin]

<<< ../../../frameworks/_examples/csp1d-business.rs#entry [Rust]

:::

::: info Rust 实际求解方式
Rust 本例使用默认启发式求解主问题，并估计用于搜索候选的价格信息；不会因为方法名含 MILP 就调用原生优化器。若要从实际 LP 求解中读取对偶值，需要配置 `Csp1dLpSolver` 适配器。对偶值反映约束变化对目标的影响，可用于指导候选搜索。业务输出同时保留 `status`、欠产和迭代记录。
:::

## 4. 设备规则放在哪一层？

### 最小扩展：隔离原料 M2

实现 `Csp1dDomainPolicy` 的 `isFeasible`，再注入求解配置的 `extensionSet.domainPolicies`。该策略只排除 M2，不覆盖框架默认幅宽检查。相同配置同时用于普通入口和列生成入口。

本场景还显式注入 `NSameGenerator` 生成两条 A 的方案：默认 `SimpleInitialCuttingPlanGenerator` 只给一条切条，三批库存下不足以交齐六卷。`initialGenerator()` 辅助函数在完整源码中解析物理量算术策略，再创建 N-Same 生成器。

::: code-group

<<< ../../../frameworks/_examples/csp1d-business.kt#rule [Kotlin]

<<< ../../../frameworks/_examples/csp1d-business.rs#rule [Rust]

:::

Rust 的实例注入位置：

<<< ../../../frameworks/_examples/csp1d-business.rs#injection [Rust]

“原料 M 不能在设备 X 上加工”可以在单方案可行性检查中判断；“设备 X 本班次最多处理 100 m”需要跨方案汇总约束。后者不能只给每个方案单独检查一次 `<=100`。

跨方案规则应通过对应上下文对象加入模型；使用列生成时，同一规则还应参与 LP 求解、候选搜索和最终整数求解。末端删除违规方案不保证剩余产出仍满足需求。

## 5. 为自己的应用添加三组检查

- 换需求模式但保持数字不变时，产出单位和产出公式随之变化；不能仍给出原来的结果。
- 禁止一个原料与设备的组合后，违规候选不能进入选择；再禁止全部可行组合，应无可交付产出。
- 总资源上界收紧后，检查所有方案使用量之和，而不是只检查选中的每条方案。

与[公共接入指南](../integration)一起处理正常无解、技术失败和原生后端报告。补充候选后找到可行解，或改用启发式求解时，都应保留实际求解状态，不将可行解标记为已证明最优。

源码回归命令统一放在[仓库验证参考](../verification)；先完成业务输入、调用与结果复核，再按需验证。


## 完整应用源码

片段来自同一个源码文件；保留导入声明和辅助函数后，从应用调用 `runBusiness` / `run_business`。构造函数也可独立用于输入校验。后端依赖沿用第一课；这些入口不创建隐式求解后端。

:::: details 展开完整 Kotlin / Rust 源码

::: code-group

<<< ../../../frameworks/_examples/csp1d-business.kt [Kotlin]

<<< ../../../frameworks/_examples/csp1d-business.rs [Rust]

:::

::::
