# OSPF BPP1D

[English](README.md)

一维精确需求装箱。长度与重量使用独立单位参数；`PackingContext<V, L, W>`
通过显式 adapter 归一物理量，在 BigDecimal、BigRational 等原数值域内保留精度，
不会因尺寸相同而合并物品身份。

## 领域与注册

- `PackingProblem`、`Item`、`BinType`、`PackingPattern`：精确 `u64` 需求、
  可选有限库存、长度／重量容量；间距只在相邻件之间，没有额外尾间距。
  组合校验不展开大件数。
- `generate_patterns`：迭代有界枚举，显式返回 `complete` 和节点数；
  `initial_patterns`：独立件数预算的 FFD/BFD 初始列，不证明候选完备或库存可行。
- `PatternAssignmentContext`：普通 MILP、LP、final 共用注册；LP 展开后松弛所有
  整数（含扩展辅助整数），保留隐式／显式边界交集，final 恢复整数域。
- `PatternModelingExtension`：追加变量、约束、目标及结果指标。此接口是纯、可重入
  建模契约，禁止依赖旧列序号或外部共享可变副作用。增删列先完整构建新模型，
  全部成功后一次替换模型、池及聚合，失败不留下半批状态。
- `pack`／`verify_plans`：独立复核精确需求、库存、实际坐标、间距、重量、
  尺寸、单位及物料；保留全部空闲区间以及原域净长度／消耗长度，供指标输出。

## 应用与证据

`Bpp1dMilp` 与 `Bpp1dColumnGeneration` 目前提供明确的 **同域 f64** solver
边界；泛型 context 不会暗中舍入后送求解器。callback 接收实际 `LinearTriadModel`、
剩余总时限与取消句柄，返回现有 Core `SolveReport`。

有限池列生成保留完整候选／扩展矩阵，未激活的组合变量暂时固定为零。这是用矩阵
规模换取任意扩展行及辅助变量的安全定价，并非无限组合定价 oracle。每个 LP 报告
必须通过原始／对偶／目标独立验证，并匹配实际模型映射与指纹；不接受外部裸影子
价格。库存耦合导致 seed LP 不可行时可转 final，但不能宣称定价收敛。

final 使用完整去重的 **generated + seeds** 池，在全新、无人工变量的模型中求解。
原始报告只证明实际候选模型；`globally_optimal` 还要求完整枚举、整数证明及默认
领域规则。自定义策略／扩展保守保留已验证可行性，不擅自证明跨阶段规则等价。
受限池不可行不升级为领域不可行。目标常数与 backend `c*x` 分离，源快照只计一次。

`optimize_tail` 保留前缀箱及坐标，扣除前缀需求／库存，分别验证局部和完整方案，
并对完整结果执行 accept policy。全局建模扩展不能直接复用为局部规则。

## 边界

`load_packing_csv` 由调用方直接解析 V 并声明单位，闭合表头为
`id,length,weight,amount` 和 `id,length,max_weight,cost,inventory`，空库存表示无限。
畸形引号、重复／未知表头、列数错误、件数溢出均失败。

`Bpp1dDiscreteLoading` 以每舱位一个显式区间适配共享 BPP，保留显式冲突、
隔离不同空间、排除贴边冲突；不依赖 BPP2D，不推断布局选择。

feature 转发 `serde`、`async`、高精度及原生后端支持。callback 编排接口本身
独立保持同步，不在内部阻塞异步 runtime；异步 solver callback 尚未提供。
`kpi` 从已验证放置重新计算原域指标；`PackingRendererAdapter` 再次校验实际放置，
通过显式物理量及数值转换产生 `RenderSchema1d`，展示精度不得用作优化证据。
渲染 DTO 支持 `serde`。
