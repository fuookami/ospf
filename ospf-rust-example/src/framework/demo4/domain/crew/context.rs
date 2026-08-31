use super::model::Crew;

/// 机组上下文 / Crew context
/// 对齐 Kotlin CrewContext
#[derive(Debug)]
pub struct CrewContext {
    pub crews: Vec<Crew>,
}

impl CrewContext {
    pub fn new() -> Self {
        Self { crews: Vec::new() }
    }
}
