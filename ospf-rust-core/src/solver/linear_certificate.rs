//! 含完整变量边界的 Farkas 证书复核。 / Farkas certificate verification including all variable bounds.

use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use crate::error::{CoreError, Result, SolverError};
use crate::model::intermediate::LinearTriadModel;
use super::audit::validate_linear_model_for_backend;

fn invalid(message: &str) -> CoreError {
    SolverError::ContractViolation(format!("linear certificate: {message}")).into()
}

fn exact(value: f64) -> Result<BigRational> {
    BigRational::from_float(value).ok_or_else(|| invalid("nonfinite rational operand"))
}

/// 精确验证二进制 f64 最优对偶的驻点和边界界，容差仅用于最终目标间隙。
/// Exactly verify binary-f64 optimal-dual stationarity and bound support; tolerance applies only to the final objective gap.
pub(crate) fn validate_linear_optimal_dual(
    model: &LinearTriadModel,
    values: &[f64],
    dual: &[f64],
    reported_objective: Option<f64>,
) -> Result<()> {
    use crate::model::ObjectiveCategory;
    validate_linear_model_for_backend(model)?;
    let minimizing = model.objective_category == ObjectiveCategory::Minimum;
    if model.has_integer_variables() || values.len() != model.num_variables() || dual.len() != model.num_constraints()
        || values.iter().any(|value| !value.is_finite())
        || dual.iter().any(|value| !value.is_finite() || if minimizing { *value > 0.0 } else { *value < 0.0 })
    { return Err(invalid("invalid optimal-dual dimensions, domain or signs")); }
    let mut activity = vec![BigRational::zero(); model.num_variables()];
    let mut bound_objective = BigRational::zero();
    let tolerance = exact(1e-7)?;
    for (index, row) in model.A.rows.iter().enumerate() {
        let multiplier = exact(dual[index])?;
        bound_objective += &multiplier * exact(model.b[index])?;
        let mut primal_activity = BigRational::zero();
        for (column, coefficient) in &row.entries {
            let coefficient = exact(*coefficient)?;
            activity[*column] += &multiplier * &coefficient;
            primal_activity += coefficient * exact(values[*column])?;
        }
        if primal_activity > exact(model.b[index])? + &tolerance { return Err(invalid("optimal-dual incumbent violates an exact source row")); }
    }
    let mut primal = BigRational::zero();
    for (column, activity) in activity.into_iter().enumerate() {
        let value = values[column];
        let lower = model.lb[column];
        let upper = model.ub[column];
        let value = exact(value)?;
        if (lower.is_finite() && value < exact(lower)? - &tolerance) || (upper.is_finite() && value > exact(upper)? + &tolerance) {
            return Err(invalid("optimal-dual incumbent violates bounds"));
        }
        let cost = exact(model.c[column])?;
        primal += &cost * value;
        let residual = cost - activity;
        if residual.is_zero() { continue; }
        let bound = if (residual > BigRational::zero()) == minimizing { lower } else { upper };
        if !bound.is_finite() { return Err(invalid("nonzero optimal-dual residual has an unbounded extremal direction")); }
        bound_objective += residual * exact(bound)?;
    }
    if let Some(reported) = reported_objective {
        let reported = exact(reported)?;
        let magnitude = primal.abs().max(reported.abs()).max(BigRational::one());
        if (&primal - reported).abs() > &tolerance * magnitude { return Err(invalid("reported objective differs from exact primal evaluation")); }
    }
    let gap = (&primal - &bound_objective).abs();
    let magnitude = primal.abs().max(bound_objective.abs()).max(BigRational::one());
    if gap > tolerance * magnitude { return Err(invalid("optimal-dual extremal-bound gap is not closed")); }
    Ok(())
}

