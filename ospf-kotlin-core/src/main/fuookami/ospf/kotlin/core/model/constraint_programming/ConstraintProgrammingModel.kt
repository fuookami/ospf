/**
 * CP 模型注册器与生命周期。 / CP model registry and lifecycle.
 */
package fuookami.ospf.kotlin.core.model.constraint_programming

import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.mechanism.MetaConstraintGroup
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.ObjectiveId
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.variable.AbstractVariableItem
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Fatal
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.Try
import fuookami.ospf.kotlin.utils.functional.ok

/**
 * 独立的 CP 模型，不继承线性/二次 MetaModel。 / Independent CP model, intentionally not a Linear/Quadratic MetaModel.
 * 模型中的注册表保持插入顺序，snapshot 会在校验后复制所有集合。 / Registries preserve insertion order, and snapshot copies every collection after validation.
 */
class ConstraintProgrammingModel(
    val name: String = "constraint-programming-model",
    val objectCategory: ObjectCategory = ObjectCategory.Minimum
) : ConstraintGroupRegistry, AutoCloseable {
    private data class VariableEntry(
        val variable: AbstractVariableItem<*, *>,
        val domain: IntegerDomain
    )

    private data class ConstraintEntry(
        val id: ConstraintId,
        val name: String,
        val groupName: String?,
        val constraint: ConstraintProgrammingConstraint
    )

    private data class ObjectiveEntry(
        val id: ObjectiveId,
        val category: ObjectCategory,
        val name: String,
        val expression: ConstraintProgrammingExpression
    )

    private val variables = LinkedHashMap<VariableId, VariableEntry>()
    private val intervals = LinkedHashMap<IntervalId, IntervalVariable>()
    private val expressions = LinkedHashMap<String, ConstraintProgrammingExpression>()
    private val constraints = LinkedHashMap<ConstraintId, ConstraintEntry>()
    private val objectives = LinkedHashMap<ObjectiveId, ObjectiveEntry>()
    private val groups = LinkedHashMap<String, MetaConstraintGroup>()
    private var currentGroupName: String? = null
    private var closed = false

    /** 模型是否已关闭。 / Whether the model is closed. */
    val isClosed: Boolean
        get() = closed

    /** 已注册标量变量数量。 / Number of registered scalar variables. */
    val variableCount: Int
        get() = variables.size

    /** 已注册约束数量。 / Number of registered constraints. */
    val constraintCount: Int
        get() = constraints.size

    /**
     * 注册现有 OSPF 整数变量。 / Register an existing OSPF integer variable.
     *
     * @param variable OSPF 标量变量 / OSPF scalar variable
     * @param domain CP 值域；为空时按变量类型推导 / CP domain, inferred from variable type when null
     * @return 稳定变量 ID或错误 / Stable variable ID or an error
     */
    fun registerVariable(
        variable: AbstractVariableItem<*, *>,
        domain: IntegerDomain? = null
    ): Ret<VariableId> {
        if (closed) {
            return closedFailure()
        }
        val reference = ConstraintProgrammingExpression.variable(variable, domain)
        if (reference.failed) {
            return propagateModelFailure(reference)
        }
        val id = variableIdOf(variable)
        if (variables.containsKey(id)) {
            return Failed(
                ErrorCode.IllegalArgument,
                "CP 变量 ID 重复：$id / Duplicate CP variable ID: $id"
            )
        }
        variables[id] = VariableEntry(variable, reference.value!!.domain)
        return ok(id)
    }

    /**
     * 使用显式稳定 ID 注册变量。显式 ID 适用于跨模型重建场景；变量表达式自身的 ID 仍按 OSPF variable identity 引用。 / Register a variable with an explicit stable ID. / Explicit IDs are useful across model rebuilds; expressions still reference the OSPF variable identity.
     */
    fun registerVariable(
        id: VariableId,
        variable: AbstractVariableItem<*, *>,
        domain: IntegerDomain
    ): Ret<VariableId> {
        if (closed) {
            return closedFailure()
        }
        if (id.value.isBlank()) {
            return Failed(ErrorCode.IllegalArgument, "CP 变量 ID 不能为空 / CP variable ID must not be blank")
        }
        val reference = ConstraintProgrammingExpression.variable(variable, domain)
        if (reference.failed) {
            return propagateModelFailure(reference)
        }
        if (variables.containsKey(id)) {
            return Failed(ErrorCode.IllegalArgument, "CP 变量 ID 重复：$id / Duplicate CP variable ID: $id")
        }
        variables[id] = VariableEntry(variable, reference.value!!.domain)
        return ok(id)
    }

    /** 注册 interval 结构变量。 / Register an interval structural variable. */
    fun registerInterval(interval: IntervalVariable): Ret<IntervalId> {
        if (closed) {
            return closedFailure()
        }
        if (interval.id.value.isBlank()) {
            return Failed(
                ErrorCode.IllegalArgument,
                "interval ID 不能为空 / Interval ID must not be blank"
            )
        }
        if (intervals.containsKey(interval.id)) {
            return Failed(
                ErrorCode.IllegalArgument,
                "interval ID 重复：${interval.id} / Duplicate interval ID: ${interval.id}"
            )
        }
        intervals[interval.id] = interval
        return ok(interval.id)
    }

    /** 注册具名表达式。 / Register a named expression. */
    fun registerExpression(
        name: String,
        expression: ConstraintProgrammingExpression
    ): Ret<String> {
        if (closed) {
            return closedFailure()
        }
        if (name.isBlank()) {
            return Failed(ErrorCode.IllegalArgument, "CP 表达式名称不能为空 / CP expression name must not be blank")
        }
        if (expressions.containsKey(name)) {
            return Failed(ErrorCode.IllegalArgument, "CP 表达式名称重复：$name / Duplicate CP expression name: $name")
        }
        expressions[name] = expression
        return ok(name)
    }

    /** 注册约束。 / Register a constraint. */
    fun addConstraint(
        constraint: ConstraintProgrammingConstraint,
        id: ConstraintId? = null,
        name: String = "",
        group: MetaConstraintGroup? = null
    ): Ret<ConstraintId> {
        if (closed) {
            return closedFailure()
        }
        val resolvedId = id ?: ConstraintId("constraint-${constraints.size}")
        if (resolvedId.value.isBlank()) {
            return Failed(ErrorCode.IllegalArgument, "CP 约束 ID 不能为空 / CP constraint ID must not be blank")
        }
        if (constraints.containsKey(resolvedId)) {
            return Failed(
                ErrorCode.IllegalArgument,
                "CP 约束 ID 重复：$resolvedId / Duplicate CP constraint ID: $resolvedId"
            )
        }
        group?.let { registerConstraintGroup(it) }
        constraints[resolvedId] = ConstraintEntry(
            id = resolvedId,
            name = name.ifBlank { resolvedId.value },
            groupName = group?.name ?: currentGroupName,
            constraint = constraint
        )
        return ok(resolvedId)
    }

    /** 使用字符串 ID 注册约束。 / Register a constraint with a string ID. */
    fun addConstraint(
        constraint: ConstraintProgrammingConstraint,
        id: String,
        name: String = "",
        group: MetaConstraintGroup? = null
    ): Ret<ConstraintId> {
        return addConstraint(constraint, ConstraintId(id), name, group)
    }

    /** 注册目标。 / Register an objective. */
    fun addObjective(
        category: ObjectCategory,
        expression: ConstraintProgrammingExpression,
        id: ObjectiveId? = null,
        name: String = ""
    ): Ret<ObjectiveId> {
        if (closed) {
            return closedFailure()
        }
        val resolvedId = id ?: ObjectiveId("objective-${objectives.size}")
        if (resolvedId.value.isBlank()) {
            return Failed(ErrorCode.IllegalArgument, "CP 目标 ID 不能为空 / CP objective ID must not be blank")
        }
        if (objectives.containsKey(resolvedId)) {
            return Failed(
                ErrorCode.IllegalArgument,
                "CP 目标 ID 重复：$resolvedId / Duplicate CP objective ID: $resolvedId"
            )
        }
        objectives[resolvedId] = ObjectiveEntry(
            id = resolvedId,
            category = category,
            name = name.ifBlank { resolvedId.value },
            expression = expression
        )
        return ok(resolvedId)
    }

    /** 注册最小化目标。 / Register a minimization objective. */
    fun minimize(
        expression: ConstraintProgrammingExpression,
        id: ObjectiveId? = null,
        name: String = ""
    ): Ret<ObjectiveId> {
        return addObjective(ObjectCategory.Minimum, expression, id, name)
    }

    /** 注册最大化目标。 / Register a maximization objective. */
    fun maximize(
        expression: ConstraintProgrammingExpression,
        id: ObjectiveId? = null,
        name: String = ""
    ): Ret<ObjectiveId> {
        return addObjective(ObjectCategory.Maximum, expression, id, name)
    }

    /** 注册约束组；Pipeline 注册入口调用此方法。 / Register a group for Pipeline integration. */
    override fun registerConstraintGroup(group: MetaConstraintGroup) {
        if (!closed && group.name.isNotBlank()) {
            groups.putIfAbsent(group.name, group)
            currentGroupName = group.name
        }
    }

    /** 获取约束组中的约束。 / Get constraints belonging to a group. */
    fun constraintsOfGroup(group: MetaConstraintGroup): List<ConstraintProgrammingConstraint> {
        return constraints.values
            .filter { it.groupName == group.name }
            .map { it.constraint }
    }

    /**
     * 校验当前模型的所有引用。 / Validate all references in the current model.
     *
     * @return 成功或第一个结构化错误 / Success or the first structured error
     */
    fun validate(): Try {
        if (closed) {
            return closedFailure()
        }
        val knownVariables = variables.keys
        for ((expressionName, expression) in expressions) {
            val missing = expression.variables.firstOrNull { it !in knownVariables }
            if (missing != null) {
                return missingReference("表达式 $expressionName", missing)
            }
        }
        for ((intervalId, interval) in intervals) {
            val missing = interval.variables.firstOrNull { it !in knownVariables }
            if (missing != null) {
                return missingReference("interval $intervalId", missing)
            }
        }
        for ((constraintId, constraint) in constraints) {
            val missing = constraint.constraint.variables.firstOrNull { it !in knownVariables }
            if (missing != null) {
                return missingReference("约束 $constraintId", missing)
            }
        }
        for ((objectiveId, objective) in objectives) {
            val missing = objective.expression.variables.firstOrNull { it !in knownVariables }
            if (missing != null) {
                return missingReference("目标 $objectiveId", missing)
            }
        }
        return ok
    }

    /** 生成可重复编译的不可变 snapshot。 / Build an immutable, repeatably compilable snapshot. */
    fun snapshot(): Ret<ConstraintProgrammingModelSnapshot> {
        val valid = validate()
        if (valid.failed) {
            return propagateModelFailure(valid)
        }
        return ok(
            ConstraintProgrammingModelSnapshot(
                name = name,
                objectCategory = objectCategory,
                variables = variables.map { (id, entry) ->
                    ConstraintProgrammingVariableSnapshot(
                        id = id,
                        name = entry.variable.name,
                        typeName = entry.variable.type.name,
                        domain = copyDomain(entry.domain)
                    )
                },
                intervals = intervals.values.toList(),
                expressions = expressions.map { (expressionName, expression) ->
                    ConstraintProgrammingExpressionSnapshot(expressionName, expression)
                },
                constraints = constraints.values.map {
                    ConstraintProgrammingConstraintSnapshot(it.id, it.name, it.groupName, it.constraint)
                },
                objectives = objectives.values.map {
                    ConstraintProgrammingObjectiveSnapshot(it.id, it.category, it.name, it.expression)
                },
                constraintGroups = groups.keys.toList()
            )
        )
    }

    /** 关闭并释放注册表。 / Close and release the registries. */
    override fun close() {
        closed = true
        variables.clear()
        intervals.clear()
        expressions.clear()
        constraints.clear()
        objectives.clear()
        groups.clear()
        currentGroupName = null
    }
}

private fun variableIdOf(variable: AbstractVariableItem<*, *>): VariableId {
    return VariableId("${variable.identifier}:${variable.index}")
}

private fun copyDomain(domain: IntegerDomain): IntegerDomain {
    return when (domain) {
        is IntegerDomain.Interval -> IntegerDomain.Interval(domain.lowerBound, domain.upperBound)
        is IntegerDomain.Values -> IntegerDomain.Values(domain.values.toList())
    }
}

private fun missingReference(owner: String, id: VariableId): Try {
    return Failed(
        ErrorCode.IllegalArgument,
        "$owner 引用了未注册变量：$id / $owner references an unregistered variable: $id"
    )
}

private fun <T> closedFailure(): Ret<T> {
    return Failed(
        ErrorCode.ApplicationStopped,
        "CP 模型已关闭 / CP model is closed"
    )
}

private fun <T> propagateModelFailure(result: Ret<*>): Ret<T> {
    return when (result) {
        is Failed -> Failed(result.error)
        is Fatal -> Fatal(result.errors)
        else -> Failed(ErrorCode.ApplicationError, "CP 模型结果状态无效 / Invalid CP model result state")
    }
}
