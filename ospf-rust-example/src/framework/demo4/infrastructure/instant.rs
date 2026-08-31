//! 时间格式化扩展 / Instant formatting extension
//!
//! 为 OffsetDateTime 提供短格式字符串表示，对齐 Kotlin Instant.toShortString()。
//! Provides short-format string representation for OffsetDateTime, aligned with Kotlin Instant.toShortString().

use time::OffsetDateTime;

/// 时间格式化扩展 / Instant formatting extension
/// 对齐 Kotlin Instant.toShortString()
pub trait InstantExt {
    /// 将时间转换为短格式字符串（MMDDHHmm） / Convert datetime to short-format string (MMDDHHmm)
    fn to_short_string(&self) -> String;
}

impl InstantExt for OffsetDateTime {
    fn to_short_string(&self) -> String {
        format!(
            "{:02}{:02}{:02}{:02}",
            self.month() as u8,
            self.day(),
            self.hour(),
            self.minute()
        )
    }
}