/// 验证 Ax≤b、l≤x≤u 的非负行射线，并返回归一化后的严格矛盾间隙。
/// Verify a nonnegative row ray for Ax≤b, l≤x≤u and return its normalized strict contradiction margin.
///
/// 独立计算 min{(λA)x:l≤x≤u}−λb，不能仅因存在有限边界就接受射线。
/// Independently evaluate min{(λA)x:l≤x≤u}−λb; finite bounds alone never validate a ray.
/// 无界最小化方向上的任何非零残差均拒绝；容差仅用于严格矛盾间隙。
/// Reject every nonzero residual in an unbounded minimizing direction; tolerance applies only to the strict contradiction margin.
pub fn validate_linear_farkas_ray(
    model: &LinearTriadModel,
    ray: &[f64],
    tolerance: f64,
) -> Result<f64> {
    validate_linear_model_for_backend(model)?;
    if model.has_integer_variables() || ray.len() != model.num_constraints()
        || !tolerance.is_finite() || tolerance <= 0.0 || tolerance >= 0.5
        || ray.iter().any(|value| !value.is_finite() || *value < 0.0)
    { return Err(invalid("invalid continuous model, ray dimensions/signs or tolerance")); }
    let scale = ray.iter().copied().fold(0.0, f64::max);
    if scale == 0.0 { return Err(invalid("zero ray")); }
    // 精确累加二进制浮点值，避免归一化下溢或抵消把非零驻点残差抹成零。
    // Accumulate binary float values exactly so normalization underflow or cancellation cannot erase a nonzero stationarity residual.
    let scale = exact(scale)?;
    let mut activity = vec![BigRational::zero(); model.num_variables()];
    let mut rhs = BigRational::zero();
    for (index, row) in model.A.rows.iter().enumerate() {
        let multiplier = exact(ray[index])? / &scale;
        rhs += &multiplier * exact(model.b[index])?;
        for (column, coefficient) in &row.entries { activity[*column] += &multiplier * exact(*coefficient)?; }
    }
    let mut minimum = BigRational::zero();
    for (column, coefficient) in activity.into_iter().enumerate() {
        if coefficient.is_zero() { continue; }
        let bound = if coefficient > BigRational::zero() { model.lb[column] } else { model.ub[column] };
        if bound.is_finite() { minimum += coefficient * exact(bound)?; }
        else { return Err(invalid("ray has an unbounded minimizing direction, including a nonzero sub-tolerance residual")); }
    }
    let margin = &minimum - &rhs;
    let magnitude = minimum.abs().max(rhs.abs()).max(BigRational::one());
    if margin <= exact(tolerance)? * magnitude
    { return Err(invalid("ray does not establish a strict bound-aware contradiction")); }
    margin.to_f64().filter(|value| value.is_finite() && *value > 0.0).ok_or_else(|| invalid("contradiction margin cannot be represented as finite f64"))
}

#[cfg(test)]
mod tests {
    use crate::model::{MetaModel, SubObjective};
    use crate::model::flatten::Linear;
    use crate::variable::{Continuous, VariableRange};
    use super::*;

