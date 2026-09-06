//! 布尔文字模块 / Boolean literal module.

use super::variable::IntegerVariable;
use crate::error::{ModelError, Result};
use std::collections::BTreeMap;

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// 布尔文字 / Boolean literal.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BooleanLiteral {
    /// 布尔变量 / Boolean variable.
    pub variable: IntegerVariable,
    /// 是否取反 / Whether the literal is negated.
    pub negated: bool,
}

impl BooleanLiteral {
    /// 创建正文字 / Create a positive literal.
    pub fn positive(variable: IntegerVariable) -> Self {
        Self {
            variable,
            negated: false,
        }
    }

    /// 创建负文字 / Create a negated literal.
    pub fn negative(variable: IntegerVariable) -> Self {
        Self {
            variable,
            negated: true,
        }
    }

    /// 反转文字 / Negate this literal.
    pub fn negated(self) -> Self {
        Self {
            variable: self.variable,
            negated: !self.negated,
        }
    }

    /// 按赋值精确求值 / Evaluate the literal under an assignment.
    pub fn evaluate(
        &self,
        values: &BTreeMap<crate::solver::StableVariableId, i64>,
    ) -> Result<bool> {
        let value = values.get(&self.variable.stable_id).ok_or_else(|| {
            invalid(format!(
                "missing Boolean assignment for variable {}",
                self.variable.stable_id.0
            ))
        })?;
        if *value != 0 && *value != 1 {
            return Err(invalid(format!(
                "Boolean variable {} has non-Boolean value {}",
                self.variable.stable_id.0, value
            )));
        }
        Ok(if self.negated {
            *value == 0
        } else {
            *value == 1
        })
    }

    /// 按稳定身份重绑文字中的变量 / Rebind the literal variable by stable identity.
    pub(crate) fn rebind_variables(
        &self,
        variables: &BTreeMap<crate::solver::StableVariableId, IntegerVariable>,
    ) -> Result<Self> {
        Ok(Self {
            variable: super::variable::rebind_variable(&self.variable, variables)?,
            negated: self.negated,
        })
    }
}
