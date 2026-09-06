//! 时间窗口 / Time window providing time discretization and rounding
//!
//! 提供时间到数值的离散化、舍入和时间段划分能力。
//! Provides time-to-value discretization, rounding, and time slot generation capabilities.

use std::marker::PhantomData;

use time::{Duration, OffsetDateTime};

use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::solver::value::SolveValueConversionPolicy;

use crate::infrastructure::{TimeRange, merge};
use crate::{GanttError, GanttResult};

/// 持续时间单位 / Duration unit
///
/// 支持的时间精度级别，用于时间到数值的转换。
/// Supported time precision levels for time-to-value conversion.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DurationUnit {
    /// 秒 / Seconds
    Seconds,
    /// 分钟 / Minutes
    Minutes,
    /// 小时 / Hours
    Hours,
}

impl DurationUnit {
    fn nanoseconds_per_unit(self) -> f64 {
        match self {
            DurationUnit::Seconds => 1_000_000_000.0,
            DurationUnit::Minutes => 60_000_000_000.0,
            DurationUnit::Hours => 3_600_000_000_000.0,
        }
    }

    /// 尝试从数值创建持续时间 / Try to create duration from value
    pub fn try_from_value(self, value: f64) -> GanttResult<Duration> {
        let nanoseconds = value * self.nanoseconds_per_unit();
        if !nanoseconds.is_finite()
            || nanoseconds < i64::MIN as f64
            || nanoseconds > i64::MAX as f64
        {
            return Err(GanttError::InvalidDuration {
                message: format!("duration value {value} is not finite or is out of range"),
            });
        }
        Ok(Duration::nanoseconds(nanoseconds.round() as i64))
    }

    /// 从 DurationUnit 创建 Duration / Create Duration from DurationUnit
    ///
    /// 将给定数值按此单位转换为 `time::Duration`。
    /// Converts the given value to `time::Duration` using this unit.
    pub fn from_value(&self, value: f64) -> Duration {
        self.try_from_value(value).unwrap_or(Duration::ZERO)
    }

    /// 将 Duration 转换为此单位的数值 / Convert Duration to value in this unit
    ///
    /// 返回持续时间在此单位下的数值表示。
    /// Returns the numeric representation of the duration in this unit.
    pub fn to_value(&self, duration: Duration) -> f64 {
        duration.whole_nanoseconds() as f64 / self.nanoseconds_per_unit()
    }

    /// 获取上级单位 / Get upper-level unit
    ///
    /// - Seconds -> Minutes
    /// - Minutes -> Hours
    /// - Hours -> 不支持（返回 None）
    ///
    /// - Seconds -> Minutes
    /// - Minutes -> Hours
    /// - Hours -> unsupported (returns None)
    pub fn upper(&self) -> Option<DurationUnit> {
        match self {
            DurationUnit::Seconds => Some(DurationUnit::Minutes),
            DurationUnit::Minutes => Some(DurationUnit::Hours),
            DurationUnit::Hours => None,
        }
    }

    /// 获取上级单位的默认间隔 / Get default interval for upper-level unit
    ///
    /// - Seconds -> 1 分钟
    /// - Minutes -> 1 小时
    /// - Hours -> 不支持
    ///
    /// - Seconds -> 1 minute
    /// - Minutes -> 1 hour
    /// - Hours -> unsupported
    pub fn upper_interval(&self) -> Option<Duration> {
        match self {
            DurationUnit::Seconds => Some(Duration::minutes(1)),
            DurationUnit::Minutes => Some(Duration::hours(1)),
            DurationUnit::Hours => None,
        }
    }
}

/// 甘特数值适配器 / Gantt value adapter
///
/// 封装 `SolveValue` trait，使用 `AllowRounding` 策略提供简洁的 f64 转换。
/// Wraps `SolveValue` trait with `AllowRounding` policy for simplified f64 conversion.
pub struct GanttValueAdapter<V: SolveValue>(PhantomData<V>);

