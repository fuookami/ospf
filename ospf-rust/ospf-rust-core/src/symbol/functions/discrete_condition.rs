//! 离散条件的关系转换 / Relation conversion for discrete conditions

use std::fmt::Debug;
use num_traits::{FromPrimitive, ToPrimitive};
use crate::error::{ModelError, Result};
use crate::symbol::flatten::Linear;
use crate::token::Token;
use super::conditional::ConditionRelation;

/// 严格正条件的线性化信息 / Strict-positive condition linearization
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscreteConditionLinearization {
    /// 原始差值的符号 / Sign applied to the original difference
    pub sign: f64,
    /// 加到归一化多项式上的常数 / Constant added to the normalized polynomial
    pub offset: f64,
}

/// 条件差值格点证明 / Lattice proof for a condition difference
///
/// 该证明只能通过 [`derive_discrete_lattice_proof`] 从整数变量、整数系数 gcd
/// 和常数余数推导得到；字段保持私有，避免调用方把未证明的步长伪装成证明。
/// This proof can only be derived by [`derive_discrete_lattice_proof`] from integer
/// variables, the coefficient gcd, and the constant remainder. Its fields are private
/// so an unproven step cannot masquerade as a proof.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscreteConditionLatticeProof {
    lattice_step: f64,
    difference_remainder: f64,
}

impl DiscreteConditionLatticeProof {
    /// 获取格点步长 / Get the lattice step.
    pub fn lattice_step(&self) -> f64 {
        self.lattice_step
    }

    /// 获取相对零的归一化余数 / Get the normalized remainder relative to zero.
    pub fn difference_remainder(&self) -> f64 {
        self.difference_remainder
    }
}

/// 从线性条件和 token 元数据推导格点证明 / Derive a lattice proof from a linear condition and token metadata
///
/// 所有参与变量必须是整数类型，所有系数必须是有限整数；步长取系数绝对值的 gcd，
/// 余数由多项式常数对步长归一化得到。任何连续变量、非整数系数或未知 token 都会失败。
/// Every participating variable must have an integer type and every coefficient must be a
/// finite integer. The step is the gcd of absolute coefficients, and the remainder
/// normalizes the polynomial constant modulo that step. Any continuous variable,
/// non-integer coefficient, or unknown token fails.
pub fn derive_discrete_lattice_proof<V>(
    condition: &Linear<V>,
    tokens: &[Token<V>],
) -> Result<DiscreteConditionLatticeProof>
where
    V: Clone + Debug + PartialEq + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    let mut gcd: u128 = 0;
    for monomial in condition.monomials() {
        let magnitude = exact_integer_magnitude(monomial.coefficient(), "condition coefficient")?;
        if magnitude == 0 {
            continue;
        }
        if magnitude > (1_u128 << 53) {
            return Err(ModelError::InvalidConstraint(
                "discrete lattice coefficient is outside the exactly representable f64 integer range"
                    .to_string(),
            )
            .into());
        }
        gcd = gcd_u128(gcd, magnitude);
    }
    if gcd == 0 {
        return Err(ModelError::InvalidConstraint(
            "discrete lattice proof requires at least one non-zero integer coefficient".to_string(),
        )
        .into());
    }

    for monomial in condition.monomials() {
        if exact_integer_magnitude(monomial.coefficient(), "condition coefficient")? == 0 {
            continue;
        }
        let token = tokens
            .iter()
            .find(|token| token.solver_index == monomial.var_index())
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "condition solver index {} is not present in the token list",
                    monomial.var_index()
                ))
            })?;
        if !token.var_type().is_integer() {
            return Err(ModelError::InvalidConstraint(format!(
                "condition variable `{}` is not integer",
                token.name()
            ))
            .into());
        }
    }

    let lattice_step = gcd as f64;
    let constant = exact_f64_value(condition.constant_term(), "condition constant")?;
    let mut remainder = constant % lattice_step;
    if remainder < 0.0 {
        remainder += lattice_step;
    }
    if !remainder.is_finite() || remainder < 0.0 || remainder >= lattice_step {
        return Err(ModelError::InvalidConstraint(
            "failed to normalize the discrete condition remainder".to_string(),
        )
        .into());
    }
    Ok(DiscreteConditionLatticeProof {
        lattice_step,
        difference_remainder: remainder,
    })
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left
}

