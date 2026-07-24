package fuookami.ospf.kotlin.core.solver.scip

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingConstraint
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingExpression
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolveOptions
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingFeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingInfeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingConflictMinimality
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingUnknownOutput
import fuookami.ospf.kotlin.core.solver.progress.ProgressReporter
import fuookami.ospf.kotlin.core.solver.progress.SolverProgressContext
import fuookami.ospf.kotlin.core.solver.output.SolverStatus
import fuookami.ospf.kotlin.core.solver.report.SolveHandle
import fuookami.ospf.kotlin.core.solver.report.TerminationReason
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.solver.report.BoundSide
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityMember
import fuookami.ospf.kotlin.core.solver.report.EvidenceMinimality
import fuookami.ospf.kotlin.core.solver.report.EvidenceValidity
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityEvidenceSource
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.utils.functional.ok
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.Failed

class ScipConstraintProgrammingSolverIT {
    @Test
    fun shouldSolveAndRebuildAssumptionModel() = runBlocking {
        val model = ConstraintProgrammingModel("scip-cp-integration", ObjectCategory.Minimum)
        try {
            val value = fuookami.ospf.kotlin.core.variable.IntVar("value")
            model.registerVariable(value, IntegerDomain.interval(0, 5).value!!)
            val expression = ConstraintProgrammingExpression.Variable(value)
            model.addConstraint(ConstraintProgrammingConstraint.greaterOrEqual(expression, Int64(3)).value!!)
            model.minimize(expression)

            val solver = ScipConstraintProgrammingSolver()
            val output = assertIs<ConstraintProgrammingFeasibleOutput>(
                assertIs<Ok<*, *, *>>(solver.solve(model, ConstraintProgrammingSolveOptions(logEnabled = false))).value
            )
            assertEquals(Int64(3), output.solution.value(value).value)
            assertEquals(SolverStatus.Optimal, output.status)

            val snapshots = ArrayList<Int>()
            val session = assertIs<fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession>(
                solver.createSession(
                    model,
                    ConstraintProgrammingSolveOptions(
                        progressContext = SolverProgressContext(
                            reporter = ProgressReporter {
                                snapshots += it.overallProgress
                                ok
                            }
                        )
                    )
                ).value
            )
            try {
                val hintedResult = session.solve(
                    hints = fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolution(
                        values = mapOf(VariableId("${value.identifier}:${value.index}") to Int64(4))
                    )
                )
                assertTrue(
                    hintedResult is Ok<*, *, *>,
                    "SCIP hint failed: ${if (hintedResult is Failed) hintedResult.error.message else hintedResult}"
                )
                val hinted = assertIs<ConstraintProgrammingFeasibleOutput>(hintedResult.value)
                assertEquals(Int64(3), hinted.solution.value(value).value)
                assertTrue(snapshots.isNotEmpty())

                val infeasible = assertIs<ConstraintProgrammingInfeasibleOutput>(
                    assertIs<Ok<*, *, *>>(session.solve(listOf(BooleanLiteral.False))).value
                )
                assertEquals(fuookami.ospf.kotlin.core.solver.report.ProofStatus.Verified, infeasible.proofStatus)

                val rebuilt = assertIs<ConstraintProgrammingFeasibleOutput>(
                    assertIs<Ok<*, *, *>>(session.solve()).value
                )
                assertEquals(Int64(3), rebuilt.solution.value(value).value)
            } finally {
                session.close()
            }
        } finally {
            model.close()
        }
    }

    @Test
    fun shouldLowerForbiddenTableWithoutAllowingForbiddenTuple() = runBlocking {
        val model = ConstraintProgrammingModel("scip-cp-forbidden-table", ObjectCategory.Minimum)
        try {
            val first = fuookami.ospf.kotlin.core.variable.BinVar("first")
            val second = fuookami.ospf.kotlin.core.variable.BinVar("second")
            model.registerVariable(first, IntegerDomain.boolean)
            model.registerVariable(second, IntegerDomain.boolean)
            val firstExpression = ConstraintProgrammingExpression.Variable(first)
            val secondExpression = ConstraintProgrammingExpression.Variable(second)
            model.addConstraint(
                ConstraintProgrammingConstraint.forbiddenAssignments(
                    listOf(firstExpression, secondExpression),
                    listOf(listOf(Int64.zero, Int64.zero))
                ).value!!
            )
            model.minimize(firstExpression)

            val output = assertIs<ConstraintProgrammingFeasibleOutput>(
                assertIs<Ok<*, *, *>>(ScipConstraintProgrammingSolver().solve(model)).value
            )
            assertEquals(Int64.zero, output.solution.value(first).value)
            assertEquals(Int64.one, output.solution.value(second).value)
        } finally {
            model.close()
        }
    }

    @Test
    fun shouldValidateOptionsCancellationAndClosedSession() = runBlocking {
        val model = ConstraintProgrammingModel("scip-cp-boundaries", ObjectCategory.Minimum)
        try {
            val solver = ScipConstraintProgrammingSolver()
            assertTrue(
                solver.createSession(
                    model,
                    ConstraintProgrammingSolveOptions(nodeLimit = UInt64.zero)
                ).failed
            )

            val handle = SolveHandle.create()
            handle.cancel()
            val session = assertIs<fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession>(
                solver.createSession(
                    model,
                    ConstraintProgrammingSolveOptions(cancellationToken = handle.token)
                ).value
            )
            val cancelled = assertIs<ConstraintProgrammingUnknownOutput>(
                assertIs<Ok<*, *, *>>(session.solve()).value
            )
            assertEquals(TerminationReason.Cancelled, cancelled.terminationReason)
            session.close()
            assertTrue(session.solve().failed)
        } finally {
            model.close()
        }
    }

