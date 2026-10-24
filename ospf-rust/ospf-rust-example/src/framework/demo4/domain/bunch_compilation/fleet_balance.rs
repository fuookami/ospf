//! 恢复机队终点分布、缺口目标及列生命周期组件。 / Recovery fleet terminal distribution, shortage objectives and column lifecycle component.

use std::collections::{BTreeMap, HashSet};
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
use crate::framework::demo4::domain::task::model::Aircraft;

/// 机场及飞机子型号的期望终点机队；硬下界或有界缺口惩罚。 / Desired terminal fleet by airport and minor type, with a hard minimum or bounded shortage penalty.
#[derive(Debug, Clone)]
pub struct RecoveryFleetCheckpoint {
    /// 机场业务标识，不是整束端点缓存。 / Airport business identity, not a cached bunch endpoint.
    pub airport: String,
    /// 飞机子型号标识。 / Aircraft minor-type identity.
    pub aircraft_minor_type: String,
    /// 最少期望飞机数；合法但超过可用机队的硬要求保持模型不可行。 / Minimum desired aircraft count; valid hard demands exceeding the fleet remain model infeasibility.
    pub required: u64,
    /// 每缺一架飞机的费用；None 表示硬要求。 / Cost per missing aircraft; None denotes a hard requirement.
    pub shortage_cost: Option<f64>,
}

/// 与当前飞机实际初始位置绑定的不可变机队定义。 / Immutable fleet definition bound to actual initial aircraft locations.
#[derive(Debug, Clone)]
pub struct RecoveryFleetBalance {
    checkpoints: Vec<RecoveryFleetCheckpoint>,
    origins: BTreeMap<String, (String, String)>,
    aircraft: BTreeMap<String, Aircraft>,
}

/// 独立复核后的终点机队和实际模型缺口。 / Independently checked terminal fleet and actual modeled shortage.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryFleetBalanceResult {
    /// 检查点机场。 / Checkpoint airport.
    pub airport: String,
    /// 检查点飞机子型号。 / Checkpoint aircraft minor type.
    pub aircraft_minor_type: String,
    /// 期望数量。 / Required count.
    pub required: u64,
    /// 完整调度的实际终点数量，包含未使用的飞机。 / Actual terminal count including unused aircraft.
    pub available: u64,
    /// 从实际调度计算的物理缺口。 / Physical shortage derived from the actual schedule.
    pub shortage: u64,
    /// 求解器实际选择的缺口变量，硬要求没有该变量。 / Actual solver-selected shortage variable, absent for hard requirements.
    pub modeled_shortage: Option<u64>,
    /// 实际缺口变量对应的费用，允许限时可行解存在非最小松弛。 / Cost of the actual shortage variable, permitting nonminimal slack in a time-limited feasible incumbent.
    pub penalty: f64,
}

impl RecoveryFleetBalance {
    /// 准备全部引用及有限费用，在模型发生变化前失败。 / Prepare all references and finite costs, failing before model mutation.
    pub fn new(checkpoints: Vec<RecoveryFleetCheckpoint>, domains: &[RecoveryAircraftDomain]) -> GanttResult<Self> {
        let mut origins = BTreeMap::new();
        let mut aircraft = BTreeMap::new();
        let mut airports = HashSet::new();
        let mut minor_types = HashSet::new();
        for domain in domains {
            let initial = domain.previous.as_ref().map(|activity| &activity.route().arr).unwrap_or(&domain.usability.location);
            let minor = domain.aircraft.minor_type.code.0.clone();
            if origins.insert(domain.aircraft.reg_no.0.clone(), (initial.icao.0.clone(), minor.clone())).is_some() {
                return Err(invalid("duplicate fleet aircraft"));
            }
            minor_types.insert(minor);
            aircraft.insert(domain.aircraft.reg_no.0.clone(), domain.aircraft.clone());
            airports.insert(initial.icao.0.as_str());
            for candidate in &domain.candidates {
                airports.insert(candidate.recovery.route().dep.icao.0.as_str());
                airports.insert(candidate.recovery.route().arr.icao.0.as_str());
            }
        }
        let mut keys = HashSet::new();
        for checkpoint in &checkpoints {
            if checkpoint.airport.is_empty() || checkpoint.aircraft_minor_type.is_empty()
                || !airports.contains(checkpoint.airport.as_str()) || !minor_types.contains(&checkpoint.aircraft_minor_type)
                || !keys.insert((&checkpoint.airport, &checkpoint.aircraft_minor_type)) || checkpoint.required > (1_u64 << 53)
                || checkpoint.shortage_cost.is_some_and(|cost| !cost.is_finite() || cost < 0.0 || !(cost * checkpoint.required as f64).is_finite())
            { return Err(invalid("unknown, repeated or invalid fleet checkpoint")); }
        }
        Ok(Self { checkpoints, origins, aircraft })
    }

