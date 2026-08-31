use crate::framework_demo::demo2::domain::soft_security::context::SoftSecurityContext;

pub struct SoftSecurityAggregation {
    pub separated: Vec<usize>,
}

impl SoftSecurityAggregation {
    pub fn from_context(context: &SoftSecurityContext<'_>) -> Self {
        let separated = (0..context.request.cargos.len())
            .filter(|idx| context.request.cargos[*idx].requires_separation)
            .collect();
        Self { separated }
    }
}
