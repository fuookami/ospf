package fuookami.ospf.kotlin.framework.solver

import kotlin.time.Duration.Companion.ZERO
import kotlinx.coroutines.runBlocking
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingConstraint
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingExpression
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.model.mechanism.LinearMetaModel
import fuookami.ospf.kotlin.core.solver.constraint_programming.FakeConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolveOptions
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingConflict
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingFeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolverOutput
import fuookami.ospf.kotlin.core.solver.output.FeasibleSolverOutput
import fuookami.ospf.kotlin.core.solver.output.SolverStatus
import fuookami.ospf.kotlin.core.solver.report.EvidenceValidity
import fuookami.ospf.kotlin.core.solver.report.ProofStatus
import fuookami.ospf.kotlin.core.solver.report.ProblemStatus
import fuookami.ospf.kotlin.core.solver.report.TerminationReason
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.ok

class LogicBasedBendersTest {
    @Test
    fun binaryNoGoodCutEncodesBothBinaryValues() {
        val masterVariable = BinVar("x")
        val subproblemVariable = BinVar("cp-x")
        val subproblem = ConstraintProgrammingModel("no-good")
        subproblem.registerVariable(subproblemVariable)
        val binding = BinaryBendersVariableBinding(
            listOf(BinaryBendersVariable("x", masterVariable, subproblemVariable))
        )
        val master = LinearMetaModel(name = "master", converter = IntoValue.Identity)
        master.add(masterVariable)
        val context = { assignment: BendersSubproblemAssignment ->
            BendersCutContext(master, subproblem, assignment, 0, BendersProofMode.Exact)
        }

        val zeroAssignment = assertIs<Ok<BendersSubproblemAssignment, *, *>>(
            binding.bind(ConstraintProgrammingValueSource.of(mapOf("x" to Flt64.zero)), subproblem)
        ).value
        val zeroCut = assertIs<Ok<List<BendersMasterCut>, *, *>>(
            BinaryNoGoodCutOracle().feasibilityCuts(InfeasibleSubproblemResult(zeroAssignment, null), context(zeroAssignment))
        ).value.single()
        assertEquals(BendersCutKind.NoGood, zeroCut.kind)
        assertEquals(Comparison.GE, zeroCut.inequality.comparison)
        assertEquals(Flt64.zero, zeroCut.inequality.lhs.constant)
        assertEquals(Flt64.one, zeroCut.inequality.lhs.monomials.single().coefficient)

        val oneAssignment = assertIs<Ok<BendersSubproblemAssignment, *, *>>(
            binding.bind(ConstraintProgrammingValueSource.of(mapOf("x" to Flt64.one)), subproblem)
        ).value
        val oneCut = assertIs<Ok<List<BendersMasterCut>, *, *>>(
            BinaryNoGoodCutOracle().feasibilityCuts(InfeasibleSubproblemResult(oneAssignment, null), context(oneAssignment))
        ).value.single()
        assertEquals(Flt64.one, oneCut.inequality.lhs.constant)
        assertEquals(-Flt64.one, oneCut.inequality.lhs.monomials.single().coefficient)
    }

    @Test
    fun conflictCoreCutUsesOnlyProjectedMasterVariables() {
        val masterX = BinVar("x")
        val masterY = BinVar("y")
        val subproblemX = BinVar("cp-x")
        val subproblemY = BinVar("cp-y")
        val subproblem = ConstraintProgrammingModel("conflict")
        subproblem.registerVariable(subproblemX)
        subproblem.registerVariable(subproblemY)
        val binding = BinaryBendersVariableBinding(
            listOf(
                BinaryBendersVariable("x", masterX, subproblemX),
                BinaryBendersVariable("y", masterY, subproblemY)
            )
        )
        val assignment = assertIs<Ok<BendersSubproblemAssignment, *, *>>(
            binding.bind(
                ConstraintProgrammingValueSource.of(mapOf("x" to Flt64.zero, "y" to Flt64.one)),
                subproblem
            )
        ).value
        val conflict = ConstraintProgrammingConflict(
            assumptions = listOf(BooleanLiteral(subproblemX, negated = true))
        )
        val master = LinearMetaModel(name = "master", converter = IntoValue.Identity)
        master.add(masterX)
        master.add(masterY)
        val result = BinaryNoGoodCutOracle().feasibilityCuts(
            InfeasibleSubproblemResult(assignment, conflict),
            BendersCutContext(master, subproblem, assignment, 0, BendersProofMode.Exact)
        )
        val cut = assertIs<Ok<List<BendersMasterCut>, *, *>>(result).value.single()
        assertEquals(BendersCutKind.Conflict, cut.kind)
        assertEquals(1, cut.inequality.lhs.monomials.size)
        assertEquals(masterX, cut.inequality.lhs.monomials.single().symbol)
    }

