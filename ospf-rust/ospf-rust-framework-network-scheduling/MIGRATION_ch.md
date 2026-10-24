# Network / Gantt 共享内核迁移

[English](MIGRATION.md)

本说明对应 Kotlin `907d28d`、`3dc1225` 的 Rust 增量。当前包含共享基础、增强 Flow、
广义流、建设、显式韧性、根树结构、运输分配、完整路径有限主问题、七个独立入口及可选桥接，不代表两个提交的全部领域或示例已交付。
共享路径、任务/束批次、扩展行定价矩阵及 Demo4 的 Gantt 闭环已接通。
整体迁移验收仍未完成，须遵守下述证明边界及回归要求。

## Flow 与广义流

- `FlowGraph`、`SupplyDemand` 分别替换为 `FlowProblem`、`FlowCommodity`。
  问题字段不再允许外部修改，改用 `nodes()`、`arcs()`、`commodities()`、`units()`；
  `FlowContext::problem()` 和 `FlowAggregation::problem()` 返回不可变快照。
- 商品可声明独立 `unit`、`allowed_arcs`、覆盖默认费用的 `costs`、显式
  `capacity_consumption` 及 `integer_step`。整数变量表示物理步长的计数，而非整单
  二元选择；守恒、容量、资源及费用均按同一步长缩放。
- 注册在修改模型前准备并校验全部步长缩放后的容量、资源、费用系数及派生变量界，
  `minimize_cost` 为 false 也不绕过检查。非有限运算和容量缩放下溢为零事务式失败；
  管道及下游表达式复用已校验的系数，不重复计算未检查的乘积。
- `with_resources` 接入显式资源用量；`with_node_capacities` 把节点入/出处理能力
  编译为同一资源管道。跨量纲商品容量消耗必须显式映射。
- 默认硬供需。`FlowBalanceSlack` 显式声明有界弃供/缺货及非负惩罚；
  `extract_solution` 现在返回通过独立容量、资源、守恒和整数性校验的 `FlowSolution`，
  含运行、弃供、缺货及总费用。非法数值在构造或注册前边界失败。
- `GeneralizedFlowProblem` 的节点声明计量与硬净供给，弧声明有限起点投入容量、
  投入资费、无量纲业务产率及可选单位/工艺转换。跨量纲转换必须显式给定。
  `GeneralizedFlowContext` 事务注册后提供投入、产出、净供给和费用表达式；
  `GeneralizedFlowPipeline` 可追加下游约束/目标。
- `GeneralizedFlowSolverTolerance` 默认绝对/相对容差均为 `1e-7`。生产提取检查
  缺失、负数、非有限值、容量、实际净供给残差及每一步转换；不以大额循环流量
  放宽小净供给的残差，也不允许有限数相减/累加后的溢出。
- `FlowApplicationService`、`GeneralizedFlowApplicationService` 每次创建独立 context，
  返回现有 `SolveReport<领域解>`。通用 `NetworkApplicationService` 接收新 context
  工厂与 `SolveOptions`，复核完整模型 incumbent 后再调用领域提取；正常限时无解、
  取消和不可行仍保留报告。领域报告不提供定价对偶，数值审计在 `solver.*` 扩展
  中保留并附显式 warning，不能作为 LP 定价证书重放。

## 建设与显式韧性

- `NetworkDesignProblem` 在模型修改前验证通道、设施、等级、费用和预算。
  `CapacityLevel.capacity` 是总容量，不能超过名义有限包络；等级互斥，支持必选。
  `NetworkConstruction` 仅注册建设变量、互斥与预算，不隐式创建名义流，可供结构领域复用。
- `NetworkDesignContext` 组合建设与同一 Flow 内核。`FlowRegistration` 可关闭嵌入式
  运行费用目标，`with_name` 隔离各场景命名空间；同模型重复注册必须保持选项一致。
  Flow 聚合公开 `capacity_expression`、`resource_expression` 与 `cost_expression`。
- `NetworkResilienceProblem` 必须显式给出 `FailureScenario`。所有场景共享一组建设
  变量，但有独立商品流及可选供需/松弛覆盖；失效节点不自动免除需求。
- 建成总容量、场景降级上界和设施启用是独立约束。合法降级小于启用硬下界时，保留
  相互冲突的行形成正常模型不可行，不构造非法容量区间。负降级或超名义增容仍是输入错误。
- 默认目标为建设加最坏场景费用。提取区分实际 `worst_operating_cost` 与 solver 的
  `operating_cost_epigraph`，非最优 incumbent 的上包络可有余量，不能伪装成实际运行费用。