fn exact_integer_magnitude<V>(value: &V, name: &str) -> Result<u128>
where
    V: FromPrimitive + PartialEq + ToPrimitive,
{
    let value_as_f64 = exact_f64_value(value, name)?;
    if value_as_f64.fract() != 0.0 {
        return Err(ModelError::InvalidConstraint(format!(
            "{name} must be an exactly representable integer"
        ))
        .into());
    }

    if value_as_f64 < 0.0 {
        value
            .to_i128()
            .and_then(|integer| integer.checked_abs())
            .map(|integer| integer as u128)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "{name} is outside the supported integer range"
                ))
                .into()
            })
    } else {
        value.to_u128().ok_or_else(|| {
            ModelError::InvalidConstraint(format!("{name} is outside the supported integer range"))
                .into()
        })
    }
}

fn exact_f64_value<V>(value: &V, name: &str) -> Result<f64>
where
    V: FromPrimitive + PartialEq + ToPrimitive,
{
    let value_as_f64 = finite_value(value, name)?;
    let round_trip = V::from_f64(value_as_f64).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "{name} cannot be checked for exact f64 representation"
        ))
    })?;
    if &round_trip != value {
        return Err(ModelError::InvalidConstraint(format!(
            "{name} cannot be represented exactly as f64"
        ))
        .into());
    }
    Ok(value_as_f64)
}

/// 校验离散条件参数 / Validate discrete-condition parameters
pub fn validate_discrete_parameters<V>(strict_boundary: &V, delta: &V) -> Result<()>
where
    V: ToPrimitive,
{
    let boundary = finite_value(strict_boundary, "strict boundary")?;
    let delta = finite_value(delta, "discrete delta")?;
    if boundary <= 0.0 || delta <= 0.0 || boundary > delta {
        return Err(ModelError::InvalidConstraint(
            "discrete conditions require 0 < strict_boundary <= delta".to_string(),
        )
        .into());
    }
    Ok(())
}

/// 校验推导得到的格点证明 / Validate a derived lattice proof
///
/// 该入口按 `GreaterEqual` 关系检查证明中的步长和余数。
/// This entry checks the proof step and remainder for the `GreaterEqual` relation.
pub fn validate_discrete_parameters_with_proven_lattice_step<V>(
    strict_boundary: &V,
    delta: &V,
    proof: &DiscreteConditionLatticeProof,
) -> Result<()>
where
    V: ToPrimitive,
{
    validate_discrete_parameters(strict_boundary, delta)?;
    let boundary = finite_value(strict_boundary, "strict boundary")?;
    let delta = finite_value(delta, "discrete delta")?;
    validate_lattice_proof_values(
        ConditionRelation::GreaterEqual,
        boundary,
        delta,
        proof.lattice_step,
        proof.difference_remainder,
    )
}

/// 转换为严格正条件 / Convert a relation into a strict-positive condition
///
/// 对 `d = lhs - rhs`，返回 `sign * d + offset > 0` 的离散等价形式。
/// For `d = lhs - rhs`, returns the discrete equivalent `sign * d + offset > 0`.
///
/// 这是只负责公式转换的底层入口；调用方若需要证明非单位 `delta` 的离散等价性，
/// 应使用 [`to_strict_positive_condition_with_proven_lattice_step`]。
/// This is the low-level formula-only entry point; callers that need a proof of
/// discrete equivalence for a non-unit `delta` should use
/// [`to_strict_positive_condition_with_proven_lattice_step`].
pub(crate) fn to_strict_positive_condition<V>(
    relation: ConditionRelation,
    delta: &V,
) -> Result<DiscreteConditionLinearization>
where
    V: ToPrimitive,
{
    let delta = finite_value(delta, "discrete delta")?;
    if delta <= 0.0 {
        return Err(ModelError::InvalidConstraint(
            "discrete delta must be finite and positive".to_string(),
        )
        .into());
    }
    Ok(match relation {
        ConditionRelation::Greater => DiscreteConditionLinearization {
            sign: 1.0,
            offset: 0.0,
        },
        ConditionRelation::GreaterEqual => DiscreteConditionLinearization {
            sign: 1.0,
            offset: delta,
        },
        ConditionRelation::Less => DiscreteConditionLinearization {
            sign: -1.0,
            offset: 0.0,
        },
        ConditionRelation::LessEqual => DiscreteConditionLinearization {
            sign: -1.0,
            offset: delta,
        },
    })
}

