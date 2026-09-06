//! 条件关系的共享语义与线性化 / Shared conditional-relation semantics and linearization

use std::fmt::Debug;
use num_traits::{FromPrimitive, ToPrimitive};
use ospf_rust_math::symbol::Comparison;
use crate::error::{CoreError, ModelError, Result};
use crate::model::LinearInequality;
use crate::symbol::flatten::{Linear, LinearMonomial};

/// 受支持的条件关系 / Supported conditional relations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionRelation {
    /// 严格大于 / Greater than
    Greater,
    /// 大于等于 / Greater than or equal
    GreaterEqual,
    /// 严格小于 / Less than
    Less,
    /// 小于等于 / Less than or equal
    LessEqual,
}

impl TryFrom<Comparison> for ConditionRelation {
    type Error = ModelError;

    /// 从数学比较关系创建条件关系 / Create a condition relation from a math comparison
    fn try_from(value: Comparison) -> std::result::Result<Self, Self::Error> {
        match value {
            Comparison::Greater => Ok(Self::Greater),
            Comparison::GreaterEqual => Ok(Self::GreaterEqual),
            Comparison::Less => Ok(Self::Less),
            Comparison::LessEqual => Ok(Self::LessEqual),
            Comparison::Equal => Err(ModelError::InvalidConstraint(
                "conditional indicators do not support equality relations".to_string(),
            )),
        }
    }
}

/// 条件值的三值分类 / Three-valued classification of a condition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthValue {
    /// 条件成立 / Condition is true
    True,
    /// 条件不成立 / Condition is false
    False,
    /// 严格边界间隔内不可判定 / Undefined within the strict-boundary gap
    Undefined,
}

/// 条件多项式的有限范围 / Finite bounds for a conditional polynomial
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionBounds<V> {
    /// 下界 / Lower bound
    pub lower: V,
    /// 上界 / Upper bound
    pub upper: V,
}

/// 显式关系条件描述器 / Explicit relation-condition descriptor
///
/// 该类型用于新调用方表达关系、严格边界和可证明范围；旧三元 `IfFunction`
/// 保持原有语义，不通过此类型隐式迁移。
/// This type lets new callers state a relation, strict boundary, and proven
/// bounds explicitly. The legacy ternary `IfFunction` keeps its old semantics.
#[derive(Debug, Clone)]
pub struct ConditionalIfFunction<V> {
    /// 条件差值多项式 / Condition-difference polynomial
    pub condition: Linear<V>,
    /// 比较关系 / Comparison relation
    pub relation: ConditionRelation,
    /// 严格边界 / Strict boundary
    pub strict_boundary: V,
    /// 条件范围 / Condition bounds
    pub bounds: ConditionBounds<V>,
}

