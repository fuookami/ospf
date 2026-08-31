use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

pub struct DomainPolicySnapshot {
    pub domain: &'static str,
    pub step_counts: Vec<usize>,
}

pub fn mode_order() -> [Demo2PipelineMode; 3] {
    [
        Demo2PipelineMode::FullLoad,
        Demo2PipelineMode::Predistribution,
        Demo2PipelineMode::WeightRecommendation,
    ]
}

pub fn policy_matrix_snapshot() -> Vec<DomainPolicySnapshot> {
    let modes = mode_order();
    vec![
        DomainPolicySnapshot {
            domain: "stowage",
            step_counts: modes
                .iter()
                .map(|mode| {
                    crate::framework::demo2::domain::stowage::service::pipeline_list_generator::pipeline_steps(
                        *mode,
                    )
                    .len()
                })
                .collect(),
        },
        DomainPolicySnapshot {
            domain: "airworthiness",
            step_counts: modes
                .iter()
                .map(|mode| {
                    crate::framework::demo2::domain::airworthiness::service::pipeline_list_generator::pipeline_steps(
                        *mode,
                    )
                    .len()
                })
                .collect(),
        },
        DomainPolicySnapshot {
            domain: "mac_optimization",
            step_counts: modes
                .iter()
                .map(|mode| {
                    crate::framework::demo2::domain::mac_optimization::service::pipeline_list_generator::pipeline_steps(
                        *mode,
                    )
                    .len()
                })
                .collect(),
        },
        DomainPolicySnapshot {
            domain: "loading_effectiveness",
            step_counts: modes
                .iter()
                .map(|mode| {
                    crate::framework::demo2::domain::loading_effectiveness::service::pipeline_list_generator::pipeline_steps(
                        *mode,
                    )
                    .len()
                })
                .collect(),
        },
        DomainPolicySnapshot {
            domain: "express_effectiveness",
            step_counts: modes
                .iter()
                .map(|mode| {
                    crate::framework::demo2::domain::express_effectiveness::service::pipeline_list_generator::pipeline_steps(
                        *mode,
                    )
                    .len()
                })
                .collect(),
        },
        DomainPolicySnapshot {
            domain: "soft_security",
            step_counts: modes
                .iter()
                .map(|mode| {
                    crate::framework::demo2::domain::soft_security::service::pipeline_list_generator::pipeline_steps(
                        *mode,
                    )
                    .len()
                })
                .collect(),
        },
        DomainPolicySnapshot {
            domain: "redundancy",
            step_counts: modes
                .iter()
                .map(|mode| {
                    crate::framework::demo2::domain::redundancy::service::pipeline_list_generator::pipeline_steps(
                        *mode,
                    )
                    .len()
                })
                .collect(),
        },
    ]
}

