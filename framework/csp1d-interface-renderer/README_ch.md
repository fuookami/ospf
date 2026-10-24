# CSP1D 渲染器

[English](README.md) | 简体中文

基于 Tauri + Vue 展示一维分切方案。选择通过 Kotlin 或 Rust 框架渲染 DTO/适配器导出的 JSON 文件。

## 输入兼容性

- 支持当前 Kotlin/Rust 的 camelCase 渲染 DTO，并兼容 snake_case 字段。
- 数值支持 JSON 数字和数字字符串；可选字段保留 null。
- 保留产品 ID、单位长度、生产项数量、原料 ID 和余宽。
- 兼容 `Product`、`Costar`、旧版 `DefectCostar` 标签及自定义颜色。

完整转换规则与跨语言 DTO 测试见[共用渲染契约](../renderer-contract/README_ch.md)。本工具不直接读取任意领域求解结果。

## 开发与验证

在当前目录执行：

```sh
npm install
npm run dev
npm run build
npx vue-tsc --noEmit
cargo check
```