impl<V> ConditionalIfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建关系条件 / Create a relation condition
    pub fn new(
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        validate_condition_descriptor(
            &condition,
            relation,
            &strict_boundary,
            &bounds,
            "condition",
        )?;
        Ok(Self {
            condition,
            relation,
            strict_boundary,
            bounds,
        })
    }

    /// 对差值分类 / Classify a difference value
    pub fn classify(&self, difference: &V) -> Result<TruthValue> {
        classify(difference, self.relation, &self.strict_boundary)
    }

    /// 兼容二值求值 / Evaluate as a compatibility binary value
    pub fn evaluate(&self, difference: &V) -> Result<Option<V>> {
        match self.classify(difference)? {
            TruthValue::True => V::from_f64(1.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert conditional true value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::False => V::from_f64(0.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert conditional false value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::Undefined => Ok(None),
        }
    }

    /// 生成指示变量约束 / Build indicator constraints
    pub fn indicator_constraints(
        &self,
        indicator_index: usize,
    ) -> Result<Vec<LinearInequality<V>>> {
        relation_indicator_constraints(
            &self.condition,
            indicator_index,
            self.relation,
            &self.bounds,
            &self.strict_boundary,
        )
    }
}

/// 闭区间条件描述器 / Closed-interval condition descriptor
#[derive(Debug, Clone)]
pub struct IfInRangeFunction<V> {
    /// 下侧条件 / Lower-side condition
    pub lower: ConditionalIfFunction<V>,
    /// 上侧条件 / Upper-side condition
    pub upper: ConditionalIfFunction<V>,
}

fn interval_side_endpoint<V>(
    side: &ConditionalIfFunction<V>,
    side_name: &str,
    positive_coefficient: bool,
) -> Result<(usize, f64)>
where
    V: Clone + Debug + ToPrimitive,
{
    validate_condition_descriptor(
        &side.condition,
        side.relation,
        &side.strict_boundary,
        &side.bounds,
        &format!("if_in_range {side_name} condition"),
    )?;

    let monomials = side.condition.monomials();
    if monomials.len() != 1 {
        return Err(ModelError::InvalidConstraint(format!(
            "if_in_range {side_name} condition must contain exactly one variable monomial"
        ))
        .into());
    }

    let monomial = &monomials[0];
    let coefficient = finite_value(
        monomial.coefficient(),
        &format!("if_in_range {side_name} condition coefficient"),
    )?;
    if (positive_coefficient && coefficient <= 0.0) || (!positive_coefficient && coefficient >= 0.0)
    {
        let expected_sign = if positive_coefficient {
            "positive"
        } else {
            "negative"
        };
        return Err(ModelError::InvalidConstraint(format!(
            "if_in_range {side_name} condition coefficient must be {expected_sign}"
        ))
        .into());
    }

    let constant = finite_value(
        side.condition.constant_term(),
        &format!("if_in_range {side_name} condition constant"),
    )?;
    let endpoint = finite_value_from_f64(
        -constant / coefficient,
        &format!("if_in_range {side_name} endpoint"),
    )?;
    Ok((monomial.var_index(), endpoint))
}

pub(crate) fn validate_if_in_range<V>(
    lower: &ConditionalIfFunction<V>,
    upper: &ConditionalIfFunction<V>,
) -> Result<()>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    if lower.relation != ConditionRelation::GreaterEqual
        || upper.relation != ConditionRelation::GreaterEqual
    {
        return Err(ModelError::InvalidConstraint(
            "if_in_range requires greater-equal relations on both interval sides".to_string(),
        )
        .into());
    }

    let (lower_variable, lower_endpoint) = interval_side_endpoint(lower, "lower", true)?;
    let (upper_variable, upper_endpoint) = interval_side_endpoint(upper, "upper", false)?;
    if lower_variable != upper_variable {
        return Err(ModelError::InvalidConstraint(
            "if_in_range lower and upper conditions must use the same variable".to_string(),
        )
        .into());
    }
    if lower_endpoint > upper_endpoint {
        return Err(ModelError::InvalidConstraint(
            "if_in_range lower endpoint must not exceed upper endpoint".to_string(),
        )
        .into());
    }
    Ok(())
}

impl<V> IfInRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建闭区间条件 / Create a closed-interval condition
    pub fn new(lower: ConditionalIfFunction<V>, upper: ConditionalIfFunction<V>) -> Result<Self> {
        validate_if_in_range(&lower, &upper)?;
        Ok(Self { lower, upper })
    }

    /// 合并两侧分类 / Combine the two side classifications
    pub fn classify(&self, lower_difference: &V, upper_difference: &V) -> Result<TruthValue> {
        let lower = self.lower.classify(lower_difference)?;
        let upper = self.upper.classify(upper_difference)?;
        Ok(match (lower, upper) {
            (TruthValue::False, _) | (_, TruthValue::False) => TruthValue::False,
            (TruthValue::True, TruthValue::True) => TruthValue::True,
            _ => TruthValue::Undefined,
        })
    }
}

impl<V> ConditionBounds<V>
where
    V: Clone + Debug + ToPrimitive,
{
    /// 校验范围可用于求解器线性化 / Validate bounds for solver linearization
    pub fn validate(&self) -> Result<()> {
        let lower = finite_value(&self.lower, "condition lower bound")?;
        let upper = finite_value(&self.upper, "condition upper bound")?;
        if lower > upper {
            return Err(ModelError::InvalidConstraint(
                "condition lower bound must not exceed upper bound".to_string(),
            )
            .into());
        }
        Ok(())
    }
}

/// 校验关系条件的范围与可判定性 / Validate relation-condition bounds and decidability
///
/// 除了检查范围有限且有序，还拒绝整个范围都落在 `Undefined` 间隔内的条件。
/// In addition to finite ordered bounds, reject a range that lies entirely inside the
/// relation's `Undefined` interval.
pub fn validate_relation_bounds<V>(
    bounds: &ConditionBounds<V>,
    relation: ConditionRelation,
    strict_boundary: &V,
) -> Result<()>
where
    V: Clone + Debug + ToPrimitive,
{
    bounds.validate()?;
    let lower = finite_value(&bounds.lower, "condition lower bound")?;
    let upper = finite_value(&bounds.upper, "condition upper bound")?;
    let boundary = finite_value(strict_boundary, "strict boundary")?;
    if boundary <= 0.0 {
        return Err(ModelError::InvalidConstraint(
            "strict boundary must be finite and positive".to_string(),
        )
        .into());
    }

    let (undefined_lower, undefined_upper) = match relation {
        ConditionRelation::Greater | ConditionRelation::LessEqual => (0.0, boundary),
        ConditionRelation::GreaterEqual | ConditionRelation::Less => (-boundary, 0.0),
    };
    if lower > undefined_lower && upper < undefined_upper {
        return Err(ModelError::InvalidConstraint(format!(
            "condition bounds [{lower}, {upper}] lie entirely inside the Undefined interval for relation {relation:?}"
        ))
        .into());
    }
    Ok(())
}

