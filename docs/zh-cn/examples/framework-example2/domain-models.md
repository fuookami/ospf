# Framework 示例 2：上下文领域模型索引

[English](../../examples/framework-example2/domain-models.md)

本索引是 `framework_demo/demo2` 下 11 个有界上下文在当前文档仓库中的边界。
每个上下文都有拆分模型页；下方覆盖矩阵补齐数据型或只暴露管线的上下文所需的
跨上下文变量/中间值/断言/约束/目标契约。父页面仍负责说明不同模式的注册边界；
上下文模型页描述的是上下文契约，并不表示该上下文在每种模式中都会注册。

## 1. 上下文图与符号

公共依赖方向为：

```text
aircraft → stowage → {mac, airworthiness_security, soft_security,
                      mac_optimization, express_effectiveness,
                      loading_effectiveness, redundancy,
                      recommended_weight_equalization, payload_maximization}
```

`I` 表示货物集合，`J` 表示飞机舱位集合，`P` 表示飞行阶段集合。`w_i` 是按
飞机配置重量单位表示的货物重量，`a_{ij}` 是由分配/调整得到的装载值，`L_j`
是舱位 `j` 的装载数量。下式是跨上下文契约的示意；实际生效索引集由源码中
状态谓词和变量固定规则决定。

## 2. 上下文模型页面

较长模型按上下文拆分，每页可以独立对照对应的聚合根和管线生成器审阅。

| 上下文 | 数学职责 | 本地模型 |
| --- | --- | --- |
| aircraft | 飞机、甲板、舱位、燃油、ULD 和邻接数据；不定义求解变量 | [aircraft](domain-aircraft/domain-model) |
| stowage | 货物-舱位分配、调整、装载数量、预测/推荐重量及核心装载限制 | [stowage](domain-stowage/domain-model) |
| mac | 由飞机和装载数据导出的力矩、CLIM、指数和 MAC 中间值 | [mac](domain-mac/domain-model) |
| airworthiness_security | 密度、累积/区域载荷、载荷、总重量、包络线、配平和 CLIM 限制 | [适航安全](domain-airworthiness_security/domain-model) |
| soft_security | 空舱位、舱门、空载分离和压舱物等软偏好 | [软安全](domain-soft_security/domain-model) |
| mac_optimization | 纵向/横向平衡及水平安定面限制与目标 | [MAC 优化](domain-mac_optimization/domain-model) |
| express_effectiveness | 必须发运和货物优先级顺序 | [快件装载效果](domain-express_effectiveness/domain-model) |
| loading_effectiveness | 同源/同目的地邻接、装载顺序、复称、拖车和序列策略 | [装载效果](domain-loading_effectiveness/domain-model) |
| redundancy | 冗余和实验纵向平衡限制 | [冗余](domain-redundancy/domain-model) |
| recommended_weight_equalization | 货物顺序、优先级预约及推荐重量偏差 | [推荐重量均衡](domain-recommended_weight_equalization/domain-model) |
| payload_maximization | 最大载荷限制和载荷目标 | [载荷最大化](domain-payload_maximization/domain-model) |

各页面均提供 English/中文切换；中文页面位于
`docs/zh-cn/examples/framework-example2/`。

### 2.1 模板覆盖矩阵

矩阵使每个上下文的数学角色可以独立审阅，不必把 11 个模型塞进一个页面。“复用”表示上下文消费装载/MAC 符号，
并不会创建重复的求解变量。