impl<V: SolveValue> GanttValueAdapter<V> {
    /// 从 f64 创建值 / Create value from f64
    ///
    /// 使用 `AllowRounding` 策略，时间值不需要严格精度。
    /// Uses `AllowRounding` policy; time values don't need strict precision.
    pub fn from_f64(value: f64) -> V {
        V::from_f64_with_policy(value, SolveValueConversionPolicy::AllowRounding)
            .expect("GanttValueAdapter::from_f64 failed")
    }

    /// 转换为 f64 / Convert to f64
    ///
    /// 使用 `AllowRounding` 策略。
    /// Uses `AllowRounding` policy.
    pub fn to_f64(value: &V) -> f64 {
        value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .expect("GanttValueAdapter::to_f64 failed")
    }
}

/// 时间窗口，提供时间离散化和舍入功能 / Time window providing time discretization and rounding
///
/// `TimeWindow<V>` 是 Gantt 排程基础设施的核心类型，将连续时间映射到离散数值空间，
/// 支持求解器数值边界和时间范围之间的双向转换。
///
/// `TimeWindow<V>` is the core type of Gantt scheduling infrastructure,
/// mapping continuous time to a discrete numeric space, supporting bidirectional
/// conversion between solver numeric boundaries and time ranges.
///
/// # 泛型参数 / Type Parameters
///
/// - `V`: 数值类型，必须实现 `SolveValue`
/// - `V`: Numeric type, must implement `SolveValue`
#[derive(Debug, Clone)]
pub struct TimeWindow<V: SolveValue> {
    /// 基础时间范围 / Underlying time range
    pub window: TimeRange,
    /// 是否连续 / Whether continuous
    pub continues: bool,
    /// 持续时间单位 / Duration unit
    pub duration_unit: DurationUnit,
    /// 日期偏移量（用于跨日边界计算）/ Date offset (for cross-day boundary calculation)
    pub date_offset: Duration,
    /// 时间间隔 / Time interval
    pub interval: Duration,
    _marker: PhantomData<V>,
}

impl<V: SolveValue> TimeWindow<V> {
    fn validate_interval(interval: Duration) -> GanttResult<()> {
        if interval <= Duration::ZERO {
            return Err(GanttError::InvalidDuration {
                message: "time window interval must be positive".to_string(),
            });
        }
        Ok(())
    }

    /// 创建新的时间窗口 / Create new time window
    pub fn new(
        window: TimeRange,
        continues: bool,
        duration_unit: DurationUnit,
        date_offset: Duration,
        interval: Duration,
    ) -> Self {
        Self {
            window,
            continues,
            duration_unit,
            date_offset,
            interval,
            _marker: PhantomData,
        }
    }

    /// 创建经过校验的时间窗口 / Create a validated time window
    pub fn try_new(
        window: TimeRange,
        continues: bool,
        duration_unit: DurationUnit,
        date_offset: Duration,
        interval: Duration,
    ) -> GanttResult<Self> {
        Self::validate_interval(interval)?;
        Ok(Self::new(
            window,
            continues,
            duration_unit,
            date_offset,
            interval,
        ))
    }

    /// 创建秒级时间窗口 / Create seconds-level time window
    ///
    /// # 参数 / Parameters
    ///
    /// - `window`: 时间范围 / Time range
    /// - `date_offset`: 日期偏移（秒）/ Date offset (seconds)
    /// - `continues`: 是否连续 / Whether continuous
    /// - `interval`: 时间间隔（秒）/ Time interval (seconds)
    pub fn seconds(window: TimeRange, date_offset: V, continues: bool, interval: V) -> Self {
        let date_offset_dur =
            DurationUnit::Seconds.from_value(GanttValueAdapter::<V>::to_f64(&date_offset));
        let interval_dur =
            DurationUnit::Seconds.from_value(GanttValueAdapter::<V>::to_f64(&interval));
        Self::new(
            window,
            continues,
            DurationUnit::Seconds,
            date_offset_dur,
            interval_dur,
        )
    }