- `ConnectivityRequirement` 的辅助流覆盖可达、边不相交与内部节点不相交，保留平行弧，
  不用自环伪造连通。辅助流只证明名义建设图结构，不替代任何故障场景的需求容量校验。
- 两个 context 均事务式注册、拒绝跨模型复用，并在生产提取复核二元性、联动、预算、
  守恒及派生结果。下游使用各自 pipeline 访问建设、场景与费用表达式。
  `NetworkDesignApplicationService`、`NetworkResilienceApplicationService` 使用相同注册与统一报告。

## 根树结构与设施归属

- `NetworkStructureProblem` 组合已校验建设问题、根、必选终端、可选 Steiner 节点、
  合格分区请求及设施数量配额。只复用 `NetworkConstruction`，不隐式注册名义流变量、
  守恒、流容量行或运行资费。
- `RootedTreeComponent` 使用唯一父弧与按有限候选节点数推导的深度界，排除自环、
  保留平行弧稳定 ID。树弧需要已建通道和活动端点，生产提取独立沿每条父链回溯到根。
- 分区归属不隐含运输或树成员资格。未声明为候选的设施固定启用；声明的设施启用变量
  联动归属。必需分区恰好一个合格所有者，可选分区至多一个；配额统计归属数量而非物理流量。
- 合法断联、必需分区无合格设施、最低配额不可达均保留正常模型不可行；未知引用、
  重复定义和反向配额区间为输入错误。
- `NetworkStructurePipeline` 公开建设、父弧、树弧和归属表达式供下游追加约束或目标。
  注册原子回滚，重复选项必须一致，context 不能跨模型。独立入口
  `NetworkStructureApplicationService` 使用完全相同的注册及生产提取路径。

## 运输分配

- `AllocationProblem::new` 在模型修改前验证地点、服务、请求快照、可选非递减层级、
  显式兼容服务、共享配额和枢纽，再单次映射 `NetworkDesignProblem`。业务属性仍在
  allocation 问题中完整保留，不引入第二套流内核。
- `AllocationOptions` 显式声明规范容量及总费用单位。服务默认资费按容量单位计量，
  请求覆盖按请求单位计量且优先使用；异量纲容量必须声明 `service_usage`。
  `integer_step` 同时缩放守恒、费用及资源消耗，允许整数量请求在平行服务间分拆。
- 配额只使用显式用量，缺省项为零，每个配额至多由一个枢纽控制。枢纽启用通过既有
  Design 联动规则控制关联服务和指定资源行；等级仍表示总容量，预算仅限制建设费用。
  配额/容量用量的未知或不兼容请求服务键在输入边界失败。
- `AllocationContext` 直接委托 Design 的事务注册和生产提取，公开其 aggregation 并接收
  `NetworkDesignPipeline` 扩展。`AllocationApplicationService` 每次创建新 context，返回
  统一领域报告；非法 incumbent 无法绕过下层流守恒、整数步长、容量和配额校验。

## 完整路径有限主问题

- `PathCompilationProblem` 声明显式行单位、有限可选边界、完整费用单位及初始
  `PathColumn`。每条完整路径对应二元变量，payload 保留内部活动/时间，费用与
  用量允许非可加。缺省系数为零，未知行键在修改模型前失败。
- 相同列 ID 仅在 payload、规范化费用及全部系数一致时幂等，批次冲突原子回滚。
  删除清理受管行及所有目标中的旧变量项，保留无关扩展。core 尚无物理删变量接口，
  旧 token 固定为零，重加创建新变量；扩展 remove hook 清理它自己的相关引用。
- `PathCompilationContext`、`PathCompilationPipeline` 与独立入口
  `PathCompilationApplicationService` 共用事务式注册、增删列、对偶刷新与提取。
  公开行/费用表达式，生产提取审核完整模型并独立复核二元、行界和完整费用。
  `candidate_ids` 仅是最终有限池，MILP 最优不能外推到更大路径全集。
- 真实 LP 须求解 `pricing_model(&model)`。证明性刷新接收报告或
  `VerifiedPricingDuals`，重新验证当前模型，直接消费全部声明行的规范化有符号对偶，
  包含等式正反行、上下界及零价格。未映射外部约束禁止定价；需要定价的业务约束应
  预先声明为 problem 行，并为完整路径提供系数。
- scope 绑定请求、LP 模型指纹（`scope.model`）、对偶轮次、分支、分组及执行器/时隙域。
  修订、变量身份、目标快照、有效边界/行/类型和当前刷新 epoch 共同约束对偶有效性。
  模型变化或刷新失败使旧快照失效。裸价格即使与证书数值相同仍为启发式。
  完整 reduced cost 消费全部声明行；快照有效性本身不证明穷尽、无改进或整数最优。