/// 根据关系和严格边界对值分类 / Classify a value by relation and strict boundary
pub fn classify<V>(
    difference: &V,
    relation: ConditionRelation,
    strict_boundary: &V,
) -> Result<TruthValue>
where
    V: ToPrimitive,
{
    let difference = finite_value(difference, "condition difference")?;
    let boundary = finite_value(strict_boundary, "strict boundary")?;
    if boundary <= 0.0 {
        return Err(ModelError::InvalidConstraint(
            "strict boundary must be finite and positive".to_string(),
        )
        .into());
    }

    Ok(match relation {
        ConditionRelation::Greater => {
            if difference >= boundary {
                TruthValue::True
            } else if difference <= 0.0 {
                TruthValue::False
            } else {
                TruthValue::Undefined
            }
        }
        ConditionRelation::GreaterEqual => {
            if difference >= 0.0 {
                TruthValue::True
            } else if difference <= -boundary {
                TruthValue::False
            } else {
                TruthValue::Undefined
            }
        }
        ConditionRelation::Less => {
            if difference <= -boundary {
                TruthValue::True
            } else if difference >= 0.0 {
                TruthValue::False
            } else {
                TruthValue::Undefined
            }
        }
        ConditionRelation::LessEqual => {
            if difference <= 0.0 {
                TruthValue::True
            } else if difference >= boundary {
                TruthValue::False
            } else {
                TruthValue::Undefined
            }
        }
    })
}

/// 判断有限范围是否固定在单一分支 / Determine whether finite bounds select one branch
pub fn branch_coverage<V>(
    bounds: &ConditionBounds<V>,
    relation: ConditionRelation,
    strict_boundary: &V,
) -> Result<Option<TruthValue>>
where
    V: Clone + Debug + ToPrimitive,
{
    validate_relation_bounds(bounds, relation, strict_boundary)?;
    let lower = finite_value(&bounds.lower, "condition lower bound")?;
    let upper = finite_value(&bounds.upper, "condition upper bound")?;
    let boundary = finite_value(strict_boundary, "strict boundary")?;
    Ok(match relation {
        ConditionRelation::Greater if lower >= boundary => Some(TruthValue::True),
        ConditionRelation::Greater if upper <= 0.0 => Some(TruthValue::False),
        ConditionRelation::GreaterEqual if lower >= 0.0 => Some(TruthValue::True),
        ConditionRelation::GreaterEqual if upper <= -boundary => Some(TruthValue::False),
        ConditionRelation::Less if upper <= -boundary => Some(TruthValue::True),
        ConditionRelation::Less if lower >= 0.0 => Some(TruthValue::False),
        ConditionRelation::LessEqual if upper <= 0.0 => Some(TruthValue::True),
        ConditionRelation::LessEqual if lower >= boundary => Some(TruthValue::False),
        _ => None,
    })
}

