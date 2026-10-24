# 选择型超启发式搜索

[English](README.md) | 中文

迁移基线为 `ospf-kotlin` 的 `2eb7943`。本模块位于 `ospf-rust-core::solver::hyper_heuristic`，依赖已有 `heuristic` 原生骨架，不新增 crate，也不通过反复执行完整 solver 模拟会话。

自适应扩展基线为 `c66cd7f`，保留较新的共享 CMA 与异步重放修复。

## 使用入口

1. 使用 `SearchRegistry::with_builtins()` 注册单目标原生算法，使用 `register_problem` 提供独立问题工厂。
2. 使用 `HyperHeuristicConfig::new(...).algorithm(...)` 创建配置，或通过 `ConfigurationFormats` 读取 JSON/YAML/TOML。
3. `HyperHeuristicSolver::open` 创建独立运行；`advance` 推进串行片段、同步批次或一个异步完成事件，`run` 推进至停止，`close` 幂等关闭。
4. `result` 不触发评估；失败时可读取已经消费的成本、有效档案及诊断。

可执行示例：[`hyper_heuristic_demo.rs`](../../../examples/hyper_heuristic_demo.rs)。

```text
cargo run -p ospf-rust-core --example hyper_heuristic_demo
```

仓库现有基础库使用 specialization，因此需要支持该特性的工具链。测试环境仅安装 stable 时，本地验证使用了命令级 `RUSTC_BOOTSTRAP=1`；这不是本模块新增的工具链要求，不建议作为生产工具链配置。

## 支持范围

| 能力 | 实现 |
| --- | --- |
| 选择策略 | fixed、round-robin、stagnation、random、epsilon-greedy、ucb1、q-learning |
| 单目标原生算法 | genetic、gwo、sca、mvo、hs、wca、soa、warso、evo、gco、de、woa、hho、aoa、tlbo、ssa、gsa、fa、cs、ba、mfo、goa、pso、apso、clpso、shade、l-shade、cma-es、hill-climbing、ns、random-walk、simulated-annealing |
| 多目标 | NSGA-II/III；其他原生算法必须显式注册并引用标量化，档案保留原目标向量，`best()` 返回 None |
| 组合 | island、island-ga、memetic；子会话持续存在并共享父预算 |
| 算子槽位 | 种群/NSGA 的 update、mutation；轨迹的 neighborhood；PSO 的 inertia/personalWeight/socialWeight、CLPSO 的 inertia、L-SHADE 的 sizeRatio、模拟退火的 temperature |
| 执行 | 串行、线程同步批次、持续补位的线程异步完成顺序归并；不需要 async feature |
| 学习 | 实例隔离训练、冻结留出集、显式作用域重放、JSONL、等评估预算基线比较 |

Kotlin 算子目录提供 single-point/arithmetic/order/cycle/sbx（eta）更新；none/gaussian（sigma）/polynomial/swap/inversion 变异；perturbation（amount）邻域；constant/linear/geometric 参数源。Rust 还提供 two-point/uniform/blend/differential 更新、uniform 变异和 random/gaussian 邻域。全局策略支持 not-worse/always 接受及 none/periodic 交换（默认周期 5）。未知参数或不支持的槽位在首次搜索前拒绝，而非静默忽略。CMA-ES 仅在显式设置 restartOnImport=true 时提供重启分布的导入能力。

## 计费与生命周期

- 每个评估请求都消耗额度，包括不可行、重复、初始化和导入；`objective_calls` 仅统计真正调用目标函数的次数。
- 初始化是一个完整低层步。预算不足的残缺代不提交原生状态，但已经成功评估的候选可进入档案。
- 内置原生会话的低层档案只保存已提交状态成员和导入候选，容量为 100；所有有效评估候选仍返回高层。单目标高层档案按解身份去重，不删除不同解的同值目标。
- 仅在无在途片段的静止边界暂停有效计时；异步任务执行期间持续计时。阻塞的业务回调无法被强制中断，停止和时限是协作式的。
- 档案与工作集分离；接受策略只控制工作集，不能回滚算法私有状态。
- 双层选择只给真正使用的算子反馈；初始化未使用的槽位不产生虚假轨迹。

