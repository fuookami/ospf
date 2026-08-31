//! 甘特排程基础设施 / Gantt scheduling infrastructure
//!
//! 映射 Kotlin `gantt-scheduling-infrastructure` 子模块。
//! Maps the Kotlin `gantt-scheduling-infrastructure` submodule.
//!
//! # 核心类型 / Core Types
//!
//! - [`TimeRange`]: 半开时间区间 `[start, end)` / Half-open time interval
//! - [`TimeWindow`]: 时间离散化和舍入 / Time discretization and rounding
//! - [`TimeSlot`]: 时间槽接口 / Time slot trait
//! - [`DurationRange`]: 持续时间范围 / Duration range bounds
//! - [`WorkingCalendar`]: 工作日历 / Working calendar
//! - [`LocalDateOffset`]: 本地日期偏移 / Local date offset
//!
//! # 列表扩展函数 / List Extension Functions
//!
//! - [`merge`]: 合并重叠/相邻时间范围 / Merge overlapping/adjacent time ranges
//! - [`find`]: 查找与给定范围相交的所有范围 / Find intersecting ranges
//! - [`find_from`]: 查找从给定时间点开始的范围 / Find ranges from an instant
//! - [`find_until`]: 查找直到给定时间点的范围 / Find ranges until an instant

mod time_range;
mod time_window;
mod time_slot;
mod duration_range;
mod working_calendar;
mod local_date_offset;
pub mod dto;

// ========================================================================
// 公共重导出 / Public re-exports
// ========================================================================

pub use time_range::{
    TimeRange,
    SplitTimeRanges,
    distant_past,
    distant_future,
    merge,
    find,
    find_from,
    find_until,
    front_at,
    back_at,
};

pub use time_window::{
    DurationUnit,
    GanttValueAdapter,
    TimeWindow,
};

pub use time_slot::TimeSlot;

pub use duration_range::DurationRange;

pub use working_calendar::{
    ActualTime,
    ValidTimes,
    WorkingCalendar,
};

pub use local_date_offset::LocalDateOffset;

// DTO 重导出 / DTO re-exports
pub use dto::{
    GanttRenderTaskCategory,
    GanttRenderSubTaskDto,
    GanttRenderTaskDto,
    GanttRenderSchemaDto,
};
