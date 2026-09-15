# 使用领域驱动设计架构（Benders 分解）

Benders 分解把一部分决策留在主问题，把其余决策及复杂约束放入子问题。DDD 的价值在于：分解不破坏既有领域所有权；同一组 Context 可以通过**选择性注册**参与单体模型、主问题或子问题，而主从问题通过有类型的变量固定协议和切割协议通信。

## 1. 数学结构

考虑以下最小化问题，其中 $x$ 通常包含主问题的离散决策，$y$ 是给定 $x$ 后的连续追索决策：

$$
\begin{aligned}
\min_{x,y}\quad
& c^\mathsf{T}x+d^\mathsf{T}y \\
\text{s.t.}\quad
& Ax \ge b,\\
& Tx+Wy \ge h,\\
& x\in X,\quad y\ge 0.
\end{aligned}
$$

固定 $x=\bar x$ 后，子问题为：

$$
Q(\bar x)=
\min_{y\ge0}
\left\{
d^\mathsf{T}y
\;\middle|\;
Wy\ge h-T\bar x
\right\}.
$$

其对偶形式为：

$$
Q(\bar x)=
\max_{\pi\ge0}
\left\{
\pi^\mathsf{T}(h-T\bar x)
\;\middle|\;
W^\mathsf{T}\pi\le d
\right\}.
$$

主问题引入 $\theta$ 近似子问题价值函数：

$$
\begin{aligned}
\min_{x,\theta}\quad
& c^\mathsf{T}x+\theta\\
\text{s.t.}\quad
& Ax\ge b,\quad x\in X,\\
& \theta\ge
\pi^\mathsf{T}(h-Tx),
&& \forall \pi\in\mathcal E,\\
& r^\mathsf{T}(h-Tx)\le0,
&& \forall r\in\mathcal R.
\end{aligned}
$$

$\mathcal E$ 是对偶可行域的相关极点，生成**最优性切割**；$\mathcal R$ 是相关极射线，生成**可行性切割**。实际算法按需发现它们，不会预先枚举。

## 2. 如何选择分解边界

分解边界首先是数学决策，其次才是代码组织。适合放入主问题的内容通常包括：

- 决定整体组合结构的离散决策；
- 相对紧、能提供有效下界的约束；
- 子问题需要固定的耦合变量；
- 一个表示追索成本下界的 $\theta$。

适合放入子问题的内容通常包括：

- 固定主决策后可独立求解的连续变量；
- 计算量大但结构稳定的可行性或追索约束；
- 能从对偶解或不可行证书构造有效切割的部分；
- 可以按场景、资源或时间段并行拆分的部分。

不要仅因为某个目录名是 “security” 或 “capacity” 就把它放入子问题。应先证明固定 $x$ 后的子问题结构和切割有效性。

## 3. 上下文职责

| 构件 | 主要职责 | 对外协议 |
|---|---|---|
| Master Contexts | 注册 $x$、主问题约束、主目标和 $\theta$ | 主问题候选解 |
| Subproblem Contexts | 注册 $y$、耦合约束和追索目标 | 子问题状态、目标和证书 |
| Fixed-variable mapper | 把领域变量映射为本轮固定值 | `fixedVariables` |
| Cut factory | 把对偶解/射线转换为具名领域切割 | feasibility/optimality cut |
| Application/algorithm service | 迭代、界管理、停止、质量守卫和降级 | 领域解与诊断 |

领域对象拥有变量和中间值，Context 决定它们注册到哪个模型：

~~~kotlin
class StowageContext {
    fun register(model: AbstractLinearMetaModel<Flt64>): Try = TODO()

    fun registerForBendersMP(
        model: AbstractLinearMetaModel<Flt64>
    ): Try = TODO()

    fun registerForBendersSP(
        model: AbstractLinearMetaModel<Flt64>,
        fixedVariables: Map<AbstractVariableItem<*, *>, Flt64>
    ): Try = TODO()
}
~~~

这三个入口可以共享聚合和 Pipeline，但必须清楚记录各自注册的变量、中间值、目标与约束，不能依赖“某个入口碰巧先执行”的隐式状态。

## 4. 固定变量协议

`fixedVariables` 表示“主问题领域变量 → 当前候选值”，而不是“主模型列号 → 子模型列号”：

~~~kotlin
val fixedVariables =
    mutableMapOf<AbstractVariableItem<*, *>, Flt64>()

for (item in items.indices) {
    for (position in positions.indices) {
        fixedVariables[stowage.x[item, position]] =
            masterSolution[stowage.x[item, position]]
    }
}
~~~

推荐满足以下契约：

- 主从两侧使用同一领域变量身份或显式的稳定键；
- 每个子问题所需耦合变量都必须有值；
- 离散值按统一容差取整，并保留原始值用于诊断；
- 固定操作只影响本轮子问题，不污染下一轮；
- 映射缺项、重复项和越界值立即失败，不静默补零。

中间值可以继续作为上下文接口，但不能假设把所有中间值注册到两个元模型后会自动建立对应关系；主从通信必须显式。

## 5. 选择性注册

同一个领域上下文可能参与多条求解路径：

| 路径 | 注册方式 | 目的 |
|---|---|---|
| 单体 MILP | `register` | 构造完整基准模型或降级模型 |
| Benders 主问题 | `registerForBendersMP` | 注册主变量、紧约束与主目标 |
| Benders 子问题 | `registerForBendersSP` | 注册追索变量、固定关系与子目标 |

