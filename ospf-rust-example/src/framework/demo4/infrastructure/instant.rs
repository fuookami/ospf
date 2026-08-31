use time::OffsetDateTime;

/// 时间格式化扩展 / Instant formatting extension
/// 对齐 Kotlin Instant.toShortString()
pub trait InstantExt {
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