高层算法与槽位转移的 `steps` 均表示已提交的完整低层步数；残缺代中真实使用的算子仍获得反馈，但步数为零。全局仍有额度时，仅填满本片段不会令算子事件终止。评估请求被局部片段拒绝也报告 `EvaluationLimit`；完整一步恰好用满额度仍为 `SliceComplete`。槽位反馈失败不丢失已提交步数、已评估候选及其他真实使用槽位的反馈。

## 并行边界

每个 worker 在线程内部创建并独占问题、回调、会话和随机流，模型与算子不必实现 Send/Sync。工厂本身必须 Send+Sync。协调器独占全局记忆与高层学习；候选通过注册的无损档案 codec 传递，所以并行需要 archive codec。

派发前预留互不重叠的评估、步数和决策额度。同步批次按稳定算法 ID 归并；异步 `advance` 填充空闲 worker 并归并一个完成事件，保留其余在途片段，下次调用即可为快 worker 补位，不必等待慢 worker。反馈保留派发时上下文、动作和策略 revision。检查点、策略导出、停止、失败和关闭都会恰好一次排空全部在途结果，因此 `checkpoint` 和 `export_policy` 需要可变访问。不承诺跨模式轨迹一致。

## 扩展与数值边界

`SearchRegistry<O,V>` 提供问题、会话、无损 codec 和显式标量化注册。`registry.components` 提供选择器、特征、奖励、预算分配器、工作集接受策略和交换策略的独立工厂。`registry.operators` 提供类型安全的更新、变异、邻域和参数源工厂。组件使用 `RecoverableComponent` 显式声明无状态或实现 save/load；不可恢复组件在启用持久化时预检失败。

公共编排使用泛型 `O` 和 `V`；现有数值 trait 限定浮点值。内置单目标骨架/codec 的注册边界为 `f64/f64`，多目标为 `Vec<f64>/f64`。非连续编码的 genetic/NSGA 会话必须显式配置兼容的更新与变异池，如 order/cycle 和 swap/inversion；连续默认算子在预检阶段拒绝。其余原生算法不接受非连续编码，也可注册自定义会话。自定义标量化当前要求无状态，语义必须纳入问题 fingerprint。

## 策略、检查点与训练

`export_policy` 仅导出算法与算子学习策略，允许跨实例迁移；`RunOptions` 的 policy 与 checkpoint 互斥。`LearningMode::Frozen` 禁止学习和学习策略探索。`checkpoint` 包含原生状态、实际随机流、问题回调、算子与全局组件状态、记忆、预算、有效时间及轨迹。

`FileSearchStore` 提供独立版本、SHA-256 校验和、受限安全键和临时文件原子替换；未知版本只通过显式迁移接受。问题 fingerprint 必须覆盖输入、变量边界、目标方向、编码及回调运算语义。恢复需使用同一完整配置；不能将缓存或闭包本身当作可迁移状态。

`PolicyTrainer` 先按实例划分，再分配种子；验证和测试冻结。`PolicyEvaluator` 要求显式质量函数，多目标不能隐式求和。`PolicyTrainer::replay` 必须指定作用域，仅更新实际执行动作。

## SHADE 数值契约

SHADE 与 L-SHADE 支持显式算法参数 `"numerics": "shade-fdlibm-v1"`，固定柯西参数采样使用 fdlibm 正切（Rust 精确依赖 `libm=0.2.16`，Kotlin 使用 `StrictMath.tan`）、binary64 运算顺序及每对参数五次均匀抽样。省略该参数或使用 `"native"` 保留历史平台实现。不兼容的算法/契约组合及未知契约在搜索前拒绝。

契约纳入算法定义及检查点配置指纹；更换数值模式后恢复会失败，不静默改变搜索轨迹。与新版 Kotlin 互通使用普通 `save_checkpoint`；原始 Kotlin 导出保护会拒绝包括组合子节点在内的显式数值契约。本契约只覆盖该采样器，不覆盖任意目标回调、参数源实现、CMA-ES 或异步完成顺序；代表性 SHADE 运行的逐位比较不等于通用跨平台认证。

