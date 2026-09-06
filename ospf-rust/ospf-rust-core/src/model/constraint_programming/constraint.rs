//! CP 约束 AST 与原始求值 / CP constraint AST and source evaluator.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ModelError, Result};
use crate::solver::StableVariableId;

use super::domain::IntegerDomain;
use super::expression::{IntegerExpression, IntegerRelation, append_string};
use super::interval::{AutomatonTransition, CumulativeTask, IntervalValue, ReservoirEvent};
use super::literal::BooleanLiteral;
use super::variable::IntervalVariableId;

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// 双向重化方向 / Reification direction.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReificationDirection {
    /// literal 为真当且仅当约束满足 / Literal is true iff the constraint is satisfied.
    Equivalent,
    /// literal 为真时约束必须满足 / Constraint is required when the literal is true.
    ImpliedByLiteral,
    /// 约束满足时 literal 必须为真 / Literal is required when the constraint is satisfied.
    ImpliesLiteral,
}

/// 约束规划约束 / Constraint-programming constraint.
///
/// 该 AST 保留原始 CP 语义；后端只能声明确切支持的子集，不能忽略未知约束。
/// This AST preserves source CP semantics. Backends may declare only exact supported subsets
/// and must not silently ignore an unknown constraint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintProgrammingConstraint {
    /// 整数线性关系 / Integer-linear relation.
    Integer {
        /// 左侧表达式 / Left-hand expression.
        expression: IntegerExpression,
        /// 关系 / Relation.
        relation: IntegerRelation,
        /// 右侧常数 / Right-hand constant.
        rhs: i64,
    },
    /// 单个 Boolean literal 必须为真 / A Boolean literal must be true.
    Boolean {
        /// 文字 / Literal.
        literal: BooleanLiteral,
    },
    /// 布尔与 / Boolean AND.
    And {
        /// 输入文字 / Input literals.
        literals: Vec<BooleanLiteral>,
    },
    /// 布尔或 / Boolean OR.
    Or {
        /// 输入文字 / Input literals.
        literals: Vec<BooleanLiteral>,
    },
    /// 布尔异或 / Boolean XOR.
    Xor {
        /// 输入文字 / Input literals.
        literals: Vec<BooleanLiteral>,
    },
    /// 最多一个 literal 为真 / At most one literal is true.
    AtMostOne {
        /// 输入文字 / Input literals.
        literals: Vec<BooleanLiteral>,
    },
    /// 恰好一个 literal 为真 / Exactly one literal is true.
    ExactlyOne {
        /// 输入文字 / Input literals.
        literals: Vec<BooleanLiteral>,
    },
    /// 单向蕴含 / One-way implication.
    Implication {
        /// 前件 / Enforcement literal.
        enforcement: BooleanLiteral,
        /// 后件约束 / Consequent constraint.
        constraint: Box<Self>,
    },
    /// 重化关系 / Reification relation.
    Reification {
        /// 绑定文字 / Bound literal.
        literal: BooleanLiteral,
        /// 被重化约束 / Reified constraint.
        constraint: Box<Self>,
        /// 重化方向 / Reification direction.
        direction: ReificationDirection,
    },
    /// 全异约束 / All-different constraint.
    AllDifferent {
        /// 表达式集合 / Expressions.
        expressions: Vec<IntegerExpression>,
    },
    /// Element 约束 / Element constraint.
    Element {
        /// 零基索引 / Zero-based index.
        index: IntegerExpression,
        /// 常量表 / Constant table.
        values: Vec<i64>,
        /// 目标表达式 / Target expression.
        target: IntegerExpression,
    },
    /// 允许表约束 / Allowed-assignment table.
    AllowedAssignments {
        /// 表达式列 / Expression columns.
        expressions: Vec<IntegerExpression>,
        /// 允许元组 / Allowed tuples.
        tuples: Vec<Vec<i64>>,
    },
    /// 禁止表约束 / Forbidden-assignment table.
    ForbiddenAssignments {
        /// 表达式列 / Expression columns.
        expressions: Vec<IntegerExpression>,
        /// 禁止元组 / Forbidden tuples.
        tuples: Vec<Vec<i64>>,
    },
    /// 区间不重叠 / Non-overlapping intervals.
    NoOverlap {
        /// 区间身份 / Interval identities.
        intervals: Vec<IntervalVariableId>,
    },
    /// 固定需求累计约束 / Fixed-demand cumulative constraint.
    Cumulative {
        /// 任务集合 / Tasks.
        tasks: Vec<CumulativeTask>,
        /// 容量 / Capacity.
        capacity: i64,
    },
    /// 单回路约束 / Single-circuit constraint.
    Circuit {
        /// 每个节点的后继表达式 / Successor expression for each node.
        successors: Vec<IntegerExpression>,
    },
    /// 自动机约束 / Automaton constraint.
    Automaton {
        /// 输入序列 / Input sequence.
        expressions: Vec<IntegerExpression>,
        /// 初始状态 / Initial state.
        initial_state: i32,
        /// 接受状态 / Final states.
        final_states: BTreeSet<i32>,
        /// 转移 / Transitions.
        transitions: Vec<AutomatonTransition>,
    },
    /// Reservoir 液位约束 / Reservoir level constraint.
    Reservoir {
        /// 事件 / Events.
        events: Vec<ReservoirEvent>,
        /// 初始液位 / Initial level.
        initial_level: i64,
        /// 最低液位 / Minimum level.
        minimum_level: i64,
        /// 最高液位 / Maximum level.
        maximum_level: i64,
    },
}

impl ConstraintProgrammingConstraint {
    /// 构造整数关系 / Create an integer relation.
    pub fn integer(expression: IntegerExpression, relation: IntegerRelation, rhs: i64) -> Self {
        Self::Integer {
            expression,
            relation,
            rhs,
        }
    }

    /// 构造 Boolean AND / Create Boolean AND.
    pub fn and<I>(literals: I) -> Self
    where
        I: IntoIterator<Item = BooleanLiteral>,
    {
        Self::And {
            literals: literals.into_iter().collect(),
        }
    }

    /// 构造 Boolean OR / Create Boolean OR.
    pub fn or<I>(literals: I) -> Self
    where
        I: IntoIterator<Item = BooleanLiteral>,
    {
        Self::Or {
            literals: literals.into_iter().collect(),
        }
    }

    /// 构造 Boolean XOR / Create Boolean XOR.
    pub fn xor<I>(literals: I) -> Self
    where
        I: IntoIterator<Item = BooleanLiteral>,
    {
        Self::Xor {
            literals: literals.into_iter().collect(),
        }
    }

    /// 构造最多一个约束 / Create an at-most-one constraint.
    pub fn at_most_one<I>(literals: I) -> Self
    where
        I: IntoIterator<Item = BooleanLiteral>,
    {
        Self::AtMostOne {
            literals: literals.into_iter().collect(),
        }
    }

    /// 构造恰好一个约束 / Create an exactly-one constraint.
    pub fn exactly_one<I>(literals: I) -> Self
    where
        I: IntoIterator<Item = BooleanLiteral>,
    {
        Self::ExactlyOne {
            literals: literals.into_iter().collect(),
        }
    }

