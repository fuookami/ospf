# 使用领域驱动设计架构（列生成）

列生成把“从已知列中做选择”和“发现新的有价值列”拆成两个协作问题。DDD 进一步要求这条算法边界由领域语言表达：主问题不接收匿名系数数组，定价子问题也不读取求解器行号；二者通过**影子价格协议**和**领域列对象**通信。

> 列生成求得完整列集合对应 LP 松弛的最优解。只有把列生成嵌入分支树，才能称为 Branch-and-Price 并用于证明整数最优性。

## 1. 数学结构

令 $I$ 为主问题约束集合，$\mathcal P$ 为数量巨大、不能预先全部枚举的可行列集合。列 $p$ 的成本为 $c_p$，在约束 $i$ 中的系数为 $a_{ip}$，列选择变量为 $\lambda_p$。以下以最小化覆盖模型为例：

$$
\begin{aligned}
\min_{\lambda}\quad
& \sum_{p \in \mathcal P} c_p \lambda_p \\
\text{s.t.}\quad
& \sum_{p \in \mathcal P} a_{ip}\lambda_p \ge b_i,
&& \forall i \in I, \\
& \lambda_p \ge 0,
&& \forall p \in \mathcal P.
\end{aligned}
$$

受限主问题（RMP）只含当前列集 $\mathcal P' \subset \mathcal P$。设覆盖约束的对偶值为 $\pi_i$，则候选列的约化成本为：

$$
\bar c_p
= c_p - \sum_{i \in I}\pi_i a_{ip}.
$$

定价子问题求：

$$
\bar c^\star
= \min_{p \in \mathcal P}
\left(c_p-\sum_{i \in I}\pi_i a_{ip}\right).
$$

在该符号约定下，当 $\bar c^\star < -\varepsilon_{\mathrm{rc}}$ 时把新列加入 RMP；若精确定价证明不存在这样的列，则当前 RMP 已达到完整 LP 松弛的最优值。约束方向不同会改变对偶变量的符号域，但约化成本定义不变。

## 2. 上下文边界

推荐把列生成分成三个职责：

| 上下文/服务 | 拥有 | 接收 | 发布 |
|---|---|---|---|
| Generation / Pricing | 可行列的领域定义、资源状态、搜索图、标签和支配规则 | `ShadowPriceMap`、迭代信息 | 新的领域列及其成本 |
| Compilation / Master | 列变量、覆盖/平衡约束、目标项、对偶值映射 | 初始列和增量列 | 主问题解、下界、`ShadowPriceMap` |
| Selection / Application | 迭代、停止条件、去重、整数化和诊断 | 两侧结果 | 最终领域解和求解轨迹 |

例如航班恢复中：

- `FlightTaskBunch` 是列，不是一个裸向量；
- `bunch_generation` 根据航班图、规则和资源生成可行航班串；
- `bunch_compilation` 注册航班覆盖、机队平衡和容量约束；
- `bunch_selection` 或 Application 编排迭代。

同一骨架也适用于一维下料中的 cutting pattern、三维装箱中的 layer，以及 VRPTW 中的 route。

## 3. 两个跨上下文协议

### 3.1 ShadowPriceMap

`ShadowPriceMap` 应以领域键访问对偶值：

~~~kotlin
interface ShadowPriceMap {
    operator fun invoke(task: FlightTask): Flt64
    operator fun invoke(aircraft: Aircraft): Flt64
}
~~~

它隔离三件会变化的事情：

1. 主问题约束在求解器中的行号；
2. 后端返回对偶值的 API 和符号约定；
3. 定价子问题使用的领域对象。

Compilation 负责“约束 → 领域键 → 对偶值”的映射；Generation 只使用领域键，不持有主问题或 solver handle。

### 3.2 Column

从定价侧返回的列至少应包含：

| 字段 | 用途 |
|---|---|
| 稳定标识/签名 | 去重与追踪 |
| 领域内容 | 航班串、路径、切割方案等 |
| 原始成本 $c_p$ | 构造目标系数 |
| 主问题系数 $a_{ip}$ 或其可推导信息 | 增量构造列 |
| 约化成本与生成迭代 | 诊断和验证 |

主问题只从领域列构造 $\lambda_p$ 和列系数；不要让定价侧直接修改 RMP。

## 4. 生命周期

一次稳定的列生成求解应按以下顺序执行：

1. 初始化所有上下文和领域数据；
2. 生成能够使 RMP 可行的初始列，或加入有罚成本的人工列；
3. Compilation 注册主问题变量、目标和约束；
4. 求解 RMP 的 LP 松弛；
5. 按领域键提取对偶值，生成 `ShadowPriceMap`；
6. Pricing 对每个可分解实体求解定价问题；
7. 校验列可行性、重新计算约化成本、去重并批量加列；
8. 若仍有改善列则回到第 4 步；
9. 依据质量要求执行当前列集上的整数求解或进入 Branch-and-Price；
10. 把列变量值还原为领域方案并输出轨迹。

