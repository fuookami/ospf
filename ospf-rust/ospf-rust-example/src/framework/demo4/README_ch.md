# Framework Demo4 - 航班恢复

[English](README.md) | 简体中文

## 迁移状态

`RecoveryApplicationService` 已复用现有 Gantt 循环接通初始列、RMP、当前模型对偶、共享路径定价、
增列、最终 MILP 和生产提取。硬旅客覆盖、座位容量及跨飞机中转行参与同一主模型和定价生命周期。
有界旅客取消/舱位及航班改签、终点机队、自动初始列及全生命周期锁定任务也已接入同一 context 和行提供器。
链接拆分软费用和按飞机配置的闲置费用也使用该注册、定价及生产提取路径。
实际分配限制现支持关系黑白名单、通用条件和硬/软严重性，软使用量及费用也参与同一主问题。
有限机场流控现使用实际内部事件和完整停场间隔，容量及可选超额费用同样参与该生命周期。
指定快照职责在下述有限恢复活动域内完成迁移，并通过非空离线、真实 Gurobi 应用及相关回归门禁。
这不等于整个 Network / Gantt 迁移计划的交付结论，也不声明全局整数最优。
`app::run` 在 `demo4-offline` 下展示领域，在 `demo4-gurobi-cg` 下额外运行真实 Gantt 应用。
旧纯 ID 图/生成器栈及返回固定零结果的分支定价占位类型已删除；调用方改用完整恢复域、共享路径和
`RecoveryApplicationService`。

## 恢复记录

- `FlightTaskRecovery` 保留原始 `FlightTaskImpl`、实际执行飞机、恢复航线及逐活动实际起止时间，
  构造时不会改写原任务。
- `FlightTaskBunch::new(id, aircraft, activities)` 现在返回 `Result`，拒绝重复原任务、冲突飞机配置、
  航线断联及时间重叠，替换旧 ID 列表构造方式和可变端点字段。
- `task_ids()`、`dep()`、`arr()`、`time()` 均为派生视图；空束没有虚构端点或依赖系统时钟的时间。
- `arrived_when`、`departed_when` 检查所有活动，包含内部中转机场，使用恢复后的事件时间和闭区间。
- `flight_hour()`、`flight_cycle()` 消费相同的实际记录，不计入地面维护和 AOG 活动。

## 规则及费用策略

`FeasibilityJudger::judge_bunch` 校验飞机可用性、实际前序活动、连接时间、原任务变更限制、
硬相邻或付费拆分链接和剩余飞行额度。显式 `ConnectionTimeIgnoring` 链接覆盖飞机过站时间，
但不取消其他声明链接的最小时间要求。已执行前序活动决定实际起点位置。
在飞行额度到期前起飞的航段完整计入该周期，不截断跨到期航段。
飞行及虚拟飞行换机时检查原任务与实际飞机的客/货容量类别；地面活动及未声明类别不虚构该要求。
`RecoveryProblem::with_transition_policy(Arc<dyn RecoveryTransitionPolicy>)` 接入快照中的可注入转移规则和
连接时间计算：`allows` 消费实际飞机、实际前序及当前恢复记录，首个活动没有前序时也执行；
`connection_time` 可以按任务对计算过站要求，默认沿用飞机的机场过站时间。策略必须是确定性的只读
业务定义，不保存模型注册态。自动初始列、完整共享路径终止/trace 校验、定价、候选回放及生产提取
共用同一策略，不能用只在生成侧过滤的回调替代。负连接时间及策略错误返回结构化错误；合法规则
不满足则拒绝该候选，不提升为全局不可行。显式忽略连接时间的链接只覆盖过站时间，不取消自定义
规则、其他链接的硬下限、时空连续或飞行额度。
已执行前序的原任务身份必须非空、在全部飞机域中唯一，且不再属于待恢复任务目录；否则在注册前
作为输入错误拒绝。独立 `judge_bunch` 也拒绝把相同原任务再次放入恢复束，不能因其是另一时刻的
恢复版本就重复执行。前序仍决定实际起点及连接/停场间隔，但不重复消耗剩余飞行额度或产生新列。