## VRPTW 修正

普通与 BigDecimal 入口均移除直达超窗的永久排除，只保留单调容量排除。两条数值
路径共用 `PathSearchContext`，保留稳定弧身份和公开标签的真实前驱元数据。
对全部完整终点重新执行业务成本策略，再计算 reduced cost、筛选与排序；Phase I
忽略真实运行费用。未认证的成本分解或支配策略默认不参与证明性支配剪枝。
`DefaultPricingColumnSelector` 保留定价入口的 reduced-cost 排序，只截取输出。
精确图构建器支持 `with_cost_policy(instance, policy)`，标签、完整费用、对偶及
reduced cost 始终使用 BigDecimal。后续预算中断保留已接纳终点；零标签预算不创建
根标签；标签预算耗尽的诊断优先于深度截断。小多重图 oracle 覆盖两个阶段、平行弧
及非直达时间窗恢复，并验证低于 f64 精度的改进。扩展行统一提供器和每模型工厂
已接入，详见“路线扩展行与工厂”；12 类组合仍须作为当前版本门禁实际执行，不能
由任一数值路径编译成功推定通过。

## 图与路径

- 保留 `NetworkGraph<N, A>` 和包装载荷的节点/弧类型，增加 `NetworkGraphAccess`。
  `filtered` 现在返回图视图而非弧迭代器；使用 `view.arcs()` 遍历弧。
- `reverse()`、`filtered_nodes()` 和视图上的继续过滤保留稳定节点/弧身份，不克隆
  业务载荷。反向图必须通过 `view.from(arc)`、`view.to(arc)` 解释端点。
- `NetworkTopology` 提供可达、最少弧数见证、DAG 排序、SCC 和目标相关最短路；
  只有起点可达且能到达目标的负环才是 `NegativeCycle`。
- `shortest_path(&start, &target, &unit, cost)` 替换裸数值回调接口。回调返回
  `Result<Quantity<V, Unit>>`；`ShortestPathResult<V>::Found` 包含指定单位的费用。
  归一化、比较和累计均在 `V` 内完成，不把权重转换为 f64。旧回调须包装 `Quantity`，
  无量纲费用传入 `NoneUnit`。不兼容/仿射单位及非有限权重、转换或累计值返回结构化
  错误；decimal/rational 测试覆盖非负与负权搜索中超出 f64 精度的区别，以及超出其
  范围的 decimal 值。
- `PathProblem::new(graph, policy)` 后设置 `starts`、`terminals` 和 `options`，
  再调用 `PathSearchContext.search(&problem, &budget)`。`PathPolicy` 提供开始、
  转移、完整终点贡献、独立校验和可选支配；默认没有支配，不假定资源单调。
- 循环非 elementary 搜索必须设置业务 `maximum_steps`；DAG 无需该限制。
  `maximum_results` 仅限制输出，不提前停止搜索。必须同时读取结果、诊断和
  `complete()`；空结果不证明不可行或不存在改进列。

## 目标与证据

- `MetaModel::objective_snapshot` 是当前已注册变量目标的独立副本；数学符号入口
  在注册时已解析到该目标表示，和后端使用同一个来源。
- `replace_sub_objective` 原位替换受管命名目标，保留其他目标；调用方必须拥有该
  名称。`remove_objective_terms` 从所有目标移除变量项，但保留常数、权重和方向。
  路线编译 Phase I/II 刷新不再覆盖业务附加目标，移除路线后会清理旧变量目标项。
- `VerifiedPricingDuals::new` 对真实 LP 模型复核报告、指纹、primal、dual 和目标。
  只读句柄在使用前通过 `validate_for` 拒绝模型变更后的旧证书。
- `ColumnGenerationBatch` 区分穷尽、计算预算、中断和启发式，检查精确请求范围及
  子请求覆盖。标识一致不证明对偶来源；空列不自动证明无改进，也不证明整数最优。
- Gantt 旧生成策略默认包装为启发式；需要声明完成度时实现 `generate_bunch_batch`，
  返回与 `BunchPricingRequest.scope` 一致的批次。未经证书校验的有限行对偶只能
  运行启发式定价，不能获得全域作用域或提升下界。
- `Iteration` 的 `best_dual_obj`、`lower_bound` 和进度指标现为 `Option`，未知时为空。
  删除从候选 reduced cost 构造下界的接口。局部或受隐藏/固定域影响的批次不能自动
  认证完整业务域，最终有限池 MILP 最优仍不是全局整数最优。

