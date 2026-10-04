//! 全局约束函数工厂 / Global-constraint function factories

use crate::error::{ModelError, Result};
use crate::model::constraint_programming::{
    ConstraintProgrammingConstraint, CumulativeTask, IntegerExpression, IntervalVariableId,
};
use std::collections::BTreeSet;

/// 为 CP 模型构造常用全局约束 / Build common global constraints for a CP model.
pub struct GlobalConstraintFunctions;

impl GlobalConstraintFunctions {
    /// 要求整数表达式两两不同 / Require all integer expressions to have distinct values.
    pub fn all_different(expressions: Vec<IntegerExpression>) -> ConstraintProgrammingConstraint {
        ConstraintProgrammingConstraint::AllDifferent { expressions }
    }

    /// 要求列出的区间彼此不重叠 / Require the listed intervals to be pairwise non-overlapping.
    pub fn no_overlap(intervals: Vec<IntervalVariableId>) -> ConstraintProgrammingConstraint {
        ConstraintProgrammingConstraint::NoOverlap { intervals }
    }

    /// 要求固定需求任务在每个时点的总需求不超过容量。
    ///
    /// 此函数构造现有 CP AST；MIP lowerer 可通过 `MipLoweringOptions::allow_cumulative` 显式
    /// 启用有限整数时域 lowering，并受 slot/work budget 限制。
    /// Build a fixed-demand cumulative constraint. The MIP lowerer can opt in to finite integer
    /// time-slot lowering through `MipLoweringOptions::allow_cumulative`, subject to slot/work budgets.
    pub fn cumulative(
        tasks: Vec<CumulativeTask>,
        capacity: i64,
    ) -> Result<ConstraintProgrammingConstraint> {
        if capacity < 0 || tasks.iter().any(|task| task.demand < 0) {
            return Err(ModelError::InvalidConstraint(
                "cumulative capacity and task demands must be non-negative".to_owned(),
            )
            .into());
        }
        let mut interval_ids = BTreeSet::new();
        if tasks
            .iter()
            .any(|task| !interval_ids.insert(task.interval.clone()))
        {
            return Err(ModelError::InvalidConstraint(
                "cumulative cannot contain the same interval twice".to_owned(),
            )
            .into());
        }
        Ok(ConstraintProgrammingConstraint::Cumulative { tasks, capacity })
    }
}

/// Rust-friendly function spelling / Rust-friendly factory functions.
pub fn all_different(expressions: Vec<IntegerExpression>) -> ConstraintProgrammingConstraint {
    GlobalConstraintFunctions::all_different(expressions)
}

/// Rust-friendly function spelling / Rust-friendly factory functions.
pub fn no_overlap(intervals: Vec<IntervalVariableId>) -> ConstraintProgrammingConstraint {
    GlobalConstraintFunctions::no_overlap(intervals)
}

/// Rust-friendly function spelling / Rust-friendly factory functions.
pub fn cumulative(
    tasks: Vec<CumulativeTask>,
    capacity: i64,
) -> Result<ConstraintProgrammingConstraint> {
    GlobalConstraintFunctions::cumulative(tasks, capacity)
}

#[cfg(test)]
mod tests {
    use super::GlobalConstraintFunctions;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingModel, CumulativeTask, IntegerDomain,
        IntegerExpression, IntegerVariable, IntervalDuration, IntervalVariable,
    };
    use crate::solver::constraint_programming::{
        MipLoweringOptions, lower_constraint_programming_with_options,
    };

    fn variable(
        model: &mut ConstraintProgrammingModel,
        name: &str,
        lower: i64,
        upper: i64,
    ) -> IntegerVariable {
        let variable = IntegerVariable::new(name);
        model
            .register_variable(
                variable.clone(),
                IntegerDomain::range(lower, upper).expect("valid integer domain"),
            )
            .expect("register variable");
        variable
    }

    fn expression(variable: &IntegerVariable) -> IntegerExpression {
        IntegerExpression::variable(variable.clone())
    }

    #[test]
    fn global_factories_construct_constraints_accepted_by_the_cp_model() {
        let mut model = ConstraintProgrammingModel::new("global-factories");
        let first = variable(&mut model, "first", 0, 1);
        let second = variable(&mut model, "second", 0, 1);
        let first_end = variable(&mut model, "first-end", 1, 2);
        model
            .add_constraint(ConstraintDefinition::new(
                "all-different",
                GlobalConstraintFunctions::all_different(vec![
                    expression(&first),
                    expression(&second),
                ]),
            ))
            .expect("add all-different");
        model
            .register_interval(
                IntervalVariable::new(
                    "first-interval",
                    expression(&first),
                    IntervalDuration::Fixed(1),
                    expression(&first_end),
                    None,
                )
                .expect("first interval"),
            )
            .expect("register first interval");
        model
            .add_constraint(ConstraintDefinition::new(
                "no-overlap",
                GlobalConstraintFunctions::no_overlap(vec!["first-interval".into()]),
            ))
            .expect("add no-overlap");
        assert_eq!(
            model.freeze().expect("freeze CP model").constraints.len(),
            2
        );
    }

    #[test]
    fn cumulative_factory_validates_capacity_demands_and_duplicate_intervals() {
        assert!(GlobalConstraintFunctions::cumulative(Vec::new(), -1).is_err());
        assert!(
            GlobalConstraintFunctions::cumulative(
                vec![CumulativeTask {
                    interval: "task".into(),
                    demand: -1,
                }],
                1,
            )
            .is_err()
        );
        assert!(
            GlobalConstraintFunctions::cumulative(
                vec![
                    CumulativeTask {
                        interval: "task".into(),
                        demand: 1,
                    },
                    CumulativeTask {
                        interval: "task".into(),
                        demand: 1,
                    },
                ],
                2,
            )
            .is_err()
        );
    }

    #[test]
    fn cumulative_requires_explicit_mip_opt_in_and_lowers_within_budget() {
        let mut model = ConstraintProgrammingModel::new("cumulative-opt-in");
        let first_start = variable(&mut model, "first-start", 0, 2);
        let second_start = variable(&mut model, "second-start", 0, 2);
        let first_end = variable(&mut model, "first-end", 1, 3);
        let second_end = variable(&mut model, "second-end", 1, 3);
        model
            .register_interval(
                IntervalVariable::new(
                    "first",
                    expression(&first_start),
                    IntervalDuration::Fixed(1),
                    expression(&first_end),
                    None,
                )
                .expect("first interval"),
            )
            .expect("register first interval");
        model
            .register_interval(
                IntervalVariable::new(
                    "second",
                    expression(&second_start),
                    IntervalDuration::Fixed(1),
                    expression(&second_end),
                    None,
                )
                .expect("second interval"),
            )
            .expect("register second interval");
        model
            .add_constraint(ConstraintDefinition::new(
                "cumulative",
                GlobalConstraintFunctions::cumulative(
                    vec![
                        CumulativeTask {
                            interval: "first".into(),
                            demand: 1,
                        },
                        CumulativeTask {
                            interval: "second".into(),
                            demand: 1,
                        },
                    ],
                    1,
                )
                .expect("valid cumulative constraint"),
            ))
            .expect("add cumulative");
        let snapshot = model.freeze().expect("freeze CP model");
        assert!(
            lower_constraint_programming_with_options(&snapshot, MipLoweringOptions::default(),)
                .is_err()
        );
        let lowered = lower_constraint_programming_with_options(
            &snapshot,
            MipLoweringOptions {
                allow_cumulative: true,
                ..MipLoweringOptions::default()
            },
        )
        .expect("lower cumulative");
        assert!(lowered.model.num_constraints() > 0);
    }
}
