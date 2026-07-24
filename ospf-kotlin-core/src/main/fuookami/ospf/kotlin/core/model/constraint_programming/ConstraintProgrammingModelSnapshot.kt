/**
 * CP 模型的不可变 snapshot。 / Immutable snapshot of a CP model.
 */
package fuookami.ospf.kotlin.core.model.constraint_programming

import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.BoundSide
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityMember
import fuookami.ospf.kotlin.core.solver.report.ObjectiveId
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.solver.report.VariableBoundRef
import fuookami.ospf.kotlin.core.solver.report.VariableDomainRef

/** snapshot 中的标量变量定义。 / Scalar-variable definition in a snapshot. */
data class ConstraintProgrammingVariableSnapshot(
    val id: VariableId,
    val name: String,
    val typeName: String,
    val domain: IntegerDomain
)

/** snapshot 中的具名表达式。 / Named expression in a snapshot. */
data class ConstraintProgrammingExpressionSnapshot(
    val name: String,
    val expression: ConstraintProgrammingExpression
)

/** snapshot 中的约束定义。 / Constraint definition in a snapshot. */
data class ConstraintProgrammingConstraintSnapshot(
    val id: ConstraintId,
    val name: String,
    val groupName: String?,
    val constraint: ConstraintProgrammingConstraint
)

/** snapshot 中的目标定义。 / Objective definition in a snapshot. */
data class ConstraintProgrammingObjectiveSnapshot(
    val id: ObjectiveId,
    val category: ObjectCategory,
    val name: String,
    val expression: ConstraintProgrammingExpression
)

/**
 * 诊断模型中的稳定 activation 描述。该描述只引用原始模型成员，solver 辅助约束不得进入公共证据。 / Stable activation descriptor in a diagnostic model. / The descriptor references original model members only; solver auxiliaries must not leak into evidence.
 */
data class ConstraintProgrammingActivationSnapshot(
    val id: String,
    val member: InfeasibilityMember
)

/**
 * CP 模型 snapshot；所有集合按注册顺序冻结。 / CP model snapshot with all collections frozen in registration order.
 * snapshot 不持有模型的可变注册表或求解器句柄，因此可以重复编译。 / The snapshot holds no mutable model registry or solver handle and can therefore be compiled repeatedly.
 */
data class ConstraintProgrammingModelSnapshot(
    val name: String,
    val objectCategory: ObjectCategory,
    val variables: List<ConstraintProgrammingVariableSnapshot>,
    val intervals: List<IntervalVariable>,
    val expressions: List<ConstraintProgrammingExpressionSnapshot>,
    val constraints: List<ConstraintProgrammingConstraintSnapshot>,
    val objectives: List<ConstraintProgrammingObjectiveSnapshot>,
    val constraintGroups: List<String>
) {
    /** 按 ID 查找标量变量。 / Find a scalar variable by ID. */
    fun variable(id: VariableId): ConstraintProgrammingVariableSnapshot? {
        return variables.firstOrNull { it.id == id }
    }

    /** 按 ID 查找约束。 / Find a constraint by ID. */
    fun constraint(id: ConstraintId): ConstraintProgrammingConstraintSnapshot? {
        return constraints.firstOrNull { it.id == id }
    }

    /** 按 ID 查找目标。 / Find an objective by ID. */
    fun objective(id: ObjectiveId): ConstraintProgrammingObjectiveSnapshot? {
        return objectives.firstOrNull { it.id == id }
    }

    /**
     * 生成约束、上下界和稀疏值域 activation 的稳定清单。 / / Build a stable list of activations for constraints, bounds, and sparse domains.
     */
    fun diagnosticActivations(): List<ConstraintProgrammingActivationSnapshot> {
        val result = ArrayList<ConstraintProgrammingActivationSnapshot>()
        constraints.forEach { constraint ->
            result += ConstraintProgrammingActivationSnapshot(
                id = "constraint:${constraint.id.value}",
                member = InfeasibilityMember.Constraint(constraint.id)
            )
        }
        variables.forEach { variable ->
            result += ConstraintProgrammingActivationSnapshot(
                id = "variable:${variable.id.value}:lower",
                member = InfeasibilityMember.VariableBound(
                    VariableBoundRef(variable.id, BoundSide.Lower)
                )
            )
            result += ConstraintProgrammingActivationSnapshot(
                id = "variable:${variable.id.value}:upper",
                member = InfeasibilityMember.VariableBound(
                    VariableBoundRef(variable.id, BoundSide.Upper)
                )
            )
            if (variable.domain is IntegerDomain.Values && variable.domain != IntegerDomain.boolean) {
                result += ConstraintProgrammingActivationSnapshot(
                    id = "variable:${variable.id.value}:domain",
                    member = InfeasibilityMember.VariableDomain(VariableDomainRef(variable.id))
                )
            }
        }
        return result
    }
}