    @Test
    fun unverifiedConflictCoreFallsBackToCompleteAssignmentNoGood() {
        val masterX = BinVar("x")
        val masterY = BinVar("y")
        val subproblemX = BinVar("cp-x-unverified")
        val subproblemY = BinVar("cp-y-unverified")
        val subproblem = ConstraintProgrammingModel("unverified-conflict")
        subproblem.registerVariable(subproblemX)
        subproblem.registerVariable(subproblemY)
        val binding = BinaryBendersVariableBinding(
            listOf(
                BinaryBendersVariable("x", masterX, subproblemX),
                BinaryBendersVariable("y", masterY, subproblemY)
            )
        )
        val assignment = assertIs<Ok<BendersSubproblemAssignment, *, *>>(
            binding.bind(
                ConstraintProgrammingValueSource.of(mapOf("x" to Flt64.zero, "y" to Flt64.one)),
                subproblem
            )
        ).value
        val conflict = ConstraintProgrammingConflict(
            assumptions = listOf(BooleanLiteral(subproblemX, negated = true)),
            validity = EvidenceValidity.Unknown
        )
        val master = LinearMetaModel(name = "master-unverified", converter = IntoValue.Identity)
        master.add(masterX)
        master.add(masterY)
        val result = BinaryNoGoodCutOracle().feasibilityCuts(
            InfeasibleSubproblemResult(assignment, conflict),
            BendersCutContext(master, subproblem, assignment, 0, BendersProofMode.Exact)
        )
        val cut = assertIs<Ok<List<BendersMasterCut>, *, *>>(result).value.single()
        assertEquals(BendersCutKind.NoGood, cut.kind)
        assertEquals(2, cut.inequality.lhs.monomials.size)
    }

    @Test
    fun exactModeRejectsAssignmentValidityCut() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x")
        val subproblem = ConstraintProgrammingModel("assignment-cut")
        subproblem.registerVariable(subproblemVariable)
        val binding = binding(masterVariable, subproblemVariable)
        val oracle = object : BendersCutOracle {
            override fun feasibilityCuts(
                result: InfeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(emptyList())

            override fun optimalityCuts(
                result: FeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(
                listOf(
                    BendersMasterCut(
                        inequality = constantCut(),
                        kind = BendersCutKind.Heuristic,
                        validity = BendersCutValidity.Assignment,
                        proofStatus = ProofStatus.Verified,
                        source = "test-assignment"
                    )
                )
            )
        }
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver { ok(output(Flt64.zero)) },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding,
            cutOracle = oracle,
            options = options(masterVariable)
        )
        val result = engine.solve(master, subproblem)
        val failure = assertIs<fuookami.ospf.kotlin.utils.functional.Failed<*, *, *>>(result)
        assertEquals(fuookami.ospf.kotlin.utils.error.ErrorCode.IllegalArgument, failure.error.code)
    }

