//! CP 不可变快照 / Immutable CP snapshot.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ModelError, Result};
use crate::solver::{AuditFingerprint, StableConstraintId, StableVariableId, sha256_fingerprint};
use crate::variable::VariableId;

use super::constraint::ConstraintProgrammingConstraint;
use super::domain::IntegerDomain;
use super::expression::append_string;
use super::interval::{IntervalValue, IntervalVariable};
use super::objective::IntegerObjective;
use super::variable::IntegerVariable;

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// CP 变量快照 / CP variable snapshot.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSnapshot {
    /// 变量身份 / Variable identity.
    pub variable: IntegerVariable,
    /// 变量值域 / Variable domain.
    pub domain: IntegerDomain,
}

/// CP 区间快照 / CP interval snapshot.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalSnapshot {
    /// 区间定义 / Interval definition.
    pub interval: IntervalVariable,
}

/// CP 约束快照 / CP constraint snapshot.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintSnapshot {
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

/// 不可变 CP 模型快照 / Immutable constraint-programming model snapshot.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintProgrammingSnapshot {
    /// 模型名称 / Model name.
    pub name: String,
    /// 稳定身份命名空间 / Stable identity namespace.
    pub identity_namespace: String,
    /// 稳定身份 schema 版本 / Stable identity schema version.
    pub identity_schema_version: String,
    /// 规范化变量 / Canonical variables.
    pub variables: Vec<VariableSnapshot>,
    /// 规范化区间 / Canonical intervals.
    pub intervals: Vec<IntervalSnapshot>,
    /// 规范化约束 / Canonical constraints.
    pub constraints: Vec<ConstraintSnapshot>,
    /// 可选整数目标 / Optional integer objective.
    pub objective: Option<IntegerObjective>,
    /// 约束组元数据 / Constraint-group metadata.
    #[cfg_attr(feature = "serde", serde(default))]
    pub constraint_groups: BTreeMap<u64, String>,
    /// 模型审计指纹 / Model audit fingerprint.
    pub fingerprint: AuditFingerprint,
}

impl ConstraintProgrammingSnapshot {
    /// 按稳定变量 ID 查找变量 / Find a variable by stable ID.
    pub fn variable(&self, id: &StableVariableId) -> Option<&VariableSnapshot> {
        self.variables
            .iter()
            .find(|variable| variable.variable.stable_id == *id)
    }

    /// 按稳定约束 ID 查找约束 / Find a constraint by stable ID.
    pub fn constraint(&self, id: &StableConstraintId) -> Option<&ConstraintSnapshot> {
        self.constraints
            .iter()
            .find(|constraint| constraint.id == *id)
    }

    /// 返回模型指纹 / Return the model fingerprint.
    pub fn model_fingerprint(&self) -> &AuditFingerprint {
        &self.fingerprint
    }

