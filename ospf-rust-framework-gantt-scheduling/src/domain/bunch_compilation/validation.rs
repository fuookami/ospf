//! 求解器边界的真实矩阵、整数及目标校验。 / Actual matrix, integrality and objective checks at the solver boundary.

use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::evaluate_linear_solution;
use crate::{GanttError, GanttResult};

pub(super) const SOLUTION_TOLERANCE: f64 = 1e-7;

/// 校验完整后端向量，包括下游扩展；LP 调用方须传松弛后的模型。 / Validate the complete backend vector including downstream extensions; LP callers must pass the relaxed model.
pub fn validate_bunch_incumbent(
    model: &LinearTriadModel,
    values: &[f64],
    reported_objective: Option<f64>,
) -> GanttResult<f64> {
    let diagnostics = evaluate_linear_solution(model, values, SOLUTION_TOLERANCE)
        .map_err(|error| invalid(error.to_string()))?;
    if diagnostics.variable_bound_evaluations.iter().any(|item| !item.satisfied)
        || diagnostics.constraint_evaluations.iter().any(|item| !item.satisfied)
    {
        return Err(invalid("incumbent violates model bounds or rows"));
    }
    if model.var_types.iter().zip(values).any(|(kind, value)| kind.is_integer()
        && (value - value.round()).abs() > SOLUTION_TOLERANCE)
    {
        return Err(invalid("MILP incumbent has fractional integer variables"));
    }
    if model.c.len() != values.len() { return Err(invalid("objective dimension mismatch")); }
    let mut objective = 0.0;
    for (coefficient, value) in model.c.iter().zip(values) {
        if !coefficient.is_finite() { return Err(invalid("non-finite objective coefficient")); }
        objective += coefficient * value;
        if !objective.is_finite() { return Err(invalid("objective arithmetic is non-finite")); }
    }
    if let Some(reported) = reported_objective {
        let tolerance = SOLUTION_TOLERANCE * objective.abs().max(reported.abs()).max(1.0);
        if !reported.is_finite() || !tolerance.is_finite() || (objective - reported).abs() > tolerance {
            return Err(invalid("reported objective differs from the actual model and incumbent"));
        }
    }
    Ok(objective)
}

fn invalid(message: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法任务束求解结果 / invalid bunch incumbent: {message}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
    use ospf_rust_core::model::object::SubObjective;
    use ospf_rust_core::variable::{Binary, VariableRange};

    fn fixture() -> LinearTriadModel {
        let mut model = MetaModel::<f64>::new("incumbent-validation");
        let x = model.register_auto_variable_with_range::<Binary>("extension", VariableRange::bounded(0.0, 1.0)).unwrap();
        model.add_ge_constraint(&[(x, 1.0)], 0.25, "extension_rule").unwrap();
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(-2.0, x)], 0.0), "extension_cost"));
        model.try_to_linear_triad_model().unwrap()
    }

    #[test]
    fn rejects_missing_nonfinite_fractional_and_infeasible_incumbents() {
        let model = fixture();
        for values in [vec![], vec![1.0, 0.0], vec![f64::NAN], vec![f64::INFINITY], vec![f64::NEG_INFINITY], vec![-1.0], vec![2.0], vec![0.5], vec![0.0]] {
            assert!(validate_bunch_incumbent(&model, &values, None).is_err(), "{values:?}");
        }
        assert_eq!(validate_bunch_incumbent(&model, &[1.0], Some(-2.0)).unwrap(), -2.0);
        let mut relaxed = model;
        relaxed.linear_relax();
        assert_eq!(validate_bunch_incumbent(&relaxed, &[0.5], Some(-1.0)).unwrap(), -1.0);
    }

    #[test]
    fn rejects_wrong_nonfinite_and_overflowing_objectives() {
        let mut model = fixture();
        for reported in [0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(validate_bunch_incumbent(&model, &[1.0], Some(reported)).is_err());
        }
        for coefficient in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            model.c[0] = coefficient;
            assert!(validate_bunch_incumbent(&model, &[1.0], None).is_err());
        }
        model.c.clear();
        assert!(validate_bunch_incumbent(&model, &[1.0], None).is_err());
        let mut model = fixture();
        model.c[0] = f64::MAX;
        model.basic.ub[0] = 2.0;
        assert!(validate_bunch_incumbent(&model, &[2.0], None).is_err());
    }
}