    @Test
    fun duplicateCutsAreAddedOnlyOnce() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x")
        val subproblem = constrainedSubproblem(subproblemVariable)
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver { ok(output(Flt64.zero)) },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            options = options(masterVariable, maxIterations = 3, stallIterationLimit = 1)
        )
        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Unknown, report.problemStatus)
        assertEquals(1, report.cuts.size)
        assertEquals(1, report.iterations.size)
        assertEquals(BendersCutKind.Conflict, report.cuts.single().kind)
    }

    @Test
    fun unknownSubproblemCannotBeReportedAsOptimal() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x")
        val subproblem = ConstraintProgrammingModel("unknown")
        subproblem.registerVariable(subproblemVariable)
        val freeVariable = IntVar("cp-free")
        subproblem.registerVariable(freeVariable, IntegerDomain.interval(0, 2).value!!)
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver { ok(output(Flt64.zero)) },
            subproblemSolver = FakeConstraintProgrammingSolver(enumerationLimit = 1),
            binding = binding(masterVariable, subproblemVariable),
            options = options(masterVariable)
        )
        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Unknown, report.problemStatus)
        assertNotEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(ProofStatus.None, report.proof.status)
    }

    @Test
    fun exactModeDoesNotClaimOptimizationWithoutAnOptimalityCut() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x-objective")
        val subproblem = constrainedSubproblem(subproblemVariable)
        subproblem.minimize(ConstraintProgrammingExpression.Variable(subproblemVariable))
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver { ok(output(Flt64.one)) },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            options = options(masterVariable)
        )

        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(ProofStatus.None, report.proof.status)
        assertEquals(TerminationReason.IterationLimit, report.terminationReason)
        assertEquals("benders-no-optimality-proof", report.diagnostics.single().code)
    }

    @Test
    fun exactModeRejectsMissingObjectiveFromOptimizingSubproblem() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x-missing-objective")
        val subproblem = constrainedSubproblem(subproblemVariable)
        subproblem.minimize(ConstraintProgrammingExpression.Variable(subproblemVariable))
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver { ok(output(Flt64.one)) },
            subproblemSolver = MissingObjectiveConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            options = options(masterVariable)
        )

        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Unknown, report.problemStatus)
        assertEquals(ProofStatus.None, report.proof.status)
        assertEquals("benders-subproblem-objective-missing", report.diagnostics.single().code)
    }

    @Test
    fun exactModeDoesNotClaimConvergenceFromAWeakHistoricalOptimalityCut() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x-weak-cut")
        val subproblem = constrainedSubproblem(subproblemVariable)
        subproblem.minimize(ConstraintProgrammingExpression.Variable(subproblemVariable))
        val oracle = object : BendersCutOracle {
            override fun feasibilityCuts(
                result: InfeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(emptyList())

            override fun optimalityCuts(
                result: FeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(
                if (context.iteration == 0) {
                    listOf(
                        BendersMasterCut(
                            inequality = constantCut(),
                            kind = BendersCutKind.Optimality,
                            validity = BendersCutValidity.Global,
                            proofStatus = ProofStatus.Verified,
                            source = "verified-weak-optimality"
                        )
                    )
                } else {
                    emptyList()
                }
            )
        }
        var calls = 0
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver {
                ++calls
                ok(output(value = Flt64.one, obj = Flt64.one, bestBound = Flt64.zero))
            },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            cutOracle = oracle,
            options = options(masterVariable, maxIterations = 3).copy(
                completeObjectiveEvaluator = { _, _, output -> ok(output.objective!!) }
            )
        )

        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(2, calls)
        assertEquals(ProofStatus.None, report.proof.status)
        assertEquals("benders-no-convergence-proof", report.diagnostics.single().code)
        assertEquals(Flt64.one, report.iterations.last().convergenceGap)
    }

    @Test
    fun exactModeRequiresCompleteObjectiveContract() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x-objective-mismatch")
        val subproblem = constrainedSubproblem(subproblemVariable)
        subproblem.minimize(ConstraintProgrammingExpression.Variable(subproblemVariable))
        val oracle = object : BendersCutOracle {
            override fun feasibilityCuts(
                result: InfeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(emptyList())

            override fun optimalityCuts(
                result: FeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(
                if (context.iteration == 0) {
                    listOf(
                        BendersMasterCut(
                            inequality = constantCut(),
                            kind = BendersCutKind.Optimality,
                            validity = BendersCutValidity.Global,
                            proofStatus = ProofStatus.Verified,
                            source = "verified-but-incomplete"
                        )
                    )
                } else {
                    emptyList()
                }
            )
        }
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver {
                ok(output(value = Flt64.one, obj = Flt64.zero, bestBound = Flt64.zero))
            },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            cutOracle = oracle,
            options = options(masterVariable, maxIterations = 3)
        )

        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(ProofStatus.None, report.proof.status)
        assertEquals("benders-objective-contract-missing", report.diagnostics.single().code)
    }

    @Test
    fun exactModeUsesCompleteObjectiveEvaluatorForFirstStageCost() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x-complete-objective")
        val subproblem = constrainedSubproblem(subproblemVariable)
        subproblem.minimize(ConstraintProgrammingExpression.Variable(subproblemVariable))
        val oracle = object : BendersCutOracle {
            override fun feasibilityCuts(
                result: InfeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(emptyList())

            override fun optimalityCuts(
                result: FeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(
                if (context.iteration == 0) {
                    listOf(
                        BendersMasterCut(
                            inequality = constantCut(),
                            kind = BendersCutKind.Optimality,
                            validity = BendersCutValidity.Global,
                            proofStatus = ProofStatus.Verified,
                            source = "verified-complete-objective"
                        )
                    )
                } else {
                    emptyList()
                }
            )
        }
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver {
                ok(output(value = Flt64.one, obj = Flt64(2.0), bestBound = Flt64(2.0)))
            },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            cutOracle = oracle,
            options = options(masterVariable, maxIterations = 3).copy(
                completeObjectiveEvaluator = { _, _, output ->
                    ok(Flt64.one + output.objective!!)
                }
            )
        )

        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(ProofStatus.Verified, report.proof.status)
    }

    @Test
    fun exactModeReportsMismatchFromCompleteObjectiveEvaluator() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x-objective-mismatch")
        val subproblem = constrainedSubproblem(subproblemVariable)
        subproblem.minimize(ConstraintProgrammingExpression.Variable(subproblemVariable))
        val oracle = object : BendersCutOracle {
            override fun feasibilityCuts(
                result: InfeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(emptyList())

            override fun optimalityCuts(
                result: FeasibleSubproblemResult,
                context: BendersCutContext
            ): Ret<List<BendersMasterCut>> = ok(
                if (context.iteration == 0) {
                    listOf(
                        BendersMasterCut(
                            inequality = constantCut(),
                            kind = BendersCutKind.Optimality,
                            validity = BendersCutValidity.Global,
                            proofStatus = ProofStatus.Verified,
                            source = "verified-objective-mismatch"
                        )
                    )
                } else {
                    emptyList()
                }
            )
        }
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver {
                ok(output(value = Flt64.one, obj = Flt64.zero, bestBound = Flt64.zero))
            },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            cutOracle = oracle,
            options = options(masterVariable, maxIterations = 3).copy(
                completeObjectiveEvaluator = { _, _, output ->
                    ok(Flt64.one + output.objective!!)
                }
            )
        )

        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(ProofStatus.None, report.proof.status)
        assertEquals("benders-objective-inconsistent", report.diagnostics.single().code)
    }

    @Test
    fun binaryMasterConvergesAfterConflictCut() = runBlocking {
        val (master, masterVariable) = master()
        val subproblemVariable = BinVar("cp-x")
        val subproblem = constrainedSubproblem(subproblemVariable)
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver { model ->
                if (model.relationConstraints.isEmpty()) {
                    ok(output(Flt64.zero))
                } else {
                    ok(output(Flt64.one))
                }
            },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding(masterVariable, subproblemVariable),
            options = options(masterVariable)
        )
        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(ProofStatus.Verified, report.proof.status)
        assertEquals(1, report.cuts.size)
        assertEquals(2, report.iterations.size)
        assertEquals(Flt64.one, report.masterOutput?.solution?.single())
    }

    @Test
    fun integerMasterUsesHardFixedValuesAndInstallsMultiConstraintNoGood() = runBlocking {
        val masterVariable = IntVar("integer-master")
        val master = LinearMetaModel(name = "integer-master", converter = IntoValue.Identity)
        master.add(masterVariable)
        val subproblemVariable = IntVar("integer-cp")
        val subproblem = ConstraintProgrammingModel("integer-subproblem")
        subproblem.registerVariable(subproblemVariable, IntegerDomain.interval(0, 1).value!!)
        val expression = ConstraintProgrammingExpression.Variable(subproblemVariable)
        subproblem.addConstraint(ConstraintProgrammingConstraint.equal(expression, Int64.one).value!!)
        val domain = IntegerDomain.interval(0, 1).value!!
        val binding = IntegerBendersVariableBinding(
            listOf(IntegerBendersVariable("x", masterVariable, subproblemVariable, domain))
        )
        val engine = LogicBasedBendersEngine(
            masterSolver = BendersMasterProblemSolver { model ->
                ok(output(if (model.relationConstraints.isEmpty()) Flt64.zero else Flt64.one))
            },
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding,
            cutOracle = IntegerNoGoodCutOracle(
                listOf(IntegerBendersVariable("x", masterVariable, subproblemVariable, domain))
            ),
            options = LogicBasedBendersOptions(
                masterSolutionSource = { result ->
                    ok(
                        ConstraintProgrammingValueSource.of(
                            mapOf("x" to result.solution[masterVariable.index])
                        )
                    )
                }
            )
        )

        val report = assertIs<Ok<LogicBasedBendersReport, *, *>>(engine.solve(master, subproblem)).value
        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(ProofStatus.Verified, report.proof.status)
        assertEquals(1, report.cuts.size)
        assertTrue(report.cuts.single().additionalInequalities.isNotEmpty())
        assertTrue(report.cuts.single().auxiliaryVariables.isNotEmpty())
        assertEquals(Flt64.one, report.masterOutput?.solution?.single())
    }

    private fun master(): Pair<LinearMetaModel<Flt64>, BinVar> {
        val variable = BinVar("x")
        val model = LinearMetaModel(name = "master", converter = IntoValue.Identity)
        model.add(variable)
        return model to variable
    }

    private fun binding(
        masterVariable: BinVar,
        subproblemVariable: BinVar
    ): BendersVariableBinding {
        return BinaryBendersVariableBinding(
            listOf(BinaryBendersVariable("x", masterVariable, subproblemVariable))
        )
    }

    private fun constrainedSubproblem(variable: BinVar): ConstraintProgrammingModel {
        val model = ConstraintProgrammingModel("subproblem", ObjectCategory.Minimum)
        model.registerVariable(variable)
        val expression = ConstraintProgrammingExpression.Variable(variable)
        model.addConstraint(ConstraintProgrammingConstraint.equal(expression, Int64.one).value!!)
        return model
    }

    private fun output(
        value: Flt64,
        obj: Flt64 = Flt64.zero,
        bestBound: Flt64? = null
    ): FeasibleSolverOutput<Flt64> {
        return FeasibleSolverOutput(
            obj = obj,
            solution = listOf(value),
            time = ZERO,
            possibleBestObj = Flt64.zero,
            gap = Flt64.zero,
            status = SolverStatus.Optimal,
            bestBound = bestBound
        )
    }

    private fun options(
        masterVariable: BinVar,
        maxIterations: Int = 100,
        stallIterationLimit: Int = 1
    ): LogicBasedBendersOptions {
        return LogicBasedBendersOptions(
            maxIterations = maxIterations,
            stallIterationLimit = stallIterationLimit,
            masterSolutionSource = { result ->
                ok(
                    ConstraintProgrammingValueSource.of(
                        mapOf("x" to result.solution[masterVariable.index])
                    )
                )
            }
        )
    }

    private fun constantCut(): fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality<Flt64> {
        return fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality(
            lhs = LinearPolynomial<Flt64>(listOf(LinearMonomial(Flt64.zero, BinVar("unused"))), Flt64.zero),
            rhs = LinearPolynomial(emptyList(), Flt64.one),
            comparison = Comparison.LE
        )
    }
}