    /// 校验稳定身份和指纹 / Validate stable identities and fingerprint.
    pub fn validate_identity(&self) -> Result<()> {
        if self.identity_namespace.trim().is_empty() {
            return Err(invalid("CP identity namespace must not be blank"));
        }
        if self.identity_schema_version.trim().is_empty() {
            return Err(invalid("CP identity schema version must not be blank"));
        }
        let canonical = self.canonicalized();
        if self.variables != canonical.variables
            || self.intervals != canonical.intervals
            || self.constraints != canonical.constraints
        {
            return Err(invalid(
                "CP snapshot contains non-canonical collection order",
            ));
        }
        let mut variables = BTreeSet::new();
        let mut local_ids = BTreeSet::new();
        let mut domains = BTreeMap::new();
        let mut variable_bindings = BTreeMap::new();
        if self
            .variables
            .windows(2)
            .any(|pair| pair[0].variable.stable_id > pair[1].variable.stable_id)
        {
            return Err(invalid("CP variables are not in canonical stable-ID order"));
        }
        for entry in &self.variables {
            entry.domain.validate()?;
            if !variables.insert(entry.variable.stable_id.clone()) {
                return Err(invalid(format!(
                    "duplicate stable variable ID {}",
                    entry.variable.stable_id.0
                )));
            }
            if !local_ids.insert(entry.variable.id) {
                return Err(invalid(format!(
                    "duplicate local variable ID {}",
                    entry.variable.id
                )));
            }
            domains.insert(entry.variable.stable_id.clone(), entry.domain.clone());
            variable_bindings.insert(entry.variable.stable_id.clone(), entry.variable.clone());
        }
        let mut constraints = BTreeSet::new();
        if self
            .constraints
            .windows(2)
            .any(|pair| pair[0].id > pair[1].id)
        {
            return Err(invalid(
                "CP constraints are not in canonical stable-ID order",
            ));
        }
        for entry in &self.constraints {
            if !constraints.insert(entry.id.clone()) {
                return Err(invalid(format!(
                    "duplicate stable constraint ID {}",
                    entry.id.0
                )));
            }
            entry
                .constraint
                .validate(&domains, &variable_bindings, &self.interval_ids())?;
        }
        let mut intervals = BTreeSet::new();
        if self
            .intervals
            .windows(2)
            .any(|pair| pair[0].interval.id > pair[1].interval.id)
        {
            return Err(invalid("CP intervals are not in canonical ID order"));
        }
        for entry in &self.intervals {
            if !intervals.insert(entry.interval.id.clone()) {
                return Err(invalid(format!(
                    "duplicate interval ID {}",
                    entry.interval.id
                )));
            }
            entry
                .interval
                .validate_structure(&domains, &variable_bindings)?;
        }
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
        for name in self.constraint_groups.values() {
            if name.trim().is_empty() {
                return Err(invalid("CP constraint-group name must not be blank"));
            }
        }
        if self.fingerprint != self.compute_fingerprint() {
            return Err(invalid(
                "CP snapshot fingerprint does not match canonical content",
            ));
        }
        Ok(())
    }

    fn interval_ids(&self) -> BTreeSet<super::variable::IntervalVariableId> {
        self.intervals
            .iter()
            .map(|interval| interval.interval.id.clone())
            .collect()
    }

    fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical
            .variables
            .sort_by(|left, right| left.variable.stable_id.cmp(&right.variable.stable_id));
        canonical
            .intervals
            .sort_by(|left, right| left.interval.id.cmp(&right.interval.id));
        for constraint in &mut canonical.constraints {
            constraint.constraint = constraint.constraint.canonicalize();
        }
        canonical
            .constraints
            .sort_by(|left, right| left.id.cmp(&right.id));
        let mut bindings = BTreeMap::new();
        for (index, entry) in canonical.variables.iter_mut().enumerate() {
            let variable = IntegerVariable::with_ids(
                VariableId::standalone(index),
                entry.variable.stable_id.clone(),
            );
            bindings.insert(variable.stable_id.clone(), variable.clone());
            entry.variable = variable;
        }
        for interval in &mut canonical.intervals {
            let rebound = match interval.interval.rebind_variables(&bindings) {
                Ok(rebound) => rebound,
                Err(_) => return canonical,
            };
            interval.interval = rebound;
        }
        for constraint in &mut canonical.constraints {
            let rebound = match constraint.constraint.rebind_variables(&bindings) {
                Ok(rebound) => rebound.canonicalize(),
                Err(_) => return canonical,
            };
            constraint.constraint = rebound;
        }
        if let Some(objective) = canonical.objective.as_ref() {
            let rebound = match objective.rebind_variables(&bindings) {
                Ok(rebound) => rebound,
                Err(_) => return canonical,
            };
            canonical.objective = Some(rebound);
        }
        canonical.fingerprint = canonical.compute_fingerprint();
        canonical
    }

    /// 校验一个候选赋值并返回区间求值 / Validate a candidate assignment and evaluate intervals.
    pub fn validate_assignment(
        &self,
        values: &BTreeMap<StableVariableId, i64>,
    ) -> Result<BTreeMap<super::variable::IntervalVariableId, IntervalValue>> {
        if values.len() != self.variables.len()
            || self
                .variables
                .iter()
                .any(|variable| !values.contains_key(&variable.variable.stable_id))
        {
            return Err(invalid(
                "CP assignment does not cover every registered variable",
            ));
        }
        for variable in &self.variables {
            let value = values
                .get(&variable.variable.stable_id)
                .ok_or_else(|| invalid("CP assignment is missing a variable"))?;
            if !variable.domain.contains(*value) {
                return Err(invalid(format!(
                    "assignment {}={} is outside its domain",
                    variable.variable.stable_id.0, value
                )));
            }
        }
        let mut interval_values = BTreeMap::new();
        for interval in &self.intervals {
            if let Some(value) = interval.interval.evaluate(values)? {
                interval_values.insert(interval.interval.id.clone(), value);
            }
        }
        for constraint in &self.constraints {
            if !constraint.constraint.evaluate(values, &interval_values)? {
                return Err(invalid(format!(
                    "assignment violates constraint {}",
                    constraint.id.0
                )));
            }
        }
        Ok(interval_values)
    }

    /// 求值目标 / Evaluate the objective.
    pub fn objective_value(&self, values: &BTreeMap<StableVariableId, i64>) -> Result<Option<i64>> {
        self.objective
            .as_ref()
            .map(|objective| objective.expression.evaluate(values))
            .transpose()
    }

    /// 返回规范化表达式引用的稳定变量 / Return stable variables referenced by the objective.
    pub fn objective_variables(&self) -> BTreeSet<StableVariableId> {
        self.objective
            .as_ref()
            .map(|objective| objective.expression.referenced_variables())
            .unwrap_or_default()
    }

    /// 校验可作为提示或恢复 incumbent 的部分赋值 / Validate a partial assignment suitable for a hint or restored incumbent.
    pub fn validate_hint(&self, values: &BTreeMap<StableVariableId, i64>) -> Result<()> {
        for (id, value) in values {
            let variable = self.variable(id).ok_or_else(|| {
                invalid(format!("assignment references unknown variable {}", id.0))
            })?;
            if !variable.domain.contains(*value) {
                return Err(invalid(format!(
                    "assignment {}={} is outside the variable domain",
                    id.0, value
                )));
            }
        }
        Ok(())
    }

    /// 按版本化 canonical JSON 编码快照 / Encode the snapshot as versioned canonical JSON.
    #[cfg(feature = "serde")]
    pub fn to_canonical_json(&self) -> Result<Vec<u8>> {
        snapshot_codec::encode(self)
    }

    /// 从 canonical JSON 解码并校验快照 / Decode and validate a snapshot from canonical JSON.
    #[cfg(feature = "serde")]
    pub fn from_canonical_json(bytes: &[u8]) -> Result<Self> {
        snapshot_codec::decode(bytes)
    }

    /// 创建带 artifact digest 的快照封装 / Create a snapshot envelope with an artifact digest.
    #[cfg(feature = "serde")]
    pub fn to_artifact(&self) -> Result<snapshot_codec::ConstraintProgrammingSnapshotArtifact> {
        snapshot_codec::artifact(self)
    }

    pub(crate) fn compute_fingerprint(&self) -> AuditFingerprint {
        let mut bytes = Vec::new();
        append_string(&mut bytes, &self.identity_namespace);
        append_string(&mut bytes, &self.identity_schema_version);
        bytes.extend_from_slice(&(self.variables.len() as u64).to_le_bytes());
        for variable in &self.variables {
            append_string(&mut bytes, &variable.variable.stable_id.0);
            variable.domain.append_canonical_bytes(&mut bytes);
        }
        bytes.extend_from_slice(&(self.intervals.len() as u64).to_le_bytes());
        for interval in &self.intervals {
            interval.interval.append_canonical_bytes(&mut bytes);
        }
        bytes.extend_from_slice(&(self.constraints.len() as u64).to_le_bytes());
        for constraint in &self.constraints {
            append_string(&mut bytes, &constraint.id.0);
            append_string(&mut bytes, &constraint.name);
            if let Some(group) = &constraint.group {
                bytes.push(1);
                append_string(&mut bytes, group);
            } else {
                bytes.push(0);
            }
            if let Some(origin) = &constraint.origin {
                bytes.push(1);
                append_string(&mut bytes, origin);
            } else {
                bytes.push(0);
            }
            constraint.constraint.append_canonical_bytes(&mut bytes);
        }
        if let Some(objective) = &self.objective {
            bytes.push(1);
            bytes.push(if objective.category.is_minimum() {
                0
            } else {
                1
            });
            objective.expression.append_canonical_bytes(&mut bytes);
        } else {
            bytes.push(0);
        }
        bytes.extend_from_slice(&(self.constraint_groups.len() as u64).to_le_bytes());
        for (id, name) in &self.constraint_groups {
            bytes.extend_from_slice(&id.to_le_bytes());
            append_string(&mut bytes, name);
        }
        sha256_fingerprint("ospf.constraint-programming.snapshot", &bytes)
    }
}

