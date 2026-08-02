/**
 * CP 模型元素稳定身份绑定辅助函数。 / Stable-identity binding helpers for CP model elements.
 */
package fuookami.ospf.kotlin.core.model.constraint_programming

import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.variable.AbstractVariableItem

/**
 * 将表达式中的源变量绑定到模型注册的稳定 ID。 / Bind source variables in an expression to model-registered stable IDs.
 *
 * @param bindings 源变量到稳定 ID 的绑定 / Source-variable to stable-ID bindings
 * @return 绑定后的表达式 / Expression with stable IDs bound
 */
internal fun ConstraintProgrammingExpression.bindStableVariableIds(
    bindings: Map<AbstractVariableItem<*, *>, VariableId>
): ConstraintProgrammingExpression {
    return when (this) {
        is ConstraintProgrammingExpression.Constant -> this
        is ConstraintProgrammingExpression.Invalid -> this
        is ConstraintProgrammingExpression.Variable -> copy(id = bindings[variable] ?: id)
        is ConstraintProgrammingExpression.Linear -> copy(
            terms = terms.map { term ->
                term.copy(id = bindings[term.variable] ?: term.id)
            }
        )
    }
}

/**
 * 将布尔文字绑定到模型注册的稳定 ID。 / Bind a Boolean literal to a model-registered stable ID.
 *
 * @param bindings 源变量到稳定 ID 的绑定 / Source-variable to stable-ID bindings
 * @return 绑定后的文字 / Literal with a stable ID bound
 */
internal fun BooleanLiteral.bindStableVariableIds(
    bindings: Map<AbstractVariableItem<*, *>, VariableId>
): BooleanLiteral {
    return when (this) {
        is BooleanLiteral.Constant -> this
        is BooleanLiteral.Variable -> copy(id = bindings[variable] ?: id)
    }
}

/**
 * 将 interval 的所有表达式和 presence literal 绑定到稳定 ID。 / Bind all interval expressions and presence literal to stable IDs.
 *
 * @param bindings 源变量到稳定 ID 的绑定 / Source-variable to stable-ID bindings
 * @return 绑定后的 interval / Interval with stable IDs bound
 */
internal fun IntervalVariable.bindStableVariableIds(
    bindings: Map<AbstractVariableItem<*, *>, VariableId>
): IntervalVariable {
    return copy(
        start = start.bindStableVariableIds(bindings),
        size = size.bindStableVariableIds(bindings),
        end = end.bindStableVariableIds(bindings),
        presence = presence?.bindStableVariableIds(bindings)
    )
}

/**
 * 递归绑定约束树中的所有源变量。 / Recursively bind all source variables in a constraint tree.
 *
 * @param bindings 源变量到稳定 ID 的绑定 / Source-variable to stable-ID bindings
 * @return 绑定后的约束 / Constraint with stable IDs bound
 */
internal fun ConstraintProgrammingConstraint.bindStableVariableIds(
    bindings: Map<AbstractVariableItem<*, *>, VariableId>
): ConstraintProgrammingConstraint {
    return when (this) {
        is ConstraintProgrammingConstraint.IntegerComparison -> copy(
            expression = expression.bindStableVariableIds(bindings)
        )
        is ConstraintProgrammingConstraint.BoolAnd -> copy(
            literals = literals.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.BoolOr -> copy(
            literals = literals.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.BoolXor -> copy(
            literals = literals.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.Literal -> copy(
            literal = literal.bindStableVariableIds(bindings)
        )
        is ConstraintProgrammingConstraint.Implication -> copy(
            enforcement = enforcement.bindStableVariableIds(bindings),
            constraint = constraint.bindStableVariableIds(bindings)
        )
        is ConstraintProgrammingConstraint.Reified -> copy(
            literal = literal.bindStableVariableIds(bindings),
            constraint = constraint.bindStableVariableIds(bindings)
        )
        is ConstraintProgrammingConstraint.AllDifferent -> copy(
            expressions = expressions.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.Element -> copy(
            index = index.bindStableVariableIds(bindings),
            values = values.map { value ->
                if (value is ConstraintProgrammingExpression) {
                    value.bindStableVariableIds(bindings)
                } else {
                    value
                }
            },
            target = target.bindStableVariableIds(bindings)
        )
        is ConstraintProgrammingConstraint.AllowedAssignments -> copy(
            expressions = expressions.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.ForbiddenAssignments -> copy(
            expressions = expressions.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.Circuit -> copy(
            successors = successors.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.Automaton -> copy(
            expressions = expressions.map { it.bindStableVariableIds(bindings) }
        )
        is ConstraintProgrammingConstraint.Reservoir -> copy(
            events = events.map { event ->
                ConstraintProgrammingConstraint.Reservoir.Event(
                    time = event.time.bindStableVariableIds(bindings),
                    levelChange = event.levelChange.bindStableVariableIds(bindings)
                )
            }
        )
        is NoOverlap -> copy(intervals = intervals.map { it.bindStableVariableIds(bindings) })
        is Cumulative -> copy(
            intervals = intervals.map { it.bindStableVariableIds(bindings) },
            demands = demands.map { it.bindStableVariableIds(bindings) },
            capacity = capacity.bindStableVariableIds(bindings)
        )
    }
}
