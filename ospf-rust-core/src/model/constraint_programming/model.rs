//! CP 可变 builder / Mutable CP model builder.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ModelError, Result};
use crate::solver::StableConstraintId;
use crate::variable::VariableId;

use super::constraint::ConstraintProgrammingConstraint;
use super::domain::IntegerDomain;
use super::interval::IntervalVariable;
use super::objective::IntegerObjective;
use super::snapshot::{
    ConstraintProgrammingSnapshot, ConstraintSnapshot, IntervalSnapshot, VariableSnapshot,
};
use super::variable::IntegerVariable;

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// 约束规划约束定义 / Constraint-programming constraint definition.
#[derive(Debug, Clone)]
pub struct ConstraintDefinition {
    /// 稳定约束身份 / Stable constraint identity.
    pub id: StableConstraintId,
    /// 展示名称 / Display name.
    pub name: String,
    /// 约束组 / Constraint group.
    pub group: Option<String>,
    /// 来源元数据 / Origin metadata.
    pub origin: Option<String>,
    /// 约束 AST / Constraint AST.
    pub constraint: ConstraintProgrammingConstraint,
}

impl ConstraintDefinition {
    /// 创建默认约束定义 / Create a default constraint definition.
    pub fn new(
        id: impl Into<StableConstraintId>,
        constraint: ConstraintProgrammingConstraint,
    ) -> Self {
        let id = id.into();
        Self {
            name: id.0.clone(),
            id,
            group: None,
            origin: None,
            constraint,
        }
    }

    /// 设置名称 / Set display name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 设置约束组 / Set constraint group.
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// 设置来源 / Set origin metadata.
    pub fn with_origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }
}

/// 可变约束规划模型 / Mutable constraint-programming model.
///
/// builder 可以继续修改；solver 只能接收 [`ConstraintProgrammingSnapshot`]，从而避免
/// 求解过程中观察到 application context 的后续变更。/ The builder remains mutable, while
/// solvers receive only [`ConstraintProgrammingSnapshot`] to prevent observing later changes.
#[derive(Debug, Clone)]
pub struct ConstraintProgrammingModel {
    /// 模型名称 / Model name.
    pub name: String,
    /// 稳定身份命名空间 / Stable identity namespace.
    pub identity_namespace: String,
    /// 稳定身份 schema 版本 / Stable identity schema version.
    pub identity_schema_version: String,
    variables: Vec<(IntegerVariable, IntegerDomain)>,
    constraints: Vec<ConstraintDefinition>,
    intervals: Vec<IntervalVariable>,
    objective: Option<IntegerObjective>,
    constraint_groups: BTreeMap<u64, String>,
}