## 兼容性边界

改造后的 Kotlin 读取器单独以工作区提交 `c66cd7fa` 加上共享 SHADE 数值契约 验证，不替换原始迁移基线。Rust 读取并保留 `previousAction` 重放元数据，不表示已迁移新版新增的自适应选择器、异构算子或约束处理扩展。向原始基线导出时拒绝非空前驱元数据，不静默丢弃。

保留既有原生快照 codec，并对齐配置默认省略、canonical JSON 分隔符、UTF-16 键排序、algorithm/slot SplitMix64 子流、槽位局部 ID 和空池受控会话包装。浮点读取保持二进制精度，持久化校验和保留源数值文本（包括指数拼写）。

验证基线独立编译 Kotlin `2eb7943` 的五个基础模块及启发式模块，不引用 Kotlin 工作区的旧编译结果。两侧使用相同的显式变量边界、初始化、可行性和目标运算。51 个代表场景覆盖 32 个单目标原生 ID、NSGA-II/III、island/memetic、同步/异步执行，以及显式标量化、order/cycle 与 swap/inversion 排列组合、有状态参数源和多种参数调度/种子。除双向策略加载与完整检查点恢复，还在串行片段边界对比原生状态、随机流、档案、预算和学习事件；对比排除有效运行时间及序列化默认值省略的差异。

标量化会话的原生状态保持标量，而受控局部档案、奖励和算子反馈保留原始目标向量。受控和组合会话的局部多目标档案按到达顺序保留非支配前缀（容量 100），只有全局多目标档案按拥挤距离裁剪；检查点恢复保持这一区分。有状态参数源可通过双方约定的 JSON schema 恢复，不限于无状态内置组件。用户自定义 codec、组件实现和其他编码仍需匹配格式及业务语义；上述代表场景不是对任意闭包或全部参数组合的认证。

双端内层奖励协议在每个受控会话中使用 `rewardStateVersion: 1` 和 `rewardState`；全局奖励状态仍位于 `components.reward`，每个受控 worker／组合叶子持有独立奖励实例。只有真实调用的槽位推进局部奖励状态，顺序为 update、mutation、neighborhood、按名称字典序排列的参数槽位。`RewardContext.action` 在内层为 `槽位/动作`，在高层为 worker ID。缺少版本时兼容旧的无版本 `rewardState` 和 Rust `reward`；未知版本、版本化状态缺失或别名冲突均拒绝。组件 ID、版本及配置指纹仍需一致，自定义状态载荷必须约定共同 schema。读取转移时保留耗时元数据，但耗时不属于确定性状态。

原始 Kotlin `2eb7943` 不认识奖励状态字段。`RunCheckpoint::validate_kotlin_v1()` 与 `FileSearchStore::save_kotlin_checkpoint()` 继续作为该基线的严格导出保护：包括嵌套子会话在内的不支持字段会在写入前拒绝，已有文件不被替换。与改造后的 Kotlin 互通应使用普通 `save_checkpoint`，不能为了通过旧版保护而删除奖励状态。无状态快照省略新增字段，继续兼容原始基线。

**不承诺跨运行环境逐位轨迹一致。** JVM 与 Rust 的超越函数可能有 ULP 级差异，并可能影响后续分支；严格一致需双方采用同一数值实现，而不能删除误差或只比较最终 best。异步完成顺序本身非确定，完整状态恢复不保证重现另一进程的线程时序；需要稳定归并顺序时使用串行/同步模式，或显式录制并重放调度日志。跨语言的数值容差验收必须与结构/随机流/预算的精确验收分开报告。

## CMA 数值契约

CMA-ES 支持 `"numerics": "cma-polar-fdlibm-v1"`。直接使用 runner 时，Rust 通过
`with_numerics` 选择 `CmaNumerics::PolarFdlibmV1`，Kotlin 选择 `CmaNumerics.PolarFdlibmV1`。
采用 Marsaglia 极坐标高斯采样：拒绝零半径和单位圆外的抽样对，每次尝试消耗两次均匀抽样，
只使用第一分量、不缓存第二分量；保留高斯分布，不依赖平台余弦。默认 native Box-Muller 不变。