| 上下文 | 变量 | 中间值契约 | 断言/生效约束边界 | 目标 |
| --- | --- | --- | --- | --- |
| aircraft | 无（配置数据） | `neighbor(j,j')`、装载顺序及阶段/燃油查找 | 标识唯一、坐标有效且飞机数据单位兼容；无求解器行 | 无 |
| stowage | `x_{ij}\in{0,1}`、`u_{ij}\in{-1,0,1}`、`y_j\ge0`、`z_j\in\mathbb Z_{\ge0}` | `s_{ij}=x_{ij}+u_{ij}+loaded_{ij}`、`L_j=\sum_i s_{ij}`、`W_j=\sum_i w_i s_{ij}` | 分配货物满足 `\sum_j s_{ij}=1`；MLA/MLW 和预约管线 | 无直接目标 |
| mac | 复用装载变量 | `T_p=\sum_j arm_jW_{jp}+fuelMoment_p+dryMoment`、`MAC_p=index_p/weight_p` | 阶段公式/单位有效；无直接求解器行 | 无直接目标 |
| airworthiness_security | 复用 `W_j`、MAC、payload | `\rho_z=\sum_{j\in z}W_j/length_z`、端点累积载荷、包络点 | 密度、区域/累积、载荷、总重量和包络线限制 | 无直接目标 |
| soft_security | 复用装载/空位符号 | `empty_j=\mathbf1[L_j=0]`、压舱/建议偏差 | 空位/舱门/分离/压舱软管线（惩罚行） | 最小化加权软惩罚 |
| mac_optimization | 复用 MAC/扭矩 | `d_p=|MAC_p-target_p|` 和横向力矩 | 纵向范围、横向平衡、安定面管线 | 按配置最小化平衡偏差 |
| express_effectiveness | 复用装载指示 | `priorityGap_{ij}=priority_i-priority_j` 及相对顺序 | 必须发运和优先级顺序管线 | 最小化优先级违反 |
| loading_effectiveness | 复用装载/顺序符号 | 同源/同目的地邻接、序列和拖车变更表达式 | 邻接、提前/保留/复称/顺序/拖车管线 | 最小化配置的装载运营惩罚 |
| redundancy | 复用 MAC/扭矩 | `R_p=availableMargin_p-usedMargin_p`、实验平衡 | 冗余及实验纵向平衡管线 | 无直接目标 |
| recommended_weight_equalization | 复用 `z_j` 和顺序 | `D_j=|z_j-recommended_j|`、预约/顺序偏差 | 货物顺序、优先级预约和均衡管线 | 最小化 `\sum_jD_j` |
| payload_maximization | 复用 `W_j` | `Payload=\sum_jW_j` | WeightRecommendation 中的最大载荷限制 | 最大化 `Payload` |

## 3. 跨上下文数学契约

### 3.1 装载桥接

对于需要装载的货物-舱位对，核心表达式为：

$$
s_{ij}=x_{ij}+u_{ij}+loaded_{ij},
\qquad
L_j=\sum_{i\in I}s_{ij},
\qquad
W_j=\sum_{i\in I}w_i s_{ij}.
$$

其中 `x` 是二元分配变量，`u` 是当前实现使用的平衡三值调整变量，`loaded`
是已装货物的固定贡献。状态谓词之外的货物-舱位对会被源码固定为零。随后
由 `W_j` 为 MAC、适航、软安全和目标上下文提供输入。

### 3.2 典型不变量族

共享装载不变量为：

$$
\sum_{j\in J}s_{ij}=1 \quad(i\in I^{assign}),
\qquad
L_j\leq MLA_j,
\qquad
W_j\leq MLW_j.
$$

上下文专属限制在这些表达式之上添加条件。例如适航区域 `z` 可以施加：

$$
\underline{\rho}_z\leq
\frac{\sum_{j\in z}W_j}{length_z}
\leq\overline{\rho}_z,
\qquad
\sum_{j\in z}W_j\leq\overline W_z.
$$

包络线上下文约束的是每个飞行阶段的 `(phaseWeight_p, MAC_p)` 点属于对应可行
区域，而不是一个单独的重量上界。

### 3.3 模式注册

| 模式 | 普通模型中的上下文 | 重要边界 |
| --- | --- | --- |
| LoadingOrder | aircraft | 仅导出，不注册元模型或求解 |
| FullLoad | stowage、mac、airworthiness_security、soft_security、mac_optimization、express_effectiveness、loading_effectiveness | 适航上下文可能被移到 Benders 子问题 |
| Predistribution | FullLoad 上下文加 redundancy | 装载顺序管线受模式控制 |
| WeightRecommendation | stowage、mac、airworthiness_security、express_effectiveness、recommended_weight_equalization、payload_maximization | 推荐偏差和载荷目标是模式专属 |

父页面给出精确的注册调用以及 Benders 主问题/子问题划分。上下文中存在某个
类或模型文件，并不等同于约束已经生效；只有模式管线生成器返回的管线才会进入
模型。

## 4. 审阅清单

扩展上下文模型页时应保持：

1. 每个变量都声明二元、三值、整数或实数类型、物理单位、状态驱动的索引集和固定规则。
2. 每个中间值先用自然语言说明，再给出带量词公式；派生表达式本身不是约束。
3. 断言要和求解器约束区分，并为每个约束标注生效管线。
4. 目标按应用模式记录，不要把 FullLoad、Predistribution 和 WeightRecommendation 合并成一个目标。
5. 单位遵循 `aircraftModel.weightUnit`；只有配置单位为千克时才能把所有数值写成 kg。
