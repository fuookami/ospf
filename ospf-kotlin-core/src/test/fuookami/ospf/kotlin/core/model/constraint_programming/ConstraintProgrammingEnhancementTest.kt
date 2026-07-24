package fuookami.ospf.kotlin.core.model.constraint_programming

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingCheckpointSupport
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingCheckpointSupportEvaluator
import fuookami.ospf.kotlin.core.solver.constraint_programming.FakeConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.utils.functional.Ok

class ConstraintProgrammingEnhancementTest {
    @Test
    fun circuitAutomatonAndReservoirEvaluateExactly() {
        val successors = listOf(IntVar("next-0"), IntVar("next-1"), IntVar("next-2"))
        val circuit = ConstraintProgrammingConstraint.circuit(
            successors.map { ConstraintProgrammingExpression.Variable(it, IntegerDomain.interval(0, 2).value!!) }
        ).value!!
        val circuitValues = mapOf(
            id(successors[0]) to Int64(1),
            id(successors[1]) to Int64(2),
            id(successors[2]) to Int64(0)
        )
        assertEquals(true, circuit.isSatisfied(circuitValues).value)
        assertEquals(false, circuit.isSatisfied(circuitValues + (id(successors[2]) to Int64(1))).value)

        val automaton = ConstraintProgrammingConstraint.automaton(
            expressions = listOf(ConstraintProgrammingExpression.Constant(Int64(0)), ConstraintProgrammingExpression.Constant(Int64(1))),
            initialState = 0,
            finalStates = setOf(2),
            transitions = listOf(
                ConstraintProgrammingConstraint.AutomatonTransition(0, Int64(0), 1),
                ConstraintProgrammingConstraint.AutomatonTransition(1, Int64(1), 2)
            )
        ).value!!
        assertEquals(true, automaton.isSatisfied(emptyMap()).value)

        val reservoir = ConstraintProgrammingConstraint.reservoir(
            events = listOf(
                ConstraintProgrammingConstraint.Reservoir.Event(
                    ConstraintProgrammingExpression.Constant(Int64(1)),
                    ConstraintProgrammingExpression.Constant(Int64(2))
                ),
                ConstraintProgrammingConstraint.Reservoir.Event(
                    ConstraintProgrammingExpression.Constant(Int64(2)),
                    ConstraintProgrammingExpression.Constant(Int64(-1))
                )
            ),
            initialLevel = Int64(0),
            minimumLevel = Int64(0),
            maximumLevel = Int64(2)
        ).value!!
        assertEquals(true, reservoir.isSatisfied(emptyMap()).value)
    }

    @Test
    fun snapshotCodecRoundTripsStableIds() {
        val variable = BinVar("serialized-x")
        val model = ConstraintProgrammingModel("serialized")
        model.registerVariable(variable)
        val expression = ConstraintProgrammingExpression.Variable(variable)
        model.addConstraint(ConstraintProgrammingConstraint.equal(expression, Int64.one).value!!, id = "x-one")
        val snapshot = model.snapshot().value!!
        val encoded = ConstraintProgrammingSnapshotCodec.encode(snapshot)
        assertTrue(encoded is Ok)
        val decoded = ConstraintProgrammingSnapshotCodec.decode(
            encoded.value!!,
            mapOf(id(variable) to variable)
        )
        assertTrue(decoded is Ok)
        assertEquals(snapshot.name, decoded.value!!.name)
        assertEquals(snapshot.variables.map { it.id }, decoded.value!!.variables.map { it.id })
        assertEquals(snapshot.constraints.map { it.id }, decoded.value!!.constraints.map { it.id })
    }

    @Test
    fun checkpointAssessmentUsesPortableSnapshotFallback() {
        val descriptor = FakeConstraintProgrammingSolver().descriptor
        val assessment = ConstraintProgrammingCheckpointSupportEvaluator.assess(descriptor)
        assertEquals(ConstraintProgrammingCheckpointSupport.RebuildFromSnapshot, assessment.support)
        val model = ConstraintProgrammingModel("checkpoint")
        val variable = BinVar("checkpoint-x")
        model.registerVariable(variable)
        val checkpoint = ConstraintProgrammingCheckpointSupportEvaluator.capture(
            model.snapshot().value!!,
            descriptor
        )
        assertTrue(checkpoint is Ok)
        assertEquals("checkpoint", checkpoint.value!!.modelName)
    }

    private fun id(variable: BinVar): VariableId {
        return VariableId("${variable.identifier}:${variable.index}")
    }

    private fun id(variable: IntVar): VariableId {
        return VariableId("${variable.identifier}:${variable.index}")
    }
}