/// 使用格点证明转换为严格正条件 / Convert with a derived lattice proof
///
/// 该入口是非单位 `delta` 调用方应使用的受检路径。它按关系校验严格边界、离散步长、
/// 格点步长和余数，再复用四种关系的转换表。
/// This is the checked path for callers using a non-unit `delta`. It validates the strict
/// boundary, discrete step, lattice step, and remainder by relation before reusing the
/// four-relation conversion table.
pub fn to_strict_positive_condition_with_proven_lattice_step<V>(
    relation: ConditionRelation,
    strict_boundary: &V,
    delta: &V,
    proof: &DiscreteConditionLatticeProof,
) -> Result<DiscreteConditionLinearization>
where
    V: ToPrimitive,
{
    validate_discrete_parameters(strict_boundary, delta)?;
    validate_lattice_proof_values(
        relation,
        finite_value(strict_boundary, "strict boundary")?,
        finite_value(delta, "discrete delta")?,
        proof.lattice_step,
        proof.difference_remainder,
    )?;
    to_strict_positive_condition(relation, delta)
}

/// 使用推导证明转换离散条件 / Convert a discrete condition with a derived lattice proof
///
/// 该入口先从条件多项式和 token 元数据推导格点证明，再复用受检转换表。
/// This entry derives the lattice proof from the condition polynomial and token
/// metadata, then reuses the checked conversion table.
pub fn to_strict_positive_condition_with_derived_lattice_proof<V>(
    relation: ConditionRelation,
    strict_boundary: &V,
    delta: &V,
    condition: &Linear<V>,
    tokens: &[Token<V>],
) -> Result<DiscreteConditionLinearization>
where
    V: Clone + Debug + PartialEq + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    let proof = derive_discrete_lattice_proof(condition, tokens)?;
    to_strict_positive_condition_with_proven_lattice_step(relation, strict_boundary, delta, &proof)
}