#[cfg(feature = "serde")]
mod snapshot_codec {
    use super::*;

    /// 当前 CP snapshot codec schema / Current CP snapshot codec schema.
    pub const CURRENT_SCHEMA_VERSION: &str = "1.0";
    const KIND: &str = "ospf.constraint-programming.snapshot";

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SnapshotPayload {
        schema_version: String,
        kind: String,
        snapshot: ConstraintProgrammingSnapshot,
    }

    /// 带 schema 和 artifact digest 的 CP snapshot / CP snapshot with schema and artifact digest.
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ConstraintProgrammingSnapshotArtifact {
        /// codec schema 版本 / Codec schema version.
        pub schema_version: String,
        /// artifact 类型 / Artifact kind.
        pub kind: String,
        /// 不可变模型快照 / Immutable model snapshot.
        pub snapshot: ConstraintProgrammingSnapshot,
        /// 覆盖 canonical payload 的摘要 / Digest covering the canonical payload.
        pub artifact_digest: AuditFingerprint,
    }

    impl ConstraintProgrammingSnapshotArtifact {
        /// 校验 artifact 的 schema、快照身份和摘要 / Validate the artifact schema, snapshot identity, and digest.
        pub fn validate(&self) -> Result<()> {
            validate_artifact(self)
        }

        /// 编码为 canonical JSON / Encode the artifact as canonical JSON.
        pub fn to_canonical_json(&self) -> Result<Vec<u8>> {
            validate_artifact(self)?;
            serde_json::to_vec(self).map_err(|error| {
                crate::error::CoreError::parsing_error(format!(
                    "failed to encode CP snapshot artifact: {error}"
                ))
            })
        }
    }

    pub fn artifact(
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> Result<ConstraintProgrammingSnapshotArtifact> {
        snapshot.validate_identity()?;
        let payload = SnapshotPayload {
            schema_version: CURRENT_SCHEMA_VERSION.to_owned(),
            kind: KIND.to_owned(),
            snapshot: snapshot.clone(),
        };
        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|error| crate::error::CoreError::parsing_error(error.to_string()))?;
        Ok(ConstraintProgrammingSnapshotArtifact {
            schema_version: payload.schema_version,
            kind: payload.kind,
            snapshot: payload.snapshot,
            artifact_digest: sha256_fingerprint(
                "ospf.constraint-programming.snapshot.artifact",
                &payload_bytes,
            ),
        })
    }

