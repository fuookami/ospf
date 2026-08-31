//! 甘特渲染数据传输对象 / Gantt render data transfer objects
//!
//! 用于甘特图渲染的任务数据传输对象。
//! Task data transfer objects for Gantt chart rendering.

use time::OffsetDateTime;
use crate::infrastructure::TimeRange;

/// 甘特渲染任务类别 / Gantt render task category
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GanttRenderTaskCategory {
    /// 正常 / Normal
    Normal,
    /// 测试 / Testing
    Testing,
    /// 不可用 / Unavailable
    Unavailable,
    /// 未知 / Unknown
    Unknown,
}

/// 甘特渲染子任务 DTO / Gantt render sub-task DTO
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct GanttRenderSubTaskDto {
    /// 子任务名称 / Sub-task name
    pub name: String,
    /// 子任务类别 / Sub-task category
    pub category: GanttRenderTaskCategory,
    /// 开始时间 / Start time
    pub start_time: Option<OffsetDateTime>,
    /// 结束时间 / End time
    pub end_time: Option<OffsetDateTime>,
    /// 附加信息 / Additional info
    pub info: Option<String>,
}

/// 甘特渲染任务 DTO / Gantt render task DTO
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct GanttRenderTaskDto {
    /// 执行者名称 / Executor name
    pub executor: String,
    /// 排序序号 / Order index
    pub order: usize,
    /// 产出名称 / Produce name
    pub produce: Option<String>,
    /// 产品列表 / Products list
    pub products: Vec<String>,
    /// 消耗列表 / Consumptions list
    pub consumption: Vec<String>,
    /// 计划时间 / Scheduled time
    pub scheduled_time: Option<TimeRange>,
    /// 实际时间 / Actual time
    pub actual_time: Option<TimeRange>,
    /// 资源列表 / Resources list
    pub resources: Vec<String>,
    /// 子任务列表 / Sub-tasks list
    pub sub_tasks: Vec<GanttRenderSubTaskDto>,
}

/// 甘特渲染数据模式 DTO / Gantt render schema DTO
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct GanttRenderSchemaDto {
    /// 任务列表 / Tasks list
    pub tasks: Vec<GanttRenderTaskDto>,
}
