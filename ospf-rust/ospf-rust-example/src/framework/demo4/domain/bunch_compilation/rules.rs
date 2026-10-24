//! 硬旅客规则的有限活动域线性化，同源系数供注册和定价使用。 / Finite-activity linearization of hard passenger rules with shared registration and pricing coefficients.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint::{ConstraintRelation, LinearInequality};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_gantt_scheduling::domain::task_compilation::adapter::next_gantt_symbol_id;
use crate::framework::demo4::domain::bunch_generation::recovery::RecoveryBunchCandidate;
use crate::framework::demo4::domain::passenger::model::Passenger;
use crate::framework::demo4::domain::task::model::{AircraftCapacity, FlightTaskRecovery};
use super::context::same_recovery;

#[derive(Debug, Clone)]
enum Usage {
    Cover(String),
    Capacity { task: String, class: String, amount: u64 },
    Incompatible(Box<(FlightTaskRecovery, FlightTaskRecovery)>),
}

/// 固定业务行身份及其列系数；不包含任何模型注册态。 / Fixed business row identity and column coefficients without model registration state.
#[derive(Debug, Clone)]
pub struct RecoveryRuleRow {
    name: String,
    usage: Usage,
    relation: ConstraintRelation,
    bound: f64,
}

impl RecoveryRuleRow {
    /// 稳定原始行名，对偶必须显式包含零。 / Stable original row name; dual maps must explicitly include zero.
    pub fn name(&self) -> &str { &self.name }

    /// 完整候选的可加行系数，与动态主问题注册严格共用。 / Additive full-candidate row coefficient shared exactly with dynamic master registration.
    pub fn coefficient(&self, candidate: &RecoveryBunchCandidate) -> f64 {
        match &self.usage {
            Usage::Cover(task) => f64::from(candidate.bunch.recovery(task).is_some()),
            Usage::Capacity { task, class, amount } => candidate.bunch.recovery(task).map_or(0.0, |activity| {
                let capacity = match &activity.aircraft().capacity {
                    AircraftCapacity::Passenger { capacities } => capacities.iter().filter(|(key, _)| &key.0 == class).map(|(_, value)| *value as f64).sum::<f64>(),
                    AircraftCapacity::Cargo { .. } => 0.0,
                };
                capacity - *amount as f64
            }),
            Usage::Incompatible(pair) => candidate.bunch.activities().iter().map(|activity| {
                f64::from(same_recovery(activity, &pair.0)) + f64::from(same_recovery(activity, &pair.1))
            }).sum(),
        }
    }
}

pub(crate) fn passenger_rows(
    tasks: &HashSet<&str>,
    variants: &[FlightTaskRecovery],
    passengers: &[Passenger],
) -> GanttResult<Vec<RecoveryRuleRow>> {
    let mut ids = HashSet::new();
    let mut demand = BTreeMap::<(String, String), u64>::new();
    let mut pairs = HashSet::new();
    for passenger in passengers {
        if passenger.id.is_empty() || !ids.insert(&passenger.id) || passenger.flights.is_empty() {
            return Err(invalid("empty, repeated or flightless passenger"));
        }
        let mut visited = HashSet::new();
        for (task, class) in &passenger.flights {
            if !tasks.contains(task.as_str()) || class.0.is_empty() || !visited.insert(task) {
                return Err(invalid("unknown or repeated passenger task or empty class"));
            }
            let amount = demand.entry((task.clone(), class.0.clone())).or_default();
            *amount = amount.checked_add(passenger.amount).ok_or_else(|| invalid("passenger demand overflow"))?;
            if *amount > (1_u64 << 53) { return Err(invalid("passenger demand not exactly representable by solver")); }
        }
        for pair in passenger.flights.windows(2) { pairs.insert((pair[0].0.clone(), pair[1].0.clone())); }
    }
    for activity in variants {
        if activity.route().arr.passenger_transfer_time < time::Duration::ZERO {
            return Err(invalid("negative passenger transfer duration"));
        }
        if let AircraftCapacity::Passenger { capacities } = &activity.aircraft().capacity {
            let mut totals = BTreeMap::<&str, u64>::new();
            for (class, value) in capacities {
                let total = totals.entry(&class.0).or_default();
                *total = total.checked_add(*value).ok_or_else(|| invalid("seat capacity overflow"))?;
                if *total > (1_u64 << 53) { return Err(invalid("seat capacity not exactly representable by solver")); }
            }
        }
    }
    let covered = demand.keys().map(|(task, _)| task.clone()).collect::<std::collections::BTreeSet<_>>();
    let mut rows = covered.into_iter().enumerate().map(|(index, task)| RecoveryRuleRow {
        name: format!("recovery_passenger_cover_{index}"), usage: Usage::Cover(task), relation: ConstraintRelation::GreaterEqual, bound: 1.0,
    }).collect::<Vec<_>>();
    rows.extend(demand.into_iter().enumerate().map(|(index, ((task, class), amount))| RecoveryRuleRow {
        name: format!("recovery_passenger_capacity_{index}"), usage: Usage::Capacity { task, class, amount }, relation: ConstraintRelation::GreaterEqual, bound: 0.0,
    }));
    // 按稳定活动域顺序生成冲突行，不枚举路线；一行禁止同时选择不兼容的两个活动版本。 / Generate conflict rows in stable activity-domain order without route enumeration; each row forbids choosing two incompatible variants together.
    for (left_index, left) in variants.iter().enumerate() {
        for (right_index, right) in variants.iter().enumerate() {
            if pairs.contains(&(left.original().plan.id.clone(), right.original().plan.id.clone()))
                && (left.route().arr != right.route().dep || right.time().0 - left.time().1 < left.route().arr.passenger_transfer_time)
            {
                rows.push(RecoveryRuleRow { name: format!("recovery_passenger_transfer_{left_index}_{right_index}"),
                    usage: Usage::Incompatible(Box::new((left.clone(), right.clone()))), relation: ConstraintRelation::LessEqual, bound: 1.0 });
            }
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone, Default)]
pub(super) struct RecoveryRuleComponent {
    pub rows: Vec<RecoveryRuleRow>,
    group: Option<Arc<ConstraintGroup>>,
}

impl RecoveryRuleComponent {
    pub fn refresh(
        &mut self,
        columns: &[(usize, RecoveryBunchCandidate)],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        if self.rows.is_empty() { return Ok(()); }
        if self.group.is_none() {
            let id = next_gantt_symbol_id();
            self.group = Some(model.create_constraint_group(id, "recovery_passenger_rules").map_err(invalid)?);
        }
        let group = self.group.clone().ok_or_else(|| invalid("missing passenger rule group"))?;
        model.remove_constraints_by_group_id(group.id);
        for row in &self.rows {
            let terms = columns.iter().map(|(variable, candidate)| LinearMonomial::new(row.coefficient(candidate), *variable)).collect();
            model.add_inequality_with_metadata(LinearInequality::new(Linear::new(terms, 0.0), row.relation, row.bound),
                &row.name, Some(group.clone()), false, 0, None).map_err(invalid)?;
        }
        Ok(())
    }
}

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复旅客规则 / invalid recovery passenger rule: {error}") }
}
