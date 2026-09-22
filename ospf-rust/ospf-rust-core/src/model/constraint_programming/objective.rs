//! CP 目标 / CP objective.

use std::collections::BTreeMap;

use crate::error::{ModelError, Result};
use crate::model::ObjectiveCategory;

use super::expression::IntegerExpression;
use super::variable::IntegerVariable;
use crate::solver::StableVariableId;

/// Default stable identity for the legacy unnamed single objective.
/// 旧的无名单目标构造使用的默认稳定身份。
pub const DEFAULT_OBJECTIVE_ID: &str = "objective";

#[cfg(feature = "serde")]
fn default_objective_id() -> String {
    DEFAULT_OBJECTIVE_ID.to_owned()
}

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// 单整数线性目标 / Single integer-linear objective.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerObjective {
    /// Stable objective identity.
    /// 稳定目标身份。
    #[cfg_attr(feature = "serde", serde(default = "default_objective_id"))]
    pub id: String,
    /// 优化方向 / Optimization direction.
    pub category: ObjectiveCategory,
    /// 目标表达式 / Objective expression.
    pub expression: IntegerExpression,
}

impl IntegerObjective {
    /// 创建最小化目标 / Create a minimization objective.
    pub fn minimize(expression: IntegerExpression) -> Self {
        Self::minimize_with_id(DEFAULT_OBJECTIVE_ID, expression)
    }

    /// 创建最大化目标 / Create a maximization objective.
    pub fn maximize(expression: IntegerExpression) -> Self {
        Self::maximize_with_id(DEFAULT_OBJECTIVE_ID, expression)
    }

    /// 创建带稳定身份的最小化目标 / Create a minimization objective with a stable identity.
    pub fn minimize_with_id(id: impl Into<String>, expression: IntegerExpression) -> Self {
        Self {
            id: id.into(),
            category: ObjectiveCategory::Minimum,
            expression,
        }
    }

    /// 创建带稳定身份的最大化目标 / Create a maximization objective with a stable identity.
    pub fn maximize_with_id(id: impl Into<String>, expression: IntegerExpression) -> Self {
        Self {
            id: id.into(),
            category: ObjectiveCategory::Maximum,
            expression,
        }
    }

    /// 为已有目标设置稳定身份 / Set the stable identity of an existing objective.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// 返回稳定目标身份 / Return the stable objective identity.
    pub fn objective_id(&self) -> &str {
        &self.id
    }

    /// 校验稳定目标身份 / Validate the stable objective identity.
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(invalid("CP objective ID must not be blank"));
        }
        Ok(())
    }

    /// 按稳定身份重绑目标中的变量 / Rebind objective variables by stable identity.
    pub(crate) fn rebind_variables(
        &self,
        variables: &BTreeMap<StableVariableId, IntegerVariable>,
    ) -> Result<Self> {
        Ok(Self {
            id: self.id.clone(),
            category: self.category,
            expression: self.expression.rebind_variables(variables)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_objective_constructors_use_a_stable_default_id() {
        let objective = IntegerObjective::maximize(IntegerExpression::constant(1));
        assert_eq!(objective.objective_id(), DEFAULT_OBJECTIVE_ID);
        assert!(objective.validate().is_ok());
    }

    #[test]
    fn named_objective_constructor_and_builder_preserve_identity() {
        let objective =
            IntegerObjective::minimize_with_id("payload", IntegerExpression::constant(1));
        assert_eq!(objective.id, "payload");
        assert_eq!(objective.with_id("renamed").id, "renamed");
    }
}
