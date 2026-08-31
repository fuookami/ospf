# BPP3D BLA Context

[English](README.md)

该 context 对应 Kotlin `bpp3d-domain-bla-context`。

## 职责

BLA 提供 bottom-up-left-justified 投影放置能力，是 layer generation 使用的二维放置原语，应与最终 packing 几何验证保持独立。

## 文件结构

- `service.rs` 包含 BLA 算法、projection、placement、配置和测试。
- `mod.rs` 重新导出公开算法入口。

## 扩展点

只有属于 BLA 放置自身的 projection 过滤或评分才应放在这里。包装规则和层候选排序属于 `layer_generation`。

## 验证

运行 `cargo test -p ospf-rust-framework-bpp3d --lib`，并关注覆盖 BL local/global generator 的 layer-generation 测试。