`judge_passengers` 独立复核所选任务束中的硬旅客行程，支持跨飞机中转，按任务和舱位汇总座位用量。
缺失所选航班不会被静默视为已服务。`RecoveryProblem::with_passengers` 在注册前建立硬覆盖、
任务/舱位聚合容量和不兼容恢复活动对的中转行；合法但无法满足的行程保留为正常模型不可行。
冲突行依据有限活动版本，不预枚举整条路线。最终提取再次运行独立旅客复核，不能由测试侧替代。

`RecoveryCostPolicy` 提供活动费用和完整任务束的终点贡献扩展点。默认 `CostCalculator` 使用实际
飞行时长和实际飞机费率，完整费用汇总拒绝 NaN、无穷及溢出。Demo4 既有费用及 solver 边界使用
`f64`，不由此承诺航空输入的任意精度或运行时单位支持。

## 共享路径生成

`RecoveryBunchGenerationContext` 接收有限 `RecoveryActivityCandidate` 集合，通过
`NetworkBunchGenerationContext` 运行共享 `PathProblem`。状态保留完整恢复序列，同一原任务的
多个恢复候选不能同时出现在同一路径。

结果同时保留业务任务束、完整费用、前驱 trace、诊断及真实完成度。非可加终点费用在排序前计算，
不启用支配；输出数量上限只截取返回候选，预算耗尽和中断均显式保留。每个请求精确声明一个无时隙飞机域。

本领域生成入口刻意**不返回无改进证明**。路径穷尽不等于 LP 来源已验证、可靠 LP 下界或全局整数最优。
`RecoveryPricingPolicy` 消费 Gantt 从当前已验证 LP 报告归一化的完整原行对偶，显式包含零。
裸价格仅走启发式计算，不能提升下界；缺行、未知外部行、非有限对偶和作用域错配均失败。

## 恢复主问题注册

`bunch_compilation::context::BunchCompilationContext::new(RecoveryMasterConfiguration)`
持有原始任务、完整飞机配置及取消费用，直接复用 Gantt `BasicBunchCompilationContext` 的受管
覆盖、取消、执行器选择行和费用目标，不再只是空编译结果列表，也不新增私有建模内核。

`add_candidates` 原子登记完整 `RecoveryBunchCandidate` 并增列。业务 ID 按飞机命名空间及长度编码；
相同 ID 的费用、原任务、恢复时间、航线及机场配置变化均失败。未知任务、原任务定义被改写和
飞机配置冲突在修改模型前失败。退休后保留不可变业务历史，相同候选重加分配新变量。

`extract_solution(model, values)` 通过 Gantt 的完整矩阵及整数生产校验后返回 `RecoverySchedule`，
保留原任务和实际时间，包括内部中转事件；取消只来自实际取消变量。只读 `master()` 暴露稳定变量
映射，下游可在同一 `MetaModel` 追加规则。独立二元枚举测试检查实际后端矩阵、回滚、硬任务和空域。

`with_resolver(configuration, Arc<RecoveryProblem<_>>)` 实现 Gantt 的迭代 context 合同。
业务目录不可变且不保存模型注册态；不同 context 独占列映射、约束组和模型身份。
完整候选身份按 UTF-8 字节长度编码活动版本，从目录回放实际记录，而不是凭任务 ID 猜时间。
旅客组件与 Gantt 加列/退休在同一模型事务中刷新，保留下游目标及约束。
初始 incumbent 只作为可改善方案，不自动固定飞机；真正的模型范围及显式生命周期操作仍生效。

## 应用入口与证明范围

### 有界旅客恢复

`with_passenger_recovery(RecoveryPassengerConfiguration)` 替换硬旅客模式。每个旅客组航段须明确提供
取消政策（整数 `0..amount` 或硬零）、允许的目标舱位及有限非负单人成本。取消只减少原舱位，改签
同时减少原舱位并增加目标舱位；剩余原舱位人数非负，禁止超额处置。取消沿行程传播；实际恢复版本
不能中转且不启用航班改签时，后续航段剩余旅客必须全部取消，而不强制取消已飞的前段。容量取实际所选飞机/舱位。
所有辅助行（含列系数为零的行）必须出现在当前模型的有符号对偶定价中。
`RecoverySchedule::passengers` 保留实际取消、改签、运输人数及分项费用。生产提取独立复核人数、
容量及实际中转，即使主模型相关行被移除也不能接收非法结果。