共同模式固定原始 fdlibm 对数的运算分组（不是分组不同的 musl log），Rust `libm=0.2.16`
exp/pow/sqrt 对应 Kotlin `StrictMath`，以正零起始顺序累加，Jacobi 分解固定 60 轮、
停止及逐对阈值 `1e-12`，不重排特征值、按列输出特征向量。模式进入配置及策略指纹。
restartOnImport 使用全部导入候选的均值及样本协方差；单候选保留协方差，更优历史 best 不丢失。
多维、标量化及 SHADE 转 CMA 的 memetic 轨迹已进行完整状态及 binary64 位模式测试；
这不是任意回调算术、舍入环境或所有机器的通用认证。

## 异步调度重放

仅支持 `Asynchronous`、parallelism >= 2 且**无时间预算**的执行模式。首次派发前调用
`record_schedule()`（Rust）或 `recordSchedule()`（Kotlin）；终止后通过 `export_schedule()` /
`exportSchedule()` 导出版本 1 的 `ScheduleLog`。`FileSearchStore` 提供校验并原子写入的
`save_schedule` / `load_schedule`，Kotlin 对应 `saveSchedule` / `loadSchedule`。
新运行绑定 `replay_schedule(&log)` / `replaySchedule(log)` 后真实执行算法，等待日志指定的
worker，即使自然完成先后反转也不改变归并顺序；不是重放目标值替身或只重放学习更新。

`begin/dispatch/complete/drain/yield/end` 记录 advance 边界、按序可用动作、随机位模式、
预留及实际预算、停止/失败标志和静止排空屏障。校验规范 JSON SHA-256、配置/问题身份、
事件配对及总量。偏离立即报错，并对已派发任务恰好一次排空；偏离后禁止生成检查点。
损坏、截断和不兼容身份不会静默回退到自然完成顺序。

静止检查点的可选 `schedule` 保存已消费前缀及游标。常规恢复后，在推进前必须显式重新绑定
**完整源日志**继续重放，或显式继续录制；源日志须匹配该前缀，检查点身份及完成计数须与运行一致。
已覆盖停止及 worker 失败排空。原始 Kotlin 导出保护拒绝该新状态。自定义回调和组件仍须有
相同的确定性语义，依赖时钟的奖励及外部副作用不在契约内；普通未录制的异步模式仍然非确定。

## 业务坐标与搜索域

`SearchProblem` 分开定义编码有效性（`valid_solution`）、可搜索域（`admissible`）和严格可行性
（`feasible`）。`admissible` 默认调用 `feasible`，既有问题无需修改。显式覆盖后可允许不可行候选
参与原生搜索，但全局及受控记忆的有效档案、工作集、奖励坐标和恢复校验仍同时要求严格可行且不在
业务禁区。`evaluate` 先消费一次请求令牌；只有编码有效且可搜索时才增加 objective 调用计数。

信用坐标必须由问题显式提供：同时实现 `has_objective_coordinates() -> true` 和
`objective_coordinates(&O) -> Result<Vec<f64>>`。坐标应按固定业务尺度归一化、越小越优；最大化需
显式取反，不能默认对向量求和或把业务目标覆盖为惩罚值。`checked_objective_coordinates` 拒绝空、
非有限及单目标多维坐标，奖励输入进一步校验参考档案与候选的维度一致。标量化原生状态保持标量，
两层奖励仍使用规范问题的原始目标向量投影，不使用标量化结果冒充原目标坐标。

`has_constraint_violation` 和 `constraint_violation` 提供独立的业务违约度回调；
`checked_constraint_violation` 拒绝缺失、错误、负值或非有限值。违约度不能绕过预算调用完整目标，
也不能由 `violation == 0` 推断严格可行。叶 `AlgorithmConfig.constraints` 现支持
`FeasibilityFirst`、`Epsilon`、`StochasticRanking` 及独立可恢复状态；组合根拒绝此字段，
应分别配置各叶。自定义工厂通过 `register_algorithm_with_constraints(id, true, factory)`
显式声明支持，且必须遵守约束事务协议。