    /// 创建经过校验的秒级时间窗口 / Create a validated seconds-level time window
    pub fn try_seconds(
        window: TimeRange,
        date_offset: V,
        continues: bool,
        interval: V,
    ) -> GanttResult<Self> {
        Self::try_new(
            window,
            continues,
            DurationUnit::Seconds,
            DurationUnit::Seconds.try_from_value(GanttValueAdapter::<V>::to_f64(&date_offset))?,
            DurationUnit::Seconds.try_from_value(GanttValueAdapter::<V>::to_f64(&interval))?,
        )
    }

    /// 创建分钟级时间窗口 / Create minutes-level time window
    ///
    /// # 参数 / Parameters
    ///
    /// - `window`: 时间范围 / Time range
    /// - `date_offset`: 日期偏移（分钟）/ Date offset (minutes)
    /// - `continues`: 是否连续 / Whether continuous
    /// - `interval`: 时间间隔（分钟）/ Time interval (minutes)
    pub fn minutes(window: TimeRange, date_offset: V, continues: bool, interval: V) -> Self {
        let date_offset_dur =
            DurationUnit::Minutes.from_value(GanttValueAdapter::<V>::to_f64(&date_offset));
        let interval_dur =
            DurationUnit::Minutes.from_value(GanttValueAdapter::<V>::to_f64(&interval));
        Self::new(
            window,
            continues,
            DurationUnit::Minutes,
            date_offset_dur,
            interval_dur,
        )
    }

    /// 创建经过校验的分钟级时间窗口 / Create a validated minutes-level time window
    pub fn try_minutes(
        window: TimeRange,
        date_offset: V,
        continues: bool,
        interval: V,
    ) -> GanttResult<Self> {
        Self::try_new(
            window,
            continues,
            DurationUnit::Minutes,
            DurationUnit::Minutes.try_from_value(GanttValueAdapter::<V>::to_f64(&date_offset))?,
            DurationUnit::Minutes.try_from_value(GanttValueAdapter::<V>::to_f64(&interval))?,
        )
    }

    /// 创建小时级时间窗口 / Create hours-level time window
    ///
    /// # 参数 / Parameters
    ///
    /// - `window`: 时间范围 / Time range
    /// - `date_offset`: 日期偏移（小时）/ Date offset (hours)
    /// - `continues`: 是否连续 / Whether continuous
    /// - `interval`: 时间间隔（小时）/ Time interval (hours)
    pub fn hours(window: TimeRange, date_offset: V, continues: bool, interval: V) -> Self {
        let date_offset_dur =
            DurationUnit::Hours.from_value(GanttValueAdapter::<V>::to_f64(&date_offset));
        let interval_dur =
            DurationUnit::Hours.from_value(GanttValueAdapter::<V>::to_f64(&interval));
        Self::new(
            window,
            continues,
            DurationUnit::Hours,
            date_offset_dur,
            interval_dur,
        )
    }

    /// 创建经过校验的小时级时间窗口 / Create a validated hours-level time window
    pub fn try_hours(
        window: TimeRange,
        date_offset: V,
        continues: bool,
        interval: V,
    ) -> GanttResult<Self> {
        Self::try_new(
            window,
            continues,
            DurationUnit::Hours,
            DurationUnit::Hours.try_from_value(GanttValueAdapter::<V>::to_f64(&date_offset))?,
            DurationUnit::Hours.try_from_value(GanttValueAdapter::<V>::to_f64(&interval))?,
        )
    }

    /// 创建新的时间窗口（保持配置）/ Create new time window with same config
    ///
    /// 保持 `duration_unit`、`date_offset`、`interval` 不变，替换 `window` 和 `continues`。
    /// Preserves `duration_unit`, `date_offset`, `interval`; replaces `window` and `continues`.
    pub fn with_window(&self, window: TimeRange, continues: bool) -> Self {
        Self {
            window,
            continues,
            duration_unit: self.duration_unit,
            date_offset: self.date_offset,
            interval: self.interval,
            _marker: PhantomData,
        }
    }

    // ========================================================================
    // 数值转换 / Value conversion
    // ========================================================================

