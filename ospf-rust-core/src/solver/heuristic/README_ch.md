# 启发式求解

[English](README.md) | 中文

`ospf-rust-core` 的元启发式层，重建在一个核心理念上：**每个元启发式都是「一个生命周期骨架 + 一组算子」**。

## 分层

| 层 | 模块 | 职责 |
| --- | --- | --- |
| 契约层 | `contract.rs` | `HeuristicSolver`、`Runner`、`RunnerSolver`、`MultiObjectiveSolver` |
| 模型适配 | `model.rs` | `VariableModel`、`ObjectiveModel`、`ObjectiveView`、`HeuristicValue` |
| 算子层 | `operator/` | 算子族（初始化、更新、变异、环境选择、多目标、强化学习等） |
| 骨架层 | `runner/` | 统一 `drive` 循环；种群、轨迹、粒子群、岛屿、SHADE、CLPSO、CMA-ES、NSGA-II/III |
| 组合层 | `composition/` | 把骨架与算子装配成算法，含 NSGA 与 memetic 调度 |
| 入口层 | `facade/` | 每个算法一个便捷函数 |
| 会话层 | `session/` | `SearchBudget`/`SearchSlice`/`SearchRandom`、问题协议与可恢复的 `RunnerSession` |

## 快速开始

```rust
use ospf_rust_core::model::callback::AbstractCallBackModel;
use ospf_rust_core::solver::heuristic::{
    SeededEntropy, Iteration, HeuristicSolver,
    composition::SearchConfig,
    facade::hill_climbing,
};

let mut model = AbstractCallBackModel::<f64>::new("demo");
// 注册变量、上下界、目标与初始解
let config = SearchConfig::new(500);
let mut solver = hill_climbing(config.stopping(), Box::new(SeededEntropy::new(42)));

let result = solver.solve(&mut model, &mut Iteration::new())?;
println!("best = {:?}", result.best_objective);
```

终止条件与档案容量集中在一处配置：

```rust
let config = SearchConfig::new(1_000)
    .with_not_better_limit(100)
    .with_time_limit(std::time::Duration::from_secs(5))
    .with_archive_capacity(8);
```

## 算子族

`operator/` 下包含：`initializer`、`fitness`、`ranking`、`boundary`、`topology`、`diversity`、`acceptance`、
`stochastic`、`evolution`、`neighborhood`、`survivor`、`migration`、`linear`、`archive`、`stopping`、`update`、
`mutation`、`rl` 与 `multi_objective`。

随机性一律通过 `ospf_rust_math::distribution::Entropy` 注入；算子不持有全局状态，因此同一 seed 可复现，
且求解器可以安全复用。

## 算法入口

`facade/` 以默认算子暴露全部算法：爬山、邻域搜索、遗传、差分进化、粒子群、APSO、CLPSO、SHADE、L-SHADE、
CMA-ES、岛屿 GA、Q 学习，以及 20 个位置更新算法（GWO、SCA、MVO、WOA、HHO、AOA、HS、WCA、SOA、TLBO、
SSA、GSA、FA、CS、BA、MFO、GOA、WarSO、EVO、GCO）。NSGA-II/III 与 memetic 调度通过 `composition` 使用。

## 会话

`session/` 把一次搜索切成可暂停的预算片段，因此可以外部调度、检查点或分布式恢复。通用 `RunnerSession` 只知道四件事
（状态类型、初始化、推进一步、展平个体），因此一份实现即可承载所有骨架——种群、轨迹、粒子群、岛屿、CLPSO、SHADE
与 CMA-ES：

```rust
use std::sync::Arc;
use ospf_rust_core::solver::heuristic::{
    RunnerSession, SearchBudget, SearchSlice, SessionCodecs,
    session::{IndividualListCodec, PopulationHooks},
};

let hooks = PopulationHooks::new(runner, Box::new(SeededEntropy::new(7)));
let codecs = SessionCodecs::new(
    Some(Box::new(IndividualListCodec::new(order.clone()))),
    Some(Box::new(IndividualListCodec::new(order))),
);
let mut session = RunnerSession::new(Box::new(hooks), problem, codecs);

let budget = Arc::new(SearchBudget::new(10_000));
let mut slice = SearchSlice::new(budget, 500, 10).with_duration(Duration::from_millis(50));
let report = session.step(&mut slice)?;
let snapshot = session.snapshot()?;   // 在此持久化
session.restore(&snapshot)?;          // 在别处恢复
```