Rust 无法判断 trait 方法是否被覆盖，因此装配约束策略时默认保留 `admissible`，不会自动设为
`true`。若要放宽默认的严格可行过滤，显式覆盖 `constraint_admissible` 并保留业务禁区，或直接
定义明确的 `admissible` 搜索域。原搜索域内仍执行自定义 `evaluate`，且只计费一次；显式扩展域
若还需要专用评估/缓存语义，应覆盖 `evaluate_constraint_admissible`。该方法默认在原可搜索域
调用原 `evaluate`，仅在明确新增域中调用计费后的 `objective`。

原生比较保留完整个体和原始 fitness；随机排序使用有界批量扫描，绝不作为随机比较器，成对历史
最优和接受判断仍确定。严格可行优先于零违约。epsilon 初值为空时只在首个完整提交步校准，随后
按 `horizon` 和 `exponent` 收紧。部分步/失败步恢复原生状态、局部档案、校准和约束随机流，
不退还已消耗请求，也不删除报告中已完成候选。导入不推进 epsilon；失败回滚约束状态，成功的
部分导入保留已获额度前缀。原生及受控检查点保留 `constraints`；旧 Kotlin 2eb7943 导出保护
明确拒绝该新状态，不通过删除状态伪装兼容。

`operator::fitness` 另提供 `ConstraintAssessment<T>`、`EpsilonConstraintEvaluator`、
`StochasticRanking`，只排序已计费的业务评估，不消费目标令牌，也不证明可行或放宽全局严格档案。

`RewardPolicy::requires_coordinates` 在打开运行时检查能力，不隐式推导标量坐标。`RewardContext`
包含实际动作 ID、评估请求数、worker 内执行耗时，以及派发前冻结的 `reference` 和完成后过滤的
`candidates` 坐标；投影不调用 objective，也不消费目标评估令牌。每个组合叶子仍持有独立奖励实例。
坐标回调失败会产生失败反馈，但不删除已经评估的有效候选。

依赖真实耗时的自定义奖励必须声明 `requires_elapsed_time() -> true`，当前调度录制/精确重放会在
绑定前拒绝该组合；普通执行仍支持。只有这种显式耗时依赖才写入转移的耗时元数据，其他策略保留
零值，避免破坏既有确定性轨迹。`RewardContext.elapsed_nanos` 始终是实际测量，不能在未声明依赖时
暗中使用它更新策略。新增内置策略如下。

## 自适应信用与接受

`learning.reward` 支持 `time-normalized`（epsilonSeconds=1e-9）、`extreme-value`
（window=50）、`dominance` 和 `hypervolume-delta`。前两者要求显式投影的单目标标量坐标；
dominance 使用有效新贡献。Extreme 通过全局动作窗口使旧信用过期，不保存无限期的动作极值。
根及每个叶拥有独立奖励状态。真实耗时奖励可用于普通运行，但明确拒绝 E4 精确录制/重放。

超体积使用任意受支持维数的越小越优坐标。`reference` 为有限数值数组，或成对配置
`referenceX`/`referenceY`，不能混用。未指定参考点时，默认收集32个有效候选后加余量冻结；
校准阶段及冻结动作信用均为0。精确递归切片默认 `maxWork=1000000`，深度上限256；
额度耗尽明确失败，绝不静默转采样。显式 `mode="sampled"` 默认 samples=16384、seed=1，
使用共同随机数并报告标准误差和99% Hoeffding绝对误差界。校准状态与 `last_measurement()`
随检查点恢复，几何失败不部分提交校准或覆盖最近成功诊断。

`memory.acceptance` 新增 `late`（length=50）、`record-to-record`（initialRatio=0.05、
horizon=1000）和 `auto`（无组件参数）。Late拒绝也推进原目标历史；Record要求标量最小化
坐标并将容差收紧到0。Auto按首个可用的decision/step/evaluation预算推导容量，只用严格改善
调整跨度，正比例缩放和平移目标不改变决策；预算提示不是停止条件。Late和Record仍有显式参数，
不能宣称无参数策略。

