//! CP 区间和调度元素 / CP intervals and scheduling elements.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ModelError, Result};
use crate::solver::StableVariableId;

use super::expression::IntegerExpression;
use super::literal::BooleanLiteral;
use super::variable::{IntegerVariable, IntervalVariableId, rebind_variable};

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// 区间时长 / Interval duration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntervalDuration {
    /// 固定时长 / Fixed duration.
    Fixed(i64),
    /// 有界整数变量时长 / Bounded integer-variable duration.
    Variable(IntegerVariable),
}

/// 区间变量 / Interval variable.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntervalVariable {
    /// 区间稳定身份 / Stable interval identity.
    pub id: IntervalVariableId,
    /// 起点表达式 / Start expression.
    pub start: IntegerExpression,
    /// 时长 / Duration.
    pub duration: IntervalDuration,
    /// 终点表达式 / End expression.
    pub end: IntegerExpression,
    /// 可选 presence literal；None 表示 mandatory / Optional presence literal; None means mandatory.
    pub presence: Option<BooleanLiteral>,
}

impl IntervalVariable {
    /// 创建区间并检查固定时长 / Create an interval and validate fixed duration.
    pub fn new(
        id: impl Into<IntervalVariableId>,
        start: IntegerExpression,
        duration: IntervalDuration,
        end: IntegerExpression,
        presence: Option<BooleanLiteral>,
    ) -> Result<Self> {
        if let IntervalDuration::Fixed(value) = duration
            && value < 0
        {
            return Err(invalid("interval fixed duration must not be negative"));
        }
        Ok(Self {
            id: id.into(),
            start,
            duration,
            end,
            presence,
        })
    }

    /// 返回所有标量变量 / Return all scalar variables referenced by the interval.
    pub fn referenced_variables(&self) -> BTreeSet<StableVariableId> {
        let mut result = self.start.referenced_variables();
        result.extend(self.end.referenced_variables());
        if let IntervalDuration::Variable(variable) = &self.duration {
            result.insert(variable.stable_id.clone());
        }
        if let Some(presence) = &self.presence {
            result.insert(presence.variable.stable_id.clone());
        }
        result
    }

    pub(crate) fn validate_bindings(
        &self,
        variables: &BTreeMap<StableVariableId, IntegerVariable>,
    ) -> Result<()> {
        self.start.validate_bindings(variables)?;
        self.end.validate_bindings(variables)?;
        if let IntervalDuration::Variable(variable) = &self.duration {
            validate_variable_binding(variable, variables)?;
        }
        if let Some(presence) = &self.presence {
            validate_variable_binding(&presence.variable, variables)?;
        }
        Ok(())
    }

    /// 按稳定身份重绑区间中的变量 / Rebind interval variables by stable identity.
    pub(crate) fn rebind_variables(
        &self,
        variables: &BTreeMap<StableVariableId, IntegerVariable>,
    ) -> Result<Self> {
        let duration = match &self.duration {
            IntervalDuration::Fixed(value) => IntervalDuration::Fixed(*value),
            IntervalDuration::Variable(variable) => {
                IntervalDuration::Variable(rebind_variable(variable, variables)?)
            }
        };
        let presence = self
            .presence
            .as_ref()
            .map(|literal| literal.rebind_variables(variables))
            .transpose()?;
        Ok(Self {
            id: self.id.clone(),
            start: self.start.rebind_variables(variables)?,
            duration,
            end: self.end.rebind_variables(variables)?,
            presence,
        })
    }

    pub(crate) fn validate_structure(
        &self,
        domains: &BTreeMap<StableVariableId, super::domain::IntegerDomain>,
        variables: &BTreeMap<StableVariableId, IntegerVariable>,
    ) -> Result<()> {
        self.validate_bindings(variables)?;
        self.start.bounds(|id| domains.get(id))?;
        self.end.bounds(|id| domains.get(id))?;
        if let Some(presence) = &self.presence {
            let domain = domains.get(&presence.variable.stable_id).ok_or_else(|| {
                invalid(format!(
                    "interval presence references unknown variable {}",
                    presence.variable.stable_id.0
                ))
            })?;
            if !domain.is_boolean() {
                return Err(invalid(format!(
                    "interval presence variable {} is not Boolean",
                    presence.variable.stable_id.0
                )));
            }
        }
        let (duration_lower, duration_upper) = match &self.duration {
            IntervalDuration::Fixed(value) => {
                if *value < 0 {
                    return Err(invalid("interval fixed duration must not be negative"));
                }
                (*value, *value)
            }
            IntervalDuration::Variable(variable) => {
                let domain = domains.get(&variable.stable_id).ok_or_else(|| {
                    invalid(format!(
                        "interval duration references unknown variable {}",
                        variable.stable_id.0
                    ))
                })?;
                if domain.lower() < 0 {
                    return Err(invalid(format!(
                        "interval duration variable {} can be negative",
                        variable.stable_id.0
                    )));
                }
                (domain.lower(), domain.upper())
            }
        };
        let (start_lower, start_upper) = self.start.bounds(|id| domains.get(id))?;
        let (end_lower, end_upper) = self.end.bounds(|id| domains.get(id))?;
        let sum_lower = i128::from(start_lower)
            .checked_add(i128::from(duration_lower))
            .ok_or_else(|| {
                invalid(format!(
                    "interval {} start-plus-duration overflows",
                    self.id
                ))
            })?;
        let sum_upper = i128::from(start_upper)
            .checked_add(i128::from(duration_upper))
            .ok_or_else(|| {
                invalid(format!(
                    "interval {} start-plus-duration overflows",
                    self.id
                ))
            })?;
        if sum_lower < i128::from(i64::MIN)
            || sum_upper > i128::from(i64::MAX)
            || i128::from(end_upper) < sum_lower
            || i128::from(end_lower) > sum_upper
        {
            return Err(invalid(format!(
                "interval {} has no representable end value in its declared domain",
                self.id
            )));
        }
        Ok(())
    }

