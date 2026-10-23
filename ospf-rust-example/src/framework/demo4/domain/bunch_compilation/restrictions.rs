//! 限制使用量行、费用及独立提取；与 Gantt 共享列事务。 / Restriction usage rows, costs and independent extraction sharing Gantt column transactions.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint::LinearInequality;
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{UInteger, VariableRange};
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_gantt_scheduling::domain::task_compilation::adapter::next_gantt_symbol_id;
use crate::framework::demo4::domain::bunch_generation::problem::RecoveryAircraftDomain;
use crate::framework::demo4::domain::bunch_generation::recovery::RecoveryBunchCandidate;
use crate::framework::demo4::domain::rule::model::Restriction;
use crate::framework::demo4::domain::rule::model::restriction::RestrictionReferences;
use crate::framework::demo4::domain::task::model::FlightTaskImpl;

/// 不可变限制目录，可共享但不持有注册状态。 / Immutable restriction directory, shareable without registration state.
#[derive(Debug, Clone, Default)]
pub struct RecoveryRestrictions {
    restrictions: Vec<Restriction>,
    maximum_violations: usize,
}

/// 使用实际完整记录复核的限制结果。 / Restriction result independently checked against complete actual records.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryRestrictionResult {
    /// 限制稳定身份。 / Stable restriction identity.
    pub restriction: String,
    /// 实际违反的原任务标识，按标识排序。 / Original identities of actual violating tasks, sorted by identity.
    pub violated_tasks: Vec<String>,
    /// 真实违反次数乘声明权重，不等于伪造的费用。 / Actual violation count times declared weight, not an invented monetary cost.
    pub weighted_violations: f64,
    /// 真实违反次数乘权重及已配置费用；未定价弱限制为零。 / Actual count times weight and configured cost; unpriced weak restrictions cost zero.
    pub penalty: f64,
}

impl RecoveryRestrictions {
    pub(crate) fn new(
        restrictions: Vec<Restriction>,
        tasks: &[FlightTaskImpl],
        domains: &[RecoveryAircraftDomain],
    ) -> GanttResult<Self> {
        let mut airports = tasks.iter().flat_map(|task| [task.dep().icao.0.as_str(), task.arr().icao.0.as_str()]).collect::<HashSet<_>>();
        for domain in domains {
            airports.insert(domain.usability.location.icao.0.as_str());
            for recovery in domain.previous.iter().chain(domain.candidates.iter().map(|candidate| &candidate.recovery)) {
                airports.extend([recovery.route().dep.icao.0.as_str(), recovery.route().arr.icao.0.as_str()]);
            }
        }
        let known_tasks = tasks.iter().map(|task| task.plan.id.as_str())
            .chain(domains.iter().filter_map(|domain| domain.previous.as_ref().map(|previous| previous.original().plan.id.as_str()))).collect();
        let references = RestrictionReferences { tasks: known_tasks,
            aircraft: domains.iter().map(|domain| domain.aircraft.reg_no.0.as_str()).collect(), airports };
        let mut ids = BTreeSet::new();
        for restriction in &restrictions {
            restriction.validate(&references).map_err(invalid)?;
            if !ids.insert(&restriction.id) || tasks.len() as u128 > (1_u128 << 53)
                || !(tasks.len() as f64 * restriction.weight).is_finite()
                || !(tasks.len() as f64 * restriction.weight * restriction.cost.unwrap_or(0.0)).is_finite()
            { return Err(invalid("repeated restriction or non-finite bounded restriction cost")); }
        }
        Ok(Self { restrictions, maximum_violations: tasks.len() })
    }

    /// 只读已验证定义。 / Read-only validated definitions.
    pub fn definitions(&self) -> &[Restriction] { &self.restrictions }

    /// 完整列的限制行系数，显式保留零；硬行使用量为零，软行由整数计数平衡。 / Full-column restriction coefficients retaining explicit zeros; hard usage must be zero and soft usage is balanced by integer counts.
    pub fn column_coefficients(&self, candidate: &RecoveryBunchCandidate) -> BTreeMap<String, f64> {
        self.restrictions.iter().enumerate().map(|(index, restriction)| {
            let count = candidate.bunch.activities().iter().filter(|activity| restriction.check(activity).violated()).count();
            (row_name(index), -(count as f64))
        }).collect()
    }

    pub(crate) fn row_names(&self) -> impl Iterator<Item = String> + '_ { (0..self.restrictions.len()).map(row_name) }