## 数值与可选桥接

`NetworkSchedulingSolverValueAdapter` 拒绝非有限 solver 转换结果；`normalize` 在
转换前检查原始量和目标都是线性单位。下游构造单位乘除之前也必须先调用
`validate_linear_units`，避免仿射单位触发低层断言。

`ospf-rust-framework-gantt-network` 不拥有模型、不启动 solver，只映射路径及批次。
领域状态和完整弧回溯保持在返回对象中。候选提取拒绝会降级完成度；独立无改进
判定不能仅依赖已截取的候选集合为空。

本地兼容工具链为 `nightly-2026-08-01`。最新 nightly 的 allocator trait 已与现有
基础库不兼容；本轮不为此修改无关容器合同。SCIP 夹具覆盖 Flow、广义流、建设、
显式韧性、根树结构、运输分配与完整路径七类独立入口。Demo5 的 Gurobi/SCIP 库内与 integration 门禁分别执行，
覆盖 direct-MIP、全路线 oracle、smoke、strict-proof 及 SCIP 合法终态。
任务/束共享批次及当前模型 LP 来源验证已接入；Demo4 主模型闭环及非空原生用例已落地，整体迁移验收仍未完成。不能以有限候选或 smoke
通过宣称完整迁移或普遍整数最优，也不能以历史测试数字替代本轮验收。

## 路线扩展行与工厂

- 将 `BranchNodeSolverConfig::extensions` 替换为 `extension_factories`，每次创建
  独立的 `Box<dyn RouteCompilationExtension<V>>`。直接 context 也改为独占 boxed 扩展，
  不再跨模型复用已注册对象。
- `pricing_rows` 接受不可变 `Arc<RoutePricingRows<V>>`，无扩展时显式传空目录。
  分开使用主问题与 builder 时，两端须注入同一目录。提供器返回物理量化完整列及可选
  逐弧系数；只有显式认证并独立回放复核的可加分解才能启用支配。
- 全部价格键必填，包括零值。core 行归一化后上下界对偶已有符号，双侧行应直接相加。
  Phase I 为两侧行增加可行性人工变量，不计真实路线费用；Phase II 将人工变量固定为零。
  路线变量上界由客户覆盖隐式保证，避免定价遗漏边界对偶；空路线、未知及重复客户会被拒绝。
- 两条数值定价路径在筛选排序前重算完整终点系数。BigDecimal 系数、价格与 reduced cost
  不经 f64；12 类矩阵覆盖两阶段与无扩展、零价扩展、非零价扩展。
- 证明定价检查受管目标、行、活动/退休变量域及完整原行对偶，拒绝未映射外部/隐藏编译行
  和价格 hook 篡改。这些检查不能取代当前模型的真实 LP 报告来源验证。

## Demo4 恢复记录与路径桥接

Gantt 任务束主问题现已区分稳定 `BunchId` 与历史模型列索引。任务覆盖相同的不同恢复候选不会
误去重，同 ID 的冲突 payload 使整批原子失败。可选 `BunchMasterConfiguration` 通过既有迭代
编译路径注册任务覆盖、取消、执行器选择及自有目标；增列与带模型的退休操作刷新真实受管行/目标，
跨时隙批次整体回滚。静态管道快照不代表任意下游行自动更新。旧图生成器将真实费用与依赖对偶的
reduced cost 分离，使用独立于 LP 轮次的节点回溯/时间 ID。Demo4 已通过实际应用循环消费共享编译生命周期。

- `FlightTaskBunch` 从可变 ID 列表和端点缓存改为不可变 `FlightTaskRecovery` 序列；
  构造返回 `Result`，任务 ID、时间、机场和飞行额度均从实际记录派生。
- `RecoveryBunchGenerationContext` 复用共享资源路径及桥接 crate，不启动嵌套全局求解。
  返回完整业务记录、终点费用、前驱与完成度；不生成无改进或下界证明。
- Demo4 `BunchCompilationContext` 现通过 `RecoveryMasterConfiguration` 复用 Gantt 受管主问题。
  完整恢复 payload 原子登记，按飞机命名空间保留身份；退休重加使用新变量，结果经过完整矩阵生产校验。
  通过不可变且可回放的 `RecoveryProblem` 实现 Gantt 迭代 trait；context 不共享注册态，初始 incumbent 不自动固定飞机。
- 硬旅客覆盖、座位用量及跨飞机不兼容中转活动对，共用同一固定行提供器参与事务注册、退休和有符号对偶定价。
  生产提取独立复核完整恢复记录和旅客调度。
