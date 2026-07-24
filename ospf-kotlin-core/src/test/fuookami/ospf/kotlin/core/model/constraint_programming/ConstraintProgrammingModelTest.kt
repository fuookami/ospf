package fuookami.ospf.kotlin.core.model.constraint_programming

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.mechanism.MetaConstraintGroup
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Ok

class ConstraintProgrammingModelTest {
    @Test
    fun snapshotShouldFreezeRegistrationOrderAndRejectDuplicateIds() {
        val model = ConstraintProgrammingModel("snapshot-model", ObjectCategory.Minimum)
        val variable = IntVar("amount")
        try {
            val variableId = model.registerVariable(variable).value!!
            val expression = ConstraintProgrammingExpression.Variable(variable)
            model.registerExpression("amount-expression", expression)
            val constraint = ConstraintProgrammingConstraint.equal(expression, Int64(2)).value!!
            model.addConstraint(constraint, ConstraintId("amount-eq"))
            model.minimize(expression)

            val snapshot = assertIs<Ok<ConstraintProgrammingModelSnapshot, *, *>>(model.snapshot()).value
            assertEquals(variableId, snapshot.variables.single().id)
            assertEquals("amount-expression", snapshot.expressions.single().name)
            assertEquals(1, snapshot.constraints.size)
            assertEquals(1, snapshot.objectives.size)

            assertIs<Failed<*, *, *>>(model.addConstraint(constraint, ConstraintId("amount-eq")))
            model.addConstraint(constraint, ConstraintId("second"))
            assertEquals(1, snapshot.constraints.size)
            assertEquals(2, model.constraintCount)
        } finally {
            model.close()
        }
    }

    @Test
    fun snapshotShouldValidateUnregisteredReferencesAndGroupRegistration() {
        val model = ConstraintProgrammingModel("validation-model")
        val variable = IntVar("unregistered")
        val expression = ConstraintProgrammingExpression.Variable(variable)
        try {
            model.registerExpression("unregistered-expression", expression)
            assertIs<Failed<*, *, *>>(model.snapshot())

            model.registerVariable(variable)
            val group = TestConstraintGroup("amount-limits")
            model.registerConstraintGroup(group)
            model.addConstraint(ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!)
            val snapshot = model.snapshot().value!!
            assertEquals(listOf("amount-limits"), snapshot.constraintGroups)
            assertEquals("amount-limits", snapshot.constraints.single().groupName)
        } finally {
            model.close()
        }
    }

    private class TestConstraintGroup(override val name: String) : MetaConstraintGroup
}
