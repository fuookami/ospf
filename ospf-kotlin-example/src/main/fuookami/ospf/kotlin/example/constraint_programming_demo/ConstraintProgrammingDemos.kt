/** Minimal CP and Logic-Based Benders examples. / 最小 CP 与 Logic-Based Benders 示例。 */
package fuookami.ospf.kotlin.example.constraint_programming_demo

import kotlin.time.Duration.Companion.ZERO
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingConstraint
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingExpression
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.model.mechanism.LinearMetaModel
import fuookami.ospf.kotlin.core.solver.constraint_programming.FakeConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolverOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingFeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.FeasibleSolverOutput
import fuookami.ospf.kotlin.core.solver.output.SolverStatus
import fuookami.ospf.kotlin.core.solver.report.ProofStatus
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.framework.solver.BendersMasterProblemSolver
import fuookami.ospf.kotlin.framework.solver.BendersProofMode
import fuookami.ospf.kotlin.framework.solver.BendersSubproblemPipeline
import fuookami.ospf.kotlin.framework.solver.BendersVariableBinding
import fuookami.ospf.kotlin.framework.solver.BinaryBendersVariable
import fuookami.ospf.kotlin.framework.solver.BinaryBendersVariableBinding
import fuookami.ospf.kotlin.framework.solver.ConstraintProgrammingPipeline
import fuookami.ospf.kotlin.framework.solver.ConstraintProgrammingValueSource
import fuookami.ospf.kotlin.framework.solver.LogicBasedBendersEngine
import fuookami.ospf.kotlin.framework.solver.LogicBasedBendersOptions
import fuookami.ospf.kotlin.framework.solver.LogicBasedBendersReport
import fuookami.ospf.kotlin.framework.solver.registerConstraintProgramming
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Fatal
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.Try
import fuookami.ospf.kotlin.utils.functional.ok

/**
 * / 使用确定性 Fake solver 的直接 core CP 示例。 / Direct core CP demo using a deterministic fake solver.
 */
object DirectConstraintProgrammingDemo {
    /** Build and solve a small all-different model. / 构造并求解小型全异模型。 */
    suspend operator fun invoke(): Ret<ConstraintProgrammingSolverOutput> {
        val model = ConstraintProgrammingModel("direct-cp-demo", ObjectCategory.Minimum)
        val values = listOf(IntVar("cp-a"), IntVar("cp-b"), IntVar("cp-c"))
        val domain = IntegerDomain.interval(0, values.size - 1)
        if (domain.failed) return propagate(domain)
        for (variable in values) {
            val registered = model.registerVariable(variable, domain = domain.value!!)
            if (registered.failed) return propagate(registered)
        }
        val expressions = values.map {
            ConstraintProgrammingExpression.Variable(it, domain.value!!)
        }
        val allDifferent = ConstraintProgrammingConstraint.allDifferent(expressions)
        if (allDifferent.failed) return propagate(allDifferent)
        val added = model.addConstraint(allDifferent.value!!, id = "direct-all-different")
        if (added.failed) return propagate(added)
        return FakeConstraintProgrammingSolver().solve(model)
    }
}

/** Domain aggregation for the framework CP demo. / framework CP 示例的领域聚合。 */
class CpDemoAggregation(
    val selection: List<BinVar> = listOf(BinVar("cp-selection-a"), BinVar("cp-selection-b"))
) {
    /** Register aggregation variables. / 注册聚合变量。 */
    fun register(model: ConstraintProgrammingModel): Try {
        for (variable in selection) {
            val result = model.registerVariable(variable)
            if (result.failed) return propagate(result)
        }
        return ok
    }
}

/** A single domain pipeline that enforces exactly one selection. / 强制恰好选择一个变量的领域管线。 */
class CpDemoSelectionPipeline(
    private val aggregation: CpDemoAggregation
) : ConstraintProgrammingPipeline {
    override val name: String = "cp-demo-selection"

    override fun register(model: ConstraintProgrammingModel): Try {
        val constraint = ConstraintProgrammingConstraint.boolXor(
            aggregation.selection.map { BooleanLiteral(it) }
        )
        if (constraint.failed) return propagate(constraint)
        val added = model.addConstraint(constraint.value!!, id = "cp-demo-exactly-one", name = name)
        return if (added.failed) propagate(added) else ok
    }
}

