//! 机组上下文模块 / Crew context module.
use super::model::Crew;

/// 机组上下文 / Crew context
/// 对齐 Kotlin CrewContext / Aligned with Kotlin CrewContext
#[derive(Debug)]
pub struct CrewContext {
    /// 机组列表 / Crew list
    pub crews: Vec<Crew>,
}

impl CrewContext {
    /// 创建新的机组上下文 / Create new crew context
    pub fn new() -> Self {
        Self { crews: Vec::new() }
    }
}