    #[test]
    fn finite_bounds_are_not_a_certificate_and_one_sided_bounds_contribute() {
        let mut model = MetaModel::new("bounded_ray");
        let x = model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(2.0, 4.0)).unwrap();
        model.add_le_constraint(&[(x, 1.0)], 3.0, "feasible").unwrap();
        model.add_sub_objective(SubObjective::minimize(Linear::constant(0.0), "zero"));
        let mut compiled = model.linear_model_snapshot().unwrap().compile_flt64().unwrap().model;
        assert!(validate_linear_farkas_ray(&compiled, &[1.0], 1e-7).is_err());
        compiled.basic.b[0] = 1.0;
        assert_eq!(validate_linear_farkas_ray(&compiled, &[1.0], 1e-7).unwrap(), 1.0);
        compiled.basic.ub[0] = f64::INFINITY;
        assert!(validate_linear_farkas_ray(&compiled, &[1.0], 1e-7).is_ok());
        compiled.basic.lb[0] = f64::NEG_INFINITY;
        assert!(validate_linear_farkas_ray(&compiled, &[1.0], 1e-7).is_err());
        assert!(validate_linear_farkas_ray(&compiled, &[-1.0], 1e-7).is_err());
        assert!(validate_linear_farkas_ray(&compiled, &[0.0], 1e-7).is_err());
    }

    #[test]
    fn tiny_nonzero_unbounded_residual_never_certifies_a_feasible_lp() {
        let mut model = MetaModel::new("tiny_free_residual");
        let x = model.register_auto_variable::<Continuous>("x").unwrap();
        model.add_le_constraint(&[(x, 1e-8)], -1.0, "feasible_at_minus_1e8").unwrap();
        model.add_sub_objective(SubObjective::minimize(Linear::constant(0.0), "zero"));
        let mut compiled = model.linear_model_snapshot().unwrap().compile_flt64().unwrap().model;
        assert!(super::super::audit::evaluate_linear_solution(&compiled, &[-1e8], 1e-7).unwrap().constraint_evaluations.iter().all(|row| row.satisfied));
        assert!(validate_linear_farkas_ray(&compiled, &[1.0], 1e-7).is_err());
        compiled.basic.lb[0] = -1e8;
        assert!(validate_linear_farkas_ray(&compiled, &[1.0], 1e-7).is_err());
        compiled.basic.lb[0] = 0.0;
        assert_eq!(validate_linear_farkas_ray(&compiled, &[1.0], 1e-7).unwrap(), 1.0);
    }

    #[test]
    fn tiny_unbounded_reduced_cost_never_certifies_an_optimal_lp() {
        use crate::solver::{SolveFingerprints, SolveProof, SolveReport, SolveSolution, ProblemStatus, TerminationReason,
            linear_model_fingerprint, linear_model_mapping, require_optimal_lp_certificate_for_linear_model};
        let mut model = MetaModel::new("tiny_free_cost");
        let x = model.register_auto_variable::<Continuous>("x").unwrap();
        model.add_le_constraint(&[], 0.0, "constant_row").unwrap();
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![crate::model::flatten::LinearMonomial::new(1e-8, x)], 0.0), "unbounded_cost"));
        let compiled = model.linear_model_snapshot().unwrap().compile_flt64().unwrap().model;
        let mut solution = SolveSolution::vector(vec![0.0]);
        solution.objective_value = Some(0.0);
        solution.dual_solution = Some(vec![0.0]);
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
            .model_mapping(Some(linear_model_mapping(&compiled).unwrap()))
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&compiled).unwrap()), ..Default::default() }).build().unwrap();
        assert!(require_optimal_lp_certificate_for_linear_model(&report, &compiled).is_err());
    }

    #[test]
    fn wide_finite_box_uses_extremal_bounds_for_min_and_max_dual_objectives() {
        use crate::model::ObjectiveCategory;
        use crate::solver::{SolveFingerprints, SolveProof, SolveReport, SolveSolution, ProblemStatus, TerminationReason,
            linear_model_fingerprint, linear_model_mapping, require_optimal_lp_certificate_for_linear_model};
        let mut model = MetaModel::new("wide_box");
        model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(0.0, 1e12)).unwrap();
        model.add_le_constraint(&[], 0.0, "constant_row").unwrap();
        model.add_sub_objective(SubObjective::minimize(Linear::constant(0.0), "zero"));
        for category in [ObjectiveCategory::Minimum, ObjectiveCategory::Maximum] {
            let mut compiled = model.linear_model_snapshot().unwrap().compile_flt64().unwrap().model;
            compiled.objective_category = category;
            compiled.c[0] = if category == ObjectiveCategory::Minimum { -1e-8 } else { 1e-8 };
            for candidate in [0.0, 1e12] {
                let mut solution = SolveSolution::vector(vec![candidate]);
                solution.objective_value = Some(compiled.c[0] * candidate);
                solution.dual_solution = Some(vec![0.0]);
                let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
                    .model_mapping(Some(linear_model_mapping(&compiled).unwrap()))
                    .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&compiled).unwrap()), ..Default::default() }).build().unwrap();
                assert_eq!(require_optimal_lp_certificate_for_linear_model(&report, &compiled).is_ok(), candidate == 1e12);
            }
        }
    }

    #[test]
    fn cancelling_large_row_activities_never_certifies_an_unbounded_lp_in_any_order() {
        use crate::model::flatten::LinearMonomial;
        use crate::solver::{SolveFingerprints, SolveProof, SolveReport, SolveSolution, ProblemStatus, TerminationReason,
            linear_model_fingerprint, linear_model_mapping, require_optimal_lp_certificate_for_linear_model};
        for order in [[0, 1, 2], [2, 1, 0], [1, 0, 2], [0, 2, 1]] {
            let mut model = MetaModel::new("cancelled_dual_activity");
            let y = model.register_auto_variable::<Continuous>("y").unwrap();
            let z = model.register_auto_variable::<Continuous>("z").unwrap();
            let rows = [[-1e16, -3e16], [-1.0, -3.0], [1e16, 3e16]];
            for index in order { model.add_le_constraint(&[(y, rows[index][0]), (z, rows[index][1])], 0.0, &format!("row_{index}")).unwrap(); }
            model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(4.0, z)], 0.0), "unbounded"));
            let compiled = model.linear_model_snapshot().unwrap().compile_flt64().unwrap().model;
            let mut solution = SolveSolution::vector(vec![0.0, 0.0]);
            solution.objective_value = Some(0.0);
            solution.dual_solution = Some(vec![-1.0; 3]);
            let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
                .model_mapping(Some(linear_model_mapping(&compiled).unwrap()))
                .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&compiled).unwrap()), ..Default::default() }).build().unwrap();
            assert!(require_optimal_lp_certificate_for_linear_model(&report, &compiled).is_err());
        }
    }

    #[test]
    fn exact_primal_rows_and_reported_objectives_cannot_be_hidden_by_cancellation() {
        use crate::model::flatten::LinearMonomial;
        use crate::solver::{SolveFingerprints, SolveProof, SolveReport, SolveSolution, ProblemStatus, TerminationReason,
            linear_model_fingerprint, linear_model_mapping, require_optimal_lp_certificate_for_linear_model};
        for objective_case in [false, true] {
            let mut model = MetaModel::new("primal_cancellation");
            for index in 0..3 { model.register_auto_variable_with_range::<Continuous>(&format!("x{index}"), VariableRange::bounded(if objective_case { 1.0 } else { 0.0 }, 1.0)).unwrap(); }
            let coefficients = [1e16, 1.0, -1e16];
            if objective_case {
                model.add_le_constraint(&[], 0.0, "constant_row").unwrap();
                model.add_sub_objective(SubObjective::minimize(Linear::new(coefficients.into_iter().enumerate().map(|(index, value)| LinearMonomial::new(value, index)).collect(), 0.0), "cost"));
            } else {
                model.add_le_constraint(&coefficients.into_iter().enumerate().collect::<Vec<_>>(), 0.0, "violated_row").unwrap();
                model.add_sub_objective(SubObjective::minimize(Linear::constant(0.0), "zero"));
            }
            let compiled = model.linear_model_snapshot().unwrap().compile_flt64().unwrap().model;
            for reported in if objective_case { vec![0.0, 1.0] } else { vec![0.0] } {
                let mut solution = SolveSolution::vector(vec![1.0; 3]);
                solution.objective_value = Some(reported);
                solution.dual_solution = Some(vec![0.0]);
                let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
                    .model_mapping(Some(linear_model_mapping(&compiled).unwrap()))
                    .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&compiled).unwrap()), ..Default::default() }).build().unwrap();
                assert_eq!(require_optimal_lp_certificate_for_linear_model(&report, &compiled).is_ok(), objective_case && reported == 1.0);
            }
        }
    }
}
