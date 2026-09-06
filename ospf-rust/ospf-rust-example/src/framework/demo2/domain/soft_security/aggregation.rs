//! 软性安全聚合 / Soft security aggregation
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::soft_security::service::limits::EmptyFlagVariables;

/// 软安全聚合 / Soft security aggregation
///
/// 持有需要分离的货物索引和空舱位标志变量。
/// Holds indices of cargos requiring separation and empty flag variables.
pub struct SoftSecurityAggregation {
    /// 需要分离的货物索引 / Indices of cargos requiring separation
    pub separated: Vec<usize>,
    /// 空舱位标志变量 / Empty position flag variables
    pub empty_flag_variables: Option<EmptyFlagVariables>,
}

impl SoftSecurityAggregation {
    /// 从软安全上下文创建聚合 / Create aggregation from soft security context
    pub fn from_context(context: &SoftSecurityContext<'_>) -> Self {
        let separated = (0..context.request.cargos.len())
            .filter(|idx| context.request.cargos[*idx].requires_separation)
            .collect();
        Self {
            separated,
            empty_flag_variables: None,
        }
    }
}
