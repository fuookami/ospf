# BPP3D 领域层

[English](README.md)

BPP3D domain 层将 Kotlin domain 子模块映射为 Rust context，同时保持 Rust 原生的 `MetaModel` 注册边界。

## Context

- `item`：货物、包装、物料、pattern、箱、需求、圆柱和连续半径模型。
- `bla`：bottom-up-left-justified 投影放置。
- `block_loading`：simple/complex block 生成，以及 DFS/MLHS 放置搜索。
- `layer_generation`：Block、BL、Circle、Pattern、Pile、Historical 候选生成。
- `layer_assignment`：RMP/final MILP 赋值、动态列、约束、目标和 shadow price。
- `packing`：最终 packed-bin 转换、几何门禁、物料汇总和 render adapter。

Kotlin `bpp3d-domain-layer-selection-context` 已有意映射到 `application/service/algorithm.rs` 和 executor 编排中；Rust 将 layer selection 作为围绕 domain context 的 application flow，而不是单独 domain context。

## 规则

domain context 持有业务模型、约束、目标、扩展策略和结果提取。application 可以编排这些 context，但不应重复堆叠领域建模细节。