impl ConstraintProgrammingModel {
    /// 创建 CP builder / Create a CP builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            identity_namespace: "model-local".to_owned(),
            identity_schema_version: "1.0".to_owned(),
            variables: Vec::new(),
            constraints: Vec::new(),
            intervals: Vec::new(),
            objective: None,
            constraint_groups: BTreeMap::new(),
        }
    }

    /// 设置稳定身份元数据 / Set stable identity metadata.
    pub fn with_identity_metadata(
        mut self,
        namespace: impl Into<String>,
        schema_version: impl Into<String>,
    ) -> Self {
        self.identity_namespace = namespace.into();
        self.identity_schema_version = schema_version.into();
        self
    }

    /// 注册整数变量 / Register an integer variable.
    pub fn register_variable(
        &mut self,
        variable: IntegerVariable,
        domain: IntegerDomain,
    ) -> Result<()> {
        domain.validate()?;
        if self
            .variables
            .iter()
            .any(|(existing, _)| existing.stable_id == variable.stable_id)
        {
            return Err(invalid(format!(
                "duplicate stable variable ID {}",
                variable.stable_id.0
            )));
        }
        if self
            .variables
            .iter()
            .any(|(existing, _)| existing.id == variable.id)
        {
            return Err(invalid(format!(
                "duplicate local variable ID {}",
                variable.id
            )));
        }
        self.variables.push((variable, domain));
        Ok(())
    }

    /// 注册整数变量的语义别名 / Semantic alias for registering an integer variable.
    pub fn add_variable(&mut self, variable: IntegerVariable, domain: IntegerDomain) -> Result<()> {
        self.register_variable(variable, domain)
    }

    /// 注册区间变量 / Register an interval variable.
    pub fn register_interval(&mut self, interval: IntervalVariable) -> Result<()> {
        if self
            .intervals
            .iter()
            .any(|existing| existing.id == interval.id)
        {
            return Err(invalid(format!("duplicate interval ID {}", interval.id)));
        }
        self.intervals.push(interval);
        Ok(())
    }

    /// 注册约束 / Register a constraint.
    pub fn add_constraint(&mut self, definition: ConstraintDefinition) -> Result<()> {
        if self
            .constraints
            .iter()
            .any(|existing| existing.id == definition.id)
        {
            return Err(invalid(format!(
                "duplicate stable constraint ID {}",
                definition.id.0
            )));
        }
        self.constraints.push(definition);
        Ok(())
    }

    /// 以稳定 ID 注册约束 / Register a constraint by stable ID.
    pub fn add_constraint_by_id(
        &mut self,
        id: impl Into<StableConstraintId>,
        constraint: ConstraintProgrammingConstraint,
    ) -> Result<()> {
        self.add_constraint(ConstraintDefinition::new(id, constraint))
    }

    /// 设置整数目标 / Set the single integer objective.
    pub fn set_objective(&mut self, objective: IntegerObjective) {
        self.objective = Some(objective);
    }

    /// 清除目标，切换到 satisfaction / Clear the objective and use satisfaction mode.
    pub fn clear_objective(&mut self) {
        self.objective = None;
    }

    /// 返回已注册变量数量 / Return registered variable count.
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// 返回已注册约束数量 / Return registered constraint count.
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// 注册约束组元数据 / Register constraint-group metadata.
    pub fn ensure_constraint_group(&mut self, id: u64, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(invalid("CP constraint-group name must not be blank"));
        }
        if let Some(existing) = self.constraint_groups.get(&id)
            && existing != &name
        {
            return Err(invalid(format!(
                "CP constraint-group {id} is already bound to a different name"
            )));
        }
        self.constraint_groups.insert(id, name);
        Ok(())
    }

    /// 校验并冻结模型 / Validate and freeze the model.
    pub fn freeze(&self) -> Result<ConstraintProgrammingSnapshot> {
        if self.identity_namespace.trim().is_empty() {
            return Err(invalid("CP identity namespace must not be blank"));
        }
        if self.identity_schema_version.trim().is_empty() {
            return Err(invalid("CP identity schema version must not be blank"));
        }

        let mut domains = BTreeMap::new();
        let mut variable_bindings = BTreeMap::new();
        let mut variables = Vec::with_capacity(self.variables.len());
        for (variable, domain) in &self.variables {
            domain.validate()?;
            if domains
                .insert(variable.stable_id.clone(), domain.clone())
                .is_some()
            {
                return Err(invalid(format!(
                    "duplicate stable variable ID {}",
                    variable.stable_id.0
                )));
            }
            variable_bindings.insert(variable.stable_id.clone(), variable.clone());
            variables.push(VariableSnapshot {
                variable: variable.clone(),
                domain: domain.clone(),
            });
        }
        variables.sort_by(|left, right| left.variable.stable_id.cmp(&right.variable.stable_id));

        let mut canonical_bindings = BTreeMap::new();
        for (index, entry) in variables.iter_mut().enumerate() {
            let variable = IntegerVariable::with_ids(
                VariableId::standalone(index),
                entry.variable.stable_id.clone(),
            );
            canonical_bindings.insert(variable.stable_id.clone(), variable.clone());
            entry.variable = variable;
        }

        let interval_ids = self
            .intervals
            .iter()
            .map(|interval| interval.id.clone())
            .collect::<BTreeSet<_>>();
        if interval_ids.len() != self.intervals.len() {
            return Err(invalid("duplicate interval ID"));
        }
        for interval in &self.intervals {
            interval.validate_structure(&domains, &variable_bindings)?;
        }
        let mut intervals = self
            .intervals
            .iter()
            .cloned()
            .map(|interval| IntervalSnapshot { interval })
            .collect::<Vec<_>>();
        for interval in &mut intervals {
            interval.interval = interval.interval.rebind_variables(&canonical_bindings)?;
        }
        intervals.sort_by(|left, right| left.interval.id.cmp(&right.interval.id));

        let interval_id_set = interval_ids;
        let mut constraints = Vec::with_capacity(self.constraints.len());
        let mut constraint_ids = BTreeSet::new();
        for definition in &self.constraints {
            if !constraint_ids.insert(definition.id.clone()) {
                return Err(invalid(format!(
                    "duplicate stable constraint ID {}",
                    definition.id.0
                )));
            }
            definition
                .constraint
                .validate(&domains, &variable_bindings, &interval_id_set)?;
            constraints.push(ConstraintSnapshot {
                id: definition.id.clone(),
                name: definition.name.clone(),
                group: definition.group.clone(),
                origin: definition.origin.clone(),
                constraint: definition
                    .constraint
                    .rebind_variables(&canonical_bindings)?
                    .canonicalize(),
            });
        }
        constraints.sort_by(|left, right| left.id.cmp(&right.id));

        if let Some(objective) = &self.objective {
            objective.expression.validate_bindings(&variable_bindings)?;
            for variable in objective.expression.referenced_variables() {
                if !domains.contains_key(&variable) {
                    return Err(invalid(format!(
                        "objective references unknown variable {}",
                        variable.0
                    )));
                }
            }
        }

        let objective = self
            .objective
            .as_ref()
            .map(|objective| objective.rebind_variables(&canonical_bindings))
            .transpose()?;

        let mut snapshot = ConstraintProgrammingSnapshot {
            name: self.name.clone(),
            identity_namespace: self.identity_namespace.clone(),
            identity_schema_version: self.identity_schema_version.clone(),
            variables,
            intervals,
            constraints,
            objective,
            constraint_groups: self.constraint_groups.clone(),
            fingerprint: crate::solver::sha256_fingerprint(
                "ospf.constraint-programming.snapshot",
                &[],
            ),
        };
        snapshot.fingerprint = snapshot.compute_fingerprint();
        snapshot.validate_identity()?;
        Ok(snapshot)
    }
}
