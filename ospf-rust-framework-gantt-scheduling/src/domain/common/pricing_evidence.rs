//! 当前模型绑定的 LP 定价证据与原行归一化。 / Current-model LP pricing evidence and original-row normalization.

use std::collections::BTreeMap;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::SolveReport;
use ospf_rust_framework::model::VerifiedPricingDuals;
use crate::{GanttError, GanttResult};

#[derive(Debug, Clone)]
pub(crate) struct PricingEvidence {
    pub model_identity: u64,
    pub certificate: VerifiedPricingDuals,
    pub row_duals: BTreeMap<String, f64>,
    pub context_revision: Option<String>,
}

impl PricingEvidence {
    pub fn new(model: &MetaModel<f64>, report: &SolveReport<f64>) -> GanttResult<Self> {
        let mut current = model.try_to_linear_triad_model().map_err(calculation)?;
        current.linear_relax();
        let certificate = VerifiedPricingDuals::new(&current, report).map_err(calculation)?;
        let mut row_duals = BTreeMap::new();
        let mut index = 0;
        for row in model.constraints() {
            let first = certificate.duals().get(index).copied().ok_or_else(|| calculation("缺少展开行对偶 / missing expanded row dual"))?;
            index += 1;
            let price = match row.inequality.relation {
                ConstraintRelation::LessEqual => first,
                ConstraintRelation::GreaterEqual => -first,
                ConstraintRelation::Equal => {
                    let second = certificate.duals().get(index).copied().ok_or_else(|| calculation("等式缺少反向对偶 / missing reverse equality dual"))?;
                    index += 1;
                    first - second
                }
            };
            if !price.is_finite() || row_duals.insert(row.name.clone(), price).is_some() {
                return Err(calculation("对偶非有限或原行名称重复 / non-finite dual or duplicate original row name"));
            }
        }
        if index != certificate.duals().len() {
            return Err(calculation("隐藏编译行没有 Gantt 定价映射 / hidden compiled rows lack Gantt-pricing mappings"));
        }
        Ok(Self { model_identity: model.model_identity(), certificate, row_duals, context_revision: None })
    }

    pub fn validate_for(&self, model: &MetaModel<f64>) -> GanttResult<()> {
        if self.model_identity != model.model_identity() {
            return Err(calculation("Gantt LP 证书不能跨模型重放 / Gantt LP certificates cannot be replayed across models"));
        }
        let mut current = model.try_to_linear_triad_model().map_err(calculation)?;
        current.linear_relax();
        self.certificate.validate_for(&current).map_err(calculation)
    }

    pub fn lower_bound(&self, model: &MetaModel<f64>) -> GanttResult<Option<f64>> {
        self.validate_for(model)?;
        let current = model.try_to_linear_triad_model().map_err(calculation)?;
        if !current.objective_category.is_minimum() { return Ok(None); }
        let solution = self.certificate.report().solution.as_ref().ok_or_else(|| calculation("缺少证书 primal / missing certified primal"))?;
        let value = current.c.iter().zip(&solution.values).map(|(cost, value)| cost * value).sum::<f64>();
        if !value.is_finite() { return Err(calculation("认证目标非有限 / non-finite certified objective")); }
        Ok(Some(value))
    }
}

fn calculation(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: error.to_string() }
}
