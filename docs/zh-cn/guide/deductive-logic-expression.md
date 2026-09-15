# 数学模型的演绎逻辑表达

传统数学规划通常把模型写成目标函数和若干等式/不等式。这个表示适合交给求解器，却不足以表达“业务规则为什么成立、依赖哪些前提、能推出什么结论”。演绎逻辑表达把约束统一看作候选解上的**命题函数**，从而在领域知识、数学模型和可执行代码之间建立同一种语义。

## 1. 从标准形式到谓词形式

传统线性模型可以写成：

$$
\begin{aligned}
\min/\max\quad & c^\mathsf{T}x\\
\text{s.t.}\quad & Ax\ \rho\ b,\\
& x\in X,
\end{aligned}
$$

其中 $\rho$ 逐行取 $\le$、$=$ 或 $\ge$。演绎逻辑形式不限定约束必须长成线性不等式：

$$
\operatorname{opt}_{x\in X} f(x)
\quad\text{s.t.}\quad
\bigwedge_{p\in C} p(x).
$$

- $X$：候选解宇宙，包含变量类型、定义域以及建模前提；
- $f:X\to\mathbb R$：目标评价函数；
- $C$：约束谓词集合；
- $p:X\to\{\mathsf{true},\mathsf{false}\}$：一个领域命题在候选解上的真假。

可行域定义为：

$$
F(C)=
\left\{
x\in X
\;\middle|\;
\bigwedge_{p\in C}p(x)
\right\}.
$$

最小化问题的最优解集合为：

$$
\operatorname{Best}_{\min}(f,C)=
\left\{
x\in F(C)
\;\middle|\;
\forall x'\in F(C),\ f(x)\le f(x')
\right\}.
$$

最大化只需把最后的 $\le$ 改为 $\ge$。该定义把“可行”和“比所有其他可行解更优”分开，便于推理最优性、临界约束和无解原因。

## 2. 具体约束也是谓词

一个具体数学规划约束可以表示为三元组：

$$
p=(g,\rho,b),
\qquad
p(x)\equiv[g(x)\ \rho\ b],
$$

其中 $g:X\to\mathbb R$ 是符号表达式，$\rho\in\{\le,=,\ge\}$。例如容量约束：

$$
\sum_{i\in I}w_i x_i\le W
$$

对应：

$$
p_{\mathrm{capacity}}(x)
\equiv
\left[
\sum_{i\in I}w_i x_i\le W
\right].
$$

而在需求分析阶段，谓词可以暂时保持更抽象的形式：

$$
p_{\mathrm{capacity}}(x)
\equiv
\text{“方案 }x\text{ 不超过资源容量”}.
$$

形式化设计的任务，是在已声明前提下证明具体数值谓词与这条领域谓词等价，而不是直接把自然语言替换成一个看似合理的公式。

## 3. 逻辑连接和量词

领域规则可通过标准逻辑构造组合：

| 形式 | 含义 | 建模示例 |
|---|---|---|
| $p\land q$ | 两条规则都成立 | 同时满足容量和时窗 |
| $p\lor q$ | 至少一条成立 | 使用自有资源或外包 |
| $\neg p$ | 规则不成立 | 禁止选择某组合 |
| $p\Rightarrow q$ | 前件成立时后件必须成立 | 启用服务器则允许流量 |
| $p\Leftrightarrow q$ | 两侧语义等价 | 指示变量与业务状态一致 |
| $\forall i\in I:p_i$ | 每个对象都满足 | 每个需求都被覆盖 |
| $\exists i\in I:p_i$ | 至少一个对象满足 | 至少选择一个方案 |

OSPF 的逻辑函数符号可把部分命题编译为线性或二次模型，但逻辑语义与数值实现必须分开记录。Big-M、辅助变量和边界是编译策略，不是领域规则本身。

## 4. 前提、定义与约束

推理中要区分三类陈述：

1. **前提/假设**：定义候选宇宙 $X$，如需求非负、索引唯一、容量单位一致；
2. **定义**：给领域概念命名，如“节点已部署”等于若干指派变量之和；
3. **约束**：筛选可行解，如每个节点最多部署一个服务器。

将前提错误地写成决策约束，会掩盖输入数据错误；将定义只写在代码局部表达式中，则失去跨上下文复用与追溯能力。

定义一个中间值 $z$ 时，其逻辑含义是：

$$
\forall x\in X,\qquad
z(x)=g(x).
$$

因此具名中间值不是近似缓存，而是可被其他谓词引用的等价定义。

## 5. 服务器放置示例

设 $x_{is}\in\{0,1\}$ 表示服务器 $s$ 是否部署到节点 $i$。Route 上下文发布：