/// 使用带余数的格点证明转换为严格正条件 / Convert with a remainder-aware lattice proof
///
/// 该入口适用于值格点与阈值不一定对齐的条件；调用方必须提供条件差值的归一化余数，
/// 函数会按关系检查严格边界和 `delta` 是否小于零两侧的最近可达格点距离。
/// Use this entry when the value lattice and threshold may not be aligned. The caller must
/// provide the normalized condition-difference remainder; the function checks the strict
/// boundary and `delta` against the nearest reachable lattice point on the relevant side.
fn validate_lattice_proof_values(
    relation: ConditionRelation,
    strict_boundary: f64,
    delta: f64,
    lattice_step: f64,
    difference_remainder: f64,
) -> Result<()> {
    if lattice_step <= 0.0 {
        return Err(ModelError::InvalidConstraint(
            "proven lattice step must be finite and positive".to_string(),
        )
        .into());
    }
    if delta > lattice_step {
        return Err(ModelError::InvalidConstraint(
            "discrete delta must not exceed the proven lattice step".to_string(),
        )
        .into());
    }
    if difference_remainder < 0.0 || difference_remainder >= lattice_step {
        return Err(ModelError::InvalidConstraint(
            "difference remainder must satisfy 0 <= remainder < proven lattice step".to_string(),
        )
        .into());
    }

    // 对 d = r + k*s，正侧最近格点距零为 r，负侧最近格点绝对距离为 s-r。
    // For d = r + k*s, the nearest positive distance is r and the nearest negative
    // absolute distance is s-r.
    let positive_distance = if difference_remainder == 0.0 {
        lattice_step
    } else {
        difference_remainder
    };
    let negative_distance = if difference_remainder == 0.0 {
        lattice_step
    } else {
        let distance = lattice_step - difference_remainder;
        if !distance.is_finite() {
            return Err(ModelError::InvalidConstraint(
                "negative lattice distance must be finite".to_string(),
            )
            .into());
        }
        distance
    };
    let (required_distance, available_distance, side) = match relation {
        ConditionRelation::Greater => (strict_boundary, positive_distance, "positive"),
        ConditionRelation::GreaterEqual => (delta, negative_distance, "negative"),
        ConditionRelation::Less => (strict_boundary, negative_distance, "negative"),
        ConditionRelation::LessEqual => (delta, positive_distance, "positive"),
    };
    if available_distance < required_distance {
        return Err(ModelError::InvalidConstraint(format!(
            "{relation:?} requires {side} lattice distance {available_distance} to be at least {required_distance}"
        ))
        .into());
    }
    Ok(())
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
    use crate::symbol::flatten::LinearMonomial;
    use crate::variable::{Continuous, Integer, VariableItem};
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    fn integer_tokens(count: usize) -> Vec<Token<f64>> {
        (0..count)
            .map(|index| {
                Token::from_generic(
                    VariableItem::<Integer>::auto(&format!("integer_{index}")),
                    index,
                )
            })
            .collect()
    }

    fn proof(condition: &Linear<f64>, tokens: &[Token<f64>]) -> DiscreteConditionLatticeProof {
        derive_discrete_lattice_proof(condition, tokens).unwrap()
    }

    fn transformed_difference(difference: f64, relation: ConditionRelation, delta: f64) -> f64 {
        let linearization = to_strict_positive_condition(relation, &delta).unwrap();
        linearization.sign * difference + linearization.offset
    }

    #[test]
    fn discrete_parameters_require_finite_positive_ordered_values() {
        assert!(validate_discrete_parameters(&0.1, &1.0).is_ok());
        assert!(validate_discrete_parameters(&1.0, &1.0).is_ok());

        for (strict_boundary, delta) in [
            (0.0, 1.0),
            (-0.1, 1.0),
            (f64::NAN, 1.0),
            (f64::INFINITY, 1.0),
            (0.1, 0.0),
            (0.1, -1.0),
            (0.1, f64::NAN),
            (0.1, f64::INFINITY),
            (2.0, 1.0),
        ] {
            assert!(validate_discrete_parameters(&strict_boundary, &delta).is_err());
        }
    }

    #[test]
    fn non_unit_delta_requires_a_derived_lattice_proof() {
        let tokens = integer_tokens(1);
        let condition = Linear::new(vec![LinearMonomial::new(5.0, 0)], 0.0);
        let lattice_proof = proof(&condition, &tokens);
        assert_eq!(lattice_proof.lattice_step(), 5.0);
        assert_eq!(lattice_proof.difference_remainder(), 0.0);

        assert!(
            validate_discrete_parameters_with_proven_lattice_step(&1e-6, &5.0, &lattice_proof,)
                .is_ok()
        );
        assert!(to_strict_positive_condition_with_proven_lattice_step(
            ConditionRelation::GreaterEqual,
            &1e-6,
            &5.0,
            &lattice_proof,
        )
        .is_ok());
    }

    #[test]
    fn lattice_proof_is_derived_and_cannot_overstate_the_real_step() {
        let tokens = integer_tokens(1);
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let lattice_proof = proof(&condition, &tokens);

        assert!(
            validate_discrete_parameters_with_proven_lattice_step(&1e-6, &5.0, &lattice_proof,)
                .is_err()
        );
        assert!(to_strict_positive_condition_with_proven_lattice_step(
            ConditionRelation::GreaterEqual,
            &1e-6,
            &5.0,
            &lattice_proof,
        )
        .is_err());
    }

    #[test]
    fn discrete_relation_conversion_table() {
        let delta = 1.0;
        assert_eq!(
            to_strict_positive_condition(ConditionRelation::Greater, &delta).unwrap(),
            DiscreteConditionLinearization {
                sign: 1.0,
                offset: 0.0
            }
        );
        assert_eq!(
            to_strict_positive_condition(ConditionRelation::GreaterEqual, &delta).unwrap(),
            DiscreteConditionLinearization {
                sign: 1.0,
                offset: 1.0
            }
        );
        assert_eq!(
            to_strict_positive_condition(ConditionRelation::Less, &delta).unwrap(),
            DiscreteConditionLinearization {
                sign: -1.0,
                offset: 0.0
            }
        );
        assert_eq!(
            to_strict_positive_condition(ConditionRelation::LessEqual, &delta).unwrap(),
            DiscreteConditionLinearization {
                sign: -1.0,
                offset: 1.0
            }
        );
    }

    #[test]
    fn integer_threshold_uses_delta_without_an_off_by_one_error() {
        let threshold = 10_i32;
        let delta = 1_i32;

        for (value, expected) in [(9_i32, false), (10_i32, true), (11_i32, true)] {
            let difference = f64::from(value - threshold);
            assert_eq!(
                transformed_difference(
                    difference,
                    ConditionRelation::GreaterEqual,
                    f64::from(delta),
                ) > 0.0,
                expected,
            );
        }
    }

    #[test]
    fn non_unit_delta_preserves_negative_coefficients_and_constant_offsets() {
        let tokens = integer_tokens(2);
        let condition = Linear::new(
            vec![LinearMonomial::new(-10.0, 0), LinearMonomial::new(5.0, 1)],
            0.0,
        );
        let difference = -2.0 * 3.0 + 7.0;

        let greater_equal = to_strict_positive_condition_with_derived_lattice_proof(
            ConditionRelation::GreaterEqual,
            &0.1,
            &5.0,
            &condition,
            &tokens,
        )
        .unwrap();
        assert_eq!(greater_equal.sign, 1.0);
        assert_eq!(greater_equal.offset, 5.0);
        assert!((greater_equal.sign * difference + greater_equal.offset) > 0.0);

        let less_equal = to_strict_positive_condition_with_derived_lattice_proof(
            ConditionRelation::LessEqual,
            &0.1,
            &5.0,
            &condition,
            &tokens,
        )
        .unwrap();
        assert_eq!(less_equal.sign, -1.0);
        assert_eq!(less_equal.offset, 5.0);
        assert!((less_equal.sign * difference + less_equal.offset) > 0.0);
    }

    #[test]
    fn lattice_proof_uses_coefficient_gcd_and_constant_remainder() {
        let tokens = integer_tokens(2);
        let condition = Linear::new(
            vec![LinearMonomial::new(10.0, 0), LinearMonomial::new(25.0, 1)],
            7.0,
        );
        let lattice_proof = proof(&condition, &tokens);

        assert_eq!(lattice_proof.lattice_step(), 5.0);
        assert_eq!(lattice_proof.difference_remainder(), 2.0);
    }

    #[test]
    fn lattice_proof_rejects_unprovable_conditions() {
        let integer_tokens = integer_tokens(2);
        let non_integer_coefficient = Linear::new(vec![LinearMonomial::new(2.5, 0)], 0.0);
        assert!(derive_discrete_lattice_proof(&non_integer_coefficient, &integer_tokens).is_err());

        let unknown_token = Linear::new(vec![LinearMonomial::new(1.0, integer_tokens.len())], 0.0);
        assert!(derive_discrete_lattice_proof(&unknown_token, &integer_tokens).is_err());

        let continuous_tokens = vec![Token::from_generic(
            VariableItem::<Continuous>::auto("continuous"),
            0,
        )];
        let continuous_condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        assert!(derive_discrete_lattice_proof(&continuous_condition, &continuous_tokens).is_err());

        let zero_coefficient_unknown_token = Linear::new(
            vec![
                LinearMonomial::new(5.0, 0),
                LinearMonomial::new(0.0, integer_tokens.len()),
            ],
            0.0,
        );
        assert_eq!(
            proof(&zero_coefficient_unknown_token, &integer_tokens).lattice_step(),
            5.0
        );
    }

    #[test]
    fn lattice_proof_rejects_high_precision_integer_coefficients() {
        let tokens = vec![Token::from_generic(
            VariableItem::<Integer>::auto("integer"),
            0,
        )];
        let coefficient = BigDecimal::from_str("9007199254740993").unwrap();
        let condition = Linear::new(
            vec![LinearMonomial::new(coefficient, 0)],
            BigDecimal::from(0),
        );

        assert!(derive_discrete_lattice_proof(&condition, &tokens).is_err());
    }

    #[test]
    fn derived_remainder_checks_both_relation_sides() {
        let tokens = integer_tokens(1);
        let condition = Linear::new(vec![LinearMonomial::new(5.0, 0)], 2.0);
        let cases = [
            (ConditionRelation::Greater, 2.0, true),
            (ConditionRelation::Greater, 3.0, false),
            (ConditionRelation::GreaterEqual, 3.0, true),
            (ConditionRelation::GreaterEqual, 4.0, false),
            (ConditionRelation::Less, 3.0, true),
            (ConditionRelation::Less, 4.0, false),
            (ConditionRelation::LessEqual, 2.0, true),
            (ConditionRelation::LessEqual, 3.0, false),
        ];
        for (relation, boundary, expected_ok) in cases {
            let result = to_strict_positive_condition_with_derived_lattice_proof(
                relation, &boundary, &boundary, &condition, &tokens,
            );
            assert_eq!(result.is_ok(), expected_ok, "{relation:?}, {boundary}");
        }
    }
}
