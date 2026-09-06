# BPP3D 领域层

:us: [English](README.md) | :cn: 简体中文

BPP3D domain 层将 Kotlin domain 子模块映射为 Rust context，同时保持 Rust 原生的 `MetaModel` 注册边界。

## 职责

- 将 BPP3D 业务词汇、几何规则、assignment 组件和最终 packing 语义保留在 domain context 中。
- 通过面向 `MetaModel` 的 component 注册变量、表达式、约束、目标和 shadow price 提取逻辑。
- 为 layer generation、layer assignment 和最终 packing 提供可复用 context，避免 application service 重复编排领域细节。

## 模块

- `item`：货物、包装、物料、pattern、箱、需求、圆柱和连续半径模型。
- `bla`：bottom-up-left-justified 投影放置。
- `block_loading`：simple/complex block 生成，以及 DFS/MLHS 放置搜索。
- `layer_generation`：Block、BL、Circle、Pattern、Pile、Historical 候选生成。
- `layer_assignment`：RMP/final MILP 赋值、动态列、约束、目标和 shadow price。
- `packing`：最终 packed-bin 转换、几何门禁、物料汇总和 render adapter。

Kotlin `bpp3d-domain-layer-selection-context` 已有意映射到 `application/service/algorithm.rs` 和 executor 编排中；Rust 将 layer selection 作为围绕 domain context 的 application flow，而不是单独 domain context。

## Public API

- `item`
- `bla`
- `block_loading`
- `layer_generation`
- `layer_assignment`
- `packing`

## 扩展点

domain context 持有业务模型、约束、目标、扩展策略和结果提取。application 可以编排这些 context，但不应重复堆叠领域建模细节。

## 生命周期与数据流

item 与 infrastructure 模型定义类型化输入，layer generation 创建候选列，layer assignment 注册 RMP/final MILP 组件并提取 shadow price，packing 将选中层转换为通过几何校验的 packed bin 和 render-ready 输出。

## 验证

领域覆盖使用 `cargo test -p ospf-rust-framework-bpp3d --lib`。如果修改的数据会通过 CSV fixture 加载或对比，同步追加 `--features serde`。

## 相关目录

- [`../application`](../application/README_ch.md)
- [`../infrastructure`](../infrastructure/README_ch.md)