$$
\operatorname{NodeAssigned}_i
=\sum_{s\in S}x_{is},
\qquad
\operatorname{ServiceAssigned}_s
=\sum_{i\in N}x_{is}.
$$

两条业务规则可写成：

$$
\begin{aligned}
p_{\mathrm{node}}(x)
&\equiv
\forall i\in N:
\operatorname{NodeAssigned}_i\le1,\\
p_{\mathrm{service}}(x)
&\equiv
\forall s\in S:
\operatorname{ServiceAssigned}_s\le1.
\end{aligned}
$$

Bandwidth 上下文中的“未部署服务器就不能产生净流出”可以写成蕴含：

$$
\forall i\in N:\quad
\neg\operatorname{Deployed}_i
\Rightarrow
\operatorname{OutFlow}_i=0.
$$

若 $\operatorname{Deployed}_i$ 与二值化后的 $\operatorname{NodeAssigned}_i$ 等价，并且已知 $0\le\operatorname{OutFlow}_i\le U_i$，则可以演绎得到常见线性实现：

$$
\operatorname{OutFlow}_i
\le U_i\operatorname{Deployed}_i.
$$

这里 $U_i$ 必须来自可证明的有效上界。随意使用一个巨大的 M 会使实现虽然“形式相似”，却可能数值不稳定，甚至在边界不足时不等价。

## 6. 知识库中的推导关系

令 $K$ 为已有领域谓词集合，$q$ 为待验证结论。记：

$$
K\models q
\quad\Longleftrightarrow\quad
\forall x\in X:
\left(
\bigwedge_{p\in K}p(x)
\right)\Rightarrow q(x).
$$

这个关系可以回答：

- **蕴含**：现有规则能否推出新结论；
- **冗余**：若 $K\models q$，把 $q$ 再加入约束不会改变可行域；
- **冲突**：若 $K\models\neg q$，新规则与已有知识不相容；
- **等价**：$K\models(q\Leftrightarrow r)$，两个表达在前提下语义一致；
- **精化**：实现谓词比抽象谓词更具体，但在所需范围内保持等价。

一致性不是“每两条规则看起来不冲突”，而是可满足性：

$$
\operatorname{SAT}(K)
\quad\Longleftrightarrow\quad
\exists x\in X:
\bigwedge_{p\in K}p(x).
$$

## 7. 从符号约束到可执行谓词

为了在测试、回调或诊断中直接判断一组数值是否满足约束，可以把具体不等式包装成谓词。浮点实现必须显式使用容差：

~~~kotlin
fun satisfied(
    lhs: Flt64,
    relation: Relation,
    rhs: Flt64,
    tolerance: Flt64
): Boolean = when (relation) {
    LessEqual -> lhs <= rhs + tolerance
    Equal -> abs(lhs - rhs) <= tolerance
    GreaterEqual -> lhs + tolerance >= rhs
}
~~~

需要分别保存：

- **精确语义**：文档与证明中的 $g(x)\rho b$；
- **求解容差**：后端判定 primal feasibility 的规则；
- **业务容差**：业务可接受的偏差；
- **展示精度**：格式化数字的位数。

四者不能用同一个“保留小数位”代替。

## 8. 可追溯表达

每条规则建议维护以下记录：

| 字段 | 示例 |
|---|---|
| Domain statement | 每个节点最多部署一台服务器 |
| Predicate ID | `route.node_assignment` |
| Preconditions | 节点集合和服务器集合已去重 |
| Formal predicate | $\forall i,\sum_s x_{is}\le1$ |
| Intermediate values | $\operatorname{NodeAssigned}_i$ |
| Executable owner | `NodeAssignmentLimit` Pipeline |
| Evidence | 边界单元测试、模型快照、小规模最优解 |

这张映射表让需求评审、数学评审、代码评审和测试评审围绕同一条语义进行。

## 9. 使用边界

- 谓词表达不会让一般 MILP 变成可自动证明的定理；
- 求解器返回可行，只说明在数值容差下满足已编译约束，不说明领域规则完整；
- 逻辑等价必须在明确的 $X$ 和先验知识 $K$ 下成立；
- 非线性逻辑的线性化需要有限上下界和正确的严格/非严格边界；
- 回调只观察当前候选值时，必须处理值缺失、非整数候选和求解阶段差异。

## 10. 下一步

- [形式化设计与形式化验证](/zh-cn/guide/formal-design-and-formal-verification)
- [DDD 基础架构](/zh-cn/guide/use-ddd-architecture)
- [线性逻辑函数符号](/zh-cn/guide/linear-functional/and)
