/// 航班 / Flight (stowage domain, 对齐 Kotlin Flight)
#[derive(Debug, Clone)]
pub struct Flight {
    pub id: String,
    pub name: String,
    pub dep: String,
    pub arr: String,
}