    /// 只读期望机队，顺序定义稳定行身份。 / Read-only target fleet whose order defines stable row identity.
    pub fn checkpoints(&self) -> &[RecoveryFleetCheckpoint] { &self.checkpoints }

    /// 从原始恢复束推导期望终点；没有原始束的飞机停留在实际初始位置。 / Derive target terminals from original recoveries; aircraft without an original bunch remain at their actual initial location.
    pub fn from_original(
        domains: &[RecoveryAircraftDomain],
        originals: &[RecoveryBunchCandidate],
        shortage_cost: Option<f64>,
    ) -> GanttResult<Self> {
        if shortage_cost.is_some_and(|cost| !cost.is_finite() || cost < 0.0) { return Err(invalid("invalid original fleet shortage cost")); }
        let base = Self::new(vec![], domains)?;
        let counts = base.terminal_counts(originals)?;
        let checkpoints = counts.into_iter().map(|((airport, aircraft_minor_type), required)| RecoveryFleetCheckpoint {
            airport, aircraft_minor_type, required, shortage_cost,
        }).collect::<Vec<_>>();
        if checkpoints.iter().any(|checkpoint| checkpoint.airport.is_empty() || checkpoint.aircraft_minor_type.is_empty()
            || checkpoint.required > (1_u64 << 53)
            || shortage_cost.is_some_and(|cost| !cost.is_finite() || cost < 0.0 || !(cost * checkpoint.required as f64).is_finite()))
        { return Err(invalid("invalid original fleet target or cost")); }
        // 原始终点可能已不能恢复，仍保留为合法的缺口或硬模型不可行。 / An original terminal may no longer be recoverable; retain a valid shortage or hard model infeasibility.
        Ok(Self { checkpoints, ..base })
    }

    /// 同源完整列系数：实际终点计数减去被替换的初始位置计数。 / Shared full-column coefficient: actual terminal count minus the replaced initial-location count.
    pub fn column_coefficients(&self, candidate: &RecoveryBunchCandidate) -> GanttResult<BTreeMap<String, f64>> {
        let origin = self.origins.get(&candidate.bunch.aircraft().reg_no.0).ok_or_else(|| invalid("unknown fleet executor"))?;
        if !candidate.bunch.aircraft().same_configuration(&self.aircraft[&candidate.bunch.aircraft().reg_no.0]) {
            return Err(invalid("changed fleet executor configuration"));
        }
        let terminal = candidate.bunch.arr().map(|airport| airport.icao.0.as_str()).unwrap_or(&origin.0);
        Ok(self.checkpoints.iter().enumerate().map(|(index, checkpoint)| {
            let matches_minor = checkpoint.aircraft_minor_type == origin.1;
            (row_name(index), f64::from(matches_minor && checkpoint.airport == terminal)
                - f64::from(matches_minor && checkpoint.airport == origin.0))
        }).collect())
    }

