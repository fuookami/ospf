/// Bunch 编译上下文 / Bunch compilation context
/// 对齐 Kotlin BunchCompilationContext
#[derive(Debug)]
pub struct BunchCompilationContext {
    pub compilations: Vec<super::model::Compilation>,
}

impl BunchCompilationContext {
    pub fn new() -> Self {
        Self { compilations: Vec::new() }
    }
}
