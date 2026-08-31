//! CP 变量身份 / Constraint-programming variable identity.

use std::collections::BTreeMap;

use crate::error::{ModelError, Result};
use crate::solver::{StableConstraintId, StableVariableId};
use crate::variable::{VariableId, new_standalone_id};

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// CP 整数变量引用 / CP integer-variable reference.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegerVariable {
    /// 进程内变量身份 / In-process variable identity.
    pub id: VariableId,
    /// 报告和快照使用的稳定身份 / Stable identity used by reports and snapshots.
    pub stable_id: StableVariableId,
}

impl IntegerVariable {
    /// 创建带稳定身份的新变量 / Create a variable with a stable identity.
    pub fn new(stable_id: impl Into<StableVariableId>) -> Self {
        Self {
            id: new_standalone_id(),
            stable_id: stable_id.into(),
        }
    }

    /// 使用显式进程内和稳定身份创建变量 / Create a variable with explicit local and stable identities.
    pub fn with_ids(id: VariableId, stable_id: impl Into<StableVariableId>) -> Self {
        Self {
            id,
            stable_id: stable_id.into(),
        }
    }
}

/// 按稳定身份重绑变量 / Rebind a variable by stable identity.
pub(crate) fn rebind_variable(
    variable: &IntegerVariable,
    bindings: &BTreeMap<StableVariableId, IntegerVariable>,
) -> Result<IntegerVariable> {
    bindings.get(&variable.stable_id).cloned().ok_or_else(|| {
        invalid(format!(
            "cannot rebind unknown variable {}",
            variable.stable_id.0
        ))
    })
}

/// CP 约束的稳定身份别名 / Stable identity alias for a CP constraint.
pub type ConstraintId = StableConstraintId;

/// 区间变量的稳定身份 / Stable identity for an interval variable.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IntervalVariableId(pub String);

impl From<String> for IntervalVariableId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for IntervalVariableId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl std::fmt::Display for IntervalVariableId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
