//! 策略注册表 / Policy registry
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

/// 领域策略快照 / Domain policy snapshot
///
/// 记录某个领域在各管线模式下的步骤数量。
/// Records the step count of a domain under each pipeline mode.
#[allow(dead_code)]
pub struct DomainPolicySnapshot {
    /// 领域名称 / Domain name
    pub domain: &'static str,
    /// 各模式下的步骤数量 / Step counts under each mode
    pub step_counts: Vec<usize>,
}

/// 管线模式顺序 / Pipeline mode order
///
/// 返回三种管线模式的固定顺序：满载、预分配、重量推荐。
/// Returns the fixed order of three pipeline modes: full load, predistribution, weight recommendation.
#[allow(dead_code)]
pub fn mode_order() -> [Demo2PipelineMode; 3] {
    [
        Demo2PipelineMode::FullLoad,
        Demo2PipelineMode::Predistribution,
        Demo2PipelineMode::WeightRecommendation,
    ]
}

/// 生成策略矩阵快照 / Generate policy matrix snapshot
///
/// 收集所有领域在各管线模式下的步骤数量，用于测试和验证。
/// Collects step counts of all domains under each pipeline mode for testing and verification.
#[allow(dead_code)]
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
                    crate::framework::demo2::domain::airworthiness_security::service::pipeline_list_generator::pipeline_steps(
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
