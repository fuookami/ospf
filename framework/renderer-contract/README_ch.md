# 渲染器数据契约

[English](README.md) | 简体中文

BPP3D、CSP1D 和 Gantt 的 Tauri 渲染器共用此导入工具，将当前 Kotlin/Rust 的**渲染 DTO**转换为 Vue 使用的 camelCase JSON。领域求解结果需要先通过所属框架的渲染适配器转换，不能直接导入任意求解结果。

| 输入差异 | 导入行为 |
| --- | --- |
| Kotlin camelCase / Rust snake_case | 两者均可读取，统一输出 camelCase。 |
| 数字字符串 / JSON 数字 | 转为有限数值；非法数值返回导入错误。 |
| BPP3D 字符串分组 / 列表分组 | 统一为列表，保留形状与包围尺寸等信息。 |
| 文本、对象或 null 的 `info`；JSON 值的 KPI | 转为字符串值记录；文本存入 `info` 键，嵌套 KPI 保留为 JSON 文本。 |
| CSP1D 的 `id`、`materialId`、`unitLength`、生产项 `amount`、`restWidth` | 存在时保留；缺失的可选字段为 null。 |
| Gantt 列表 / 映射 | 列表转为以序号为键的记录，保留重复值与顺序。 |
| Gantt 的 `actual_time` / `scheduled_time` 区间 | 转为开始/结束字段；只有计划区间的任务按该区间展示。 |
| Gantt 日期字符串 / `time::OffsetDateTime` serde 元组 | 均可读取，DTO 转换保留纳秒精度。 |

有明确偏移量的时间统一转为 UTC。Kotlin 的 `DateTimeSerializer` 输出不带偏移量的本地时间，导入时原样保留，无法反推原始时区。跨来源比较前应统一导出时区（例如 UTC）。图表显示精度为毫秒。

Rust Gantt 任务没有顶层名称和类别：展示名称取 `produce`，缺失时使用 `executor #order`，类别默认为 `Normal`。Rust 的 `order` 是排序序号，Kotlin 的 `order` 可以是业务订单标识；导入不会将两者的业务含义混为一谈。

未排期任务和子任务保留 null 时间，不绘制。空任务列表或全部未排期的数据会显示提示，不补造时间。非法时间或倒置区间会返回错误。Gantt 的消耗同时读取当前 `consumption` 和旧版 `materials`，同名键以当前字段为准。

集成测试直接引用**三个 Tauri 应用实际使用的 DTO 源文件**，无需桌面构建工具链即可验证 JSON 兼容性。在仓库根目录执行：

```sh
cargo test --manifest-path framework/renderer-contract/Cargo.toml
```

每个渲染器还支持 `npm run build` 和 `npx vue-tsc --noEmit`。Gantt 另有 `npm test`，检查 DTO 到视图的转换（使用 Node.js 24）。