    pub(crate) fn row_names(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.checkpoints.len()).map(row_name)
    }

    fn terminal_counts(&self, selected: &[RecoveryBunchCandidate]) -> GanttResult<BTreeMap<(String, String), u64>> {
        let mut terminals = self.origins.clone();
        let mut seen = HashSet::new();
        let mut tasks = HashSet::new();
        for candidate in selected {
            let executor = &candidate.bunch.aircraft().reg_no.0;
            if !seen.insert(executor) { return Err(invalid("multiple selected bunches for one fleet aircraft")); }
            let terminal = terminals.get_mut(executor).ok_or_else(|| invalid("unknown selected fleet aircraft"))?;
            if !candidate.bunch.aircraft().same_configuration(&self.aircraft[executor]) { return Err(invalid("changed selected fleet configuration")); }
            if candidate.bunch.task_ids().any(|task| !tasks.insert(task)) { return Err(invalid("duplicate fleet task across aircraft")); }
            if let Some(airport) = candidate.bunch.arr() { terminal.0 = airport.icao.0.clone(); }
        }
        let mut counts = BTreeMap::<_, u64>::new();
        for terminal in terminals.into_values() { *counts.entry(terminal).or_default() += 1; }
        Ok(counts)
    }

    fn initial_count(&self, checkpoint: &RecoveryFleetCheckpoint) -> f64 {
        self.origins.values().filter(|(airport, minor)| airport == &checkpoint.airport && minor == &checkpoint.aircraft_minor_type).count() as f64
    }

    pub(crate) fn evaluate(
        &self,
        selected: &[RecoveryBunchCandidate],
        modeled: &[Option<f64>],
    ) -> GanttResult<Vec<RecoveryFleetBalanceResult>> {
        if modeled.len() != self.checkpoints.len() { return Err(invalid("missing fleet slack mapping")); }
        let counts = self.terminal_counts(selected)?;
        self.checkpoints.iter().zip(modeled).map(|(checkpoint, value)| {
            let available = counts.get(&(checkpoint.airport.clone(), checkpoint.aircraft_minor_type.clone())).copied().unwrap_or(0);
            let shortage = checkpoint.required.saturating_sub(available);
            let (modeled_shortage, penalty) = match (checkpoint.shortage_cost, value) {
                (None, None) if shortage == 0 => (None, 0.0),
                (Some(cost), Some(value)) if value.is_finite() && value.round() >= 0.0 && value.round() <= checkpoint.required as f64
                    && (value - value.round()).abs() <= 1e-6 && *value + 1e-6 >= shortage as f64 =>
                {
                    let penalty = value.round() * cost;
                    if !penalty.is_finite() { return Err(invalid("non-finite fleet shortage cost")); }
                    (Some(value.round() as u64), penalty)
                }
                _ => return Err(invalid("missing, invalid or insufficient fleet shortage incumbent")),
            };
            Ok(RecoveryFleetBalanceResult { airport: checkpoint.airport.clone(), aircraft_minor_type: checkpoint.aircraft_minor_type.clone(),
                required: checkpoint.required, available, shortage, modeled_shortage, penalty })
        }).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct RecoveryFleetComponent {
    pub definition: Option<RecoveryFleetBalance>,
    group: Option<Arc<ConstraintGroup>>,
    slacks: BTreeMap<usize, usize>,
    objective_name: String,
}

impl RecoveryFleetComponent {
    pub fn refresh(
        &mut self,
        columns: &[(usize, RecoveryBunchCandidate)],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        let Some(definition) = &self.definition else { return Ok(()); };
        if definition.checkpoints.is_empty() { return Ok(()); }
        if self.group.is_none() {
            let id = next_gantt_symbol_id();
            self.group = Some(model.create_constraint_group(id, "recovery_fleet_balance").map_err(invalid)?);
            self.objective_name = format!("recovery_fleet_shortage_cost_{id}");
            for (index, checkpoint) in definition.checkpoints.iter().enumerate() {
                if checkpoint.shortage_cost.is_some() {
                    let variable = model.register_auto_variable_with_range::<UInteger>(&format!("recovery_fleet_shortage_{index}"),
                        VariableRange::bounded(0.0, checkpoint.required as f64)).map_err(invalid)?;
                    self.slacks.insert(index, variable);
                }
            }
        }
        let group = self.group.clone().ok_or_else(|| invalid("missing fleet group"))?;
        let coefficients = columns.iter().map(|(index, candidate)| Ok((*index, definition.column_coefficients(candidate)?))).collect::<GanttResult<Vec<_>>>()?;
        model.remove_constraints_by_group_id(group.id);
        let mut objective = Vec::new();
        for (index, checkpoint) in definition.checkpoints.iter().enumerate() {
            let name = row_name(index);
            let mut terms = coefficients.iter().map(|(variable, rows)| LinearMonomial::new(rows[&name], *variable)).collect::<Vec<_>>();
            if let Some(variable) = self.slacks.get(&index) {
                terms.push(LinearMonomial::new(1.0, *variable));
                objective.push(LinearMonomial::new(checkpoint.shortage_cost.ok_or_else(|| invalid("unmapped fleet shortage cost"))?, *variable));
            }
            model.add_inequality_with_metadata(LinearInequality::greater_equal(Linear::new(terms, definition.initial_count(checkpoint)), checkpoint.required as f64),
                &name, Some(group.clone()), false, 0, None).map_err(invalid)?;
        }
        if !objective.is_empty() {
            model.replace_sub_objective(&self.objective_name, SubObjective::minimize(Linear::new(objective, 0.0), &self.objective_name));
        }
        Ok(())
    }

    pub fn extract(&self, selected: &[RecoveryBunchCandidate], values: &[f64]) -> GanttResult<Vec<RecoveryFleetBalanceResult>> {
        let Some(definition) = &self.definition else { return Ok(vec![]); };
        let modeled = definition.checkpoints.iter().enumerate().map(|(index, checkpoint)| {
            if checkpoint.shortage_cost.is_some() {
                self.slacks.get(&index).and_then(|variable| values.get(*variable)).copied().map(Some).ok_or_else(|| invalid("missing fleet slack incumbent"))
            } else { Ok(None) }
        }).collect::<GanttResult<Vec<_>>>()?;
        definition.evaluate(selected, &modeled)
    }
}

fn row_name(index: usize) -> String { format!("recovery_fleet_balance_{index}") }

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复机队输入或结果 / invalid recovery fleet input or result: {error}") }
}
