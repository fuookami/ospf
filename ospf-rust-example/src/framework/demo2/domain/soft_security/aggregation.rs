use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::soft_security::service::limits::EmptyFlagVariables;

pub struct SoftSecurityAggregation {
    pub separated: Vec<usize>,
    pub empty_flag_variables: Option<EmptyFlagVariables>,
}

impl SoftSecurityAggregation {
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
