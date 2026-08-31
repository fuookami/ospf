//! 编组编制上下文模块 / Bunch compilation context module
/// Bunch 编译上下文 / Bunch compilation context
/// 对齐 Kotlin BunchCompilationContext
#[derive(Debug)]
pub struct BunchCompilationContext {
    /// 编译结果列表 / Compilation result list
    pub compilations: Vec<super::model::Compilation>,
}

impl BunchCompilationContext {
    /// 创建新的编译上下文 / Create new compilation context
    pub fn new() -> Self {
        Self { compilations: Vec::new() }
    }
}