    /// 获取持续时间的 V 数值 / Get V value of duration
    ///
    /// 将 `Duration` 转换为此窗口单位下的数值。
    /// Converts `Duration` to a numeric value in this window's unit.
    pub fn value_of_duration(&self, duration: Duration) -> V {
        GanttValueAdapter::<V>::from_f64(self.duration_unit.to_value(duration))
    }

    /// 获取时间点的 V 数值 / Get V value of instant
    ///
    /// 计算时间点相对于窗口起点的偏移值。
    /// Computes the offset value of an instant relative to the window start.
    pub fn value_of_instant(&self, instant: OffsetDateTime) -> V {
        let offset = instant - self.window.start;
        GanttValueAdapter::<V>::from_f64(self.duration_unit.to_value(offset))
    }

    /// 从 V 数值创建持续时间 / Create duration from V value
    ///
    /// 将数值转换为此窗口单位下的 `Duration`。
    /// Converts a numeric value to `Duration` using this window's unit.
    pub fn duration_of(&self, value: V) -> Duration {
        let fval = GanttValueAdapter::<V>::to_f64(&value);
        self.duration_unit.from_value(fval)
    }

    /// 从 V 数值创建时间点 / Create instant from V value
    ///
    /// 将数值转换为相对于窗口起点的 `OffsetDateTime`。
    /// Converts a numeric value to `OffsetDateTime` relative to the window start.
    pub fn instant_of(&self, value: V) -> OffsetDateTime {
        self.window.start + self.duration_of(value)
    }

    /// 从 i64 创建持续时间 / Create duration from i64
    pub fn duration_of_i64(&self, value: i64) -> Duration {
        self.duration_unit.from_value(value as f64)
    }

    /// 从 i64 创建时间点 / Create instant from i64
    pub fn instant_of_i64(&self, value: i64) -> OffsetDateTime {
        self.window.start + self.duration_of_i64(value)
    }

    /// 从 u64 创建持续时间 / Create duration from u64
    pub fn duration_of_u64(&self, value: u64) -> Duration {
        self.duration_unit.from_value(value as f64)
    }

    /// 从 u64 创建时间点 / Create instant from u64
    pub fn instant_of_u64(&self, value: u64) -> OffsetDateTime {
        self.window.start + self.duration_of_u64(value)
    }

    // ========================================================================
    // 舍入 / Rounding
    // ========================================================================

    /// 舍入持续时间 / Round duration
    ///
    /// 在此窗口单位下四舍五入。
    /// Rounds the duration in this window's unit.
    pub fn round_duration(&self, duration: Duration) -> Duration {
        let value = self.duration_unit.to_value(duration);
        let rounded = value.round();
        self.duration_unit.from_value(rounded)
    }

    /// 向下取整持续时间 / Floor duration
    pub fn floor_duration(&self, duration: Duration) -> Duration {
        let value = self.duration_unit.to_value(duration);
        let floored = value.floor();
        self.duration_unit.from_value(floored)
    }

    /// 向上取整持续时间 / Ceil duration
    pub fn ceil_duration(&self, duration: Duration) -> Duration {
        let value = self.duration_unit.to_value(duration);
        let ceiled = value.ceil();
        self.duration_unit.from_value(ceiled)
    }

    /// 舍入时间点 / Round instant
    ///
    /// 相对于窗口起点进行四舍五入。
    /// Rounds the instant relative to the window start.
    pub fn round_instant(&self, instant: OffsetDateTime) -> OffsetDateTime {
        let offset = instant - self.window.start;
        self.window.start + self.round_duration(offset)
    }

    /// 向下取整时间点 / Floor instant
    pub fn floor_instant(&self, instant: OffsetDateTime) -> OffsetDateTime {
        let offset = instant - self.window.start;
        self.window.start + self.floor_duration(offset)
    }

    /// 向上取整时间点 / Ceil instant
    pub fn ceil_instant(&self, instant: OffsetDateTime) -> OffsetDateTime {
        let offset = instant - self.window.start;
        self.window.start + self.ceil_duration(offset)
    }

    // ========================================================================
    // 时间段划分 / Time slot generation
    // ========================================================================