    fn evaluate(&self, selected: &[RecoveryBunchCandidate], counts: &[Option<f64>]) -> GanttResult<Vec<RecoveryRestrictionResult>> {
        if counts.len() != self.restrictions.len() { return Err(invalid("missing restriction count mapping")); }
        self.restrictions.iter().zip(counts).map(|(restriction, count)| {
            let violated_tasks = selected.iter().flat_map(|candidate| candidate.bunch.activities()).filter(|activity| restriction.check(activity).violated())
                .map(|activity| activity.original().plan.id.clone()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
            let expected = violated_tasks.len() as f64;
            match (restriction.is_hard(), count) {
                (true, None) if expected == 0.0 => {},
                (false, Some(count)) if count.is_finite() && *count >= -1e-6 && *count <= self.maximum_violations as f64 + 1e-6
                    && (*count - expected).abs() <= 1e-6 => {},
                _ => return Err(invalid("hard restriction violation or illegal restriction-count incumbent")),
            }
            let weighted_violations = expected * restriction.weight;
            let penalty = weighted_violations * restriction.cost.unwrap_or(0.0);
            if !weighted_violations.is_finite() || !penalty.is_finite() { return Err(invalid("non-finite extracted restriction cost")); }
            Ok(RecoveryRestrictionResult { restriction: restriction.id.clone(), violated_tasks, weighted_violations, penalty })
        }).collect()
    }

    pub(crate) fn validate_results(&self, selected: &[RecoveryBunchCandidate], results: &[RecoveryRestrictionResult]) -> GanttResult<()> {
        if results.len() != self.restrictions.len() { return Err(invalid("missing restriction results")); }
        let counts = self.restrictions.iter().zip(results).map(|(restriction, result)| {
            if restriction.is_hard() { None } else { Some(result.violated_tasks.len() as f64) }
        }).collect::<Vec<_>>();
        if self.evaluate(selected, &counts)? != results { return Err(invalid("restriction results differ from full actual recoveries")); }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct RecoveryRestrictionComponent {
    pub definition: Option<RecoveryRestrictions>,
    group: Option<Arc<ConstraintGroup>>,
    counts: BTreeMap<usize, usize>,
    objective_name: String,
}

impl RecoveryRestrictionComponent {
    pub fn refresh(&mut self, columns: &[(usize, RecoveryBunchCandidate)], model: &mut MetaModel<f64>) -> GanttResult<()> {
        let Some(definition) = &self.definition else { return Ok(()); };
        if definition.restrictions.is_empty() { return Ok(()); }
        if self.group.is_none() {
            let id = next_gantt_symbol_id();
            self.group = Some(model.create_constraint_group(id, "recovery_restrictions").map_err(invalid)?);
            self.objective_name = format!("recovery_restriction_cost_{id}");
            for (index, restriction) in definition.restrictions.iter().enumerate() {
                if !restriction.is_hard() {
                    let variable = model.register_auto_variable_with_range::<UInteger>(&format!("recovery_restriction_count_{index}"),
                        VariableRange::bounded(0.0, definition.maximum_violations as f64)).map_err(invalid)?;
                    self.counts.insert(index, variable);
                }
            }
        }
        let group = self.group.clone().ok_or_else(|| invalid("missing restriction group"))?;
        model.remove_constraints_by_group_id(group.id);
        let coefficients = columns.iter().map(|(variable, candidate)| (*variable, definition.column_coefficients(candidate))).collect::<Vec<_>>();
        let mut objective = Vec::new();
        for (index, restriction) in definition.restrictions.iter().enumerate() {
            let name = row_name(index);
            let mut terms = coefficients.iter().map(|(variable, rows)| LinearMonomial::new(rows[&name], *variable)).collect::<Vec<_>>();
            if let Some(variable) = self.counts.get(&index) {
                terms.push(LinearMonomial::new(1.0, *variable));
                objective.push(LinearMonomial::new(restriction.weight * restriction.cost.unwrap_or(0.0), *variable));
            }
            model.add_inequality_with_metadata(LinearInequality::equal(Linear::new(terms, 0.0), 0.0),
                &name, Some(group.clone()), false, 0, None).map_err(invalid)?;
        }
        model.replace_sub_objective(&self.objective_name, SubObjective::minimize(Linear::new(objective, 0.0), &self.objective_name));
        Ok(())
    }

    pub fn extract(&self, selected: &[RecoveryBunchCandidate], values: &[f64]) -> GanttResult<Vec<RecoveryRestrictionResult>> {
        let Some(definition) = &self.definition else { return Ok(vec![]); };
        let counts = definition.restrictions.iter().enumerate().map(|(index, restriction)| {
            if restriction.is_hard() { Ok(None) } else {
                self.counts.get(&index).and_then(|variable| values.get(*variable)).copied().map(Some).ok_or_else(|| invalid("missing restriction-count incumbent"))
            }
        }).collect::<GanttResult<Vec<_>>>()?;
        definition.evaluate(selected, &counts)
    }
}

fn row_name(index: usize) -> String { format!("recovery_restriction_{index}") }

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复限制输入或结果 / invalid recovery restriction input or result: {error}") }
}
