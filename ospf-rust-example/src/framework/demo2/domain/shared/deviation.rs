//! 有界绝对偏差惩罚；边界来自同一源变量，不使用任意 Big-M。 / Bounded absolute-deviation penalties; bounds derive from source variables, never arbitrary Big-M.

use ospf_rust_core::error::Result;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{UContinuousVariableItem, VariableRange};
use super::loading_context::invalid;

pub(crate) fn register(
    model: &mut MetaModel,
    terms: &[(usize, f64)],
    target: f64,
    name: &str,
) -> Result<usize> {
    // 每次运算向外舍入，保证不会用偏小的 f64 上界截掉合法源解。
    // Round every interval operation outward to avoid excluding valid source solutions with an underestimated f64 bound.
    let mut lower = -target;
    let mut upper = -target;
    for (column, coefficient) in terms {
        let range = model.variable_range_by_index(*column).ok_or_else(|| invalid("missing deviation variable"))?;
        let (Some(lb), Some(ub)) = (range.lower_bound, range.upper_bound) else { return Err(invalid("deviation requires finite physical variable bounds")); };
        let (lb, ub) = if *coefficient >= 0.0 { (lb, ub) } else { (ub, lb) };
        lower = (lower + (coefficient * lb).next_down()).next_down();
        upper = (upper + (coefficient * ub).next_up()).next_up();
    }
    let bound = lower.abs().max(upper.abs()).next_up();
    if !bound.is_finite() { return Err(invalid("deviation physical bound overflow")); }
    let deviation = model.register_variable(UContinuousVariableItem::auto(name))?;
    model.set_variable_range_by_index(deviation, VariableRange::bounded(0.0, bound))?;
    for sign in [-1.0, 1.0] {
        let mut row = terms.iter().map(|(column, coefficient)| (*column, sign * coefficient)).collect::<Vec<_>>();
        row.push((deviation, -1.0));
        model.add_linear_constraint(&row, ConstraintRelation::LessEqual, sign * target, &format!("{name}_{sign}"))?;
    }
    Ok(deviation)
}