/**
 * / 持有 aggregation 与 pipeline 的 framework 风格 Context。 / Framework-style context that owns aggregation and pipelines.
 */
class CpDemoContext(
    private val aggregation: CpDemoAggregation = CpDemoAggregation(),
    private val pipelines: List<ConstraintProgrammingPipeline> = listOf(CpDemoSelectionPipeline(aggregation))
) {
    /** Build a registered CP model. / 构造并注册 CP 模型。 */
    fun buildModel(): Ret<ConstraintProgrammingModel> {
        val model = ConstraintProgrammingModel("framework-cp-demo")
        val registered = aggregation.register(model)
        if (registered.failed) return propagate(registered)
        val pipelineResult = pipelines.registerConstraintProgramming(model)
        return if (pipelineResult.failed) propagate(pipelineResult) else ok(model)
    }
}

/** A subproblem pipeline used by the Benders demo. / Benders 示例使用的子问题管线。 */
class CpDemoSubproblemPipeline(
    private val variable: BinVar
) : BendersSubproblemPipeline {
    override fun register(
        model: ConstraintProgrammingModel,
        binding: BendersVariableBinding
    ): Try {
        val expression = ConstraintProgrammingExpression.Variable(variable)
        val constraint = ConstraintProgrammingConstraint.equal(expression, Int64.one)
        if (constraint.failed) return propagate(constraint)
        val added = model.addConstraint(constraint.value!!, id = "benders-required-selection")
        return if (added.failed) propagate(added) else ok
    }
}

/**
 * / 一个两轮收敛的二进制主问题 Logic-Based Benders 示例。 / A two-iteration binary-master Logic-Based Benders demo.
 */
object LogicBasedBendersDemo {
    /** Build and solve the demo. / 构造并求解示例。 */
    suspend operator fun invoke(): Ret<LogicBasedBendersReport> {
        val masterVariable = BinVar("master-selection")
        val master = LinearMetaModel(name = "benders-master", converter = IntoValue.Identity)
        val masterAdded = master.add(masterVariable)
        if (masterAdded.failed) return propagate(masterAdded)

        val subproblemVariable = BinVar("subproblem-selection")
        val subproblem = ConstraintProgrammingModel("benders-subproblem")
        val registered = subproblem.registerVariable(subproblemVariable)
        if (registered.failed) return propagate(registered)
        val binding = BinaryBendersVariableBinding(
            listOf(BinaryBendersVariable("selection", masterVariable, subproblemVariable))
        )
        val pipelineResult = CpDemoSubproblemPipeline(subproblemVariable).register(subproblem, binding)
        if (pipelineResult.failed) return propagate(pipelineResult)

        var masterCalls = 0
        val masterSolver = BendersMasterProblemSolver { model ->
            val value = if (masterCalls++ == 0) Flt64.zero else Flt64.one
            ok(
                FeasibleSolverOutput(
                    obj = Flt64.zero,
                    solution = listOf(value),
                    time = ZERO,
                    possibleBestObj = Flt64.zero,
                    gap = Flt64.zero,
                    status = SolverStatus.Optimal
                )
            )
        }
        val engine = LogicBasedBendersEngine(
            masterSolver = masterSolver,
            subproblemSolver = FakeConstraintProgrammingSolver(),
            binding = binding,
            options = LogicBasedBendersOptions(
                proofMode = BendersProofMode.Exact,
                masterSolutionSource = { output ->
                    ok(
                        ConstraintProgrammingValueSource.of(
                            mapOf("selection" to output.solution[masterVariable.index])
                        )
                    )
                }
            )
        )
        return engine.solve(master, subproblem)
    }
}

private fun <T> propagate(result: Ret<*>): Ret<T> {
    return when (result) {
        is Failed -> Failed(result.error)
        is Fatal -> Fatal(result.errors)
        else -> Failed(
            ErrorCode.ApplicationError,
            "CP demo 结果状态无效 / Invalid CP demo result state"
        )
    }
}