    /// 按默认间隔划分的时间段列表 / List of time slots divided by default interval
    pub fn time_slots(&self) -> Vec<TimeRange> {
        self.time_slots_of(self.interval)
    }

    /// 按默认间隔尝试划分时间段 / Try to divide time slots by default interval
    pub fn try_time_slots(&self) -> GanttResult<Vec<TimeRange>> {
        self.try_time_slots_of(self.interval)
    }

    /// 按指定间隔划分时间段 / Divide time slots by specified interval
    ///
    /// 最后一个时间段可能不足一个完整间隔。
    /// The last slot may be shorter than a full interval.
    pub fn time_slots_of(&self, interval: Duration) -> Vec<TimeRange> {
        self.try_time_slots_of(interval).unwrap_or_default()
    }

    /// 按指定间隔尝试划分时间段 / Try to divide time slots by specified interval
    pub fn try_time_slots_of(&self, interval: Duration) -> GanttResult<Vec<TimeRange>> {
        Self::validate_interval(interval)?;
        let mut slots = Vec::new();
        let start = self.window.start;
        let end = self.window.end;
        let mut current = start;

        while current < end {
            let remaining = end - current;
            let slot_duration = remaining.min(interval);
            slots.push(TimeRange::new(current, current + slot_duration));
            current += slot_duration;
        }

        Ok(slots)
    }

    /// 按上级间隔划分的舍入时间段 / Generate rounded time slots by upper interval
    ///
    /// 生成按上级单位边界对齐的时间段，并排除指定的时间范围。
    /// Generates time slots aligned to the upper unit boundary,
    /// excluding specified time ranges.
    ///
    /// # 参数 / Parameters
    ///
    /// - `interval`: 时间段间隔 / Time slot interval
    /// - `excluded_times`: 排除的时间范围 / Excluded time ranges
    pub fn round_time_slots_of(
        &self,
        interval: Duration,
        excluded_times: &[TimeRange],
    ) -> Vec<TimeRange> {
        if interval <= Duration::ZERO {
            return vec![];
        }
        let start = self.window.start;
        let end = self.window.end;

        if start >= end {
            return vec![];
        }

        let upper_interval = self.duration_unit.upper_interval();

        let mut slots = Vec::new();

        // Stage 1: Initial segment from start to first aligned boundary
        let first_boundary = if let Some(upper_int) = upper_interval {
            self.ceil_instant(start + upper_int)
        } else {
            end
        };

        let first_boundary = first_boundary.min(end);

        // Generate initial slots
        let mut current = start;
        while current < first_boundary {
            let remaining = first_boundary - current;
            let slot_duration = remaining.min(interval);
            if slot_duration > Duration::ZERO {
                slots.push(TimeRange::new(current, current + slot_duration));
            }
            current += slot_duration;
        }

        // Stage 2: Middle segments aligned to upper boundary
        if let Some(upper_int) = upper_interval {
            let mut aligned = first_boundary;
            while aligned + upper_int <= end {
                let segment_start = aligned;
                let segment_end = aligned + upper_int;

                let mut seg_current = segment_start;
                while seg_current < segment_end {
                    let remaining = segment_end - seg_current;
                    let slot_duration = remaining.min(interval);
                    if slot_duration > Duration::ZERO {
                        slots.push(TimeRange::new(seg_current, seg_current + slot_duration));
                    }
                    seg_current += slot_duration;
                }

                aligned = segment_end;
            }

            // Stage 3: Final unaligned segment
            if aligned < end {
                let mut final_current = aligned;
                while final_current < end {
                    let remaining = end - final_current;
                    let slot_duration = remaining.min(interval);
                    if slot_duration > Duration::ZERO {
                        slots.push(TimeRange::new(final_current, final_current + slot_duration));
                    }
                    final_current += slot_duration;
                }
            }
        }

        // Handle exclusions
        if excluded_times.is_empty() {
            return slots;
        }

        // Clip exclusions to window bounds and merge
        let clipped: Vec<TimeRange> = excluded_times
            .iter()
            .filter_map(|ex| self.window.intersection(ex))
            .collect();
        let merged_exclusions = merge(&clipped);

        if merged_exclusions.is_empty() {
            return slots;
        }

        // Remove excluded ranges from generated slots
        let mut result = Vec::new();
        for slot in &slots {
            let diff = slot.difference_with_many(&merged_exclusions);
            result.extend(diff);
        }

        result
    }

