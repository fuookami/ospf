# 基础设施

:us: [English](README.md) | :cn: 简体中文

本目录包含 Gantt Scheduling 领域共享的时间、日历和渲染 DTO 基础设施。

## 职责

- 表达时间区间、时间窗口、时间槽和持续时间范围。
- 提供工作日历工具和日历策略扩展点。
- 通过 `GanttValueAdapter` 转换 solver/domain 时间值。
- 定义下游可视化或 API 层使用的渲染 DTO。

## 模块

- `time_range.rs`：半开时间区间，以及 `merge`、`find`、`find_from`、`find_until` 等区间列表辅助函数。
- `time_window.rs`：时间离散化、舍入和值适配逻辑。
- `time_slot.rs`：时间槽 trait。
- `duration_range.rs`：持续时间范围边界。
- `working_calendar.rs`：工作日历计算。
- `calendar_policy.rs`：日历策略抽象和组合策略。
- `local_date_offset.rs`：本地日期偏移表达。
- `dto/`：Gantt 可视化渲染 DTO。

## Public API

- `TimeRange`
- `TimeWindow`
- `TimeSlot`
- `DurationRange`
- `WorkingCalendar`
- `CalendarPolicy`
- `CompositeCalendarPolicy`
- `DefaultCalendarPolicy`
- `CalendarQuery`
- `LocalDateOffset`
- `GanttRenderSchemaDto`
- `GanttRenderTaskDto`
- `GanttRenderSubTaskDto`
- `GanttRenderTaskCategory`

## 扩展点

新增日历行为通过 `CalendarPolicy` 实现，并用 `CompositeCalendarPolicy` 组合。solver/domain 时间转换保留在 `GanttValueAdapter`，仅可视化结构保留在 `dto/` 下。

## 生命周期与数据流

domain model 使用 time range、window、slot 和 duration bound；calendar policy 响应可用性查询；solver-facing 流程通过 adapter 转换时间值；application/reporting 在输出边界生成 render DTO。

## 验证

修改时间工具、日历策略或 render DTO 转换时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../domain`](../domain/README_ch.md)
- [`../application`](../application/README_ch.md)