`RecoveryPassengerLegPolicy::flight_change_costs` 显式配置 `(目标航班, 目标舱位)` 到每人总改签费用的映射，
空映射关闭航班改签。目标须为另一同原始 OD 的航班类型任务；原航班及目标航班的实际运输版本均须保持
旅客航段 OD，原航班备降到其他机场不能被默认为仍服务原 OD。
改签减少原舱位并增加目标航班相应舱位需求，航班总改签费用不再重复计入舱位改签费用。
涉及航班改签的相邻段使用有界整数连接流：后段每位继续旅行的旅客必须有实际可行的前序连接，出流不超过
前段实际运输人数。两端均绑定实际选中完整恢复记录，而不是只核对任务 ID 或原航班估算时间；上界取旅客组
人数，不使用任意大 M。支持组内拆分及大于 1 的人数。结果保留实际航班/舱位改签、前后航班连接人数和分项
费用。生产入口独立拒绝缺失/错误 OD 的活动版本、连接流残差及超容量，即使相关模型行已被删除。

旧常量符号旅客模型、机队/链接伪松弛模型及重复的 aggregation/pipeline 注册入口已删除；
调用方改用当前受管 context、`with_passenger_recovery` 及机队 builder，不保留第二套建模内核。

### 链接拆分与飞机闲置

`Link::split_cost = None` 保留硬同机相邻语义；`Some(cost)` 允许拆分，要求费用有限非负且两端均为
待恢复任务。真正保留的相邻链接仍须满足最小连接时间，不能用软费用绕过实际连接可行性。
软链接注册 `kept + 0.5 * cancel_from + 0.5 * cancel_to + slack >= 1`，松弛范围为 `0..1`。
保留链接或两端取消的物理损失为 0，单端取消为 0.5，两端执行但拆分为 1。此公式遵循指定快照
`FlightLink`/阈值松弛语义，不复制 `FlightLinkLimit` 中与之矛盾的 `slack >= 1`。

`with_aircraft_idle_costs(BTreeMap<飞机注册号, 费用>)` 配置飞机闲置费用，未知飞机、负数和非有限值在
注册前失败。仅执行非空活动束才算使用飞机；未选择束和选中空束都算闲置。
该组件管理独立目标及约束组，在同一加删列事务中刷新；全部行及显式零系数参与完整有符号对偶定价。
`RecoverySchedule::operating_costs` 返回物理链接损失、实际模型松弛、闲置状态和分项费用。
限时可行解允许非最小的合法链接松弛，费用按实际松弛而不是物理最小值计算。生产入口独立复核这些
量，即使相关行被移除也拒绝非法辅助变量。半整数网格 oracle 覆盖 0.5 松弛，不能用纯二元枚举替代。

### 实际分配限制

旧 `Restriction::check()` 固定返回 `NotMatter` 的占位 API 及纯 ID `FeasibilityJudger::judge` 已替换。
`RestrictionType` 现在表示 `Weak`、`ViolableStrong`、`Strong` 严重性，业务种类由 `RestrictionRule`
区分。关系限制按有向机场对与实际飞机判断黑/白名单，不适用于非飞行活动；通用条件支持原任务集合、
实际出发/到达事件、双向航线完整时间包含、飞机集合与条件合取。所有时间范围为闭区间，缺少要求的
时间不命中，恢复记录绝不回退到原始航线或估算时间。`check_task`、`check_aircraft` 和
`check_assignment(RestrictionAssignment)` 复用同一检查口径；自定义只读条件通过
`GeneralRestrictionPredicate` 接入，不持有模型状态。

`RecoveryProblem::with_restrictions` 在模型修改前校验身份、引用、时间范围、权重、费用及有界总费用。
`Strong` 且 `cost=None` 为硬限制，贯穿自动初始列、定价、候选回放、受管行及独立生产提取；
配置费用的 `Strong` 与弱/可违反限制允许违反。通用限制的 `enabled_aircraft`/`disabled_aircraft`
是适用域及豁免域过滤器，按快照与命中条件合取，不是额外的反向黑名单。弱/可违反限制未配置费用
时仅报告违反次数和权重，货币费用为零，不猜测缺省价格。

