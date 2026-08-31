# BPP3D 层生成 Context

[English](README.md)

该 context 对应 Kotlin `bpp3d-domain-layer-generation-context`。

## 职责

layer_generation 为列生成创建候选层。它组合 block、BLA、circle packing、Pattern、Pile、Historical 策略，同时保留 request-scoped 包装规则策略和用于 Kotlin 对比的 diagnostics。

## 文件结构

- `mod.rs` 是公开 context shim。
- `model.rs` 定义 request、demand entry、trace、generator trait 和 result。
- `context.rs` 组合 generator 并进行候选去重。
- `block_layer_generator.rs`、`bl_layer_generator.rs`、`circle_packing_layer_generator.rs` 包含直接生成器。
- `deferred_generators.rs` 定义 Pattern/Pile/Historical 配置和 block-loading fallback。
- `pattern_generator.rs` 通过 `pattern/` 下的片段实现 Pattern 策略。
- `pile_generator.rs`、`historical_generator.rs`、`scoring.rs` 包含其余延后策略和排序 helper。
- `tests/` 按策略、包装规则、圆柱和质量覆盖分组。

## 扩展点

新增候选来源应实现 `LayerGenerator`。业务规则通过 `LayerGenerationPackageRulePolicy` 注入；不要把任意闭包存进可序列化 item model。source 级 diagnostics 和 coverage metric 应保持可用于 Kotlin baseline 对比。

## 验证

运行 `cargo test -p ospf-rust-framework-bpp3d --lib`；大规模 Pattern/Pile/Historical 对比使用 serde fixture quality report。
