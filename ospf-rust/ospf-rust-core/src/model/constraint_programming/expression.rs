//! CP 整数表达式 / Constraint-programming integer expressions.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ModelError, Result};
use crate::solver::StableVariableId;

use super::domain::IntegerDomain;
use super::variable::{IntegerVariable, rebind_variable};

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// 整数线性项 / Integer-linear term.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegerTerm {
    /// 变量 / Variable.
    pub variable: IntegerVariable,
    /// 整数系数 / Integer coefficient.
    pub coefficient: i64,
}

/// 规范化整数仿射表达式 / Normalized integer-affine expression.
///
/// 构造时会合并重复稳定变量、删除零系数，并按稳定 ID 排序。系数和求值
/// 使用 checked `i128` 作为中间类型。/ Construction merges duplicate stable variables,
/// removes zero coefficients, and sorts by stable ID. Coefficient aggregation and evaluation
/// use checked `i128` intermediates.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegerExpression {
    /// 常数项 / Constant term.
    pub constant: i64,
    /// 规范化线性项 / Normalized linear terms.
    pub terms: Vec<IntegerTerm>,
}

impl IntegerExpression {
    /// 创建常量表达式 / Create a constant expression.
    pub const fn constant(value: i64) -> Self {
        Self {
            constant: value,
            terms: Vec::new(),
        }
    }

    /// 创建变量表达式 / Create a variable expression.
    pub fn variable(variable: IntegerVariable) -> Self {
        Self {
            constant: 0,
            terms: vec![IntegerTerm {
                variable,
                coefficient: 1,
            }],
        }
    }

    /// 创建并规范化线性表达式 / Create and normalize an integer-linear expression.
    pub fn linear<I>(constant: i64, terms: I) -> Result<Self>
    where
        I: IntoIterator<Item = IntegerTerm>,
    {
        let mut normalized: BTreeMap<StableVariableId, (IntegerVariable, i128)> = BTreeMap::new();
        for term in terms {
            if term.coefficient == 0 {
                continue;
            }
            let entry = normalized
                .entry(term.variable.stable_id.clone())
                .or_insert_with(|| (term.variable.clone(), 0));
            if entry.0.id != term.variable.id {
                return Err(invalid(format!(
                    "stable variable {} is bound to multiple local identities",
                    term.variable.stable_id.0
                )));
            }
            entry.1 = entry
                .1
                .checked_add(i128::from(term.coefficient))
                .ok_or_else(|| invalid("integer expression coefficient overflow"))?;
        }

        let mut result = Vec::with_capacity(normalized.len());
        for (_, (variable, coefficient)) in normalized {
            if coefficient == 0 {
                continue;
            }
            result.push(IntegerTerm {
                variable,
                coefficient: i64::try_from(coefficient)
                    .map_err(|_| invalid("integer expression coefficient exceeds i64"))?,
            });
        }
        Ok(Self {
            constant,
            terms: result,
        })
    }

    /// 返回表达式引用的稳定变量 / Return stable variables referenced by the expression.
    pub fn referenced_variables(&self) -> BTreeSet<StableVariableId> {
        self.terms
            .iter()
            .map(|term| term.variable.stable_id.clone())
            .collect()
    }

    /// 精确求值 / Evaluate exactly under stable variable assignments.
    pub fn evaluate(&self, values: &BTreeMap<StableVariableId, i64>) -> Result<i64> {
        let mut total = i128::from(self.constant);
        for term in &self.terms {
            let value = values.get(&term.variable.stable_id).ok_or_else(|| {
                invalid(format!(
                    "missing assignment for variable {}",
                    term.variable.stable_id.0
                ))
            })?;
            let product = i128::from(*value)
                .checked_mul(i128::from(term.coefficient))
                .ok_or_else(|| invalid("integer expression multiplication overflow"))?;
            total = total
                .checked_add(product)
                .ok_or_else(|| invalid("integer expression accumulation overflow"))?;
        }
        i64::try_from(total).map_err(|_| invalid("integer expression value exceeds i64"))
    }