软限制由 `0..任务数` 的整数计数平衡完整列使用量，目标为实际次数乘声明权重及费用；完整原行对偶
显式包含零并与主模型共用系数。`RecoverySchedule::restrictions` 返回违反任务、权重及分项费用。
生产提取独立复核实际记录、硬违反及整数计数，即使相关行已被移除仍拒绝伪造 incumbent。
上下文各自拥有变量与约束组，刷新与 Gantt 共用模型事务，保留下游扩展目标和约束。

### 有限机场流控

`with_flow_controls(Vec<FlowControl>)` 把有限机场事件/停场规则注册到同一主问题，不启动第二个求解器，
不构建通用全局时空框架。每条规则显式声明机场、场景、有限时间范围、正 `interval`、每窗容量及
每窗固定外部 `current_usage`；外部使用量不是可再次选择的任务。内部容量窗左闭右开，最后一窗包含
整体末端事件，防止相邻窗重复计数。最后一窗可截短；束的独立 `arrived_when`/`departed_when` 查询
仍保留原来的闭区间语义。

`Departure`、`Arrival` 检查束内每个实际活动事件，`DepartureArrival` 每活动每窗最多计一次，不能只看
整束端点。`Stay` 遵循指定快照的完整包含语义：前序实际到达至当前起飞（地面活动则至结束）的完整
间隔必须落在窗口内，不把局部重叠解释为通用停机坪占位。初始活动使用真实已执行前序；没有前序时
按出发事件检查。空束不虚构无限停场，也不再次计入已执行前序的独立事件。

`FlowControlCondition` 按实际航线推导国内/区域/国际类别，结合实际子型号及可选
`FlowControlPredicate`。条件过滤针对飞行活动，修正快照中直接排除所有飞行的反向判断；无条件规则
仍可显式覆盖地面活动事件。旧纯地点 `evaluate` 与占位条件 DTO 已替换为实际记录/窗口检查接口。

`over_cost=None` 表示硬上界；配置有限非负费用时使用有界整数超额变量，上界来自任务数和固定外部
用量而非任意大 M。合法外部使用量已超过硬容量时保留为模型不可行，不在输入阶段掩盖冲突。
未知引用、重复规则、非正间隔、非法时间/费用、非精确整数或非有限有界总费用在注册前失败。
全部容量行及显式零系数与完整有符号对偶定价共用同一提供器，刷新参与同一 Gantt 事务并保留扩展项。
`RecoverySchedule::flow_controls` 保留实际容量窗、外部及恢复用量、物理超额、真实模型松弛和费用。
生产提取重新检查完整实际事件和整数超额，即使相关模型行已删除也拒绝非法结果；限时解的合法
非最小松弛按真实变量收费，不用物理最小值替代。

### 机队与初始列

`with_fleet_balance` 配置机场/子型号终点硬下界或有界整数缺口费用；`with_original_fleet_balance`
从完整原始束推导目标。未使用飞机停留在实际初始位置，有已执行前序时以其实际到达位置为起点。
每列贡献“实际终点计数减原起点计数”，空束增量为零。`RecoverySchedule::fleet_balance` 区分物理
缺口、求解器实际 slack 及费用，允许限时可行解存在非最小 slack。未知引用、非法费用在注册前失败；
合法但无法满足的硬目标保留为模型不可行。机队及旅客组件只管理各自目标，与加删列共用模型事务。

`with_locked_tasks` 在初始生成、定价、候选回放及生产提取全过程禁止取消或换机执行锁定任务。
`with_departure_locks(Vec<Lock>)` 另外锁定原任务的精确实际开始时刻并禁止取消，不把它解释为
计划时间、最早时间或自动飞机锁定。`Lock` 从旧 ID-only DTO 改为含 `departure_time` 的实际时刻规则；
`locked_departure(task_id)` 提供只读查询。时刻锁定与飞机锁定可按任意 builder 顺序组合或独立清空，
清空一种规则不会意外恢复另一种规则禁止的取消。未知任务、重复规则或重复任务锁在注册前失败；
合法时刻在有限活动域中没有匹配版本时，保留正常无 incumbent，不凭空制造恢复活动。
实际时刻过滤贯穿自动初始列、共享路径定价、候选回放及独立生产提取。

