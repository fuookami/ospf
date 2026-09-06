//! CP 目标 / CP objective.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::model::ObjectiveCategory;

use super::expression::IntegerExpression;
use super::variable::IntegerVariable;
use crate::solver::StableVariableId;

/// 单整数线性目标 / Single integer-linear objective.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerObjective {
    /// 优化方向 / Optimization direction.
    pub category: ObjectiveCategory,
    /// 目标表达式 / Objective expression.
    pub expression: IntegerExpression,
}

impl IntegerObjective {
    /// 创建最小化目标 / Create a minimization objective.
    pub fn minimize(expression: IntegerExpression) -> Self {
        Self {
            category: ObjectiveCategory::Minimum,
            expression,
        }
    }

    /// 创建最大化目标 / Create a maximization objective.
    pub fn maximize(expression: IntegerExpression) -> Self {
        Self {
            category: ObjectiveCategory::Maximum,
            expression,
        }
    }

    /// 按稳定身份重绑目标中的变量 / Rebind objective variables by stable identity.
    pub(crate) fn rebind_variables(
        &self,
        variables: &BTreeMap<StableVariableId, IntegerVariable>,
    ) -> Result<Self> {
        Ok(Self {
            category: self.category,
            expression: self.expression.rebind_variables(variables)?,
        })
    }
}
