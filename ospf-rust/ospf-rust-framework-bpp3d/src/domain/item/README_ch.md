# BPP3D 货物 Context

:us: [English](README.md) | :cn: 简体中文

该 context 对应 Kotlin `bpp3d-domain-item-context`。

## 职责

item 定义包件、物料、实际货物、pattern、箱、层、需求键、圆柱契约和连续半径圆柱建模等稳定领域词汇。其他 context 应依赖这些模型，不要重复定义货物语义。

## 文件结构

- `model.rs` 是公开 model shim。按 Kotlin 模型粒度拆分的文件位于 `model/`，包括 `package.rs`、`material.rs`、`item.rs`、`pattern.rs`、`bin.rs`、`layer.rs`、`schema.rs`。
- `model/package_attribute/` 包含包装类型、变形、悬空、朝向、两两堆叠、放置级堆叠和主 `PackageAttribute` 逻辑。
- `model/continuous_radius/` 包含连续圆柱半径 prototype、PWL 注册配置、solver 结果提取和 item shape 回写。
- `service.rs` 包含货物领域服务，例如合并和排序辅助能力。

## Public API

- `Package`
- `PackageAttribute`
- `PackageShape`
- `PackageOrientationRule`
- `PackagePairStackingRule`
- `PackagePlacementStackingRule`
- `Material`
- `MaterialKey`
- `ActualItem`
- `Bin`
- `BinLayer`
- `Bpp3dDemandKey`
- `PatternConfig`
- `ContinuousCylinderRadiusSolverPrototype`
- `ContinuousRadiusModelRegistration`

## 扩展点

货物规则通过 `PackageAttribute`、`PackageOrientationRule`、pair/placement stacking rules、`PatternConfig` 和 continuous-radius weight functions 扩展。物料厂商/供应商、cargo attribute key 等业务身份字段保留在本 context。

## 生命周期与数据流

application 输入会被标准化为 package、material、bin、layer、demand key 和可选 continuous-radius registration plan。下游 generation、assignment 与 packing context 将这些模型作为不可重复定义的领域词汇读取。

## 验证

修改 item 领域语义时运行 `cargo test -p ospf-rust-framework-bpp3d --lib`；涉及 CSV 字段时同步运行 serde 测试。

## 相关目录

- [`../layer_generation`](../layer_generation/README_ch.md)
- [`../layer_assignment`](../layer_assignment/README_ch.md)
- [`../packing`](../packing/README_ch.md)