    @Test
    fun shouldShrinkAssumptionConflictWithProofGate() = runBlocking {
        val model = ConstraintProgrammingModel("scip-cp-conflict", ObjectCategory.Minimum)
        try {
            val required = fuookami.ospf.kotlin.core.variable.BinVar("required")
            val redundant = fuookami.ospf.kotlin.core.variable.BinVar("redundant")
            model.registerVariable(required, IntegerDomain.boolean)
            model.registerVariable(redundant, IntegerDomain.boolean)
            model.addConstraint(
                ConstraintProgrammingConstraint.equal(
                    ConstraintProgrammingExpression.Variable(required),
                    Int64.zero
                ).value!!,
                id = "force-required-zero"
            )

            val session = assertIs<fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession>(
                ScipConstraintProgrammingSolver().createSession(
                    model,
                    ConstraintProgrammingSolveOptions(
                        collectConflict = true,
                        shrinkConflict = true
                    )
                ).value
            )
            try {
                val output = assertIs<ConstraintProgrammingInfeasibleOutput>(
                    assertIs<Ok<*, *, *>>(
                        session.solve(
                            assumptions = listOf(BooleanLiteral(required), BooleanLiteral(redundant))
                        )
                    ).value
                )
                val conflict = assertNotNull(output.conflict)
                assertEquals(listOf(BooleanLiteral(required)), conflict.assumptions)
                assertEquals(ConstraintProgrammingConflictMinimality.Irreducible, conflict.minimality)
                assertEquals(
                    setOf(VariableId("${required.identifier}:${required.index}")),
                    conflict.variableIds
                )
                val evidence = assertNotNull(output.report?.diagnostics?.infeasibilityEvidence)
                assertEquals(InfeasibilityEvidenceSource.ConstraintConflict, evidence.source)
                assertEquals(EvidenceValidity.Verified, evidence.validity)
                assertEquals(EvidenceMinimality.Irreducible, evidence.minimality)
            } finally {
                session.close()
            }
        } finally {
            model.close()
        }
    }

    @Test
    fun shouldProjectVerifiedConflictToVariableLowerBound() = runBlocking {
        val model = ConstraintProgrammingModel("scip-cp-bound-conflict", ObjectCategory.Minimum)
        try {
            val value = fuookami.ospf.kotlin.core.variable.IntVar("bounded-value")
            model.registerVariable(value, IntegerDomain.interval(1, 3).value!!)
            model.addConstraint(
                ConstraintProgrammingConstraint.lessOrEqual(
                    ConstraintProgrammingExpression.Variable(value),
                    Int64.zero
                ).value!!,
                id = "force-at-most-zero"
            )
            val session = assertIs<fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession>(
                ScipConstraintProgrammingSolver().createSession(
                    model,
                    ConstraintProgrammingSolveOptions(
                        collectConflict = true,
                        shrinkConflict = true,
                        deterministic = true
                    )
                ).value
            )
            try {
                val output = assertIs<ConstraintProgrammingInfeasibleOutput>(
                    assertIs<Ok<*, *, *>>(session.solve()).value
                )
                val evidence = assertNotNull(output.report?.diagnostics?.infeasibilityEvidence)
                assertEquals(EvidenceValidity.Verified, evidence.validity)
                assertEquals(EvidenceMinimality.Irreducible, evidence.minimality)
                assertTrue(
                    evidence.members.contains(
                        InfeasibilityMember.VariableBound(
                            fuookami.ospf.kotlin.core.solver.report.VariableBoundRef(
                                VariableId("${value.identifier}:${value.index}"),
                                BoundSide.Lower
                            )
                        )
                    )
                )
                assertTrue(evidence.constraintIds.contains(fuookami.ospf.kotlin.core.solver.report.ConstraintId("force-at-most-zero")))
            } finally {
                session.close()
            }
        } finally {
            model.close()
        }
    }

    @Test
    fun shouldKeepSparseDomainAsIndependentConflictMember() = runBlocking {
        val model = ConstraintProgrammingModel("scip-cp-domain-conflict", ObjectCategory.Minimum)
        try {
            val value = fuookami.ospf.kotlin.core.variable.IntVar("sparse-value")
            model.registerVariable(value, IntegerDomain.values(listOf(0, 2)).value!!)
            model.addConstraint(
                ConstraintProgrammingConstraint.equal(
                    ConstraintProgrammingExpression.Variable(value),
                    Int64.one
                ).value!!,
                id = "force-missing-domain-value"
            )
            val session = assertIs<fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession>(
                ScipConstraintProgrammingSolver().createSession(
                    model,
                    ConstraintProgrammingSolveOptions(
                        collectConflict = true,
                        shrinkConflict = true,
                        deterministic = true
                    )
                ).value
            )
            try {
                val output = assertIs<ConstraintProgrammingInfeasibleOutput>(
                    assertIs<Ok<*, *, *>>(session.solve()).value
                )
                val evidence = assertNotNull(output.report?.diagnostics?.infeasibilityEvidence)
                assertEquals(EvidenceValidity.Verified, evidence.validity)
                assertTrue(
                    evidence.members.contains(
                        InfeasibilityMember.VariableDomain(
                            fuookami.ospf.kotlin.core.solver.report.VariableDomainRef(
                                VariableId("${value.identifier}:${value.index}")
                            )
                        )
                    )
                )
                assertEquals(
                    EvidenceMinimality.Irreducible,
                    evidence.minimality
                )
            } finally {
                session.close()
            }
        } finally {
            model.close()
        }
    }
}