自定义业务目标使用Late或Auto前须注册 `SearchRegistry::set_objective_codec`；历史保存原始
业务O而非投影坐标，不要求 `O: Serialize`。标量与向量内置注册表已提供原生codec。
接受策略只改变工作集：拒绝的有效候选仍可进入档案，不回滚算法私有状态。
旧Kotlin 2eb7943导出保护明确拒绝这些新组件，不删除状态伪装兼容。

## 非平稳选择

高层 `selection` 和算法槽位 `selection` 均支持 `frr-mab`（window=50、exploration=0.5、
decay=0.8）、`choice-function`（rate=0.2、recencyWeight=0.1）和 `adaptive-pursuit`
（rate=0.2、minimumProbability=0.01）。可用动作掩码保留规范动作顺序，概率追逐在掩码内重新归一。

快照保留完整窗口、质量、转移对、概率、lastUsed、前驱、pending 决策及 clock；恢复按动作 ID
匹配，并验证定义与概率下界。`LearningMode::Frozen` 不更新信用。Choice function 在派发时冻结
前驱，包括“无前驱”，异步乱序完成不能偷偷改变归因。旧 Kotlin 2eb7943 导出保护明确拒绝这些
新状态，不删除学习状态伪装兼容。

`SelectionPolicy::predecessor` 和 `discard` 提供兼容默认实现。准备失败及未执行槽位清理
pending，不伪造反馈；每个真实执行的槽位只观察一次。本地 terminal 仅依据自身执行结果，
不读取有竞态的共享全局计数器。停止/失败排空、E4 调度重放及静止检查点保留完整选择器状态。
共同固定数值向量与跨端完整轨迹覆盖代表性binary64互通，不仅依靠Rust自身续跑；
不据此认证全部平台、回调或参数组合。

## 原生导入同步

PSO/APSO/CLPSO通过 `runner::ContextStateSynchronizer` 导入：每个已计费候选匹配原目的状态中
最近的粒子，按规范变量顺序计算并夹紧位移速度；来源粒子的真实历史best更优时保留。按个人best
排序后从最差槽位倒序替换，不再一律将速度清零。CLPSO同步后重建合法的自示范索引。

SHADE/L-SHADE合并种群去重排序，将被淘汰的目的成员放入有界external archive，并保留真实
`M_F`、`M_CR`和记忆游标，绝不为导入解伪造成功参数样本。CMA继续沿用完整协方差重启协议，
单解导入保留原协方差。导入仍走普通评估预算，算法切换不清空暂停状态。

## 局部搜索与正交动作集

更新槽位支持 `pattern-search`（`initialStep=0.1`、`contraction=0.5`、`sweeps=32`）、`opposition`、
`mutation-stage`（`operator`、`rate=1`）、`neighborhood-stage`（`operator`），以及
`pipeline`、`vnd`（`maxTrials=50`）、`elite-local`（`amount=1`）。三个组合使用非空 `stages` 数组。
VND严格改善后回到首邻域；elite-local仅处理调用方已排序的Top-K并保留其余父代。
opposition仅反射具有有限上下界的维度，未界定维度不变。

`VariableModel::local_evaluation`默认不提供能力；原生单目标及NSGA会话显式提供带生命周期的
`LocalEvaluation`/`EvaluatedSolution`账本，目标类型保留在模型内。局部seed、坐标/模式试探及
流水线每阶段产物都经同一完整评估入口计费；排序不调用目标，句柄不能跨账本比较或保存为跨步缓存。
最终输出仍由外层重新评估，不贴旧fitness。pattern剩余额度不超过2时停止试探，给外层保留空间。
没有显式账本的普通变量模型不能执行需要目标判断的局部动作，不隐式绕过预算调用目标。

嵌套深度最多16、每个组合1..64阶段、每棵动作树最多1024节点。自定义子算子须通过
`register_*_with_capabilities`声明无状态和兼容编码，返回实例也须声明`stateless`；旧注册方式保留
连续顶层使用，但不能被默认为可嵌套或兼容排列。有状态组合应独立实现可恢复组件，不能套无状态包装丢状态。

