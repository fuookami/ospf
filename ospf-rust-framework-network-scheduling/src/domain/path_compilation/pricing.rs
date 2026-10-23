//! 对偶快照的有效性与 LP 证书来源是两个不同合同。 / Dual-snapshot validity and LP-certificate provenance are distinct contracts.

use std::collections::BTreeMap;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{ModelFingerprint, linear_model_fingerprint};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_framework::model::{ColumnGenerationScope, VerifiedPricingDuals};
use crate::error::{NetworkSchedulingError, Result};
use super::aggregation::PathCompilationAggregation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PathModelState {
    fingerprint: ModelFingerprint,
    detail: String,
}

impl PathModelState {
    pub fn capture(model: &MetaModel<f64>, compiled: &LinearTriadModel) -> Result<Self> {
        let fingerprint = linear_model_fingerprint(compiled).map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
        // 后端指纹之外保留常数、权重、变量身份及实际类型；不包含求解结果。 / Also retain constants, weights, variable identities and actual types, but not solution values.
        let detail = format!("{:?}|{:?}|{:?}", model.objective_snapshot(),
            model.tokens().iter().map(|token| token.id()).collect::<Vec<_>>(), compiled.var_types);
        Ok(Self { fingerprint, detail })
    }
}

/// 当前模型、修订、请求和业务域的只读对偶；裸对偶永不升级为证书。 / Read-only duals for the current model, revision, request and domain; raw duals never become certificates.
#[derive(Debug, Clone)]
pub struct PathCompilationDuals {
    pub(super) owner: u64,
    pub(super) revision: u64,
    pub(super) epoch: u64,
    pub(super) state: PathModelState,
    pub(super) scope: ColumnGenerationScope,
    pub(super) prices: BTreeMap<String, f64>,
    pub(super) certificate: Option<VerifiedPricingDuals>,
}

impl PathCompilationDuals {
    /// 全部声明行的有符号规范对偶，包含零值 / Signed normalized duals for every declared row, including zeros.
    pub fn prices(&self) -> &BTreeMap<String, f64> { &self.prices }

    /// 本轮完整请求、分支、分组和执行器/时隙域 / Current request, branch, groups and executor/slot domain.
    pub fn scope(&self) -> &ColumnGenerationScope { &self.scope }

    /// 仅说明来源曾通过 LP 证书校验；使用前仍须验证当前模型和作用域 / Indicates LP-verified provenance only; current model and scope must still be validated before use.
    pub fn has_verified_lp_source(&self) -> bool { self.certificate.is_some() }
}

pub(super) fn validate_scope(scope: &ColumnGenerationScope, lp: &LinearTriadModel) -> Result<()> {
    let fingerprint = linear_model_fingerprint(lp).map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
    if [&scope.request, &scope.model, &scope.dual, &scope.branch].iter().any(|id| id.trim().is_empty())
        || scope.groups.iter().chain(&scope.executor_slots).any(|id| id.trim().is_empty()) || scope.model != fingerprint.value
    {
        return Err(NetworkSchedulingError::contract("定价请求域缺失或模型指纹不匹配 / missing pricing-request scope or mismatched model fingerprint"));
    }
    Ok(())
}

pub(super) fn normalize_certificate_duals<V, C>(
    aggregation: &PathCompilationAggregation<V, C>,
    model: &MetaModel<f64>,
    duals: &[f64],
) -> Result<BTreeMap<String, f64>>
where
    V: SolveValue + UnitConversionValue,
    C: Clone + PartialEq,
{
    let mut prices = aggregation.problem().rows().iter().map(|row| (row.id.clone(), 0.0)).collect::<BTreeMap<_, _>>();
    let mut offset = 0;
    for constraint in model.constraints() {
        let row = aggregation.rows.iter().find(|row| row.name == constraint.name)
            .ok_or_else(|| NetworkSchedulingError::contract("定价报告含未映射约束 / pricing report contains an unmapped constraint"))?;
        let first = *duals.get(offset).filter(|value| value.is_finite())
            .ok_or_else(|| NetworkSchedulingError::contract("缺失或非有限 LP 展开行对偶 / missing or nonfinite expanded LP row dual"))?;
        offset += 1;
        let normalized = match constraint.inequality.relation {
            ConstraintRelation::LessEqual => first,
            ConstraintRelation::GreaterEqual => -first,
            ConstraintRelation::Equal => {
                let second = *duals.get(offset).filter(|value| value.is_finite())
                    .ok_or_else(|| NetworkSchedulingError::contract("等式缺少有限反向行对偶 / equality lacks a finite reverse-row dual"))?;
                offset += 1;
                first - second
            }
        };
        let price = prices.get_mut(&row.row).ok_or_else(|| NetworkSchedulingError::contract("缺少声明定价行 / missing declared pricing row"))?;
        *price += normalized;
        if !price.is_finite() { return Err(NetworkSchedulingError::contract("聚合路径对偶溢出 / aggregated path dual overflow")); }
    }
    if offset != duals.len() { return Err(NetworkSchedulingError::contract("LP 展开行对偶数量不匹配 / expanded LP dual count mismatch")); }
    Ok(prices)
}