~~~kotlin
while (iteration < maxIterations) {
    val masterResult = compilation.solveRelaxation()
    val shadowPrices = compilation.shadowPriceMap(masterResult)

    val candidates = generation.generate(iteration, shadowPrices)
    val accepted = candidates
        .filter { generation.isFeasible(it) }
        .filter { compilation.reducedCost(it, shadowPrices) < -reducedCostTolerance }
        .distinctBy { it.signature }

    if (accepted.isEmpty()) {
        break
    }
    compilation.addColumns(accepted)
    iteration += 1
}
~~~

上面是职责示意，不是对某个固定 API 签名的承诺。实际实现应由 Context 暴露注册和分析能力，由算法服务负责循环。

## 5. 初始列与可行性

初始列是工程上最容易被忽略的部分：

- 每个必须覆盖的主问题行至少应被某个初始列覆盖；
- 若暂时无法构造可行列，可增加高罚成本人工列并在收敛前确认其取值归零；
- 预先锁定的任务、资源可用性和不可拆分规则应在初始列与定价中使用同一套判定；
- Pricing 发布列后，Compilation 仍应在插入前进行轻量契约校验。

若 RMP 不可行，不应继续读取对偶值。应区分“初始列不足”和“原问题确实不可行”。

## 6. 停止条件与数值策略

至少记录并配置：

| 配置 | 建议含义 |
|---|---|
| $\varepsilon_{\mathrm{rc}}$ | 接受负约化成本列的容差 |
| `maxIterations` | 最大迭代数 |
| `maxColumnsPerIteration` | 单轮加入列数 |
| `duplicateTolerance` | 系数或领域签名去重规则 |
| `stallLimit` | 下界或目标长期无改善的上限 |
| `pricingTimeLimit` | 单个或整轮定价时间上限 |

以下状态必须分别报告：

- **证明收敛**：精确定价未找到改善列；
- **启发式停止**：启发式定价未找到列，但不能证明不存在；
- **资源停止**：达到时间或迭代上限；
- **失败**：主问题、对偶提取或定价出错。

## 7. Branch-and-Price 的额外义务

若要求整数最优，需要在分支节点重复列生成，并保证分支规则能传递到定价子问题。分支条件应尽量使用领域关系，例如“两个任务是否在同一条航班串”或“某条边是否被使用”，而不是只固定某个当前列变量。

每个节点必须维护：

- 节点专属的分支约束；
- 与分支约束一致的 Pricing 可行域；
- 节点下界、 incumbent 和剪枝原因；
- 生成列的作用域及可复用条件。

只在根节点完成列生成后把当前 RMP 改成整数模型，是一种实用启发式，但不能替代 Branch-and-Price 的全局最优性证明。

## 8. 验证清单

### Pricing 单元测试

- 对小图或小组合空间穷举所有可行列，与 Pricing 返回的最小约化成本比较；
- 验证资源扩展、支配规则不会删除潜在最优标签；
- 覆盖零对偶、正/负对偶、刚好落在容差边界的情况；
- 验证返回列满足全部领域规则，成本和签名稳定。

### Master 单元测试

- 加一列后，变量数、目标系数和每个受影响行的系数正确；
- `ShadowPriceMap` 的领域键、行和符号一一对应；
- 重复列不会被二次插入；
- 人工列、锁定列和空覆盖行有明确处理。

### 集成与基准测试

- 在可枚举的小实例上，把所有列一次性加入完整模型，与列生成的 LP 目标比较；
- 重新计算每个已选列的约化成本，并检查终止时不存在小于 $-\varepsilon_{\mathrm{rc}}$ 的列；
- 验证下界单调性、列数、迭代数和停止原因；
- 若使用 Branch-and-Price，与完整小规模 MILP 的整数最优值比较。

## 9. 常见错误

- 用 solver 行号作为 `ShadowPriceMap` 的公开键；
- Pricing 和 Master 分别实现一份列成本或可行性逻辑；
- 将“未找到列”一律报告为“已证明最优”；
- 忽略等式、上下界和稳定化项对约化成本的贡献；
- 把列生成等同于 Branch-and-Price；
- 在 Application 中展开领域列的所有系数，导致上下文边界失效。

## 10. 相关示例

- [复杂示例 3：一维分切](/zh-cn/examples/framework-example3)
- [复杂示例 4：航班恢复](/zh-cn/examples/framework-example4)
- [复杂示例 5：VRPTW Branch-and-Price](/zh-cn/examples/framework-example5)
- [DDD 基础架构](/zh-cn/guide/use-ddd-architecture)