/// 生成范围驱动的关系指示约束 / Build range-driven relation-indicator constraints
///
/// 返回的两条约束精确编码 `indicator = 1` 的真分支和 `indicator = 0` 的假分支。
/// The returned rows encode the true branch for `indicator = 1` and the false
/// branch for `indicator = 0`.
pub fn relation_indicator_constraints<V>(
    polynomial: &Linear<V>,
    indicator_index: usize,
    relation: ConditionRelation,
    bounds: &ConditionBounds<V>,
    strict_boundary: &V,
) -> Result<Vec<LinearInequality<V>>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    validate_linear_polynomial(polynomial, "condition")?;
    validate_relation_bounds(bounds, relation, strict_boundary)?;
    let lower = finite_value(&bounds.lower, "condition lower bound")?;
    let upper = finite_value(&bounds.upper, "condition upper bound")?;
    let boundary = finite_value(strict_boundary, "strict boundary")?;

    let (lower_rhs, lower_indicator, upper_rhs, upper_indicator) =
        relation_linearization_values(relation, lower, upper, boundary)?;
    let indicator_coefficient = |coefficient: f64| -> Result<V> {
        let converted = V::from_f64(coefficient)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "failed to convert conditional indicator coefficient".to_string(),
                )
            })
            .map_err(CoreError::from)?;
        finite_value(&converted, "conditional indicator coefficient")?;
        Ok(converted)
    };
    let rhs = |value: f64| -> Result<V> {
        let converted = V::from_f64(value)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "failed to convert conditional constraint bound".to_string(),
                )
            })
            .map_err(CoreError::from)?;
        finite_value(&converted, "conditional constraint bound")?;
        Ok(converted)
    };
    let row = |coefficient: f64| -> Result<Linear<V>> {
        let mut monomials = polynomial.monomials().to_vec();
        monomials.push(LinearMonomial::new(
            indicator_coefficient(coefficient)?,
            indicator_index,
        ));
        Ok(Linear::new(monomials, polynomial.constant_term().clone()))
    };

    Ok(vec![
        LinearInequality::greater_equal(
            row(finite_negate(
                lower_indicator,
                "lower indicator coefficient",
            )?)?,
            rhs(lower_rhs)?,
        ),
        LinearInequality::less_equal(
            row(finite_negate(
                upper_indicator,
                "upper indicator coefficient",
            )?)?,
            rhs(upper_rhs)?,
        ),
    ])
}

fn validate_linear_polynomial<V>(polynomial: &Linear<V>, name: &str) -> Result<()>
where
    V: ToPrimitive,
{
    finite_value(polynomial.constant_term(), &format!("{name} constant"))?;
    for monomial in polynomial.monomials() {
        finite_value(monomial.coefficient(), &format!("{name} coefficient"))?;
    }
    Ok(())
}

fn finite_add(lhs: f64, rhs: f64, name: &str) -> Result<f64> {
    finite_value_from_f64(lhs + rhs, name)
}

fn finite_sub(lhs: f64, rhs: f64, name: &str) -> Result<f64> {
    finite_value_from_f64(lhs - rhs, name)
}

fn finite_negate(value: f64, name: &str) -> Result<f64> {
    finite_value_from_f64(-value, name)
}

fn finite_value_from_f64(value: f64, name: &str) -> Result<f64> {
    if !value.is_finite() {
        return Err(
            ModelError::InvalidConstraint(format!("conditional {name} must be finite")).into(),
        );
    }
    Ok(value)
}

fn validate_condition_descriptor<V>(
    condition: &Linear<V>,
    relation: ConditionRelation,
    strict_boundary: &V,
    bounds: &ConditionBounds<V>,
    name: &str,
) -> Result<()>
where
    V: Clone + Debug + ToPrimitive,
{
    validate_linear_polynomial(condition, name)?;
    validate_relation_bounds(bounds, relation, strict_boundary)?;
    let lower = finite_value(&bounds.lower, "condition lower bound")?;
    let upper = finite_value(&bounds.upper, "condition upper bound")?;
    let boundary = finite_value(strict_boundary, "strict boundary")?;
    relation_linearization_values(relation, lower, upper, boundary)?;
    Ok(())
}

fn relation_linearization_values(
    relation: ConditionRelation,
    lower: f64,
    upper: f64,
    boundary: f64,
) -> Result<(f64, f64, f64, f64)> {
    finite_value_from_f64(lower, "lower bound")?;
    finite_value_from_f64(upper, "upper bound")?;
    finite_value_from_f64(boundary, "strict boundary")?;
    let values = match relation {
        ConditionRelation::Greater => (
            lower,
            finite_sub(boundary, lower, "greater lower indicator")?,
            0.0,
            upper,
        ),
        ConditionRelation::GreaterEqual => (
            lower,
            finite_negate(lower, "greater-equal lower indicator")?,
            finite_negate(boundary, "greater-equal upper rhs")?,
            finite_add(upper, boundary, "greater-equal upper indicator")?,
        ),
        ConditionRelation::Less => (
            0.0,
            lower,
            upper,
            finite_sub(
                finite_negate(boundary, "less upper boundary")?,
                upper,
                "less upper indicator",
            )?,
        ),
        ConditionRelation::LessEqual => (
            boundary,
            finite_sub(lower, boundary, "less-equal lower indicator")?,
            upper,
            finite_negate(upper, "less-equal upper indicator")?,
        ),
    };
    for (value, name) in [
        (values.0, "lower rhs"),
        (values.1, "lower indicator"),
        (values.2, "upper rhs"),
        (values.3, "upper indicator"),
    ] {
        finite_value_from_f64(value, name)?;
    }
    Ok(values)
}