- `RecoveryApplicationService` 复用既有 Gantt 循环，不建立私有 CG。正常无 incumbent 保留终止原因，有限池不可行映射领域 Unknown。
  外层不带全局证明或界；保留最终候选 ID、数值主问题报告和逐轮 trace。硬覆盖域需可行初始列。
- 有界旅客取消/舱位改签共用动态容量/中转定价行；逐航段人数平衡防止负用量，取消沿行程传播且只减少原舱位。
  生产提取独立检查实际运输人数、实际飞机容量、中转及费用。同 OD 航班改签以有界整数连接流绑定实际选中恢复
  版本，支持拆分及非二元人数；原始和实际目标 OD 都须匹配。航班费用单列，原航班估算时间或未选中的其他版本
  不能授权实际连接。
- 终点机队计算实际起终点，含未使用飞机和已执行前序；有界整数缺口分别返回物理短缺与实际模型 slack。
  机队和旅客各自管理自有目标，保留下游扩展。
- 自动初始列复用同一共享路径生成，以完整原始记录为偏好，实际恢复仍取有限活动域；锁定任务贯穿整个生命周期。
  保留搜索证据、预算/中断及无 incumbent 诊断，不保证全局硬域种子可行，也不新增 Phase I。
- `demo4-offline` 覆盖非空 fake solver 闭环；`demo4-gurobi-cg` 用例实际运行 Gurobi，校验内部事件/生效链接，
  以及真实 RMP 对偶增列改善、跨飞机旅客和完整活动域独立 oracle，另有机队缺口、自动种子/锁定、舱位改签、
  真实对偶驱动的航班改签改善及拆分/非二元连接人数用例。
  仅 feature 转发不是证据；该 feature 的 CLI 以原始束自动种子、机队和锁定规则运行真实应用。
- Demo4 定价暂不签发无改进证书，旧不完整对偶退休回调禁用自动退休；显式退休/重加受测。
  这不是通用精确分支定价树；旧常量旅客、机队/链接伪松弛和重复 aggregation/pipeline API 已删除，
  以及纯 ID 恢复图/生成器和固定零结果的求解占位类型均已删除，改用受管 context、共享路径及实际应用。
  快照编译管道的链接拆分/飞机闲置软惩罚已接入受管列生命周期、完整有符号对偶定价及独立提取，
  半单位取消损失和空束仍闲置具有离线网格及原生测试。实际分配限制已支持关系/通用条件、生成与回放硬校验、
  有界整数软使用量、完整对偶行及独立提取；有限流控已将内部实际事件/完整停场包含、分窗容量、
  有界整数超额、对偶行与生产复核接入同一生命周期。容量窗事件端点归属显式定义，不外推为任意重叠停机坪
  占位或全局时空框架；飞行活动换机时也检查客/货容量类别。Demo4 当前交付范围见本节末，
  整体 Network / Gantt 计划不由此宣告完成。
  入口和命令见示例的成对 README。
- 快照审计另补齐 Demo4 固定方案释放建议：较差完整 reduced cost、最高完整费用、实际换机次数和
  可复现无放回随机策略，含独占 FIFO 禁忌、零配额和失败原子性；它不修改主模型或解除分支决定。
  `Lock` 由 ID-only DTO 改为精确实际 `departure_time`，通过 `with_departure_locks` 接入初始列、
  定价、回放及生产提取，与飞机锁定独立组合；合法时刻无候选保留正常无 incumbent。
  示例的快照职责对应表明确有限活动域及不新增全局货运/机组求解器的边界。
- Demo4 已执行前序的原任务身份在飞机域间唯一，并与待恢复目录互斥；共享路径完整束校验独立拒绝
  再次执行。前序决定实际起点和停场，但不作为新列重复收费或扣减剩余飞行额度。独立桥接还在
  搜索前拒绝空飞机/活动身份，避免返回无法按规范身份回放的列。
- Demo4 的 `RecoveryTransitionPolicy` 对应快照可注入转移规则及连接时间计算；实际前序和当前记录
  贯穿初始列、路径终止/trace 校验、定价、候选回放及独立生产提取。非法负时间返回错误，合法
  不满足拒绝候选；忽略连接时间的链接不绕过自定义规则或其他硬下限，不改变任何定价证明边界。
- Demo4 指定快照职责及其有限活动域应用迁移已完成，当前门禁包含非空离线、真实 Gurobi、相关
  Core/框架特性和严格 Clippy、Network SCIP、Demo5 两后端与全仓回归。此项交付独立于整个
  Network / Gantt 计划的总体结论；不新增全局下界、完整路径域整数最优或连续时间恢复承诺。
