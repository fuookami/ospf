# Infrastructure

:us: English | :cn: [简体中文](README_ch.md)

This directory contains time, calendar, and rendering DTO infrastructure shared by the Gantt scheduling domain.

## Responsibilities

- Represent time ranges, time windows, time slots, and duration ranges.
- Provide working-calendar utilities and calendar policy extension points.
- Convert solver/domain time values through `GanttValueAdapter`.
- Define render DTOs used by downstream visualization or API layers.

## Modules

- `time_range.rs`: half-open time ranges and range list helpers such as `merge`, `find`, `find_from`, and `find_until`.
- `time_window.rs`: time discretization, rounding, and value adapter logic.
- `time_slot.rs`: time slot trait.
- `duration_range.rs`: duration range bounds.
- `working_calendar.rs`: working-calendar calculation.
- `calendar_policy.rs`: calendar policy abstraction and composite policy.
- `local_date_offset.rs`: local date offset representation.
- `dto/`: render DTOs for Gantt visualization.

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

## Extension Points

Add calendar behavior through `CalendarPolicy` implementations and compose them with `CompositeCalendarPolicy`. Keep solver/domain time conversion in `GanttValueAdapter` and keep visualization-only structures under `dto/`.

## Lifecycle and Data Flow

Domain models use time ranges, windows, slots, and duration bounds; calendar policies answer availability queries; solver-facing flows convert time values through adapters; application/reporting code emits render DTOs at the output boundary.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing time utilities, calendar policies, or render DTO conversions.

## Related Directories

- [`../domain`](../domain/README.md)
- [`../application`](../application/README.md)