`OrthogonalOperatorPortfolio::continuous(size)`要求至少4个体，构建分布、自适应差分、围捕、逃逸、
局部精调五生态位；`permutation(size)`要求正规模，仅用order及inversion/swap。两者都返回普通
`AlgorithmConfig`列表，业务模型仍负责合法初始解。每个槽位不超过两个动作，不混用连续与排列编码。

## 业务破坏修复扩展

`heuristic::operator::neighborhood`提供泛型`RuinResult<T>`、`Ruin<T>`、`RandomRuin`、
`WorstCostRuin<T>`、`RelatednessRuin<T>`、`InsertionCost<T>`、`Repair<T>`、
`RegretKInsertRepair<T>`、`GreedyInsertRepair<T>`和`RuinRecreateOperator<T>`。
`T: Clone + PartialEq`的等价关系须代表稳定唯一身份；重复或非自反身份（例如NaN）被拒绝。
移除无放回，保留/移除结果保持原顺序；动作还检查自定义破坏和修复没有复制、丢失或替换元素。

移除收益、相关距离和插入增量代价均由类型化回调注入，不序列化闭包，也不隐式调用完整目标。
收益/代价必须有限，相关距离还须非负；插入代价返回`None`表示不可插入。Regret-K先处理可选位置
不足K的元素，再按遗憾值及最低代价选择，K=1为全局贪心。不可修复、遗憾值溢出或回调失败显式报错，
不会静默丢弃元素。有限代价排序保留JVM的正负零顺序与稳定同值位置。

`runner::RuinRecreateSearchSession`接收独占`SearchProblem`、破坏修复动作、原O个体codec和
`RuinRecreateSessionConfig { amount, random, stateless_estimates }`。仅接受合法移除数的排列问题；
通过普通`SearchRegistry::register_algorithm`注册，无需修改协调器或硬编码业务算法。
该直接会话不声明原生约束事务能力，不能通过自定义注册的能力标志绕过事务预检。
完整邻居和导入均调用`SearchProblem::evaluate`计费，部分估计不能用作未计费的完整业务目标。
失败报告保留已完成前缀、已评估候选和真实请求数，不退款、不重复反馈。

会话保存current、迭代和独立随机流；估计须显式无状态，或用`with_estimate_state`提供`EstimateState`。
估计恢复失败会尝试回滚旧估计状态，拒绝意外状态；恢复不调用目标。问题回调状态沿现有外层
problem-state协议独立保存，直接使用会话时通过`problem()`/`problem_mut()`保存/恢复；amount等配置
由外层配置指纹保护。原O不要求Serde，codec不能用标量化替换业务目标。

## 配置与自适应数值

`ConfigurationFormats::encode_format` / `decode_format`支持`Json`、`Yaml`、`Toml`，
共用结构校验和显式版本迁移；组件参数、版本、能力、编码及嵌套stages在首次评估前由注册表校验。
输入上限为4MiB UTF-8，拒绝重复键及非有限数。TOML省略模型声明的缺省Option，
业务参数中的null则报错，不静默丢失；日期时间值转ISO字符串，普通字符串不变。
约束配置/快照压缩声明的默认值，但必须保留steps、可空scale和独立约束random。

新增FRR-MAB的log/pow、epsilon的pow、采样HV误差及Auto的sqrt固定为fdlibm运算，
对应Kotlin的StrictMath，而非平台原生近似。共同探针实测发现过1ULP差异，固定实现后
50000个原语/公式结果及容量边界具有共同黄金向量；不据此认证任意回调算术或全部机器/工具链。
原有显式CMA/SHADE数值模式不变。历史档案使用稳定的完整个体比较，
不消费仅用于种群随机排序的独立约束流。

注册了无损archive codec的自定义算法与Kotlin共用空槽位控制层，保留局部档案和决策历史。
当前Rust自定义工厂API不暴露operator bindings，因此显式拒绝配置但不会消费的slots。
旧Rust裸自定义检查点仍按原直通模式恢复，不伪造未保存的控制层历史；
未提供archive codec的会话保留非恢复直通路径。
