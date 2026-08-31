# BPP3D 基础设施层

[English](README.md)

该 context 对应 Kotlin `bpp3d-infrastructure`。

## 职责

infrastructure 包含 domain 与 application 共享的 typed geometry、orientation、packing shape、PWL approximation 和 render DTO 适配。这里是 Rust 保持物理量和类型化几何，同时在 solver/render 边界提供标量适配的层。

## 文件结构

- `geometry.rs` 暴露类型化点、向量、尺寸、AABB、放置和标量转换 helper，具体片段在 `geometry/` 下。
- `orientation.rs` 定义朝向类别和旋转语义。
- `packing_shape.rs` 定义长方体/圆柱装箱形状和轴感知外接尺寸。
- `pwl_approximation.rs` 提供连续圆柱半径建模使用的半径与半径平方 PWL 支持。
- `renderer.rs` 包含 application 输出使用的 render DTO、shape 和 axis enum。

## 扩展点

跨 context 共享的新几何适配放在这里。裸 `f64` 转换应限制在 solver/render 边界，面向 domain 的 API 继续使用 `Quantity<V, U>` 和 unit trait。

## 验证

使用常规 crate 检查，并通过 `cargo test -p ospf-rust-framework-bpp3d --lib` 覆盖基础设施测试。