预算语义与 Kotlin 一致：一次领取依次记入片段、父片段（若有）与全局预算；任一层拒绝即回滚，因此只有成功才真正消费。

状态编解码与 Kotlin 的 `RunnerStateCodecs` 对应：`IndividualListCodec`（种群）、`IslandStateCodec`、
`TrajectoryStateCodec`、`ParticleStateCodec`、`ClpsoStateCodec`、`ShadeStateCodec`、`CmaEsStateCodec`。
提供 importer 时会话支持 `import_solutions`。

快照就是普通的 `serde_json::Value` 树，可用 `serde_json::to_string` 落盘，并由另一个进程恢复。`serde` 与
`serde_json` 已是本 crate 的常规依赖；`serde` 特性仅为向后兼容保留。

快照文本与 Kotlin 基线互通：

- 字段名与 Kotlin 逐字一致（`initialized`、`iteration`、`stagnation`、`state`、`archive`、`activeNanos`），
  `version` 与 `activeNanos` 取默认值时省略，对应 kotlinx 的 `encodeDefaults = false`；
- 骨架状态采用 Kotlin 的 `NativeState` 形状（`individuals` / `vectors` / `counters` / `solutions` / `children`），
  空字段省略；
- 所有数值以 `Double.toString()` 形式的字符串编码（`1.0`、`1.0E300`），与 Kotlin `SearchCodecs.flt64` 一致。

因此两侧任一方写出的快照，另一侧都能读取。

目标类型与会话泛型与 Kotlin 的 `SearchProblem<ObjValue, V>` 一致：单目标用 `SearchProblem<f64, V>`，多目标用
`SearchProblem<Vec<f64>, V>`；偏序一律由 `dominates` 表达，因此同一份会话实现两者通用。NSGA 风格搜索使用
`NsgaSessionHooks`：

```rust
let hooks = NsgaSessionHooks::new(size, dominance, update, mutation, entropy, divisions);
let codecs = SessionCodecs::new(
    Some(Box::new(MultiObjectiveArchiveCodec::new(order.clone()))),
    Some(Box::new(MultiObjectiveArchiveCodec::new(order))),
);
let mut session = RunnerSession::new(Box::new(hooks), problem, codecs);
```

`select_front` 是可复用的末层截断：`divisions` 为 `None` 时按拥挤距离，否则按 Das–Dennis 参考点。

会话档案与 Kotlin 的 `Archive` 语义一致：单目标保留最优 `capacity` 个，多目标保留非支配集并在超容量时按拥挤距离裁剪，
新解支配旧成员时淘汰旧成员。`RunnerSession::new` 的默认容量为单目标 1、多目标 100（对应 Kotlin 的 `archiveAmount`）；
需要自定义时用 `RunnerSession::with_archive_capacity`。

## 与 Kotlin 基线的差异

- **同步 API。** 不引入 `async`，目标评估在调用方线程内完成。
- **无值转换器。** `V: HeuristicValue` 直接提供 `f64` 双向转换。
- **规范顺序。** 解是 `HashMap<VariableId, V>`，位置语义一律经 `VariableModel::variable_order()`。
- **串行执行。** 代内个体串行推进，保证结果可复现。
- **不单独建 crate。** 工作区只有单层，全部代码位于 `ospf-rust-core/src/solver/heuristic`。

## 确定性

`SeededEntropy` 复用 Kotlin 测试中的 LCG（乘子 `6364136223846793005`、增量 `1442695040888963407`、
取高 53 位），因此固定 seed 在两侧产生同一序列。

## 相关文档

- [算子族说明](operator/README_ch.md)
- [骨架说明](runner/README_ch.md)
