# BPP3D 装箱 Context

:us: [English](README.md) | :cn: 简体中文

该 context 对应 Kotlin `bpp3d-domain-packing-context`。

## 职责

packing 将选中层和已知坐标放置转换为最终 packed bins，验证几何，汇总物料使用，并适配 render DTO。它是 layer assignment 之后的最终几何门禁。

## 文件结构

- `model.rs` 定义 packed item、packed bin、material summary、material plan 和 package-solution adapter。
- `service.rs` 是公开 service shim。
- `service/geometry_guard.rs` 验证长方体/圆柱重叠、边界和横向圆柱支撑。
- `service/layer_placement_adapter.rs` 和 `layer_trace_replay_adapter.rs` 将生成 trace 转为已知坐标 packed bin。
- `service/packer.rs`、`material_packer.rs`、`renderer_adapter.rs` 生成最终汇总和 render 输出。

## Public API

- `PackedItem`
- `PackedBin`
- `MaterialSummary`
- `MaterialPackingPlan`
- `PackingGeometryGuard`
- `PackingGeometryContract`
- `Packer`
- `PackingResult`
- `PackingAggregation`
- `MaterialPacker`
- `PackingRendererAdapter`
- `KnownCoordinatePlacement`
- `LayerPlacementAdapter`
- `LayerTraceReplayAdapter`

## 扩展点

最终验证规则加入 geometry guard。新的 trace replay 路径加入 adapter。候选生成保留在 `layer_generation`，这里只负责拒绝或转换最终已知坐标放置。

## 生命周期与数据流

selected layer assignment 与 known-coordinate placement 会被 replay 为 packed item，geometry guard 拒绝不可行的重叠或支撑状态，material packer 汇总使用量，render adapter 输出 DTO-ready loading plan。

## 验证

运行 `cargo test -p ospf-rust-framework-bpp3d --lib`，重点关注 packing geometry 和 render adapter 测试。

## 相关目录

- [`../item`](../item/README_ch.md)
- [`../layer_assignment`](../layer_assignment/README_ch.md)
- [`../../infrastructure`](../../infrastructure/README_ch.md)
