# BPP3D 基础设施层

:us: [English](README.md) | :cn: 简体中文

该 context 对应 Kotlin `bpp3d-infrastructure`。

## 职责

infrastructure 包含 domain 与 application 共享的 typed geometry、orientation、packing shape、PWL approximation 和 render DTO 适配。这里是 Rust 保持物理量和类型化几何，同时在 solver/render 边界提供标量适配的层。

## 文件结构

- `geometry.rs` 暴露类型化点、向量、尺寸、AABB、放置和标量转换 helper，具体片段在 `geometry/` 下。
- `orientation.rs` 定义朝向类别和旋转语义。
- `packing_shape.rs` 定义长方体/圆柱装箱形状和轴感知外接尺寸。
- `pwl_approximation.rs` 提供连续圆柱半径建模使用的半径与半径平方 PWL 支持；`ErrorDriven` 停止条件和 `max_relative_error` 使用每段真实最大相对误差，而不是中点采样值。使用 `PwlRadiusApproximationConfig` 配置，并调用 `try_from_radius_interval` 获取参数错误；自定义断点必须有限、为正、严格递增、与区间端点一致（允许容差），且不超过 `max_segments`。已弃用的 `from_radius_interval` 兼容入口对非法输入仍可能 panic，不用于模型注册。
- `renderer.rs` 包含 application 输出使用的 render DTO、shape 和 axis enum。

## Public API

- `MetricPoint2`
- `MetricPoint3`
- `MetricSize2`
- `MetricSize3`
- `MetricAabb2`
- `MetricAabb3`
- `MetricPlacement2`
- `MetricPlacement3`
- `Orientation`
- `PackingShape3`
- `PwlBreakpointStrategy`
- `PwlRadiusApproximationConfig`
- `PwlApproximationError`
- `PwlRadiusSquaredApproximation`
- `ConservativeRadiusEnvelope`
- `RenderLoadingPlanDto`
- `SchemaDto`

## 扩展点

跨 context 共享的新几何适配放在这里。裸 `f64` 转换应限制在 solver/render 边界，面向 domain 的 API 继续使用 `Quantity<V, U>` 和 unit trait。

## 生命周期与数据流

domain context 使用类型化 metric geometry 和带单位 shape；solver 边界显式转换为标量坐标；continuous-radius 建模使用 PWL helper；application/reporting 在输出边界序列化 render DTO。

## 验证

使用常规 crate 检查，并通过 `cargo test -p ospf-rust-framework-bpp3d --lib` 覆盖基础设施测试。

## 相关目录

- [`../domain`](../domain/README_ch.md)
- [`../application`](../application/README_ch.md)
