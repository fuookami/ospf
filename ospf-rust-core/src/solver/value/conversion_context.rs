//! 求解值转换上下文 / Solve value conversion context
//!
//! 提供求解值转换过程中的诊断信息，用于错误定位。
//! Provides diagnostic information during solve value conversion for error localization.

/// 求解值转换上下文 / Solve value conversion context
///
/// 携带字段路径等诊断信息，用于在转换失败时定位问题字段。
/// Carries diagnostic information such as field path, used to locate the problem field when conversion fails.
#[derive(Debug, Clone, Default)]
pub struct SolveValueConversionContext {
    /// 字段路径（用于报错定位）/ Field path for diagnostics
    pub field_path: String,
}

impl SolveValueConversionContext {
    /// 创建转换上下文 / Create conversion context
    ///
    /// # 参数 / Parameters
    /// - `field_path`: 字段路径，用于错误信息定位 / Field path for error message localization
    pub fn new(field_path: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
        }
    }
}
