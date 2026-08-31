/// 输出数据传输对象 / Output data transfer object
/// 对齐 Kotlin Output（Kotlin 也为空类）
#[derive(Debug, Clone, Default)]
pub struct Output {
    pub status: String,
    pub assignments: Vec<String>,
}
