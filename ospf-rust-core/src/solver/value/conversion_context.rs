//! 求解值转换上下文
//! Solve value conversion context

#[derive(Debug, Clone, Default)]
pub struct SolveValueConversionContext {
    /// 字段路径（用于报错定位）
    /// Field path for diagnostics
    pub field_path: String,
}

impl SolveValueConversionContext {
    pub fn new(field_path: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
        }
    }
}
