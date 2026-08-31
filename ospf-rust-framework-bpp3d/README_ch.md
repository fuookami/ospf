# OSPF Rust Framework BPP3D

🇺🇸 [English](README.md) | 🇨🇳 简体中文

本 crate 是 `ospf-kotlin-framework-bpp3d` 的 Rust 迁移目标。
当前状态已经提供 BPP3D infrastructure、item domain、layer assignment、
layer generation、packing context，以及第一版 application 层编排 API。

application API 当前覆盖列生成配置/状态/结果、层生成编排、深度边界朝向校验、
已知坐标放置适配、装箱分析和 renderer DTO 输出。启用 `serde` feature 后，
还提供 CSV schema guard、多表 dataset loader、typed CSV record 和 request draft 构造。
application service 也提供 mock RMP/final executor 接缝，使 materialized CSV request
可以先跑通编排流程，再接入真实 solver adapter。
当前还提供 solver-agnostic 的 `MetaModel` RMP/final executor 骨架，覆盖注册诊断、
no-op shadow price 和 final solution extraction 边界。
solver-backed executor wrapper 已能消费可插拔 backend result，后续 Gurobi/SCIP adapter
可以接在 application service 之外。
非 `async` 构建下，`ColumnGenerationSolverMetaModelBackend` 已可把 framework
`ColumnGenerationSolver` 的 LP/MILP 输出适配到同一 executor backend 契约。
application service 也提供一轮 RMP -> shadow-price generation -> column refresh ->
final 编排入口，用于验证列更新链路。
当前 MetaModel 层分配 wiring 已注册最小可求解的需求覆盖、final 赋值启用、
箱深度和箱数目标 pipeline；`serde` CSV materialized request 也可以通过
同一套 solver-backed one-round flow 和可插拔 backend 验证。
层候选当前携带显式 demand coverage metadata，application 会在 RMP/final 前
为 materialized/CSV flow 补默认 item coverage。
layer generation result 也已暴露 block 和 placement trace，`BlockLayerGenerator`
能产出 simple-block trace 候选供 coverage-aware flow 使用。
selected coverage layers 现在还能沿 final execution analysis path 转成最小 packed bins
和 renderer DTO，便于把最终选层直接接到渲染输出。
生成层的 placement trace 也会穿过一轮 application state，selected generated layers
可以回放成 packed bins，并由 CSV render fixture 验证。
simple-block generated layers 的 block trace 现在也走同一回放路径；空数据、
trace mismatch 和无可用箱型等 final flow 会输出结构化 diagnostics。
高级 generator skeleton、solver dataset no-run diagnostics，以及连续半径 CSV
保守渲染 fixture 也已作为迁移护栏接入。
trace replay 已下沉到 packing domain adapter；serde smoke fixture、
PatternedItem/PackageAttribute skeleton，以及延后的 objective/limit diagnostics
也覆盖了下一批 Kotlin 迁移护栏。
serde dataset suite 当前已包含多 smoke fixture、fake one-round 执行、
feature-matrix no-run diagnostics、PatternedItem demand coverage、
PackageAttribute packing diagnostics、延后注册计划和带请求上下文的 generator diagnostics。
它还支持 JSON manifest/目录驱动的 fixture 加载、CSV 的 PatternedItem /
PackageAttribute 物化，以及保守的非空 Pattern/Pile/Historical 层候选。
crate 内回归 fixture 当前还覆盖 manifest 加载、统一 batch report、业务规则 diagnostics、
语义化 deferred objective/constraint 注册，以及可配置的 Pattern/Pile/Historical 候选生成。
Kotlin Gurobi 的 grouped-layer 和 material-width-amount CSV 样例现在也可通过
同一套 serde fixture suite 适配，覆盖 PWL 半径区间、cuboid/cylinder 混合货物
和 Kotlin 风格 axis 枚举 token。
fixture manifest 现在携带 group/tag/backend-smoke 元数据，batch report 会汇总
load、materialize、no-run、fake execution、backend-smoke、render plan 和 diagnostics 数量。
CSV 业务规则校验也会聚合 PatternedItem 分组、PackageAttribute、标签和默认 coverage
诊断。延后 Pattern/Pile/Historical generator 已可在多个物品间轮换，并保留
shadow-price-aware 评分诊断；RestAmount/TailBinLoadingRate/BinLoadingOrder 的延后入口
也已有 MetaModel 注册语义测试。

详细迁移目标、清单和验收标准见 [bpp3d.md](bpp3d.md)。