    /// 返回引用的稳定变量 / Return referenced stable variables.
    pub fn referenced_variables(&self) -> BTreeSet<StableVariableId> {
        let mut result = BTreeSet::new();
        self.collect_references(&mut result, &mut BTreeSet::new());
        result
    }

    /// 返回引用的区间 / Return referenced intervals.
    pub fn referenced_intervals(&self) -> BTreeSet<IntervalVariableId> {
        let mut result = BTreeSet::new();
        self.collect_references(&mut BTreeSet::new(), &mut result);
        result
    }

    /// 对一个原始赋值进行精确求值 / Evaluate the constraint exactly under an assignment.
    pub fn evaluate(
        &self,
        values: &BTreeMap<StableVariableId, i64>,
        intervals: &BTreeMap<IntervalVariableId, IntervalValue>,
    ) -> Result<bool> {
        match self {
            Self::Integer {
                expression,
                relation,
                rhs,
            } => Ok(relation.evaluate(expression.evaluate(values)?, *rhs)),
            Self::Boolean { literal } => literal.evaluate(values),
            Self::And { literals } => literals
                .iter()
                .map(|literal| literal.evaluate(values))
                .collect::<Result<Vec<_>>>()
                .map(|values| values.into_iter().all(|value| value)),
            Self::Or { literals } => literals
                .iter()
                .map(|literal| literal.evaluate(values))
                .collect::<Result<Vec<_>>>()
                .map(|values| values.into_iter().any(|value| value)),
            Self::Xor { literals } => literals
                .iter()
                .map(|literal| literal.evaluate(values))
                .collect::<Result<Vec<_>>>()
                .map(|values| values.into_iter().filter(|value| *value).count() % 2 == 1),
            Self::AtMostOne { literals } => literals
                .iter()
                .map(|literal| literal.evaluate(values))
                .collect::<Result<Vec<_>>>()
                .map(|values| values.into_iter().filter(|value| *value).count() <= 1),
            Self::ExactlyOne { literals } => literals
                .iter()
                .map(|literal| literal.evaluate(values))
                .collect::<Result<Vec<_>>>()
                .map(|values| values.into_iter().filter(|value| *value).count() == 1),
            Self::Implication {
                enforcement,
                constraint,
            } => {
                if enforcement.evaluate(values)? {
                    constraint.evaluate(values, intervals)
                } else {
                    Ok(true)
                }
            }
            Self::Reification {
                literal,
                constraint,
                direction,
            } => {
                let literal_value = literal.evaluate(values)?;
                let constraint_value = constraint.evaluate(values, intervals)?;
                Ok(match direction {
                    ReificationDirection::Equivalent => literal_value == constraint_value,
                    ReificationDirection::ImpliedByLiteral => !literal_value || constraint_value,
                    ReificationDirection::ImpliesLiteral => !constraint_value || literal_value,
                })
            }
            Self::AllDifferent { expressions } => {
                let mut seen = BTreeSet::new();
                for expression in expressions {
                    if !seen.insert(expression.evaluate(values)?) {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Element {
                index,
                values: table,
                target,
            } => {
                let index = index.evaluate(values)?;
                let index = usize::try_from(index)
                    .map_err(|_| invalid("Element index must be non-negative"))?;
                let selected = table
                    .get(index)
                    .ok_or_else(|| invalid("Element index is outside the table"))?;
                Ok(target.evaluate(values)? == *selected)
            }
            Self::AllowedAssignments {
                expressions,
                tuples,
            } => {
                let values = evaluate_columns(expressions, values)?;
                Ok(tuples
                    .iter()
                    .any(|tuple| tuple.as_slice() == values.as_slice()))
            }
            Self::ForbiddenAssignments {
                expressions,
                tuples,
            } => {
                let values = evaluate_columns(expressions, values)?;
                Ok(!tuples
                    .iter()
                    .any(|tuple| tuple.as_slice() == values.as_slice()))
            }
            Self::NoOverlap { intervals: ids } => {
                for (left_index, left_id) in ids.iter().enumerate() {
                    let Some(left) = intervals.get(left_id) else {
                        continue;
                    };
                    for right_id in ids.iter().skip(left_index + 1) {
                        let Some(right) = intervals.get(right_id) else {
                            continue;
                        };
                        if left.start < right.end && right.start < left.end {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            Self::Cumulative { tasks, capacity } => {
                if *capacity < 0 {
                    return Err(invalid("Cumulative capacity must not be negative"));
                }
                let mut points = BTreeSet::new();
                for task in tasks {
                    if task.demand < 0 {
                        return Err(invalid("Cumulative demand must not be negative"));
                    }
                    let Some(interval) = intervals.get(&task.interval) else {
                        continue;
                    };
                    points.insert(interval.start);
                    points.insert(interval.end);
                }
                for point in points {
                    let load = tasks.iter().try_fold(0i128, |load, task| {
                        let Some(interval) = intervals.get(&task.interval) else {
                            return Ok(load);
                        };
                        if interval.start <= point && point < interval.end {
                            load.checked_add(i128::from(task.demand))
                                .ok_or_else(|| invalid("Cumulative load overflow"))
                        } else {
                            Ok(load)
                        }
                    })?;
                    if load > i128::from(*capacity) {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Circuit { successors } => {
                let size = successors.len();
                if size == 0 {
                    return Ok(true);
                }
                let mut next = Vec::with_capacity(size);
                for expression in successors {
                    let value = expression.evaluate(values)?;
                    let value = usize::try_from(value)
                        .map_err(|_| invalid("Circuit successor must be non-negative"))?;
                    if value >= size {
                        return Ok(false);
                    }
                    next.push(value);
                }
                let mut visited = vec![false; size];
                let mut current = 0usize;
                for _ in 0..size {
                    if visited[current] {
                        return Ok(false);
                    }
                    visited[current] = true;
                    current = next[current];
                }
                Ok(current == 0 && visited.into_iter().all(|value| value))
            }
            Self::Automaton {
                expressions,
                initial_state,
                final_states,
                transitions,
            } => {
                let mut state = *initial_state;
                for expression in expressions {
                    let value = expression.evaluate(values)?;
                    let Some(transition) = transitions.iter().find(|transition| {
                        transition.from_state == state && transition.value == value
                    }) else {
                        return Ok(false);
                    };
                    state = transition.to_state;
                }
                Ok(final_states.contains(&state))
            }
            Self::Reservoir {
                events,
                initial_level,
                minimum_level,
                maximum_level,
            } => {
                if minimum_level > maximum_level
                    || initial_level < minimum_level
                    || initial_level > maximum_level
                {
                    return Err(invalid("Reservoir levels are inconsistent"));
                }
                let mut changes = Vec::<(i64, usize, i128)>::with_capacity(events.len());
                for (order, event) in events.iter().enumerate() {
                    let time = event.time.evaluate(values)?;
                    let change = event.level_change.evaluate(values)?;
                    changes.push((time, order, i128::from(change)));
                }
                changes.sort_by_key(|(time, order, _)| (*time, *order));
                let mut level = i128::from(*initial_level);
                for (_, _, change) in changes {
                    level = level
                        .checked_add(change)
                        .ok_or_else(|| invalid("Reservoir level overflow"))?;
                    if level < i128::from(*minimum_level) || level > i128::from(*maximum_level) {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    /// 验证结构和引用 / Validate structure and references.
    pub(crate) fn validate(
        &self,
        domains: &BTreeMap<StableVariableId, IntegerDomain>,
        variables: &BTreeMap<StableVariableId, super::variable::IntegerVariable>,
        intervals: &BTreeSet<IntervalVariableId>,
    ) -> Result<()> {
        self.validate_bindings(variables)?;
        for variable in self.referenced_variables() {
            if !domains.contains_key(&variable) {
                return Err(invalid(format!(
                    "constraint references unknown variable {}",
                    variable.0
                )));
            }
        }
        for interval in self.referenced_intervals() {
            if !intervals.contains(&interval) {
                return Err(invalid(format!(
                    "constraint references unknown interval {interval}"
                )));
            }
        }
        self.validate_expression_bounds(domains)?;
        match self {
            Self::And { literals }
            | Self::Or { literals }
            | Self::Xor { literals }
            | Self::AtMostOne { literals }
            | Self::ExactlyOne { literals } => {
                for literal in literals {
                    validate_literal(literal, domains, variables)?;
                }
            }
            Self::Boolean { literal } => {
                validate_literal(literal, domains, variables)?;
            }
            Self::Implication {
                enforcement,
                constraint,
            } => {
                validate_literal(enforcement, domains, variables)?;
                constraint.validate(domains, variables, intervals)?;
            }
            Self::Reification {
                literal,
                constraint,
                ..
            } => {
                validate_literal(literal, domains, variables)?;
                constraint.validate(domains, variables, intervals)?;
            }
            Self::Element { index, values, .. } => {
                if values.is_empty() {
                    return Err(invalid("Element table must not be empty"));
                }
                let (lower, upper) = index.bounds(|id| domains.get(id))?;
                let value_count = i64::try_from(values.len())
                    .map_err(|_| invalid("Element table is too large"))?;
                if lower < 0 || upper >= value_count {
                    return Err(invalid("Element index domain is outside the table"));
                }
            }
            Self::AllowedAssignments {
                expressions,
                tuples,
            }
            | Self::ForbiddenAssignments {
                expressions,
                tuples,
            } => {
                for tuple in tuples {
                    if tuple.len() != expressions.len() {
                        return Err(invalid(
                            "assignment table tuple width does not match expressions",
                        ));
                    }
                }
            }
            Self::Cumulative { tasks, capacity } => {
                if *capacity < 0 || tasks.iter().any(|task| task.demand < 0) {
                    return Err(invalid(
                        "Cumulative capacity and demand must be non-negative",
                    ));
                }
                let mut task_intervals = BTreeSet::new();
                if tasks
                    .iter()
                    .any(|task| !task_intervals.insert(task.interval.clone()))
                {
                    return Err(invalid("Cumulative cannot contain the same interval twice"));
                }
            }
            Self::NoOverlap { intervals } => {
                let mut unique = BTreeSet::new();
                if intervals
                    .iter()
                    .any(|interval| !unique.insert(interval.clone()))
                {
                    return Err(invalid("NoOverlap cannot contain a duplicate interval"));
                }
            }
            Self::Circuit { successors } => {
                if successors.is_empty() {
                    return Err(invalid("Circuit successor expressions must not be empty"));
                }
                let size = i64::try_from(successors.len())
                    .map_err(|_| invalid("Circuit has too many successor expressions"))?;
                for successor in successors {
                    let (lower, upper) = successor.bounds(|id| domains.get(id))?;
                    if lower < 0 || upper >= size {
                        return Err(invalid(
                            "Circuit successor domain must be within the circuit node range",
                        ));
                    }
                }
            }
            Self::Automaton {
                expressions,
                final_states,
                transitions,
                ..
            } => {
                if expressions.is_empty() || final_states.is_empty() || transitions.is_empty() {
                    return Err(invalid(
                        "Automaton expressions, final states, and transitions must not be empty",
                    ));
                }
                let mut transition_keys = BTreeSet::new();
                if transitions.iter().any(|transition| {
                    !transition_keys.insert((transition.from_state, transition.value))
                }) {
                    return Err(invalid(
                        "Automaton cannot contain duplicate transitions for one state and value",
                    ));
                }
            }
            Self::Reservoir {
                initial_level,
                minimum_level,
                maximum_level,
                events,
                ..
            } => {
                if events.is_empty() {
                    return Err(invalid("Reservoir events must not be empty"));
                }
                if minimum_level > maximum_level
                    || initial_level < minimum_level
                    || initial_level > maximum_level
                {
                    return Err(invalid("Reservoir levels are inconsistent"));
                }
            }
            Self::Integer { .. } | Self::AllDifferent { .. } => {}
        }
        Ok(())
    }

    fn validate_expression_bounds(
        &self,
        domains: &BTreeMap<StableVariableId, IntegerDomain>,
    ) -> Result<()> {
        match self {
            Self::Integer { expression, .. } => {
                expression.bounds(|id| domains.get(id))?;
            }
            Self::Implication { constraint, .. } | Self::Reification { constraint, .. } => {
                constraint.validate_expression_bounds(domains)?;
            }
            Self::AllDifferent { expressions }
            | Self::Circuit {
                successors: expressions,
            } => {
                for expression in expressions {
                    expression.bounds(|id| domains.get(id))?;
                }
            }
            Self::Element { index, target, .. } => {
                index.bounds(|id| domains.get(id))?;
                target.bounds(|id| domains.get(id))?;
            }
            Self::AllowedAssignments { expressions, .. }
            | Self::ForbiddenAssignments { expressions, .. }
            | Self::Automaton { expressions, .. } => {
                for expression in expressions {
                    expression.bounds(|id| domains.get(id))?;
                }
            }
            Self::Reservoir { events, .. } => {
                for event in events {
                    event.time.bounds(|id| domains.get(id))?;
                    event.level_change.bounds(|id| domains.get(id))?;
                }
            }
            Self::Boolean { .. }
            | Self::And { .. }
            | Self::Or { .. }
            | Self::Xor { .. }
            | Self::AtMostOne { .. }
            | Self::ExactlyOne { .. }
            | Self::NoOverlap { .. }
            | Self::Cumulative { .. } => {}
        }
        Ok(())
    }

    fn validate_bindings(
        &self,
        variables: &BTreeMap<StableVariableId, super::variable::IntegerVariable>,
    ) -> Result<()> {
        match self {
            Self::Integer { expression, .. } => expression.validate_bindings(variables),
            Self::Boolean { literal } => validate_variable_binding(&literal.variable, variables),
            Self::And { literals }
            | Self::Or { literals }
            | Self::Xor { literals }
            | Self::AtMostOne { literals }
            | Self::ExactlyOne { literals } => literals
                .iter()
                .try_for_each(|literal| validate_variable_binding(&literal.variable, variables)),
            Self::Implication {
                enforcement,
                constraint,
            } => {
                validate_variable_binding(&enforcement.variable, variables)?;
                constraint.validate_bindings(variables)
            }
            Self::Reification {
                literal,
                constraint,
                ..
            } => {
                validate_variable_binding(&literal.variable, variables)?;
                constraint.validate_bindings(variables)
            }
            Self::AllDifferent { expressions }
            | Self::Circuit {
                successors: expressions,
            } => expressions
                .iter()
                .try_for_each(|expression| expression.validate_bindings(variables)),
            Self::Element { index, target, .. } => {
                index.validate_bindings(variables)?;
                target.validate_bindings(variables)
            }
            Self::AllowedAssignments { expressions, .. }
            | Self::ForbiddenAssignments { expressions, .. }
            | Self::Automaton { expressions, .. } => expressions
                .iter()
                .try_for_each(|expression| expression.validate_bindings(variables)),
            Self::NoOverlap { .. } | Self::Cumulative { .. } => Ok(()),
            Self::Reservoir { events, .. } => events.iter().try_for_each(|event| {
                event.time.validate_bindings(variables)?;
                event.level_change.validate_bindings(variables)
            }),
        }
    }

    pub(crate) fn append_canonical_bytes(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Integer {
                expression,
                relation,
                rhs,
            } => {
                bytes.push(0);
                expression.append_canonical_bytes(bytes);
                bytes.push(relation.tag());
                bytes.extend_from_slice(&rhs.to_le_bytes());
            }
            Self::Boolean { literal } => {
                bytes.push(1);
                append_literal(bytes, literal);
            }
            Self::And { literals } => append_literals(bytes, 2, literals, true),
            Self::Or { literals } => append_literals(bytes, 3, literals, true),
            Self::Xor { literals } => append_literals(bytes, 4, literals, true),
            Self::AtMostOne { literals } => append_literals(bytes, 16, literals, true),
            Self::ExactlyOne { literals } => append_literals(bytes, 17, literals, true),
            Self::Implication {
                enforcement,
                constraint,
            } => {
                bytes.push(5);
                append_literal(bytes, enforcement);
                constraint.append_canonical_bytes(bytes);
            }
            Self::Reification {
                literal,
                constraint,
                direction,
            } => {
                bytes.push(6);
                append_literal(bytes, literal);
                bytes.push(match direction {
                    ReificationDirection::Equivalent => 0,
                    ReificationDirection::ImpliedByLiteral => 1,
                    ReificationDirection::ImpliesLiteral => 2,
                });
                constraint.append_canonical_bytes(bytes);
            }
            Self::AllDifferent { expressions } => {
                bytes.push(7);
                append_expressions(bytes, expressions, true);
            }
            Self::Element {
                index,
                values,
                target,
            } => {
                bytes.push(8);
                index.append_canonical_bytes(bytes);
                bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
                for value in values {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                target.append_canonical_bytes(bytes);
            }
            Self::AllowedAssignments {
                expressions,
                tuples,
            } => {
                bytes.push(9);
                append_expressions(bytes, expressions, false);
                append_tuples(bytes, tuples, true);
            }
            Self::ForbiddenAssignments {
                expressions,
                tuples,
            } => {
                bytes.push(10);
                append_expressions(bytes, expressions, false);
                append_tuples(bytes, tuples, true);
            }
            Self::NoOverlap { intervals } => {
                bytes.push(11);
                let mut intervals = intervals
                    .iter()
                    .map(|interval| interval.0.as_str())
                    .collect::<Vec<_>>();
                intervals.sort_unstable();
                bytes.extend_from_slice(&(intervals.len() as u64).to_le_bytes());
                for interval in intervals {
                    append_string(bytes, interval);
                }
            }
            Self::Cumulative { tasks, capacity } => {
                bytes.push(12);
                bytes.extend_from_slice(&capacity.to_le_bytes());
                let mut tasks = tasks.iter().collect::<Vec<_>>();
                tasks.sort_by(|left, right| {
                    left.interval
                        .0
                        .cmp(&right.interval.0)
                        .then(left.demand.cmp(&right.demand))
                });
                bytes.extend_from_slice(&(tasks.len() as u64).to_le_bytes());
                for task in tasks {
                    append_string(bytes, &task.interval.0);
                    bytes.extend_from_slice(&task.demand.to_le_bytes());
                }
            }
            Self::Circuit { successors } => {
                bytes.push(13);
                append_expressions(bytes, successors, false);
            }
            Self::Automaton {
                expressions,
                initial_state,
                final_states,
                transitions,
            } => {
                bytes.push(14);
                append_expressions(bytes, expressions, false);
                bytes.extend_from_slice(&initial_state.to_le_bytes());
                bytes.extend_from_slice(&(final_states.len() as u64).to_le_bytes());
                for state in final_states {
                    bytes.extend_from_slice(&state.to_le_bytes());
                }
                bytes.extend_from_slice(&(transitions.len() as u64).to_le_bytes());
                let mut transitions = transitions.iter().collect::<Vec<_>>();
                transitions.sort_by_key(|transition| {
                    (transition.from_state, transition.value, transition.to_state)
                });
                for transition in transitions {
                    bytes.extend_from_slice(&transition.from_state.to_le_bytes());
                    bytes.extend_from_slice(&transition.value.to_le_bytes());
                    bytes.extend_from_slice(&transition.to_state.to_le_bytes());
                }
            }
            Self::Reservoir {
                events,
                initial_level,
                minimum_level,
                maximum_level,
            } => {
                bytes.push(15);
                bytes.extend_from_slice(&initial_level.to_le_bytes());
                bytes.extend_from_slice(&minimum_level.to_le_bytes());
                bytes.extend_from_slice(&maximum_level.to_le_bytes());
                bytes.extend_from_slice(&(events.len() as u64).to_le_bytes());
                for event in events {
                    event.time.append_canonical_bytes(bytes);
                    event.level_change.append_canonical_bytes(bytes);
                }
            }
        }
    }

    pub(crate) fn canonicalize(&self) -> Self {
        fn literal_key(literal: &BooleanLiteral) -> (&str, bool) {
            (&literal.variable.stable_id.0, literal.negated)
        }

        fn expression_key(expression: &IntegerExpression) -> Vec<u8> {
            let mut key = Vec::new();
            expression.append_canonical_bytes(&mut key);
            key
        }

        match self {
            Self::And { literals } => {
                let mut literals = literals.clone();
                literals.sort_by(|left, right| literal_key(left).cmp(&literal_key(right)));
                Self::And { literals }
            }
            Self::Or { literals } => {
                let mut literals = literals.clone();
                literals.sort_by(|left, right| literal_key(left).cmp(&literal_key(right)));
                Self::Or { literals }
            }
            Self::Xor { literals } => {
                let mut literals = literals.clone();
                literals.sort_by(|left, right| literal_key(left).cmp(&literal_key(right)));
                Self::Xor { literals }
            }
            Self::AtMostOne { literals } => {
                let mut literals = literals.clone();
                literals.sort_by(|left, right| literal_key(left).cmp(&literal_key(right)));
                Self::AtMostOne { literals }
            }
            Self::ExactlyOne { literals } => {
                let mut literals = literals.clone();
                literals.sort_by(|left, right| literal_key(left).cmp(&literal_key(right)));
                Self::ExactlyOne { literals }
            }
            Self::Implication {
                enforcement,
                constraint,
            } => Self::Implication {
                enforcement: enforcement.clone(),
                constraint: Box::new(constraint.canonicalize()),
            },
            Self::Reification {
                literal,
                constraint,
                direction,
            } => Self::Reification {
                literal: literal.clone(),
                constraint: Box::new(constraint.canonicalize()),
                direction: *direction,
            },
            Self::AllDifferent { expressions } => {
                let mut expressions = expressions.clone();
                expressions.sort_by_key(expression_key);
                Self::AllDifferent { expressions }
            }
            Self::AllowedAssignments {
                expressions,
                tuples,
            } => {
                let mut tuples = tuples.clone();
                tuples.sort();
                Self::AllowedAssignments {
                    expressions: expressions.clone(),
                    tuples,
                }
            }
            Self::ForbiddenAssignments {
                expressions,
                tuples,
            } => {
                let mut tuples = tuples.clone();
                tuples.sort();
                Self::ForbiddenAssignments {
                    expressions: expressions.clone(),
                    tuples,
                }
            }
            Self::NoOverlap { intervals } => {
                let mut intervals = intervals.clone();
                intervals.sort_unstable();
                Self::NoOverlap { intervals }
            }
            Self::Cumulative { tasks, capacity } => {
                let mut tasks = tasks.clone();
                tasks.sort_by(|left, right| {
                    left.interval
                        .0
                        .cmp(&right.interval.0)
                        .then(left.demand.cmp(&right.demand))
                });
                Self::Cumulative {
                    tasks,
                    capacity: *capacity,
                }
            }
            Self::Automaton {
                expressions,
                initial_state,
                final_states,
                transitions,
            } => {
                let mut transitions = transitions.clone();
                transitions.sort_by_key(|transition| {
                    (transition.from_state, transition.value, transition.to_state)
                });
                Self::Automaton {
                    expressions: expressions.clone(),
                    initial_state: *initial_state,
                    final_states: final_states.clone(),
                    transitions,
                }
            }
            _ => self.clone(),
        }
    }

    /// 按稳定身份重绑约束中的变量 / Rebind constraint variables by stable identity.
    pub(crate) fn rebind_variables(
        &self,
        variables: &BTreeMap<StableVariableId, super::variable::IntegerVariable>,
    ) -> Result<Self> {
        let rebind_literals = |literals: &[BooleanLiteral]| {
            literals
                .iter()
                .map(|literal| literal.rebind_variables(variables))
                .collect::<Result<Vec<_>>>()
        };
        let rebind_expressions = |expressions: &[IntegerExpression]| {
            expressions
                .iter()
                .map(|expression| expression.rebind_variables(variables))
                .collect::<Result<Vec<_>>>()
        };

        Ok(match self {
            Self::Integer {
                expression,
                relation,
                rhs,
            } => Self::Integer {
                expression: expression.rebind_variables(variables)?,
                relation: *relation,
                rhs: *rhs,
            },
            Self::Boolean { literal } => Self::Boolean {
                literal: literal.rebind_variables(variables)?,
            },
            Self::And { literals } => Self::And {
                literals: rebind_literals(literals)?,
            },
            Self::Or { literals } => Self::Or {
                literals: rebind_literals(literals)?,
            },
            Self::Xor { literals } => Self::Xor {
                literals: rebind_literals(literals)?,
            },
            Self::AtMostOne { literals } => Self::AtMostOne {
                literals: rebind_literals(literals)?,
            },
            Self::ExactlyOne { literals } => Self::ExactlyOne {
                literals: rebind_literals(literals)?,
            },
            Self::Implication {
                enforcement,
                constraint,
            } => Self::Implication {
                enforcement: enforcement.rebind_variables(variables)?,
                constraint: Box::new(constraint.rebind_variables(variables)?),
            },
            Self::Reification {
                literal,
                constraint,
                direction,
            } => Self::Reification {
                literal: literal.rebind_variables(variables)?,
                constraint: Box::new(constraint.rebind_variables(variables)?),
                direction: *direction,
            },
            Self::AllDifferent { expressions } => Self::AllDifferent {
                expressions: rebind_expressions(expressions)?,
            },
            Self::Element {
                index,
                values,
                target,
            } => Self::Element {
                index: index.rebind_variables(variables)?,
                values: values.clone(),
                target: target.rebind_variables(variables)?,
            },
            Self::AllowedAssignments {
                expressions,
                tuples,
            } => Self::AllowedAssignments {
                expressions: rebind_expressions(expressions)?,
                tuples: tuples.clone(),
            },
            Self::ForbiddenAssignments {
                expressions,
                tuples,
            } => Self::ForbiddenAssignments {
                expressions: rebind_expressions(expressions)?,
                tuples: tuples.clone(),
            },
            Self::NoOverlap { intervals } => Self::NoOverlap {
                intervals: intervals.clone(),
            },
            Self::Cumulative { tasks, capacity } => Self::Cumulative {
                tasks: tasks.clone(),
                capacity: *capacity,
            },
            Self::Circuit { successors } => Self::Circuit {
                successors: rebind_expressions(successors)?,
            },
            Self::Automaton {
                expressions,
                initial_state,
                final_states,
                transitions,
            } => Self::Automaton {
                expressions: rebind_expressions(expressions)?,
                initial_state: *initial_state,
                final_states: final_states.clone(),
                transitions: transitions.clone(),
            },
            Self::Reservoir {
                events,
                initial_level,
                minimum_level,
                maximum_level,
            } => Self::Reservoir {
                events: events
                    .iter()
                    .map(|event| {
                        Ok(ReservoirEvent {
                            time: event.time.rebind_variables(variables)?,
                            level_change: event.level_change.rebind_variables(variables)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
                initial_level: *initial_level,
                minimum_level: *minimum_level,
                maximum_level: *maximum_level,
            },
        })
    }

    fn collect_references(
        &self,
        variables: &mut BTreeSet<StableVariableId>,
        intervals: &mut BTreeSet<IntervalVariableId>,
    ) {
        match self {
            Self::Integer { expression, .. } => variables.extend(expression.referenced_variables()),
            Self::Boolean { literal } => {
                variables.insert(literal.variable.stable_id.clone());
            }
            Self::And { literals }
            | Self::Or { literals }
            | Self::Xor { literals }
            | Self::AtMostOne { literals }
            | Self::ExactlyOne { literals } => {
                for literal in literals {
                    variables.insert(literal.variable.stable_id.clone());
                }
            }
            Self::Implication {
                enforcement,
                constraint,
            } => {
                variables.insert(enforcement.variable.stable_id.clone());
                constraint.collect_references(variables, intervals);
            }
            Self::Reification {
                literal,
                constraint,
                ..
            } => {
                variables.insert(literal.variable.stable_id.clone());
                constraint.collect_references(variables, intervals);
            }
            Self::AllDifferent { expressions }
            | Self::Circuit {
                successors: expressions,
            } => {
                for expression in expressions {
                    variables.extend(expression.referenced_variables());
                }
            }
            Self::Element { index, target, .. } => {
                variables.extend(index.referenced_variables());
                variables.extend(target.referenced_variables());
            }
            Self::AllowedAssignments { expressions, .. }
            | Self::ForbiddenAssignments { expressions, .. } => {
                for expression in expressions {
                    variables.extend(expression.referenced_variables());
                }
            }
            Self::NoOverlap { intervals: values } => intervals.extend(values.iter().cloned()),
            Self::Cumulative { tasks, .. } => {
                intervals.extend(tasks.iter().map(|task| task.interval.clone()))
            }
            Self::Automaton { expressions, .. } => {
                for expression in expressions {
                    variables.extend(expression.referenced_variables());
                }
            }
            Self::Reservoir { events, .. } => {
                for event in events {
                    variables.extend(event.time.referenced_variables());
                    variables.extend(event.level_change.referenced_variables());
                }
            }
        }
    }
}

fn evaluate_columns(
    expressions: &[IntegerExpression],
    values: &BTreeMap<StableVariableId, i64>,
) -> Result<Vec<i64>> {
    expressions
        .iter()
        .map(|expression| expression.evaluate(values))
        .collect()
}

fn validate_variable_binding(
    variable: &super::variable::IntegerVariable,
    variables: &BTreeMap<StableVariableId, super::variable::IntegerVariable>,
) -> Result<()> {
    let expected = variables.get(&variable.stable_id).ok_or_else(|| {
        invalid(format!(
            "constraint references unknown variable {}",
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

fn validate_literal(
    literal: &BooleanLiteral,
    domains: &BTreeMap<StableVariableId, IntegerDomain>,
    variables: &BTreeMap<StableVariableId, super::variable::IntegerVariable>,
) -> Result<()> {
    validate_variable_binding(&literal.variable, variables)?;
    let domain = domains.get(&literal.variable.stable_id).ok_or_else(|| {
        invalid(format!(
            "Boolean literal references unknown variable {}",
            literal.variable.stable_id.0
        ))
    })?;
    if !domain.is_boolean() {
        return Err(invalid(format!(
            "variable {} is not Boolean",
            literal.variable.stable_id.0
        )));
    }
    Ok(())
}

fn append_literal(bytes: &mut Vec<u8>, literal: &BooleanLiteral) {
    append_string(bytes, &literal.variable.stable_id.0);
    bytes.push(u8::from(literal.negated));
}

fn append_literals(bytes: &mut Vec<u8>, tag: u8, literals: &[BooleanLiteral], unordered: bool) {
    bytes.push(tag);
    let mut literals = literals.iter().collect::<Vec<_>>();
    if unordered {
        literals.sort_by_key(|literal| (literal.variable.stable_id.0.as_str(), literal.negated));
    }
    bytes.extend_from_slice(&(literals.len() as u64).to_le_bytes());
    for literal in literals {
        append_literal(bytes, literal);
    }
}

fn append_expressions(bytes: &mut Vec<u8>, expressions: &[IntegerExpression], unordered: bool) {
    let mut expressions = expressions.iter().collect::<Vec<_>>();
    if unordered {
        expressions.sort_by_key(|expression| {
            let mut key = Vec::new();
            expression.append_canonical_bytes(&mut key);
            key
        });
    }
    bytes.extend_from_slice(&(expressions.len() as u64).to_le_bytes());
    for expression in expressions {
        expression.append_canonical_bytes(bytes);
    }
}

fn append_tuples(bytes: &mut Vec<u8>, tuples: &[Vec<i64>], unordered: bool) {
    let mut tuples = tuples.to_vec();
    if unordered {
        tuples.sort();
    }
    bytes.extend_from_slice(&(tuples.len() as u64).to_le_bytes());
    for tuple in tuples {
        bytes.extend_from_slice(&(tuple.len() as u64).to_le_bytes());
        for value in tuple {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BooleanLiteral, ConstraintProgrammingConstraint};
    use crate::model::constraint_programming::{
        AutomatonTransition, ConstraintDefinition, ConstraintProgrammingModel, CumulativeTask,
        IntegerDomain, IntegerExpression, IntegerVariable, IntervalDuration, IntervalVariable,
        ReservoirEvent,
    };
    use std::collections::BTreeMap;

    fn freeze_with_constraint(
        variable: Option<(IntegerVariable, IntegerDomain)>,
        constraint: ConstraintProgrammingConstraint,
    ) -> crate::model::constraint_programming::ConstraintProgrammingSnapshot {
        let mut model = ConstraintProgrammingModel::new("constraint-boundary");
        if let Some((variable, domain)) = variable {
            model
                .register_variable(variable, domain)
                .expect("register boundary variable");
        }
        model
            .add_constraint(ConstraintDefinition::new("boundary", constraint))
            .expect("register boundary constraint");
        model.freeze().expect("valid boundary snapshot")
    }

    #[test]
    fn empty_and_single_element_collections_keep_their_source_semantics() {
        let values = BTreeMap::new();
        let intervals = BTreeMap::new();
        assert!(
            ConstraintProgrammingConstraint::and([])
                .evaluate(&values, &intervals)
                .expect("empty AND")
        );
        assert!(
            !ConstraintProgrammingConstraint::or([])
                .evaluate(&values, &intervals)
                .expect("empty OR")
        );
        assert!(
            !ConstraintProgrammingConstraint::xor([])
                .evaluate(&values, &intervals)
                .expect("empty XOR")
        );
        assert!(
            ConstraintProgrammingConstraint::at_most_one([])
                .evaluate(&values, &intervals)
                .expect("empty at-most-one")
        );
        assert!(
            !ConstraintProgrammingConstraint::exactly_one([])
                .evaluate(&values, &intervals)
                .expect("empty exactly-one")
        );
        assert!(
            ConstraintProgrammingConstraint::AllDifferent {
                expressions: vec![],
            }
            .evaluate(&values, &intervals)
            .expect("empty AllDifferent")
        );
        assert!(
            ConstraintProgrammingConstraint::AllDifferent {
                expressions: vec![IntegerExpression::constant(7)],
            }
            .evaluate(&values, &intervals)
            .expect("single AllDifferent")
        );
        assert!(
            ConstraintProgrammingConstraint::NoOverlap { intervals: vec![] }
                .evaluate(&values, &intervals)
                .expect("empty NoOverlap")
        );
        assert!(
            !ConstraintProgrammingConstraint::AllowedAssignments {
                expressions: vec![],
                tuples: vec![],
            }
            .evaluate(&values, &intervals)
            .expect("empty allowed table")
        );
        assert!(
            ConstraintProgrammingConstraint::ForbiddenAssignments {
                expressions: vec![],
                tuples: vec![],
            }
            .evaluate(&values, &intervals)
            .expect("empty forbidden table")
        );
    }

    #[test]
    fn element_rejects_empty_table_and_accepts_a_singleton_table() {
        let index = IntegerVariable::new("element/index");
        let target = IntegerVariable::new("element/target");
        let mut invalid_model = ConstraintProgrammingModel::new("empty-element");
        invalid_model
            .register_variable(
                index.clone(),
                IntegerDomain::range(0, 0).expect("index domain"),
            )
            .expect("index");
        invalid_model
            .register_variable(
                target.clone(),
                IntegerDomain::range(0, 0).expect("target domain"),
            )
            .expect("target");
        invalid_model
            .add_constraint_by_id(
                "empty-element",
                ConstraintProgrammingConstraint::Element {
                    index: IntegerExpression::variable(index.clone()),
                    values: vec![],
                    target: IntegerExpression::variable(target.clone()),
                },
            )
            .expect("constraint registration");
        assert!(invalid_model.freeze().is_err());

        let singleton = freeze_with_constraint(
            Some((
                index.clone(),
                IntegerDomain::range(0, 0).expect("index domain"),
            )),
            ConstraintProgrammingConstraint::Element {
                index: IntegerExpression::variable(index),
                values: vec![0],
                target: IntegerExpression::constant(0),
            },
        );
        assert!(
            singleton
                .validate_assignment(&BTreeMap::from([(
                    crate::solver::StableVariableId::from("element/index"),
                    0,
                )]))
                .is_ok()
        );
    }

    #[test]
    fn circuit_checks_successor_domains_and_accepts_a_single_node_loop() {
        let successor = IntegerVariable::new("circuit/successor");
        let mut invalid_model = ConstraintProgrammingModel::new("invalid-circuit");
        invalid_model
            .register_variable(
                successor.clone(),
                IntegerDomain::range(0, 1).expect("domain"),
            )
            .expect("successor");
        invalid_model
            .add_constraint_by_id(
                "invalid-circuit",
                ConstraintProgrammingConstraint::Circuit {
                    successors: vec![IntegerExpression::variable(successor)],
                },
            )
            .expect("constraint registration");
        assert!(invalid_model.freeze().is_err());

        let snapshot = freeze_with_constraint(
            None,
            ConstraintProgrammingConstraint::Circuit {
                successors: vec![IntegerExpression::constant(0)],
            },
        );
        assert!(snapshot.validate_assignment(&BTreeMap::new()).is_ok());
    }

    #[test]
    fn automaton_rejects_duplicate_transitions_and_unknown_variables() {
        let variable = IntegerVariable::new("automaton/value");
        let duplicate = ConstraintProgrammingConstraint::Automaton {
            expressions: vec![IntegerExpression::variable(variable.clone())],
            initial_state: 0,
            final_states: [1].into_iter().collect(),
            transitions: vec![
                AutomatonTransition {
                    from_state: 0,
                    value: 0,
                    to_state: 1,
                },
                AutomatonTransition {
                    from_state: 0,
                    value: 0,
                    to_state: 2,
                },
            ],
        };
        let mut duplicate_model = ConstraintProgrammingModel::new("duplicate-automaton");
        duplicate_model
            .register_variable(variable.clone(), IntegerDomain::boolean())
            .expect("automaton variable");
        duplicate_model
            .add_constraint_by_id("duplicate", duplicate)
            .expect("constraint registration");
        assert!(duplicate_model.freeze().is_err());

        let unknown = IntegerVariable::new("automaton/unknown");
        let mut unknown_model = ConstraintProgrammingModel::new("unknown-automaton-variable");
        unknown_model
            .add_constraint_by_id(
                "unknown",
                ConstraintProgrammingConstraint::Automaton {
                    expressions: vec![IntegerExpression::variable(unknown)],
                    initial_state: 0,
                    final_states: [0].into_iter().collect(),
                    transitions: vec![AutomatonTransition {
                        from_state: 0,
                        value: 0,
                        to_state: 0,
                    }],
                },
            )
            .expect("constraint registration");
        assert!(unknown_model.freeze().is_err());
    }

    #[test]
    fn cumulative_and_reservoir_enforce_numeric_boundaries() {
        let mut negative_capacity = ConstraintProgrammingModel::new("negative-capacity");
        negative_capacity
            .add_constraint_by_id(
                "negative-capacity",
                ConstraintProgrammingConstraint::Cumulative {
                    tasks: vec![],
                    capacity: -1,
                },
            )
            .expect("constraint registration");
        assert!(negative_capacity.freeze().is_err());

        let interval = IntervalVariable::new(
            "cumulative/task",
            IntegerExpression::constant(0),
            IntervalDuration::Fixed(1),
            IntegerExpression::constant(1),
            None,
        )
        .expect("interval");
        let mut negative_demand = ConstraintProgrammingModel::new("negative-demand");
        negative_demand
            .register_interval(interval)
            .expect("interval registration");
        negative_demand
            .add_constraint_by_id(
                "negative-demand",
                ConstraintProgrammingConstraint::Cumulative {
                    tasks: vec![CumulativeTask {
                        interval: "cumulative/task".into(),
                        demand: -1,
                    }],
                    capacity: 1,
                },
            )
            .expect("constraint registration");
        assert!(negative_demand.freeze().is_err());

        let mut invalid_reservoir = ConstraintProgrammingModel::new("invalid-reservoir");
        invalid_reservoir
            .add_constraint_by_id(
                "invalid-reservoir",
                ConstraintProgrammingConstraint::Reservoir {
                    events: vec![ReservoirEvent {
                        time: IntegerExpression::constant(0),
                        level_change: IntegerExpression::constant(0),
                    }],
                    initial_level: 2,
                    minimum_level: 0,
                    maximum_level: 1,
                },
            )
            .expect("constraint registration");
        assert!(invalid_reservoir.freeze().is_err());
    }

    #[test]
    fn reservoir_applies_same_time_events_in_input_order() {
        let increasing_then_decreasing = ConstraintProgrammingConstraint::Reservoir {
            events: vec![
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(1),
                },
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(-1),
                },
            ],
            initial_level: 0,
            minimum_level: 0,
            maximum_level: 1,
        };
        assert!(
            increasing_then_decreasing
                .evaluate(&BTreeMap::new(), &BTreeMap::new())
                .expect("reservoir evaluation")
        );

        let decreasing_then_increasing = ConstraintProgrammingConstraint::Reservoir {
            events: vec![
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(-1),
                },
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(1),
                },
            ],
            initial_level: 0,
            minimum_level: 0,
            maximum_level: 1,
        };
        assert!(
            !decreasing_then_increasing
                .evaluate(&BTreeMap::new(), &BTreeMap::new())
                .expect("reservoir evaluation")
        );
    }

    #[test]
    fn canonical_fingerprint_normalizes_commutative_constraint_members() {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let build = |literals: Vec<BooleanLiteral>| {
            let mut model = ConstraintProgrammingModel::new("canonical-boolean-operands");
            model
                .register_variable(x.clone(), IntegerDomain::boolean())
                .expect("x");
            model
                .register_variable(y.clone(), IntegerDomain::boolean())
                .expect("y");
            model
                .add_constraint(ConstraintDefinition::new(
                    "at-most-one",
                    ConstraintProgrammingConstraint::at_most_one(literals),
                ))
                .expect("constraint");
            model.freeze().expect("snapshot")
        };
        let left = build(vec![
            BooleanLiteral::positive(x.clone()),
            BooleanLiteral::positive(y.clone()),
        ]);
        let right = build(vec![
            BooleanLiteral::positive(y.clone()),
            BooleanLiteral::positive(x.clone()),
        ]);
        assert_eq!(left.fingerprint, right.fingerprint);
    }

    #[test]
    fn canonical_fingerprint_normalizes_tables_intervals_tasks_and_automaton_transitions() {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let build = |constraint: ConstraintProgrammingConstraint| {
            let mut model = ConstraintProgrammingModel::new("canonical-collections");
            model
                .register_variable(x.clone(), IntegerDomain::range(0, 1).expect("x domain"))
                .expect("x");
            model
                .register_variable(y.clone(), IntegerDomain::range(0, 1).expect("y domain"))
                .expect("y");
            model
                .register_interval(
                    IntervalVariable::new(
                        "interval-a",
                        IntegerExpression::constant(0),
                        IntervalDuration::Fixed(1),
                        IntegerExpression::constant(1),
                        None,
                    )
                    .expect("interval a"),
                )
                .expect("interval a registration");
            model
                .register_interval(
                    IntervalVariable::new(
                        "interval-b",
                        IntegerExpression::constant(2),
                        IntervalDuration::Fixed(1),
                        IntegerExpression::constant(3),
                        None,
                    )
                    .expect("interval b"),
                )
                .expect("interval b registration");
            model
                .add_constraint(ConstraintDefinition::new("collection", constraint))
                .expect("constraint");
            model.freeze().expect("snapshot")
        };

        let all_different_left = build(ConstraintProgrammingConstraint::AllDifferent {
            expressions: vec![
                IntegerExpression::variable(x.clone()),
                IntegerExpression::variable(y.clone()),
            ],
        });
        let all_different_right = build(ConstraintProgrammingConstraint::AllDifferent {
            expressions: vec![
                IntegerExpression::variable(y.clone()),
                IntegerExpression::variable(x.clone()),
            ],
        });
        assert_eq!(
            all_different_left.fingerprint,
            all_different_right.fingerprint
        );

        for (left, right) in [
            (
                ConstraintProgrammingConstraint::AllowedAssignments {
                    expressions: vec![IntegerExpression::variable(x.clone())],
                    tuples: vec![vec![0], vec![1]],
                },
                ConstraintProgrammingConstraint::AllowedAssignments {
                    expressions: vec![IntegerExpression::variable(x.clone())],
                    tuples: vec![vec![1], vec![0]],
                },
            ),
            (
                ConstraintProgrammingConstraint::ForbiddenAssignments {
                    expressions: vec![IntegerExpression::variable(x.clone())],
                    tuples: vec![vec![0], vec![1]],
                },
                ConstraintProgrammingConstraint::ForbiddenAssignments {
                    expressions: vec![IntegerExpression::variable(x.clone())],
                    tuples: vec![vec![1], vec![0]],
                },
            ),
            (
                ConstraintProgrammingConstraint::NoOverlap {
                    intervals: vec!["interval-a".into(), "interval-b".into()],
                },
                ConstraintProgrammingConstraint::NoOverlap {
                    intervals: vec!["interval-b".into(), "interval-a".into()],
                },
            ),
            (
                ConstraintProgrammingConstraint::Cumulative {
                    tasks: vec![
                        CumulativeTask {
                            interval: "interval-a".into(),
                            demand: 1,
                        },
                        CumulativeTask {
                            interval: "interval-b".into(),
                            demand: 2,
                        },
                    ],
                    capacity: 2,
                },
                ConstraintProgrammingConstraint::Cumulative {
                    tasks: vec![
                        CumulativeTask {
                            interval: "interval-b".into(),
                            demand: 2,
                        },
                        CumulativeTask {
                            interval: "interval-a".into(),
                            demand: 1,
                        },
                    ],
                    capacity: 2,
                },
            ),
            (
                ConstraintProgrammingConstraint::Automaton {
                    expressions: vec![IntegerExpression::variable(x.clone())],
                    initial_state: 0,
                    final_states: [1, 2].into_iter().collect(),
                    transitions: vec![
                        AutomatonTransition {
                            from_state: 0,
                            value: 1,
                            to_state: 1,
                        },
                        AutomatonTransition {
                            from_state: 0,
                            value: 0,
                            to_state: 0,
                        },
                    ],
                },
                ConstraintProgrammingConstraint::Automaton {
                    expressions: vec![IntegerExpression::variable(x.clone())],
                    initial_state: 0,
                    final_states: [2, 1].into_iter().collect(),
                    transitions: vec![
                        AutomatonTransition {
                            from_state: 0,
                            value: 0,
                            to_state: 0,
                        },
                        AutomatonTransition {
                            from_state: 0,
                            value: 1,
                            to_state: 1,
                        },
                    ],
                },
            ),
        ] {
            assert_eq!(build(left).fingerprint, build(right).fingerprint);
        }
    }

    #[test]
    fn canonical_fingerprint_preserves_ordered_automaton_and_reservoir_sequences() {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let build = |constraint: ConstraintProgrammingConstraint| {
            let mut model = ConstraintProgrammingModel::new("canonical-ordered");
            model
                .register_variable(x.clone(), IntegerDomain::range(0, 1).expect("domain"))
                .expect("variable");
            model
                .register_variable(y.clone(), IntegerDomain::range(0, 1).expect("domain"))
                .expect("variable");
            model
                .add_constraint(ConstraintDefinition::new("ordered", constraint))
                .expect("constraint");
            model.freeze().expect("snapshot")
        };
        let automaton_a = build(ConstraintProgrammingConstraint::Automaton {
            expressions: vec![
                IntegerExpression::variable(x.clone()),
                IntegerExpression::variable(y.clone()),
            ],
            initial_state: 0,
            final_states: [0].into_iter().collect(),
            transitions: vec![AutomatonTransition {
                from_state: 0,
                value: 0,
                to_state: 0,
            }],
        });
        let automaton_b = build(ConstraintProgrammingConstraint::Automaton {
            expressions: vec![
                IntegerExpression::variable(y.clone()),
                IntegerExpression::variable(x.clone()),
            ],
            initial_state: 0,
            final_states: [0].into_iter().collect(),
            transitions: vec![AutomatonTransition {
                from_state: 0,
                value: 0,
                to_state: 0,
            }],
        });
        assert_ne!(automaton_a.fingerprint, automaton_b.fingerprint);

        let reservoir_a = build(ConstraintProgrammingConstraint::Reservoir {
            events: vec![
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(1),
                },
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(-1),
                },
            ],
            initial_level: 0,
            minimum_level: 0,
            maximum_level: 1,
        });
        let reservoir_b = build(ConstraintProgrammingConstraint::Reservoir {
            events: vec![
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(-1),
                },
                ReservoirEvent {
                    time: IntegerExpression::constant(0),
                    level_change: IntegerExpression::constant(1),
                },
            ],
            initial_level: 0,
            minimum_level: 0,
            maximum_level: 1,
        });
        assert_ne!(reservoir_a.fingerprint, reservoir_b.fingerprint);
    }
}
