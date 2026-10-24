# Gantt Scheduling 渲染器

[English](README.md) | 简体中文

基于 Tauri + Vue 展示任务排程。选择通过 Kotlin 或 Rust 框架渲染 DTO/适配器导出的 JSON 文件。

## 输入兼容性

- 支持 Kotlin 的 camelCase 开始/结束字段，以及 Rust 的 snake_case `actual_time` / `scheduled_time` 区间。
- 支持本地日期字符串、ISO 8601 时间和 `time::OffsetDateTime` 默认 serde 元组。有偏移量的时间转为 UTC；Kotlin 无偏移量时间保留导出时本地时间，跨来源比较前应统一导出时区。
- Rust 的产品、消耗和资源列表转为序号键记录；兼容 Kotlin 映射及旧版 `materials`。
- 没有实际或计划时间的任务保留但不绘制；空数据或全部未排期时显示提示，不补造时间。

完整转换规则与跨语言 DTO 测试见[共用渲染契约](../renderer-contract/README_ch.md)。本工具不直接读取任意领域求解结果。

## 开发与验证

在当前目录执行：

```sh
npm install
npm run dev
npm run build
npx vue-tsc --noEmit
npm test
cargo check
```