    pub fn encode(snapshot: &ConstraintProgrammingSnapshot) -> Result<Vec<u8>> {
        artifact(snapshot)?.to_canonical_json()
    }

    pub fn decode(bytes: &[u8]) -> Result<ConstraintProgrammingSnapshot> {
        Ok(decode_artifact(bytes)?.snapshot)
    }

    pub fn decode_artifact(bytes: &[u8]) -> Result<ConstraintProgrammingSnapshotArtifact> {
        let artifact: ConstraintProgrammingSnapshotArtifact = serde_json::from_slice(bytes)
            .map_err(|error| {
                crate::error::CoreError::parsing_error(format!(
                    "failed to decode CP snapshot artifact: {error}"
                ))
            })?;
        validate_artifact(&artifact)?;
        let canonical = artifact.to_canonical_json()?;
        if canonical != bytes {
            return Err(invalid("CP snapshot artifact is not canonical JSON"));
        }
        Ok(artifact)
    }

    fn validate_artifact(artifact: &ConstraintProgrammingSnapshotArtifact) -> Result<()> {
        if artifact.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(invalid(format!(
                "unsupported CP snapshot codec schema version '{}'",
                artifact.schema_version
            )));
        }
        if artifact.kind != KIND {
            return Err(invalid(format!(
                "unexpected CP snapshot artifact kind '{}'",
                artifact.kind
            )));
        }
        artifact.snapshot.validate_identity()?;
        let payload = SnapshotPayload {
            schema_version: artifact.schema_version.clone(),
            kind: artifact.kind.clone(),
            snapshot: artifact.snapshot.clone(),
        };
        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|error| crate::error::CoreError::parsing_error(error.to_string()))?;
        let expected_digest = sha256_fingerprint(
            "ospf.constraint-programming.snapshot.artifact",
            &payload_bytes,
        );
        if artifact.artifact_digest != expected_digest {
            return Err(invalid(
                "CP snapshot artifact digest does not match canonical payload",
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
pub use snapshot_codec::{
    CURRENT_SCHEMA_VERSION as CURRENT_CP_SNAPSHOT_SCHEMA_VERSION,
    ConstraintProgrammingSnapshotArtifact,
};

#[cfg(feature = "serde")]
pub use snapshot_codec::decode_artifact as decode_cp_snapshot_artifact;

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        BooleanLiteral, ConstraintDefinition, ConstraintProgrammingConstraint,
        ConstraintProgrammingModel, IntegerDomain, IntegerExpression, IntegerObjective,
        IntegerRelation, IntegerVariable,
    };
    use crate::variable::VariableId;

    fn snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model = ConstraintProgrammingModel::new("codec");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 2).expect("domain"))
            .expect("variable");
        model
            .register_variable(y, IntegerDomain::boolean())
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "x-lower",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x),
                    IntegerRelation::GreaterOrEqual,
                    1,
                ),
            ))
            .expect("constraint");
        model.freeze().expect("snapshot")
    }

    #[test]
    fn canonical_snapshot_round_trip_preserves_identity_and_digest() {
        let snapshot = snapshot();
        let encoded = snapshot.to_canonical_json().expect("encode snapshot");
        let decoded =
            ConstraintProgrammingSnapshot::from_canonical_json(&encoded).expect("decode snapshot");
        assert_eq!(decoded, snapshot);
        let artifact = decode_cp_snapshot_artifact(&encoded).expect("decode artifact");
        artifact.validate().expect("artifact is valid");
        assert_eq!(artifact.snapshot.fingerprint, snapshot.fingerprint);
    }

    #[test]
    fn snapshot_codec_rejects_future_schema_digest_and_fingerprint_tampering() {
        let snapshot = snapshot();
        let artifact = snapshot.to_artifact().expect("artifact");

        let mut future = artifact.clone();
        future.schema_version = "99.0".to_owned();
        assert!(future.validate().is_err());

        let mut bad_digest = artifact.clone();
        bad_digest.artifact_digest.value = "tampered".to_owned();
        assert!(bad_digest.validate().is_err());

        let mut bad_fingerprint = artifact;
        bad_fingerprint.snapshot.fingerprint.value = "tampered".to_owned();
        assert!(bad_fingerprint.validate().is_err());
    }

    #[test]
    fn snapshot_codec_rejects_non_canonical_order_and_json() {
        let snapshot = snapshot();
        let artifact = snapshot.to_artifact().expect("artifact");
        let pretty = serde_json::to_vec_pretty(&artifact).expect("pretty JSON");
        assert!(ConstraintProgrammingSnapshot::from_canonical_json(&pretty).is_err());

        let mut unordered = artifact;
        unordered.snapshot.variables.reverse();
        assert!(unordered.validate().is_err());
    }

    #[test]
    fn canonical_artifact_normalizes_unordered_constraint_members() {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut first = ConstraintProgrammingModel::new("unordered-artifact");
        first
            .register_variable(x.clone(), IntegerDomain::boolean())
            .expect("x");
        first
            .register_variable(y.clone(), IntegerDomain::boolean())
            .expect("y");
        first
            .add_constraint(ConstraintDefinition::new(
                "and",
                ConstraintProgrammingConstraint::and([
                    BooleanLiteral::positive(y.clone()),
                    BooleanLiteral::positive(x.clone()),
                ]),
            ))
            .expect("first constraint");

        let mut second = ConstraintProgrammingModel::new("unordered-artifact");
        second
            .register_variable(y.clone(), IntegerDomain::boolean())
            .expect("y");
        second
            .register_variable(x.clone(), IntegerDomain::boolean())
            .expect("x");
        second
            .add_constraint(ConstraintDefinition::new(
                "and",
                ConstraintProgrammingConstraint::and([
                    BooleanLiteral::positive(x),
                    BooleanLiteral::positive(y),
                ]),
            ))
            .expect("second constraint");

        let first = first.freeze().expect("first snapshot");
        let second = second.freeze().expect("second snapshot");
        let first_bytes = first.to_canonical_json().expect("first JSON");
        let second_bytes = second.to_canonical_json().expect("second JSON");
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first_bytes, second_bytes);
        assert_eq!(
            first.to_artifact().expect("first artifact").artifact_digest,
            second
                .to_artifact()
                .expect("second artifact")
                .artifact_digest
        );
    }

    #[test]
    fn canonical_artifact_ignores_independent_local_variable_ids() {
        let build = |x_id: VariableId, y_id: VariableId| {
            let x = IntegerVariable::with_ids(x_id, "x");
            let y = IntegerVariable::with_ids(y_id, "y");
            let mut model = ConstraintProgrammingModel::new("independent-local-ids");
            model
                .register_variable(y.clone(), IntegerDomain::boolean())
                .expect("y");
            model
                .register_variable(x.clone(), IntegerDomain::boolean())
                .expect("x");
            model
                .add_constraint(ConstraintDefinition::new(
                    "and",
                    ConstraintProgrammingConstraint::and([
                        BooleanLiteral::positive(y),
                        BooleanLiteral::positive(x.clone()),
                    ]),
                ))
                .expect("constraint");
            model.set_objective(IntegerObjective::minimize(IntegerExpression::variable(x)));
            model.freeze().expect("snapshot")
        };

        let first = build(VariableId::standalone(101), VariableId::standalone(102));
        let second = build(VariableId::standalone(901), VariableId::standalone(902));
        assert_eq!(first, second);
        assert_eq!(
            first.to_canonical_json().expect("first JSON"),
            second.to_canonical_json().expect("second JSON")
        );
        assert_eq!(
            first.to_artifact().expect("first artifact").artifact_digest,
            second
                .to_artifact()
                .expect("second artifact")
                .artifact_digest
        );
    }
}
