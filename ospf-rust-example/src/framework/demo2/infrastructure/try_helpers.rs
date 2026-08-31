//! Try 辅助函数 / Try helper functions
/// Try 辅助函数模块 / Try helper functions module
///
/// 对齐 Kotlin TryHelpers / Aligned with Kotlin TryHelpers

/// 将 Option 转换为 Result
pub fn try_option<T>(value: Option<T>, message: &str) -> Result<T, String> {
    value.ok_or_else(|| message.to_string())
}

/// 将布尔条件转换为 Result
pub fn try_require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}