private class MissingObjectiveConstraintProgrammingSolver : ConstraintProgrammingSolver {
    private val delegate = FakeConstraintProgrammingSolver()

    override val descriptor
        get() = delegate.descriptor

    override suspend fun solve(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions
    ): Ret<ConstraintProgrammingSolverOutput> {
        return delegate.solve(model, options).map(::withoutObjective)
    }

    override fun createSession(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions
    ): Ret<ConstraintProgrammingSession> {
        return delegate.createSession(model, options).map { session ->
            object : ConstraintProgrammingSession {
                override val model: ConstraintProgrammingModel
                    get() = session.model
                override val options: ConstraintProgrammingSolveOptions
                    get() = session.options
                override val isClosed: Boolean
                    get() = session.isClosed

                override suspend fun solve(
                    assumptions: List<BooleanLiteral>,
                    fixedValues: Map<VariableId, Int64>,
                    hints: fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolution?
                ): Ret<ConstraintProgrammingSolverOutput> {
                    return session.solve(assumptions, fixedValues, hints).map(::withoutObjective)
                }

                override fun close() {
                    session.close()
                }
            }
        }
    }

    private fun withoutObjective(output: ConstraintProgrammingSolverOutput): ConstraintProgrammingSolverOutput {
        return if (output is ConstraintProgrammingFeasibleOutput) {
            output.copy(objective = null)
        } else {
            output
        }
    }
}