    /// 根据变量域计算保守的表达式上下界 / Compute conservative expression bounds from variable domains.
    pub fn bounds<'a, F>(&self, mut domain_of: F) -> Result<(i64, i64)>
    where
        F: FnMut(&StableVariableId) -> Option<&'a IntegerDomain>,
    {
        let mut lower = i128::from(self.constant);
        let mut upper = i128::from(self.constant);
        for term in &self.terms {
            let domain = domain_of(&term.variable.stable_id).ok_or_else(|| {
                invalid(format!(
                    "missing domain for variable {}",
                    term.variable.stable_id.0
                ))
            })?;
            let coefficient = i128::from(term.coefficient);
            let (minimum, maximum) = if coefficient >= 0 {
                (i128::from(domain.lower()), i128::from(domain.upper()))
            } else {
                (i128::from(domain.upper()), i128::from(domain.lower()))
            };
            lower = lower
                .checked_add(coefficient.checked_mul(minimum).ok_or_else(|| {
                    invalid("integer expression lower-bound multiplication overflow")
                })?)
                .ok_or_else(|| invalid("integer expression lower bound overflow"))?;
            upper = upper
                .checked_add(coefficient.checked_mul(maximum).ok_or_else(|| {
                    invalid("integer expression upper-bound multiplication overflow")
                })?)
                .ok_or_else(|| invalid("integer expression upper bound overflow"))?;
        }
        Ok((
            i64::try_from(lower)
                .map_err(|_| invalid("integer expression lower bound exceeds i64"))?,
            i64::try_from(upper)
                .map_err(|_| invalid("integer expression upper bound exceeds i64"))?,
        ))
    }

    pub(crate) fn validate_bindings(
        &self,
        variables: &BTreeMap<StableVariableId, IntegerVariable>,
    ) -> Result<()> {
        let mut previous = None;
        for term in &self.terms {
            if term.coefficient == 0 {
                return Err(invalid(
                    "integer expression must not contain zero coefficients",
                ));
            }
            if previous
                .as_ref()
                .is_some_and(|id: &StableVariableId| id >= &term.variable.stable_id)
            {
                return Err(invalid(
                    "integer expression terms must be sorted and unique by stable ID",
                ));
            }
            let expected = variables.get(&term.variable.stable_id).ok_or_else(|| {
                invalid(format!(
                    "expression references unknown variable {}",
                    term.variable.stable_id.0
                ))
            })?;
            if expected.id != term.variable.id {
                return Err(invalid(format!(
                    "stable variable {} is bound to a different local identity",
                    term.variable.stable_id.0
                )));
            }
            previous = Some(term.variable.stable_id.clone());
        }
        Ok(())
    }

    /// 按稳定身份重绑表达式中的变量 / Rebind expression variables by stable identity.
    pub(crate) fn rebind_variables(
        &self,
        variables: &BTreeMap<StableVariableId, IntegerVariable>,
    ) -> Result<Self> {
        let terms = self
            .terms
            .iter()
            .map(|term| {
                Ok(IntegerTerm {
                    variable: rebind_variable(&term.variable, variables)?,
                    coefficient: term.coefficient,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            constant: self.constant,
            terms,
        })
    }

    pub(crate) fn append_canonical_bytes(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.constant.to_le_bytes());
        bytes.extend_from_slice(&(self.terms.len() as u64).to_le_bytes());
        for term in &self.terms {
            append_string(bytes, &term.variable.stable_id.0);
            bytes.extend_from_slice(&term.coefficient.to_le_bytes());
        }
    }
}

/// 整数关系 / Integer relation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerRelation {
    /// 等于 / Equal.
    Equal,
    /// 不等于 / Not equal.
    NotEqual,
    /// 小于等于 / Less than or equal.
    LessOrEqual,
    /// 大于等于 / Greater than or equal.
    GreaterOrEqual,
}

impl IntegerRelation {
    /// 求值关系 / Evaluate the relation.
    pub const fn evaluate(self, left: i64, right: i64) -> bool {
        match self {
            Self::Equal => left == right,
            Self::NotEqual => left != right,
            Self::LessOrEqual => left <= right,
            Self::GreaterOrEqual => left >= right,
        }
    }

    pub(crate) const fn tag(self) -> u8 {
        match self {
            Self::Equal => 0,
            Self::NotEqual => 1,
            Self::LessOrEqual => 2,
            Self::GreaterOrEqual => 3,
        }
    }
}

pub(crate) fn append_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