    // ========================================================================
    // 上级窗口 / Upper-level window
    // ========================================================================

    /// 获取上级时间间隔 / Get upper-level interval
    ///
    /// - Seconds -> 1 分钟
    /// - Minutes -> 1 小时
    /// - Hours -> 不支持
    pub fn upper_interval(&self) -> Option<Duration> {
        self.duration_unit.upper_interval()
    }

    /// 获取上级时间窗口 / Get upper-level time window
    ///
    /// 创建一个新的 `TimeWindow`，使用上级时间单位。
    /// Creates a new `TimeWindow` with the upper-level duration unit.
    pub fn upper(&self) -> Option<TimeWindow<V>> {
        let upper_unit = self.duration_unit.upper()?;
        let upper_int = upper_unit.upper_interval()?;
        Some(TimeWindow {
            window: self.window,
            continues: self.continues,
            duration_unit: upper_unit,
            date_offset: self.date_offset,
            interval: upper_int,
            _marker: PhantomData,
        })
    }

    // ========================================================================
    // 包含检查 / Contains checks
    // ========================================================================

    /// 是否包含时间点 / Check if contains instant
    pub fn contains_instant(&self, instant: OffsetDateTime) -> bool {
        self.window.contains_instant(instant)
    }

    /// 是否包含时间范围 / Check if contains time range
    pub fn contains_range(&self, range: &TimeRange) -> bool {
        self.window.contains_range(range)
    }

