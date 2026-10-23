//! 锁定实际出发时刻，不从任务 ID 或原始估算时间猜测。 / Lock actual departure times without inferring them from task IDs or original estimates.

use time::OffsetDateTime;

/// 指定任务必须执行且保持精确的实际开始时刻；飞机锁定独立声明。 / Require execution at an exact actual start time; aircraft locks are declared independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    /// 非空且唯一的锁定标识。 / Nonempty unique lock identifier.
    pub id: String,
    /// 关联的待恢复原任务，不接受未知或已执行任务。 / Associated original recovery task, excluding unknown or executed tasks.
    pub task_id: String,
    /// 要求的实际活动开始时刻，不是计划时间或下界。 / Required actual activity start, neither a scheduled time nor a lower bound.
    pub departure_time: OffsetDateTime,
}
