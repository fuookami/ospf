//! Demo2 领域服务 / Demo2 domain service.
pub mod domain_pipeline;
pub mod policy_registry;

#[cfg(test)]
mod tests {
    use super::policy_registry::{DomainPolicySnapshot, policy_matrix_snapshot};

    #[test]
    fn pipeline_mode_matrix_snapshot() {
        let snapshots = policy_matrix_snapshot();
        let expected = vec![
            DomainPolicySnapshot {
                domain: "stowage",
                step_counts: vec![2, 1, 1],
            },
            DomainPolicySnapshot {
                domain: "airworthiness",
                step_counts: vec![4, 4, 4],
            },
            DomainPolicySnapshot {
                domain: "mac_optimization",
                step_counts: vec![1, 2, 2],
            },
            DomainPolicySnapshot {
                domain: "loading_effectiveness",
                step_counts: vec![2, 1, 2],
            },
            DomainPolicySnapshot {
                domain: "express_effectiveness",
                step_counts: vec![1, 0, 1],
            },
            DomainPolicySnapshot {
                domain: "soft_security",
                step_counts: vec![3, 2, 3],
            },
            DomainPolicySnapshot {
                domain: "redundancy",
                step_counts: vec![3, 0, 3],
            },
        ];
        assert_eq!(snapshots.len(), expected.len());
        for (snapshot, expected_snapshot) in snapshots.iter().zip(expected.iter()) {
            assert_eq!(snapshot.domain, expected_snapshot.domain);
            assert_eq!(snapshot.step_counts, expected_snapshot.step_counts);
        }
    }
}
