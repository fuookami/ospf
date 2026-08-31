# OSPF Rust Framework BPP3D

:us: [English](README.md) | :cn: 简体中文

本 crate 是 `ospf-kotlin-framework-bpp3d` 的 Rust 迁移目标。
当前已提供 BPP3D infrastructure、item domain、BLA/block-loading/layer-generation context、layer-assignment pipeline、packing context、CSV 协议适配、renderer DTO，以及 application 层 column-generation 编排。

建模路径以 `MetaModel` 为轴心：RMP 和 final MILP 共享 `LayerAssignmentContext`、`LayerAssignmentAggregation`、变量组件、需求/深度/启用约束和目标 pipeline。application service 负责 solver backend、CSV 物化、trace/KPI 诊断和 renderer 适配；placement/block trace replay 继续留在 packing domain。

当前迁移覆盖：

1. solver-agnostic `MetaModel` RMP/final executor、可插拔 backend wrapper，以及一轮 RMP -> shadow-price generation -> column refresh -> final flow。
2. 层候选 demand coverage metadata、稳定 demand shadow-price key、selected-layer 提取，以及基于 placement/block trace 的 packed-bin/render replay。
3. BLA local/global 层候选、simple-block 层候选、多轮 axis `ComplexBlockGenerator`、有界 DFS 空间分裂、MLHS branch/depth 候选排序、固定/离散半径圆柱 circle-packing 网格候选，以及 request-aware Pattern/Pile/Historical 生成器。
4. 轴感知长方体和圆柱几何、带 guard 的横向圆柱 final validation、PWL continuous-radius metadata/fixture，以及 renderer `actualVolume` 输出。
5. CSV schema guard、Kotlin Gurobi grouped-layer/material-width-amount 适配、manifest 和递归目录 fixture 加载、backend survey/no-run/fake fallback 报告，以及 feature-matrix diagnostics。
6. PatternedItem 保守 demand coverage、PackageAttribute 校验/装箱 diagnostics，以及 RestAmount/TailBinLoadingRate/BinLoadingOrder 的 MetaModel 注册入口。

manifest 现在已镜像 Kotlin Gurobi CSV 样例全集，共 22 个 fixture，包括 19 个 Kotlin 派生 dataset 样例和 3 个本地回归 fixture。全量 manifest suite 在本地具备 solver 时已由真实 Gurobi 10 和 SCIP feature-gated 测试覆盖。

剩余差距见 [bpp3d.md](bpp3d.md)。简而言之，当前 crate 已形成可运行的 Rust 风格框架基线并具备全量 manifest 真实 solver 覆盖；剩余边界是已记录的长期非目标。
