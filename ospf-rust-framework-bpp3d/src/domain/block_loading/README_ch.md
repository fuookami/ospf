# BPP3D 块装载 Context

:us: [English](README.md) | :cn: 简体中文

该 context 对应 Kotlin `bpp3d-domain-block-loading-context`。

## 职责

block_loading 负责构建 simple/complex block，并通过 DFS 或多层启发式搜索生成可行块放置。layer_generation 的多种策略会把它作为保守 fallback 和候选引擎。

## 文件结构

- `model.rs` 暴露 item view、simple/complex block、block placement、space 和 block-loading context 数据。
- `service.rs` 是公开 service shim。
- `service/simple_block_generator.rs`、`complex_block_generator.rs`、`depth_first_search_algorithm.rs`、`multi_layer_heuristic_search_algorithm.rs` 对应 Kotlin service 文件。
- `service/tests/` 按 cuboid、cylinder、limit、DFS、MLHS 分组测试。

## Public API

- `ItemView`
- `Block`
- `SimpleBlock`
- `ComplexBlock`
- `Space`
- `BlockPlacement`
- `SimpleBlockGenerator`
- `ComplexBlockGenerator`
- `DepthFirstSearchAlgorithm`
- `MultiLayerHeuristicSearchAlgorithm`

## 扩展点

新增 block 组合器或搜索启发式应放在这里。包装规则过滤可由 `layer_generation` 注入；最终几何拒绝属于 `packing`。

## 生命周期与数据流

item view 先转换为 simple block，再按需要组合 complex block，随后由 DFS 或 MLHS 搜索生成 block placement。layer-generation 策略把这些 placement 作为候选层或 fallback 列使用。

## 验证

运行 `cargo test -p ospf-rust-framework-bpp3d --lib`，重点关注 block-loading 和 layer-generation fallback 测试。

## 相关目录

- [`../item`](../item/README_ch.md)
- [`../layer_generation`](../layer_generation/README_ch.md)
- [`../packing`](../packing/README_ch.md)
