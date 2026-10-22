# 选择型超启发式搜索

[English](README.md) | 中文

迁移基线为 `ospf-kotlin` 的 `2eb7943`。本模块位于 `ospf-rust-core::solver::hyper_heuristic`，依赖已有 `heuristic` 原生骨架，不新增 crate，也不通过反复执行完整 solver 模拟会话。

## 使用入口

1. 使用 `SearchRegistry::with_builtins()` 注册单目标原生算法，使用 `register_problem` 提供独立问题工厂。
2. 使用 `HyperHeuristicConfig::new(...).algorithm(...)` 创建配置，或通过 `ConfigurationFormats` 读取 JSON/YAML。
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