    /// 精确求值；非 presence 区间不激活时不检查时间关系 / Evaluate exactly; inactive optional intervals have no active time relation.
    pub fn evaluate(
        &self,
        values: &BTreeMap<StableVariableId, i64>,
    ) -> Result<Option<IntervalValue>> {
        if let Some(presence) = &self.presence
            && !presence.evaluate(values)?
        {
            return Ok(None);
        }
        let start = self.start.evaluate(values)?;
        let duration = match &self.duration {
            IntervalDuration::Fixed(value) => *value,
            IntervalDuration::Variable(variable) => {
                *values.get(&variable.stable_id).ok_or_else(|| {
                    invalid(format!(
                        "missing interval duration variable {}",
                        variable.stable_id.0
                    ))
                })?
            }
        };
        if duration < 0 {
            return Err(invalid(format!(
                "interval {} has negative duration",
                self.id
            )));
        }
        let expected_end = i128::from(start)
            .checked_add(i128::from(duration))
            .ok_or_else(|| invalid(format!("interval {} end overflows i64", self.id)))?;
        let end = self.end.evaluate(values)?;
        if i128::from(end) != expected_end {
            return Err(invalid(format!(
                "interval {} end does not equal start plus duration",
                self.id
            )));
        }
        Ok(Some(IntervalValue {
            start,
            end,
            duration,
        }))
    }

    pub(crate) fn append_canonical_bytes(&self, bytes: &mut Vec<u8>) {
        self.id.append_canonical_bytes(bytes);
        self.start.append_canonical_bytes(bytes);
        match &self.duration {
            IntervalDuration::Fixed(value) => {
                bytes.push(0);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            IntervalDuration::Variable(variable) => {
                bytes.push(1);
                super::expression::append_string(bytes, &variable.stable_id.0);
            }
        }
        self.end.append_canonical_bytes(bytes);
        if let Some(presence) = &self.presence {
            bytes.push(1);
            super::expression::append_string(bytes, &presence.variable.stable_id.0);
            bytes.push(u8::from(presence.negated));
        } else {
            bytes.push(0);
        }
    }
}

fn validate_variable_binding(
    variable: &IntegerVariable,
    variables: &BTreeMap<StableVariableId, IntegerVariable>,
) -> Result<()> {
    let expected = variables.get(&variable.stable_id).ok_or_else(|| {
        invalid(format!(
            "interval references unknown variable {}",
            variable.stable_id.0
        ))
    })?;
    if expected.id != variable.id {
        return Err(invalid(format!(
            "stable variable {} is bound to a different local identity",
            variable.stable_id.0
        )));
    }
    Ok(())
}

impl IntervalVariableId {
    pub(crate) fn append_canonical_bytes(&self, bytes: &mut Vec<u8>) {
        super::expression::append_string(bytes, &self.0);
    }
}

/// 已激活区间的求值结果 / Evaluated active interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalValue {
    /// 起点 / Start.
    pub start: i64,
    /// 终点 / End.
    pub end: i64,
    /// 时长 / Duration.
    pub duration: i64,
}

/// 固定需求的累计任务 / Fixed-demand cumulative task.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CumulativeTask {
    /// 任务区间 / Task interval.
    pub interval: IntervalVariableId,
    /// 固定需求 / Fixed demand.
    pub demand: i64,
}

/// 自动机转移 / Automaton transition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AutomatonTransition {
    /// 起始状态 / Source state.
    pub from_state: i32,
    /// 接受的输入值 / Accepted input value.
    pub value: i64,
    /// 目标状态 / Target state.
    pub to_state: i32,
}

/// Reservoir 事件 / Reservoir event.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReservoirEvent {
    /// 事件时间 / Event time.
    pub time: IntegerExpression,
    /// 液位变化 / Level change.
    pub level_change: IntegerExpression,
}
