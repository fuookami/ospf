//! 由模型绑定 LP 证书构造的只读定价对偶。 / Read-only pricing duals constructed from a model-bound LP certificate.

use ospf_rust_core::error::Result;
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{
    ModelFingerprint, SolveReport, linear_model_fingerprint,
    require_optimal_lp_certificate_for_linear_model,
};

/// 已独立验证的 LP 报告；裸对偶不能构造本类型。 / Independently validated LP report; raw duals cannot construct this type.
#[derive(Debug, Clone)]
pub struct VerifiedPricingDuals {
    report: SolveReport<f64>,
    fingerprint: ModelFingerprint,
}

impl VerifiedPricingDuals {
    /// 按当前实际 LP 模型复核 primal、dual、目标及报告证据。 / Verify primal, dual, objective and report evidence against the actual LP model.
    pub fn new(model: &LinearTriadModel, report: &SolveReport<f64>) -> Result<Self> {
        require_optimal_lp_certificate_for_linear_model(report, model)?;
        Ok(Self { report: report.clone(), fingerprint: linear_model_fingerprint(model)? })
    }

    /// 读取证书中的完整行对偶。 / Read complete row duals from the certificate.
    pub fn duals(&self) -> &[f64] {
        self.report.solution.as_ref()
            .and_then(|solution| solution.dual_solution.as_deref()).unwrap_or(&[])
    }

    /// 读取原始报告，不允许改变证书。 / Read the source report without mutating the certificate.
    pub fn report(&self) -> &SolveReport<f64> {
        &self.report
    }

    /// 读取已认证模型身份。 / Read the certified model fingerprint.
    pub fn fingerprint(&self) -> &ModelFingerprint {
        &self.fingerprint
    }

    /// 在使用前重新检查当前模型；修改后的模型不能重放旧报告。 / Recheck the current model before use; changed models cannot replay old reports.
    pub fn validate_for(&self, model: &LinearTriadModel) -> Result<()> {
        require_optimal_lp_certificate_for_linear_model(&self.report, model).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
    use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveProof, SolveSolution, TerminationReason};
    use ospf_rust_core::variable::{Continuous, VariableRange};

    #[test]
    fn changing_the_model_rejects_a_previously_valid_report() {
        let mut meta = MetaModel::<f64>::new("pricing-certificate");
        let x = meta.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(0.0, 10.0)).unwrap();
        meta.add_linear_constraint(&[(x, 1.0)], ConstraintRelation::LessEqual, 2.0, "capacity").unwrap();
        meta.set_linear_objective(vec![1.0], ObjectiveCategory::Maximum);
        let model = meta.try_to_linear_triad_model().unwrap();
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution { value: None, values: vec![2.0], stable_values: Default::default(),
                objective: Some(2.0), objective_value: Some(2.0), dual_solution: Some(vec![1.0]),
                quadratic_dual_solution: None, pool: Vec::new() })
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&model).unwrap()), ..Default::default() })
            .build().unwrap();
        let duals = VerifiedPricingDuals::new(&model, &report).unwrap();
        assert_eq!(duals.duals(), &[1.0]);
        let mut changed = model.clone();
        changed.c[0] = 2.0;
        assert!(duals.validate_for(&changed).is_err());
        assert!(VerifiedPricingDuals::new(&changed, &report).is_err());
    }
}