选择性注册应由 Context 或 Pipeline 组合实现，不应在 Application 中复制公式。业务模式也可以为同一中间值绑定不同实现，但公开语义必须保持不变。

## 6. 迭代生命周期

1. 初始化领域对象和参与分解的 Context；
2. 分别构造并注册主问题和子问题；
3. 主问题求解得到 $\bar x$、$\bar\theta$ 和下界；
4. 通过 `fixedVariables` 把 $\bar x$ 固定到子问题；
5. 求解子问题并分类处理：
   - 最优：提取对偶极点并生成最优性切割；
   - 不可行：提取 Farkas 证书/极射线并生成可行性切割；
   - 无界、超时或求解器错误：进入独立的异常策略；
6. 在领域层复核切割系数和方向，再加入主问题；
7. 更新上下界、gap、切割统计与收敛轨迹；
8. 未收敛时回到第 3 步；
9. 分析最终主问题解，必要时用子问题恢复 $y$；
10. 执行质量守卫；策略允许时降级到完整 MILP。

~~~kotlin
while (iteration < config.maxIterations) {
    val master = solveMaster()
    val fixed = mapFixedVariables(master)
    val sub = solveSubproblem(fixed)

    when (sub.status) {
        Optimal -> addOptimalityCut(cutFactory.fromDual(sub))
        Infeasible -> addFeasibilityCut(cutFactory.fromRay(sub))
        else -> return handleSubproblemFailure(sub)
    }

    updateBounds(master, sub)
    if (converged()) break
    iteration += 1
}
~~~

伪代码只说明职责；后端能否提供可靠对偶值、Farkas 证书和增量加约束能力，是实际实现的必要前置条件。

## 7. 界、收敛与切割重复

对最小化问题，主问题目标通常给出下界：

$$
LB_k = c^\mathsf{T}x^k+\theta^k.
$$

当子问题可行时，当前解给出上界候选：

$$
UB_k = c^\mathsf{T}x^k+Q(x^k).
$$

可使用绝对与相对 gap：

$$
\operatorname{gap}_{abs}=UB-LB,
\qquad
\operatorname{gap}_{rel}
=\frac{UB-LB}{\max(1,|UB|)}.
$$

停止时同时检查：

- $LB$ 与 $UB$ 均有效且顺序正确；
- gap 满足配置容差；
- 最新切割没有显著违反；
- 没有人工松弛或不可接受的降级状态；
- 相同切割不会因数值噪声反复加入。

切割应有规范化后的稳定签名，并记录来源上下文、迭代、类型、违反量和对偶证书。

## 8. 多子问题

当子问题可按场景 $s\in S$ 分解时，可以采用：

- **single-cut**：用一个 $\theta$ 和一条聚合切割，主问题小但可能收敛慢；
- **multi-cut**：每个场景使用 $\theta_s$ 和独立切割，主问题更大但信息更强；
- **并行求解**：每个场景子问题独立运行，Application 汇总确定性结果。

无论采用哪种方式，场景概率、目标权重和缺失场景的失败策略都必须在领域协议中明确，不能只存在于线程编排代码中。

## 9. 质量守卫与降级

生产应用不应把“求解器返回成功”当作唯一质量条件。可以监控：

- Benders gap 超限；
- 达到时间/迭代限制；
- 若干轮没有改善；
- 单位切割带来的界提升过低；
- 目标轨迹异常或切割反复；
- 子问题证书缺失或数值不稳定。

若策略允许，失败或质量不足时可降级到完整 MILP。降级模型必须复用同一组 Context 的 `register` 路径，并在结果中记录降级原因；不能用另一套未验证公式临时拼装。

## 10. 验证清单

### 分解等价性

- 在小实例上同时构造完整 MILP 与 Benders 模型，比较可行性和最优目标；
- 对固定的 $\bar x$，比较子问题目标与手工计算的 $Q(\bar x)$；
- 验证 MP/SP 选择性注册的并集覆盖完整模型，交集只包含有意共享的语义。

### 切割测试

- 最优性切割在生成点 $\bar x$ 处给出正确下界；
- 可行性切割排除当前不可行 $\bar x$，但不排除已知可行解；
- 对偶符号、常数项和 $T x$ 系数与后端约定一致；
- 切割命名、去重和容差边界稳定。

### 生命周期测试

- `fixedVariables` 完整、无陈旧值，离散值容差一致；
- 每轮 $LB$ 不下降、incumbent $UB$ 不上升（最小化问题，允许容差）；
- 分别覆盖最优、不可行、无界、超时和后端错误；
- 质量守卫触发正确的失败或 MILP 降级路径。

## 11. 常见错误

- 按代码目录而不是耦合矩阵选择分解边界；
- 把子问题不可行误当成算法失败，不生成可行性切割；
- 使用主/子模型位置索引映射变量；
- 在 Application 中复制 MP/SP 约束；
- 用不可比较的目标值计算 gap；
- 后端没有有效证书时仍声称生成了严格 Benders 切割；
- 降级路径使用另一套领域规则。

## 12. 相关内容

- [复杂示例 2：航空货运装载规划](/zh-cn/examples/framework-example2)
- [DDD 基础架构](/zh-cn/guide/use-ddd-architecture)
- [形式化设计与形式化验证](/zh-cn/guide/formal-design-and-formal-verification)