设置 `RecoveryApplicationConfiguration::initial_generation = Some(RecoveryInitialConfiguration { originals })`
即可用同一共享路径生成自动初始列。原始完整记录仅提供顺序偏好，实际时间/航线仍取有限恢复活动域，
不从任务 ID 猜测。每飞机优先保留原任务顺序，再最少新增活动、最低完整费用。这种启发式种子不保证
全局硬旅客规则可行，仍没有 Phase I 或第二个全局求解器。`RecoverySolution::initial_generation`
保留实际搜索 trace、范围、预算及中断；没有 incumbent 时报告诊断也保留完成度和生成列数，不签发无改进证明。

构造 `RecoveryProblem::new(configuration, domains, links, cost)`，可选调用 `with_passengers`，然后将
`Arc<RecoveryProblem<_>>` 及 `RecoveryApplicationConfiguration` 传给
`RecoveryApplicationService::new(solver).solve(...)`。每次调用创建新 context/policy/模型。
硬覆盖或硬旅客域须提供可行初始列；当前入口不实现 Phase I 或嵌套全局求解来修复不可行种子。

统一返回 `GanttResult<SolveReport<RecoverySolution>>`；正常无 incumbent 保留终止原因，有限池
不可行映射为领域 `Unknown`，原状态保存在 `demo4.finiteMasterStatus`。技术失败和非法 incumbent 返回错误。
结果保留完整调度、最终 `candidate_ids`、逐轮 trace/作用域/reduced cost 和有限主问题报告。
外层不带全局最优证明或推测界，`demo4.proofScope=finite-candidates-only`。
即使路径搜索穷尽，也暂不签发无改进证书；这不能解释为通用精确分支定价树。
旧 reduced-cost 退休回调缺少扩展对偶，因此此 policy 保守禁用自动退休；context 的显式退休/重加受测。

### 固定方案释放建议

`bunch_compilation::service::FreeAircraftSelector` 不再使用“全部飞机减去已使用飞机”的占位实现。
通过 `FreeAircraftSelectorConfiguration` 依次配置较差 reduced cost、最高完整费用、最多实际换机次数及
不重复随机释放配额，默认各为 3，FIFO 禁忌容量为 10，严格阈值为 0，种子为 42。
`FreeAircraftSelectionRequest` 使用每飞机唯一的完整固定束、合法飞机目录、隐藏飞机及每架固定飞机
显式存在的完整 reduced cost；零价格不能遗漏，不能用只覆盖任务行的旧价格回调填充。
费用与 reduced cost 分离，换机次数从原任务和实际飞机派生，不读过期缓存。

隐藏飞机首先进入建议集合，不重复消耗配额；较差 reduced cost 策略不受禁忌限制，其余策略遵守
跨调用 FIFO 禁忌。零配额及零禁忌容量合法，有限无放回采样替换快照的随机重试上限；同种子/相同
输入可复现，但不承诺 Kotlin RNG 逐位一致。非法引用、冲突飞机配置、重复固定飞机和非有限费用/价格
在更新随机或禁忌状态前失败。每个应用应独占选择器状态，不跨分支共享。

该服务仅提供局部搜索的业务建议，不修改模型、不解除真正的分支决定、不签发定价证据；当前
`RecoveryApplicationService` 沿用 Gantt 默认自由执行器策略，不自动把建议施加为新的固定或隐藏域。

## 快照职责对应

本对应仅以 `907d28d` 和 `3dc1225` 为来源，不使用后续 Kotlin 改动，也不要求保留重复内核或同名文件。

