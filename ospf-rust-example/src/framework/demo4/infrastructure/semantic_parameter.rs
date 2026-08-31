/// 语义参数 / Semantic parameter
/// 对齐 Kotlin SemanticParameter
#[derive(Debug, Clone)]
pub struct SemanticParameter {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

impl SemanticParameter {
    pub fn new(name: &str, value: f64, unit: &str) -> Self {
        Self {
            name: name.to_string(),
            value,
            unit: unit.to_string(),
        }
    }
}