    /// 是否与时间范围有交集 / Check if intersects with time range
    pub fn intersects(&self, range: &TimeRange) -> bool {
        self.window.intersects(range)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn h(hour: i8) -> OffsetDateTime {
        datetime!(2020-08-30 00:00 UTC) + Duration::hours(hour as i64)
    }

    #[test]
    fn test_value_of_duration() {
        let window: TimeWindow<f64> =
            TimeWindow::hours(TimeRange::new(h(8), h(12)), 0.0, true, 1.0);
        let val = window.value_of_duration(Duration::hours(2));
        assert!((val - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_value_of_instant() {
        let window: TimeWindow<f64> =
            TimeWindow::hours(TimeRange::new(h(8), h(12)), 0.0, true, 1.0);
        let val = window.value_of_instant(h(11));
        assert!((val - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_minutes_value_of() {
        let window: TimeWindow<f64> =
            TimeWindow::minutes(TimeRange::new(h(8), h(12)), 0.0, true, 15.0);
        let val = window.value_of_duration(Duration::minutes(30));
        assert!((val - 30.0).abs() < 1e-10);

        let instant_val = window.value_of_instant(datetime!(2020-08-30 08:45 UTC));
        assert!((instant_val - 45.0).abs() < 1e-10);
    }

    #[test]
    fn test_duration_of() {
        let window: TimeWindow<f64> =
            TimeWindow::minutes(TimeRange::new(h(8), h(12)), 0.0, true, 15.0);
        let dur = window.duration_of(30.0);
        assert_eq!(dur, Duration::minutes(30));
    }

    #[test]
    fn test_instant_of() {
        let window: TimeWindow<f64> =
            TimeWindow::minutes(TimeRange::new(h(8), h(12)), 0.0, true, 15.0);
        let instant = window.instant_of(45.0);
        assert_eq!(instant, datetime!(2020-08-30 08:45 UTC));
    }

    #[test]
    fn test_time_slots() {
        let window: TimeWindow<f64> =
            TimeWindow::hours(TimeRange::new(h(8), h(12)), 0.0, true, 1.0);
        let slots = window.time_slots();
        assert_eq!(slots.len(), 4);
        assert_eq!(slots[0], TimeRange::new(h(8), h(9)));
        assert_eq!(slots[3], TimeRange::new(h(11), h(12)));
    }

    #[test]
    fn test_fractional_hour_time_slots_preserve_boundaries() {
        let window: TimeWindow<f64> =
            TimeWindow::try_hours(TimeRange::new(h(8), h(10)), 0.0, true, 0.5).unwrap();

        let slots = window.try_time_slots().unwrap();

        assert_eq!(slots.len(), 4);
        assert_eq!(slots[0], TimeRange::new(h(8), h(8) + Duration::minutes(30)));
        assert_eq!(
            slots[3],
            TimeRange::new(h(9) + Duration::minutes(30), h(10))
        );
    }

    #[test]
    fn test_time_slots_keep_short_final_slot() {
        let window: TimeWindow<f64> =
            TimeWindow::try_minutes(TimeRange::new(h(8), h(9)), 0.0, true, 40.0).unwrap();

        let slots = window.try_time_slots().unwrap();

        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0], TimeRange::new(h(8), h(8) + Duration::minutes(40)));
        assert_eq!(slots[1], TimeRange::new(h(8) + Duration::minutes(40), h(9)));
    }

    #[test]
    fn test_invalid_interval_is_rejected_without_looping() {
        let result = TimeWindow::<f64>::try_hours(TimeRange::new(h(8), h(10)), 0.0, true, 0.0);
        assert!(matches!(result, Err(GanttError::InvalidDuration { .. })));

        let unchecked = TimeWindow::<f64>::hours(TimeRange::new(h(8), h(10)), 0.0, true, 0.0);
        assert!(unchecked.time_slots().is_empty());
    }

    #[test]
    fn test_non_finite_duration_is_rejected() {
        let result = DurationUnit::Seconds.try_from_value(f64::NAN);
        assert!(matches!(result, Err(GanttError::InvalidDuration { .. })));
    }

    #[test]
    fn test_round_time_slots_with_excluded_times() {
        let window: TimeWindow<f64> =
            TimeWindow::hours(TimeRange::new(h(8), h(12)), 0.0, true, 1.0);
        let excluded = vec![TimeRange::new(h(9), h(10))];
        let slots = window.round_time_slots_of(Duration::hours(1), &excluded);

        // Total duration should be 3 hours (4 - 1 excluded)
        let total_duration: Duration = slots.iter().map(|s| s.duration()).sum();
        assert_eq!(total_duration, Duration::hours(3));
    }

    #[test]
    fn test_round_time_slots_all_excluded() {
        let window: TimeWindow<f64> =
            TimeWindow::hours(TimeRange::new(h(8), h(12)), 0.0, true, 1.0);
        let excluded = vec![TimeRange::new(h(8), h(12))];
        let slots = window.round_time_slots_of(Duration::hours(1), &excluded);
        assert!(slots.is_empty());
    }

    #[test]
    fn test_round_time_slots_adjacent_excluded() {
        let window: TimeWindow<f64> =
            TimeWindow::hours(TimeRange::new(h(8), h(12)), 0.0, true, 1.0);
        let excluded = vec![TimeRange::new(h(9), h(10)), TimeRange::new(h(10), h(11))];
        let slots = window.round_time_slots_of(Duration::hours(1), &excluded);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0], TimeRange::new(h(8), h(9)));
        assert_eq!(slots[1], TimeRange::new(h(11), h(12)));
    }

    #[test]
    fn test_upper_interval() {
        assert_eq!(
            DurationUnit::Seconds.upper_interval(),
            Some(Duration::minutes(1))
        );
        assert_eq!(
            DurationUnit::Minutes.upper_interval(),
            Some(Duration::hours(1))
        );
        assert_eq!(DurationUnit::Hours.upper_interval(), None);
    }

    #[test]
    fn test_round_duration() {
        let window: TimeWindow<f64> =
            TimeWindow::minutes(TimeRange::new(h(8), h(12)), 0.0, true, 15.0);
        let rounded = window.round_duration(Duration::minutes(47));
        // 47 rounds to 47 in whole minutes (already integral)
        assert_eq!(rounded, Duration::minutes(47));
    }
}