fn finite_value<V>(value: &V, name: &str) -> Result<f64>
where
    V: ToPrimitive,
{
    let value = value.to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(format!("{name} cannot be converted to f64"))
    })?;
    if !value.is_finite() {
        return Err(ModelError::InvalidConstraint(format!("{name} must be finite")).into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_relation_matrix() {
        let g = 0.1;
        assert_eq!(
            classify(&0.1, ConditionRelation::Greater, &g).unwrap(),
            TruthValue::True
        );
        assert_eq!(
            classify(&0.05, ConditionRelation::Greater, &g).unwrap(),
            TruthValue::Undefined
        );
        assert_eq!(
            classify(&0.0, ConditionRelation::Greater, &g).unwrap(),
            TruthValue::False
        );
        assert_eq!(
            classify(&0.0, ConditionRelation::GreaterEqual, &g).unwrap(),
            TruthValue::True
        );
        assert_eq!(
            classify(&-0.05, ConditionRelation::GreaterEqual, &g).unwrap(),
            TruthValue::Undefined
        );
        assert_eq!(
            classify(&-0.1, ConditionRelation::GreaterEqual, &g).unwrap(),
            TruthValue::False
        );
        assert_eq!(
            classify(&-0.1, ConditionRelation::Less, &g).unwrap(),
            TruthValue::True
        );
        assert_eq!(
            classify(&-0.05, ConditionRelation::Less, &g).unwrap(),
            TruthValue::Undefined
        );
        assert_eq!(
            classify(&0.0, ConditionRelation::Less, &g).unwrap(),
            TruthValue::False
        );
        assert_eq!(
            classify(&0.0, ConditionRelation::LessEqual, &g).unwrap(),
            TruthValue::True
        );
    }

    #[test]
    fn rejects_invalid_bounds_and_equality() {
        assert!(ConditionBounds {
            lower: 2.0,
            upper: 1.0
        }
        .validate()
        .is_err());
        assert!(ConditionRelation::try_from(Comparison::Equal).is_err());
        assert!(classify(&0.0, ConditionRelation::Greater, &0.0).is_err());
    }

    #[test]
    fn rejects_ranges_entirely_inside_the_undefined_gap() {
        let cases = [
            (ConditionRelation::Greater, 0.01, 0.09),
            (ConditionRelation::GreaterEqual, -0.09, -0.01),
            (ConditionRelation::Less, -0.09, -0.01),
            (ConditionRelation::LessEqual, 0.01, 0.09),
        ];
        for (relation, lower, upper) in cases {
            let bounds = ConditionBounds { lower, upper };
            assert!(validate_relation_bounds(&bounds, relation, &0.1).is_err());
            assert!(branch_coverage(&bounds, relation, &0.1).is_err());
        }
        let bounds = ConditionBounds {
            lower: 0.01,
            upper: 0.09,
        };
        assert!(ConditionalIfFunction::new(
            Linear::constant(0.0),
            ConditionRelation::Greater,
            0.1,
            bounds,
        )
        .is_err());

        let false_bounds = ConditionBounds {
            lower: -2.0,
            upper: 0.0,
        };
        assert_eq!(
            branch_coverage(&false_bounds, ConditionRelation::Greater, &0.1).unwrap(),
            Some(TruthValue::False),
        );
    }

    #[test]
    fn descriptor_rejects_non_finite_coefficients_and_constant() {
        let bounds = ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        };
        assert!(ConditionalIfFunction::new(
            Linear::constant(f64::NAN),
            ConditionRelation::Greater,
            0.1,
            bounds.clone(),
        )
        .is_err());
        assert!(ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(f64::INFINITY, 0)], 0.0),
            ConditionRelation::Greater,
            0.1,
            bounds,
        )
        .is_err());
    }

    #[test]
    fn linearization_rejects_non_finite_intermediate_arithmetic() {
        let bounds = ConditionBounds {
            lower: -1.0,
            upper: f64::MAX,
        };
        assert!(relation_indicator_constraints(
            &Linear::constant(0.0),
            7,
            ConditionRelation::GreaterEqual,
            &bounds,
            &f64::MAX,
        )
        .is_err());
        assert!(ConditionalIfFunction::new(
            Linear::constant(0.0),
            ConditionRelation::GreaterEqual,
            f64::MAX,
            bounds,
        )
        .is_err());
    }
}
