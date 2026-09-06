//! CP 整数值域 / Constraint-programming integer domains.

use crate::error::{ModelError, Result};

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// 精确整数值域 / Exact integer domain.
///
/// 值域只承载 `i64`。稀疏值域在构造时排序并去重，不会被退化成只保留
/// 最小值和最大值的连续区间。稀疏值域不会扩展为其连续包络。
/// Domains contain only `i64` values. Sparse domains are sorted and
/// deduplicated at construction time and are never widened to their hull.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntegerDomain {
    /// 闭整数区间 / Closed integer interval.
    Range {
        /// 下界 / Lower bound.
        lower: i64,
        /// 上界 / Upper bound.
        upper: i64,
    },
    /// 规范化后的稀疏整数值 / Normalized sparse integer values.
    Values(Vec<i64>),
}

impl IntegerDomain {
    /// 校验公开值域的规范形态 / Validate the canonical shape of a public domain.
    ///
    /// 构造函数通常会生成合法值域，但反序列化和公开 snapshot 字段可能重新引入非法的
    /// `Range` 或 `Values`。后端不得接收此类值域。/ Builder constructors normally produce
    /// valid domains, but deserialization and public snapshot fields can reintroduce malformed
    /// `Range` or `Values` values. Backends must never receive such a domain.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Range { lower, upper } if lower > upper => Err(invalid(format!(
                "integer domain lower bound exceeds upper bound: {lower} > {upper}"
            ))),
            Self::Values(values) if values.is_empty() => {
                Err(invalid("integer sparse domain must not be empty"))
            }
            Self::Values(values) if values.windows(2).any(|pair| pair[0] >= pair[1]) => Err(
                invalid("integer sparse domain must be strictly increasing and deduplicated"),
            ),
            _ => Ok(()),
        }
    }

    /// 创建闭整数区间 / Create a closed integer interval.
    pub fn range(lower: i64, upper: i64) -> Result<Self> {
        let domain = Self::Range { lower, upper };
        domain.validate()?;
        Ok(domain)
    }

    /// 创建规范化稀疏值域 / Create a normalized sparse domain.
    pub fn values<I>(values: I) -> Result<Self>
    where
        I: IntoIterator<Item = i64>,
    {
        let mut values = values.into_iter().collect::<Vec<_>>();
        if values.is_empty() {
            return Err(invalid("integer sparse domain must not be empty"));
        }
        values.sort_unstable();
        values.dedup();
        let domain = Self::Values(values);
        domain.validate()?;
        Ok(domain)
    }

    /// 返回布尔值域 `{0, 1}` / Return the Boolean domain `{0, 1}`.
    pub fn boolean() -> Self {
        Self::Values(vec![0, 1])
    }

    /// 返回下界 / Return the lower bound.
    pub fn lower(&self) -> i64 {
        match self {
            Self::Range { lower, .. } => *lower,
            Self::Values(values) => values[0],
        }
    }

    /// 返回上界 / Return the upper bound.
    pub fn upper(&self) -> i64 {
        match self {
            Self::Range { upper, .. } => *upper,
            Self::Values(values) => values[values.len() - 1],
        }
    }

    /// 返回值域基数 / Return the domain cardinality.
    pub fn cardinality(&self) -> u128 {
        match self {
            Self::Range { lower, upper } => (*upper as i128 - *lower as i128 + 1) as u128,
            Self::Values(values) => values.len() as u128,
        }
    }

    /// 判断值是否属于值域 / Check whether a value belongs to the domain.
    pub fn contains(&self, value: i64) -> bool {
        match self {
            Self::Range { lower, upper } => (*lower..=*upper).contains(&value),
            Self::Values(values) => values.binary_search(&value).is_ok(),
        }
    }

    /// 判断是否为布尔值域 / Check whether this is exactly the Boolean domain.
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Values(values) if values == &[0, 1])
    }

    /// 在规模限制内枚举值域 / Enumerate the domain within a size limit.
    pub fn enumerate(&self, maximum_amount: usize) -> Result<Vec<i64>> {
        if self.cardinality() > maximum_amount as u128 {
            return Err(invalid(format!(
                "integer domain cardinality {} exceeds enumeration limit {}",
                self.cardinality(),
                maximum_amount
            )));
        }
        match self {
            Self::Values(values) => Ok(values.clone()),
            Self::Range { lower, upper } => {
                let mut result = Vec::with_capacity(self.cardinality() as usize);
                let mut current = *lower;
                loop {
                    result.push(current);
                    if current == *upper {
                        break;
                    }
                    current = current
                        .checked_add(1)
                        .ok_or_else(|| invalid("integer domain enumeration overflow"))?;
                }
                Ok(result)
            }
        }
    }

    pub(crate) fn append_canonical_bytes(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Range { lower, upper } => {
                bytes.push(b'R');
                bytes.extend_from_slice(&lower.to_le_bytes());
                bytes.extend_from_slice(&upper.to_le_bytes());
            }
            Self::Values(values) => {
                bytes.push(b'V');
                bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
                for value in values {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IntegerDomain;

    #[test]
    fn sparse_values_are_sorted_and_deduplicated() {
        let domain = IntegerDomain::values([3, 1, 3, -2, 1]).expect("valid sparse domain");
        assert_eq!(domain, IntegerDomain::Values(vec![-2, 1, 3]));
    }

    #[test]
    fn invalid_and_empty_domains_are_rejected() {
        assert!(IntegerDomain::range(2, 1).is_err());
        assert!(IntegerDomain::values(Vec::<i64>::new()).is_err());
    }

    #[test]
    fn extreme_range_cardinality_uses_wide_intermediate() {
        let domain = IntegerDomain::range(i64::MIN, i64::MAX).expect("valid range");
        assert_eq!(domain.cardinality(), 1u128 << 64);
    }
}
