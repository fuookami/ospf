/// 输入数据传输对象 / Input data transfer object
/// 对齐 Kotlin Input（Kotlin 也为空类）
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub flights: Vec<String>,
    pub aircrafts: Vec<String>,
}