| 快照职责 | Rust 对应及明确边界 |
| --- | --- |
| `FlightTaskBunch`、恢复分配及内部机场事件 | 不可变恢复记录及派生查询；原任务保持不变。 |
| 航线图、标签、任务换序、初始列 | 有限实际活动域、共享 Network elementary path 和原始顺序偏好；可行换序/备降由显式恢复版本表达，不生成所有连续时间方案。 |
| 可用性、前序、连接、飞行时长和循环 | 同一实际记录的完整终止校验及生产回放，不复制静态计划的错误排除剪枝。 |
| `RuleChecker`、`ConnectionTimeCalculator` 运算符 | 只读 `RecoveryTransitionPolicy` 贯穿初始列、共享路径、回放和生产提取；默认规则与原入口一致。 |
| `Lock`、链接、限制和流控 | 独立飞机/时刻锁定、硬/软链接、实际分配限制与有限容量窗，均进入当前受管生命周期。 |
| 编译管道、机队及旅客取消/改签 | Gantt 主模型及独占组件，同源完整列系数/有符号对偶、增删列事务和独立提取。 |
| `FreeAircraftSelector` | 完整费用/实际换机/显式完整 reduced cost 的有状态建议服务，不解除分支约束。 |
| `BranchAndPriceAlgorithm` | 复用 Gantt 循环及统一报告，有限候选最优不提升为全局整数证明。 |
| Cargo/Crew 空聚合、空输入输出及泛型数量示例 | 保留领域示例边界，不凭空添加货运或机组全局求解系统；类型示例不能当作航空端到端验证。 |

## Feature 门禁

```text
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-offline -- --list
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-offline
```

`demo4-offline` 无须原生求解器即可启用领域和桥接测试，覆盖非空多机场恢复记录、生效的链接及旅客
违规、独立排列穷举 oracle、多恢复候选、终点费用、预算及空束。

```text
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- --list
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- --include-ignored
cargo +nightly-2026-08-01 run -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- framework:demo4
```

原生测试不检测变量后跳过：运行时和许可证必须实际有效。用例验证非空恢复内部机场事件、生效链接、
从较差种子经真实 RMP 对偶生成更优列、跨飞机旅客中转及全活动域二元 oracle。另有机队实际缺口费用、
原始束/锁定任务自动种子、舱位改签、真实对偶驱动的航班改签种子改善及拆分/非二元人数用例，
均走真实 RMP/定价/最终 MILP 循环。链接拆分/闲置费用用例验证真实 RMP 对偶改善初始方案、
费用改变最终选择及实际半单位松弛，并与独立网格/手算结果比较。
限制原生用例验证实际时间条件影响种子费用、真实 RMP 对偶驱动增列改善，以及大于 1 的违反计数。
流控原生用例验证实际内部事件改变初始选择、真实对偶增列改善及非二元超额的最终 MILP 提取。
已执行历史原生用例以不同于初始可用地点的真实到达点生成恢复束，同时验证前序没有重选、重新
收费或重复扣减剩余小时/循环，完整停场行仍消费真实前序时间。
可注入转移原生用例要求比飞机默认值更长的实际过站时间，通过相同 Gantt 循环选中晚出发活动版本，
独立验证内部事件、实际时间及费用；离线用例另覆盖首活动规则、实际前序、非法时间及整批注册回滚。
列表核对、实际执行和严格 Clippy 均须重跑；不可用历史测试数字替代当前验收。

## 交付与证明边界

按指定快照核查的职责及 API 映射见上表；核查补齐了固定方案释放建议、精确实际出发锁定、历史任务
防重及贯穿生命周期的可注入转移规则。当前有限恢复活动域、硬/软旅客合同和编译管道费用共用受管
主链，不再保留待替换的私有生成/求解内核。Demo4 迁移及本轮应用、受影响数值/异步特性、原生后端和
全仓回归验收已完成；受影响 Core 和框架包的严格 Clippy 也纳入门禁。后续修改仍须重跑当前版本门禁，不能
以新增服务单元测试或历史测试数字替代应用验收。流控的有限容量窗、逐活动事件、
完整停场与生产复核等合同
不外推为任意重叠停机坪占位、所有连续时间恢复方案或通用全局时空框架。
旅客和费用用例通过不能单独替代完整验收。默认全仓中明确忽略的可选后端测试，不计入对应原生门禁；
Demo4、Network 独立入口和 Demo5 原生门禁须分别实际执行。
若要认证完整路径域下界，还须增加与当前 LP 来源、实际归一化价格和完整域绑定的无改进证据；
不能将现有启发式终止或有限池 MILP 报告提升为全局整数证明。
