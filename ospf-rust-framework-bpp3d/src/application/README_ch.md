# BPP3D 应用层

:us: [English](README.md) | :cn: 简体中文

该 context 对应 Kotlin `bpp3d-application`，同时承接 Kotlin `bpp3d-domain-layer-selection-context` 在 Rust 侧的列生成编排职责。

## 职责

application 层负责编排 CSV 加载、solver backend 选择、列生成流程、结构化报告、render DTO、fixture baseline 和 Kotlin 对比 artifact。它不应持有领域变量或约束族；这些能力保留在 `domain/*` context 中，并通过面向 `MetaModel` 的 component 和 pipeline 注册。

## 文件结构

- `service.rs` 是公开应用服务 shim，具体实现拆在 `service/` 下。
- `service/config.rs`、`state.rs`、`algorithm.rs`、`standard_executors.rs` 覆盖列生成配置、生命周期状态和层选择编排。
- `service/executor.rs` 包含 RMP/final executor 契约以及 MetaModel 执行器适配。
- `service/application_service.rs` 编排 one-round 与 CSV materialized flow。
- `service/fixture_suite.rs`、`dataset_suite.rs`、`layer_quality.rs`、`reporting_helpers.rs` 支撑大规模 fixture、Kotlin baseline 和质量报告。
- `csv.rs` 是公开 CSV 边界，解析器、物化器、Kotlin adapter、schema guard 和测试拆在 `csv/` 下。
- `report.rs` 是公开结构化报告边界，报告 DTO 拆在 `report/` 下。

## Public API

- `ColumnGenerationApplicationService`
- `ColumnGenerationAlgorithm`
- `ColumnGenerationConfig`
- `ColumnGenerationRmpExecutor`
- `ColumnGenerationFinalExecutor`
- `MetaModelSolverBackend`
- `MetaModelRmpExecutor`
- `MetaModelFinalExecutor`
- `ColumnGenerationResult`
- `Bpp3dRunReport`
- `Bpp3dFixtureReport`
- 启用 `serde` 时的 `CsvDatasetLoader`。

## 扩展点

新增 solver 行为应通过 `MetaModelSolverBackend` 或 RMP/final executor trait 接入。最终几何校验可通过 `ColumnGenerationApplicationService::with_geometry_guard` 注入 `PackingGeometryContract` 实现，无需修改 application flow。新增数据协议应放在 CSV materializer 边界。新增对比输出应通过 fixture suite 和 run-report DTO 实现，不要把领域建模逻辑塞回 application flow。

## 生命周期与数据流

CSV 或内存输入先物化为 application request，layer generation 提供候选列，RMP executor 求解迭代 assignment 模型，final executor 求解已选列 MILP，最后由 packing/report adapter 生成结构化输出。layer selection 在 Rust 中保留为围绕 domain context 的 application 编排职责。

## 验证

完整应用协议面使用 `cargo check -p ospf-rust-framework-bpp3d --features serde`，fixture/report/CSV 覆盖使用 `cargo test -p ospf-rust-framework-bpp3d --features serde --lib`。

## 相关目录

- [`../domain`](../domain/README_ch.md)
- [`../infrastructure`](../infrastructure/README_ch.md)
