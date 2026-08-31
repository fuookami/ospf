//! 语义参数模块 / Semantic parameter module
//!
//! 定义航班调度中的语义参数，如成本系数、时间权重等。
//! Defines semantic parameters for airline scheduling, such as cost coefficients and time weights.

/// 语义参数 / Semantic parameter
/// 对齐 Kotlin SemanticParameter
#[derive(Debug, Clone)]
pub struct SemanticParameter {
    /// 参数名称 / Parameter name
    pub name: String,
    /// 参数值 / Parameter value
    pub value: f64,
    /// 参数单位 / Parameter unit
    pub unit: String,
}

impl SemanticParameter {
    /// 创建新的语义参数 / Create a new semantic parameter
    pub fn new(name: &str, value: f64, unit: &str) -> Self {
        Self {
            name: name.to_string(),
            value,
            unit: unit.to_string(),
        }
    }
}
