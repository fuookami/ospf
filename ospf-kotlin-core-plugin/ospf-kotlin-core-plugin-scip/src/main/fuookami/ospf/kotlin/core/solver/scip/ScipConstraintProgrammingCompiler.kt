/**
 * CP snapshot 到 JSCIP 的编译器。 / Compiler from a CP snapshot to JSCIP.
 */
package fuookami.ospf.kotlin.core.solver.scip

import java.math.BigInteger
import jscip.Constraint
import jscip.Scip
import jscip.SCIP_Vartype
import jscip.Variable
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingConstraint
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingExpression
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot
import fuookami.ospf.kotlin.core.model.constraint_programming.Cumulative
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.model.constraint_programming.IntervalId
import fuookami.ospf.kotlin.core.model.constraint_programming.IntervalVariable
import fuookami.ospf.kotlin.core.model.constraint_programming.NoOverlap
import fuookami.ospf.kotlin.core.model.constraint_programming.ReificationDirection
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityMember
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Fatal
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.ok

/** 编译后可求解的 SCIP 模型句柄集合。 / Compiled SCIP model handles. */
class ScipConstraintProgrammingCompiledModel internal constructor(
    val variables: Map<VariableId, Variable>,
    val intervals: Map<IntervalId, ScipConstraintProgrammingInterval>,
    val constraints: Map<ConstraintId, List<Constraint>>,
    val activations: Map<String, ScipConstraintProgrammingActivation>,
    internal val allVariables: List<Variable>,
    internal val allConstraints: List<Constraint>
) : AutoCloseable {
    private var closed = false

    override fun close() {
        if (closed) {
            return
        }
        closed = true
        allConstraints.asReversed().forEach { constraint ->
            try {
                owner?.releaseCons(constraint)
            } catch (_: Throwable) {
                // Native cleanup is best effort at the adapter boundary. / 原生资源清理在 adapter 边界尽力完成。
            }
        }
        allVariables.asReversed().forEach { variable ->
            try {
                owner?.releaseVar(variable)
            } catch (_: Throwable) {
                // See the comment above. / 同上。
            }
        }
        owner = null
    }

    private var owner: Scip? = null

    internal fun attachOwner(scip: Scip): ScipConstraintProgrammingCompiledModel {
        owner = scip
        return this
    }
}

/** 编译后的 interval 引用。 / Compiled interval reference. */
data class ScipConstraintProgrammingInterval(
    val start: Variable,
    val size: Int,
    val end: Variable
)

/**
 * 原始模型成员到 SCIP activation 变量的映射。 / Mapping from an original model member to a SCIP activation variable.
 */
data class ScipConstraintProgrammingActivation(
    val id: String,
    val member: InfeasibilityMember,
    val variable: Variable
)

/**
 * 受约束的 CP 编译器。所有辅助变量和约束都由该对象持有并可统一释放。 / / Restricted CP compiler. All auxiliary handles are owned and released together.
 */
class ScipConstraintProgrammingCompiler(
    private val scip: Scip,
    private val snapshot: ConstraintProgrammingModelSnapshot,
    private val sparseDomainLimit: Int = DEFAULT_SPARSE_DOMAIN_LIMIT,
    private val decompositionLimit: Int = DEFAULT_DECOMPOSITION_LIMIT
) {
    private val variables = LinkedHashMap<VariableId, Variable>()
    private val intervals = LinkedHashMap<IntervalId, ScipConstraintProgrammingInterval>()
    private val constraintMap = LinkedHashMap<ConstraintId, List<Constraint>>()
    private val activations = LinkedHashMap<String, ScipConstraintProgrammingActivation>()
    private val allVariables = ArrayList<Variable>()
    private val allConstraints = ArrayList<Constraint>()
    private var auxiliaryIndex = 0
    private var activeActivation: Variable? = null

    /** 编译 snapshot。 / Compile the snapshot. */
    fun compile(
        assumptions: List<BooleanLiteral> = emptyList(),
        fixedValues: Map<VariableId, Int64> = emptyMap(),
        diagnosticMode: Boolean = false,
        activeActivationIds: Set<String>? = null
    ): Ret<ScipConstraintProgrammingCompiledModel> {
        return try {
            compileVariables(diagnosticMode).flatMapResult {
                compileIntervals().flatMapResult {
                    compileConstraints(diagnosticMode).flatMapResult {
                        compileAssumptions(assumptions).flatMapResult {
                            compileFixedValues(fixedValues).flatMapResult {
                                compileActivationStates(diagnosticMode, activeActivationIds).flatMapResult {
                                    compileObjectives().map {
                                        ScipConstraintProgrammingCompiledModel(
                                            variables = variables.toMap(),
                                            intervals = intervals.toMap(),
                                            constraints = constraintMap.toMap(),
                                            activations = activations.toMap(),
                                            allVariables = allVariables.toList(),
                                            allConstraints = allConstraints.toList()
                                        ).attachOwner(scip)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } catch (error: Throwable) {
            cleanup()
            Failed(
                ErrorCode.OREngineModelingException,
                "SCIP CP 编译失败：${error.message ?: error::class.simpleName} / " +
                    "SCIP CP compilation failed: ${error.message ?: error::class.simpleName}"
            )
        }
    }

    /** 编译本轮固定 assumptions。 / Compile fixed assumptions for this solve. */
    private fun compileAssumptions(assumptions: List<BooleanLiteral>): Ret<Unit> {
        for ((index, literal) in assumptions.withIndex()) {
            val form = literalForm(literal)
            if (form.failed) {
                return propagate(form)
            }
            val constraint = addFormConstraint("cp-assumption-$index", form.value!!, 1.0, 1.0)
            if (constraint.failed) {
                return propagate(constraint)
            }
        }
        return ok(Unit)
    }

    /** 编译本轮必须固定的整数值。 / Compile integer values fixed for this solve. */
    private fun compileFixedValues(fixedValues: Map<VariableId, Int64>): Ret<Unit> {
        for ((id, value) in fixedValues) {
            val definition = snapshot.variable(id)
                ?: return Failed(
                    ErrorCode.DataNotFound,
                    "固定值引用了未注册 CP 变量：$id / Fixed value references an unregistered CP variable: $id"
                )
            if (!definition.domain.contains(value)) {
                return Failed(
                    ErrorCode.ORSolutionInvalid,
                    "固定值超出 CP 变量值域：$id=$value / Fixed value is outside the CP variable domain: $id=$value"
                )
            }
            val variable = variables[id]
                ?: return Failed(
                    ErrorCode.DataNotFound,
                    "固定值缺少 SCIP 变量：$id / SCIP model misses fixed CP variable: $id"
                )
            val exact = value.toLong().toDouble()
            if (kotlin.math.abs(exact) > MAX_EXACT_DOUBLE_INTEGER) {
                return Failed(
                    ErrorCode.Other,
                    "固定值超出 SCIP double 精度范围：$id=$value / Fixed value exceeds SCIP double precision: $id=$value"
                )
            }
            val result = addLinearConstraint(
                name = "cp-fixed-${id.value.replace(Regex("[^A-Za-z0-9_]+"), "_")}",
                variables = arrayOf(variable),
                coefficients = doubleArrayOf(1.0),
                lowerBound = exact,
                upperBound = exact
            )
            if (result.failed) {
                return propagate(result)
            }
        }
        return ok(Unit)
    }

    /** 失败时释放已创建的 native 句柄。 / Release native handles after an unsuccessful compilation. */
    fun cleanup() {
        allConstraints.asReversed().forEach { constraint ->
            try {
                scip.releaseCons(constraint)
            } catch (_: Throwable) {
            }
        }
        allVariables.asReversed().forEach { variable ->
            try {
                scip.releaseVar(variable)
            } catch (_: Throwable) {
            }
        }
        allConstraints.clear()
        allVariables.clear()
    }

    private fun compileVariables(diagnosticMode: Boolean): Ret<Unit> {
        for (definition in snapshot.variables) {
            val bounds = if (diagnosticMode) {
                diagnosticBaseBounds(definition)
            } else {
                solverBounds(definition.domain)
            }
            if (bounds.failed) {
                return propagate(bounds)
            }
            val type = if (definition.domain == IntegerDomain.boolean) {
                SCIP_Vartype.SCIP_VARTYPE_BINARY
            } else {
                SCIP_Vartype.SCIP_VARTYPE_INTEGER
            }
            val variable = scip.createVar(
                definition.name.ifBlank { definition.id.value },
                bounds.value!!.first,
                bounds.value!!.second,
                0.0,
                type
            )
            variables[definition.id] = variable
            allVariables += variable
            if (diagnosticMode) {
                compileDiagnosticBounds(definition, variable).onFailure { return it }
            }
            val sparseDomain = definition.domain as? IntegerDomain.Values
            if (sparseDomain != null && sparseDomain != IntegerDomain.boolean) {
                val values = sparseDomain.values
                if (values.size > sparseDomainLimit) {
                    return Failed(
                        ErrorCode.Other,
                        "稀疏值域超出 SCIP 编译规模上限 / Sparse domain exceeds the SCIP compilation limit"
                    )
                }
                compileSparseDomain(definition.id, variable, values, diagnosticMode).onFailure { return it }
            }
        }
        return ok(Unit)
    }

    private fun compileSparseDomain(
        id: VariableId,
        variable: Variable,
        values: List<Int64>,
        diagnosticMode: Boolean
    ): Ret<Unit> {
        val selectors = values.mapIndexed { index, value ->
            val selector = scip.createVar(
                "cp-domain-${id.value}-$index",
                0.0,
                1.0,
                0.0,
                SCIP_Vartype.SCIP_VARTYPE_BINARY
            )
            allVariables += selector
            selector to value
        }
        val activation = if (diagnosticMode) {
            activationFor("variable:${id.value}:domain", InfeasibilityMember.VariableDomain(
                fuookami.ospf.kotlin.core.solver.report.VariableDomainRef(id)
            ))
        } else {
            null
        }
        val one = withActivation(activation?.variable) {
            addLinearConstraint(
                name = "cp-domain-${id.value}-exactly-one",
                variables = selectors.map { it.first }.toTypedArray(),
                coefficients = DoubleArray(selectors.size) { 1.0 },
                lowerBound = 1.0,
                upperBound = 1.0
            )
        }
        if (one.failed) {
            return propagate(one)
        }
        val vars = ArrayList<Variable>(selectors.size + 1)
        val coefficients = ArrayList<Double>(selectors.size + 1)
        vars += variable
        coefficients += 1.0
        for ((selector, value) in selectors) {
            val converted = safeDouble(value, "CP sparse domain value")
            if (converted.failed) {
                return propagate(converted)
            }
            vars += selector
            coefficients += -converted.value!!
        }
        val link = withActivation(activation?.variable) {
            addLinearConstraint(
                name = "cp-domain-${id.value}-link",
                variables = vars.toTypedArray(),
                coefficients = coefficients.toDoubleArray(),
                lowerBound = 0.0,
                upperBound = 0.0
            )
        }
        if (link.failed) {
            return propagate(link)
        }
        return ok(Unit)
    }

    /**
     * 在诊断模型中为原始上下界创建可关闭的 activation 约束。 / / Create switchable activation constraints for original bounds in diagnostic mode.
     */
    private fun compileDiagnosticBounds(
        definition: fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingVariableSnapshot,
        variable: Variable
    ): Ret<Unit> {
        val base = diagnosticBaseBounds(definition)
        if (base.failed) {
            return propagate(base)
        }
        val original = solverBounds(definition.domain)
        if (original.failed) {
            return propagate(original)
        }
        val (baseLower, baseUpper) = base.value!!
        val (lower, upper) = original.value!!
        if (lower > baseLower) {
            val activation = activationFor(
                id = "variable:${definition.id.value}:lower",
                member = InfeasibilityMember.VariableBound(
                    fuookami.ospf.kotlin.core.solver.report.VariableBoundRef(
                        definition.id,
                        fuookami.ospf.kotlin.core.solver.report.BoundSide.Lower
                    )
                )
            )
            val result = withActivation(activation.variable) {
                addLinearConstraint(
                    name = "cp-bound-${definition.id.value}-lower",
                    variables = arrayOf(variable),
                    coefficients = doubleArrayOf(1.0),
                    lowerBound = lower,
                    upperBound = scip.infinity()
                )
            }
            if (result.failed) {
                return propagate(result)
            }
        }
        if (upper < baseUpper) {
            val activation = activationFor(
                id = "variable:${definition.id.value}:upper",
                member = InfeasibilityMember.VariableBound(
                    fuookami.ospf.kotlin.core.solver.report.VariableBoundRef(
                        definition.id,
                        fuookami.ospf.kotlin.core.solver.report.BoundSide.Upper
                    )
                )
            )
            val result = withActivation(activation.variable) {
                addLinearConstraint(
                    name = "cp-bound-${definition.id.value}-upper",
                    variables = arrayOf(variable),
                    coefficients = doubleArrayOf(1.0),
                    lowerBound = -scip.infinity(),
                    upperBound = upper
                )
            }
            if (result.failed) {
                return propagate(result)
            }
        }
        return ok(Unit)
    }

    /**
     * 计算诊断专用的安全基础值域；所有原始边界都必须包含在其中。 / / Compute a safe diagnostic base domain containing every original bound.
     */
    private fun diagnosticBaseBounds(
        definition: fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingVariableSnapshot
    ): Ret<Pair<Double, Double>> {
        val original = solverBounds(definition.domain)
        if (original.failed) {
            return propagate(original)
        }
        val lower = original.value!!.first
        val upper = original.value!!.second
        return if (lower < -MAX_EXACT_DOUBLE_INTEGER || upper > MAX_EXACT_DOUBLE_INTEGER) {
            Failed(
                ErrorCode.Other,
                "无法为变量构造安全诊断基础值域：${definition.id.value} / " +
                    "Cannot construct a safe diagnostic base domain for variable: ${definition.id.value}"
            )
        } else {
            val bounds = if (definition.domain == IntegerDomain.boolean) {
                0.0 to 1.0
            } else if (lower >= 0.0) {
                0.0 to MAX_EXACT_DOUBLE_INTEGER
            } else {
                -MAX_EXACT_DOUBLE_INTEGER to MAX_EXACT_DOUBLE_INTEGER
            }
            ok(bounds)
        }
    }

    private fun compileIntervals(): Ret<Unit> {
        for (interval in snapshot.intervals) {
            if (interval.optional) {
                return Failed(
                    ErrorCode.Other,
                    "SCIP CP 首版不支持 optional interval / Optional intervals are unsupported in the first SCIP CP compiler"
                )
            }
            val start = asVariable(interval.start)
            val end = asVariable(interval.end)
            val size = constantValue(interval.size)
            if (start == null || end == null || size == null || size < Int64.zero) {
                return Failed(
                    ErrorCode.Other,
                    "SCIP CP 首版只支持固定 duration 的标量 interval / " +
                    "The first SCIP CP compiler only supports scalar fixed-duration intervals"
                )
            }
            val sizeDouble = safeDouble(size, "interval size")
            if (sizeDouble.failed) {
                return propagate(sizeDouble)
            }
            val link = linearForm(interval.end).flatMapResult { endForm ->
                linearForm(interval.start).flatMapResult { startForm ->
                    addFormConstraint(
                        name = "cp-interval-${interval.id.value}-link",
                        form = combine(
                            endForm,
                            startForm,
                            1.0,
                            -1.0
                        ).copy(constant = endForm.constant - startForm.constant - sizeDouble.value!!),
                        lowerBound = 0.0,
                        upperBound = 0.0
                    )
                }
            }
            if (link.failed) {
                return propagate(link)
            }
            val sizeLong = size.toLong()
            if (sizeLong > Int.MAX_VALUE.toLong()) {
                return Failed(
                    ErrorCode.Other,
                    "interval duration 超出 JSCIP Int 范围 / Interval duration exceeds the JSCIP Int range"
                )
            }
            intervals[interval.id] = ScipConstraintProgrammingInterval(start, sizeLong.toInt(), end)
        }
        return ok(Unit)
    }

    private fun compileConstraints(diagnosticMode: Boolean): Ret<Unit> {
        for (entry in snapshot.constraints) {
            val activation = if (diagnosticMode) {
                activationFor(
                    id = "constraint:${entry.id.value}",
                    member = InfeasibilityMember.Constraint(entry.id)
                )
            } else {
                null
            }
            val compiled = withActivation(activation?.variable) {
                compileConstraint(entry.constraint, entry.id.value)
            }
            if (compiled.failed) {
                return propagate(compiled)
            }
            constraintMap[entry.id] = compiled.value!!
        }
        return ok(Unit)
    }

    /** 固定本轮 activation 状态；关闭的成员不会以隐藏基础约束保留。 / Fix activation states for this round. */
    private fun compileActivationStates(
        diagnosticMode: Boolean,
        activeActivationIds: Set<String>?
    ): Ret<Unit> {
        if (!diagnosticMode) {
            return ok(Unit)
        }
        val active = activeActivationIds ?: activations.keys
        val unknown = activeActivationIds?.filterNot { it in activations.keys }.orEmpty()
        if (unknown.isNotEmpty()) {
            return Failed(
                ErrorCode.DataNotFound,
                "SCIP CP conflict activation ID 未知：${unknown.joinToString(",")} / " +
                    "SCIP CP conflict activation IDs are unknown: ${unknown.joinToString(",")}"
            )
        }
        for ((id, activation) in activations) {
            val result = addLinearConstraint(
                name = "cp-activation-state-$id",
                variables = arrayOf(activation.variable),
                coefficients = doubleArrayOf(1.0),
                lowerBound = if (id in active) 1.0 else 0.0,
                upperBound = if (id in active) 1.0 else 0.0
            )
            if (result.failed) {
                return propagate(result)
            }
        }
        return ok(Unit)
    }

    private fun compileConstraint(
        constraint: ConstraintProgrammingConstraint,
        name: String
    ): Ret<List<Constraint>> {
        return when (constraint) {
            is ConstraintProgrammingConstraint.IntegerComparison -> {
                linearForm(constraint.expression).flatMapResult { form ->
                    safeDouble(constraint.rhs, "comparison rhs").flatMapResult { rhs ->
                        val bounds = when (constraint.comparison) {
                            fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingComparison.Equal -> 0.0 to 0.0
                            fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingComparison.LessOrEqual -> -scip.infinity() to 0.0
                            fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingComparison.GreaterOrEqual -> 0.0 to scip.infinity()
                        }
                        addFormConstraint(
                            name = name,
                            form = form,
                            lowerBound = bounds.first + rhs,
                            upperBound = bounds.second + rhs
                        )
                    }
                }
            }

            is ConstraintProgrammingConstraint.Literal -> {
                literalForm(constraint.literal).flatMapResult { form ->
                    addFormConstraint(name, form, 1.0, 1.0)
                }
            }

            is ConstraintProgrammingConstraint.BoolAnd -> {
                val result = ArrayList<Constraint>()
                for ((index, literal) in constraint.literals.withIndex()) {
                    val compiled = literalForm(literal).flatMapResult {
                        addFormConstraint("$name-and-$index", it, 1.0, 1.0)
                    }
                    if (compiled.failed) {
                        return propagate(compiled)
                    }
                    result += compiled.value!!
                }
                ok(result)
            }

            is ConstraintProgrammingConstraint.BoolOr -> {
                val forms = constraint.literals.map { literalForm(it) }
                val failed = forms.firstOrNull { it.failed }
                if (failed != null) {
                    propagate(failed)
                } else {
                    val form = forms.map { it.value!! }.reduceOrNull(::plusForm)
                        ?: LinearForm(emptyMap(), 0.0)
                    addFormConstraint(name, form, 1.0, scip.infinity())
                }
            }

            is ConstraintProgrammingConstraint.BoolXor -> {
                val forms = constraint.literals.map { literalForm(it) }
                val failed = forms.firstOrNull { it.failed }
                if (failed != null) {
                    propagate(failed)
                } else {
                    val form = forms.map { it.value!! }.reduceOrNull(::plusForm)
                        ?: LinearForm(emptyMap(), 0.0)
                    addFormConstraint(name, form, 1.0, 1.0)
                }
            }

            is ConstraintProgrammingConstraint.Implication -> compileImplication(
                constraint.enforcement,
                constraint.constraint,
                name
            )

            is ConstraintProgrammingConstraint.Reified -> compileReified(constraint, name)

            is ConstraintProgrammingConstraint.AllDifferent -> compileAllDifferent(constraint, name)

            is ConstraintProgrammingConstraint.Element -> compileElement(constraint, name)

            is ConstraintProgrammingConstraint.AllowedAssignments -> compileAllowedAssignments(constraint, name)

            is ConstraintProgrammingConstraint.ForbiddenAssignments -> compileForbiddenAssignments(constraint, name)

            is NoOverlap -> compileNoOverlap(constraint, name)

            is Cumulative -> compileCumulative(constraint, name)

            is ConstraintProgrammingConstraint.Circuit -> unsupportedConstraint(
                "SCIP CP 首版不支持 Circuit / Circuit is unsupported by the first SCIP CP compiler"
            )

            is ConstraintProgrammingConstraint.Automaton -> unsupportedConstraint(
                "SCIP CP 首版不支持 Automaton / Automaton is unsupported by the first SCIP CP compiler"
            )

            is ConstraintProgrammingConstraint.Reservoir -> unsupportedConstraint(
                "SCIP CP 首版不支持 Reservoir / Reservoir is unsupported by the first SCIP CP compiler"
            )
        }
    }

    private fun unsupportedConstraint(message: String): Ret<List<Constraint>> {
        return Failed(ErrorCode.Other, message)
    }

    private fun compileImplication(
        enforcement: BooleanLiteral,
        child: ConstraintProgrammingConstraint,
        name: String
    ): Ret<List<Constraint>> {
        if (enforcement.constant == false) {
            return ok(emptyList())
        }
        if (enforcement.constant == true) {
            return compileConstraint(child, name)
        }
        val enforcementVariable = enforcementVariable(enforcement, name)
        if (enforcementVariable.failed) {
            return propagate(enforcementVariable)
        }
        val indicator = enforcementVariable.value!!
        if (child !is ConstraintProgrammingConstraint.IntegerComparison) {
            val nested = compileConstraint(child, "$name-child")
            if (nested.failed || nested.value!!.size != 1) {
                return Failed(
                    ErrorCode.Other,
                    "复杂 indicator 约束必须可编译为单个 SCIP constraint / " +
                        "Complex indicators must compile to one SCIP constraint"
                )
            }
            val wrapped = scip.createConsSuperindicator(name, indicator, nested.value!!.single())
            val registered = registerConstraint(name, wrapped)
            return if (registered.failed) {
                propagate(registered)
            } else {
                ok(listOf(registered.value!!))
            }
        }
        val formResult = linearForm(child.expression)
        if (formResult.failed) {
            return propagate(formResult)
        }
        val form = formResult.value!!
        val rhs = safeDouble(child.rhs, "indicator rhs")
        if (rhs.failed) {
            return propagate(rhs)
        }
        val result = ArrayList<Constraint>()
        when (child.comparison) {
            fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingComparison.LessOrEqual -> {
                val constraint = createIndicator(name, indicator, form, -scip.infinity(), rhs.value!!)
                if (constraint.failed) return propagate(constraint)
                result += constraint.value!!
            }
            fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingComparison.GreaterOrEqual -> {
                val constraint = createIndicator(name, indicator, negateForm(form), -scip.infinity(), -rhs.value!!)
                if (constraint.failed) return propagate(constraint)
                result += constraint.value!!
            }
            fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingComparison.Equal -> {
                val lower = createIndicator("$name-ge", indicator, negateForm(form), -scip.infinity(), -rhs.value!!)
                val upper = createIndicator("$name-le", indicator, form, -scip.infinity(), rhs.value!!)
                if (lower.failed) return propagate(lower)
                if (upper.failed) return propagate(upper)
                result += lower.value!!
                result += upper.value!!
            }
        }
        return ok(result)
    }

    private fun compileReified(
        constraint: ConstraintProgrammingConstraint.Reified,
        name: String
    ): Ret<List<Constraint>> {
        if (constraint.direction == ReificationDirection.Implies) {
            return compileImplication(constraint.literal, constraint.constraint, name)
        }
        return Failed(
            ErrorCode.Other,
            "SCIP CP 首版只支持单向 reification / The first SCIP CP compiler only supports one-way reification"
        )
    }

    private fun compileAllDifferent(
        constraint: ConstraintProgrammingConstraint.AllDifferent,
        name: String
    ): Ret<List<Constraint>> {
        if (constraint.expressions.size > decompositionLimit) {
            return Failed(ErrorCode.Other, "AllDifferent 分解规模超限 / AllDifferent decomposition exceeds the limit")
        }
        val result = ArrayList<Constraint>()
        for (i in constraint.expressions.indices) {
            for (j in i + 1 until constraint.expressions.size) {
                val left = linearForm(constraint.expressions[i])
                val right = linearForm(constraint.expressions[j])
                if (left.failed) return propagate(left)
                if (right.failed) return propagate(right)
                val difference = combine(left.value!!, right.value!!, 1.0, -1.0)
                val bounds = formBounds(difference)
                    ?: return Failed(ErrorCode.Other, "AllDifferent 需要有限表达式界 / AllDifferent requires finite expression bounds")
                val bigM = maxOf(kotlin.math.abs(bounds.first), kotlin.math.abs(bounds.second)) + 1.0
                if (!bigM.isFinite() || bigM > MAX_EXACT_DOUBLE_INTEGER) {
                    return Failed(
                        ErrorCode.Other,
                        "AllDifferent Big-M 超出 SCIP 精确整数范围 / AllDifferent Big-M exceeds SCIP's exact integer range"
                    )
                }
                val order = auxiliaryBinary("$name-order-$i-$j")
                val upperForm = difference.copy(
                    variables = difference.variables + (order to bigM)
                )
                val lowerForm = negateForm(difference).copy(
                    variables = negateForm(difference).variables + (order to -bigM)
                )
                val upper = addFormConstraint(
                    "$name-upper-$i-$j",
                    upperForm,
                    -scip.infinity(),
                    bigM - 1.0
                )
                val lower = addFormConstraint(
                    "$name-lower-$i-$j",
                    lowerForm,
                    -scip.infinity(),
                    -1.0
                )
                if (upper.failed) return propagate(upper)
                if (lower.failed) return propagate(lower)
                result += upper.value!!
                result += lower.value!!
            }
        }
        return ok(result)
    }

    private fun compileElement(
        constraint: ConstraintProgrammingConstraint.Element,
        name: String
    ): Ret<List<Constraint>> {
        val constants = constraint.values.map { value ->
            when (value) {
                is Int64 -> value
                is Long -> Int64(value)
                is Int -> Int64(value.toLong())
                else -> return Failed(ErrorCode.Other, "Element 首版只支持常量数组 / First Element compiler supports constants only")
            }
        }
        if (constants.size > decompositionLimit) {
            return Failed(ErrorCode.Other, "Element 分解规模超限 / Element decomposition exceeds the limit")
        }
        val selectors = constants.indices.map { index -> auxiliaryBinary("$name-select-$index") }
        val result = ArrayList<Constraint>()
        val one = addLinearConstraint("$name-one", selectors.toTypedArray(), DoubleArray(selectors.size) { 1.0 }, 1.0, 1.0)
        if (one.failed) return propagate(one)
        result += one.value!!
        val indexForm = linearForm(constraint.index)
        val targetForm = linearForm(constraint.target)
        if (indexForm.failed) return propagate(indexForm)
        if (targetForm.failed) return propagate(targetForm)
        val indexSelectors = selectorsToForm(selectors, constants.indices.map { Int64(it.toLong()) })
        if (indexSelectors.failed) return propagate(indexSelectors)
        val targetSelectors = selectorsToForm(selectors, constants)
        if (targetSelectors.failed) return propagate(targetSelectors)
        val indexLink = combine(indexForm.value!!, indexSelectors.value!!, 1.0, -1.0)
        val targetLink = combine(targetForm.value!!, targetSelectors.value!!, 1.0, -1.0)
        val indexConstraint = addFormConstraint("$name-index", indexLink, 0.0, 0.0)
        val targetConstraint = addFormConstraint("$name-target", targetLink, 0.0, 0.0)
        if (indexConstraint.failed) return propagate(indexConstraint)
        if (targetConstraint.failed) return propagate(targetConstraint)
        result += indexConstraint.value!!
        result += targetConstraint.value!!
        return ok(result)
    }

    private fun compileAllowedAssignments(
        constraint: ConstraintProgrammingConstraint.AllowedAssignments,
        name: String
    ): Ret<List<Constraint>> {
        if (constraint.tuples.size > decompositionLimit) {
            return Failed(ErrorCode.Other, "Allowed table 分解规模超限 / Allowed table decomposition exceeds the limit")
        }
        val selectors = constraint.tuples.indices.map { auxiliaryBinary("$name-tuple-$it") }
        val result = ArrayList<Constraint>()
        val one = addLinearConstraint("$name-one", selectors.toTypedArray(), DoubleArray(selectors.size) { 1.0 }, 1.0, 1.0)
        if (one.failed) return propagate(one)
        result += one.value!!
        for (column in constraint.expressions.indices) {
            val expression = linearForm(constraint.expressions[column])
            if (expression.failed) return propagate(expression)
            val values = selectorsToForm(selectors, constraint.tuples.map { it[column] })
            if (values.failed) return propagate(values)
            val link = addFormConstraint(
                "$name-link-$column",
                combine(expression.value!!, values.value!!, 1.0, -1.0),
                0.0,
                0.0
            )
            if (link.failed) return propagate(link)
            result += link.value!!
        }
        return ok(result)
    }

    private fun compileForbiddenAssignments(
        constraint: ConstraintProgrammingConstraint.ForbiddenAssignments,
        name: String
    ): Ret<List<Constraint>> {
        if (constraint.tuples.size > decompositionLimit) {
            return Failed(
                ErrorCode.Other,
                "Forbidden table 分解规模超限 / Forbidden table decomposition exceeds the limit"
            )
        }
        val result = ArrayList<Constraint>()
        for ((row, tuple) in constraint.tuples.withIndex()) {
            val differences = ArrayList<Variable>()
            for (column in constraint.expressions.indices) {
                val expression = linearForm(constraint.expressions[column])
                if (expression.failed) {
                    return propagate(expression)
                }
                val bounds = formBounds(expression.value!!)
                    ?: return Failed(
                        ErrorCode.Other,
                        "Forbidden table 需要有限表达式界 / Forbidden table requires finite expression bounds"
                    )
                val value = safeDouble(tuple[column], "forbidden table tuple value")
                if (value.failed) {
                    return propagate(value)
                }
                val left = auxiliaryBinary("$name-$row-$column-left")
                val right = auxiliaryBinary("$name-$row-$column-right")

                val leftBigM = maxOf(1.0, bounds.second - (value.value!! - 1.0))
                if (!leftBigM.isFinite() || leftBigM > MAX_EXACT_DOUBLE_INTEGER) {
                    return Failed(
                        ErrorCode.Other,
                        "Forbidden table Big-M 超出 SCIP 精确整数范围 / Forbidden table Big-M exceeds SCIP's exact integer range"
                    )
                }
                val leftForm = expression.value!!.copy(
                    variables = expression.value!!.variables + (left to leftBigM)
                )
                val leftConstraint = addFormConstraint(
                    "$name-$row-$column-left-link",
                    leftForm,
                    -scip.infinity(),
                    value.value!! - 1.0 + leftBigM
                )
                if (leftConstraint.failed) {
                    return propagate(leftConstraint)
                }
                result += leftConstraint.value!!

                val rightBigM = maxOf(1.0, value.value!! + 1.0 - bounds.first)
                if (!rightBigM.isFinite() || rightBigM > MAX_EXACT_DOUBLE_INTEGER) {
                    return Failed(
                        ErrorCode.Other,
                        "Forbidden table Big-M 超出 SCIP 精确整数范围 / Forbidden table Big-M exceeds SCIP's exact integer range"
                    )
                }
                val rightForm = expression.value!!.copy(
                    variables = expression.value!!.variables + (right to -rightBigM)
                )
                val rightConstraint = addFormConstraint(
                    "$name-$row-$column-right-link",
                    rightForm,
                    value.value!! + 1.0 - rightBigM,
                    scip.infinity()
                )
                if (rightConstraint.failed) {
                    return propagate(rightConstraint)
                }
                result += rightConstraint.value!!
                differences += left
                differences += right
            }
            val differenceConstraint = addLinearConstraint(
                "$name-$row-difference",
                differences.toTypedArray(),
                DoubleArray(differences.size) { 1.0 },
                1.0,
                scip.infinity()
            )
            if (differenceConstraint.failed) {
                return propagate(differenceConstraint)
            }
            result += differenceConstraint.value!!
        }
        return ok(result)
    }

    private fun compileNoOverlap(noOverlap: NoOverlap, name: String): Ret<List<Constraint>> {
        val result = ArrayList<Constraint>()
        for (i in noOverlap.intervals.indices) {
            for (j in i + 1 until noOverlap.intervals.size) {
                val first = intervals[noOverlap.intervals[i].id]
                    ?: return Failed(ErrorCode.IllegalArgument, "未编译 interval / Interval was not compiled")
                val second = intervals[noOverlap.intervals[j].id]
                    ?: return Failed(ErrorCode.IllegalArgument, "未编译 interval / Interval was not compiled")
                val order = auxiliaryBinary("$name-order-$i-$j")
                val firstStartBounds = formBounds(LinearForm(linkedMapOf(first.start to 1.0), 0.0))
                    ?: return Failed(ErrorCode.Other, "NoOverlap 需要有限 start 值域 / NoOverlap requires finite start bounds")
                val secondStartBounds = formBounds(LinearForm(linkedMapOf(second.start to 1.0), 0.0))
                    ?: return Failed(ErrorCode.Other, "NoOverlap 需要有限 start 值域 / NoOverlap requires finite start bounds")
                val bigM = maxOf(firstStartBounds.second - secondStartBounds.first + first.size, secondStartBounds.second - firstStartBounds.first + second.size) + 1.0
                if (!bigM.isFinite() || bigM > MAX_EXACT_DOUBLE_INTEGER) {
                    return Failed(
                        ErrorCode.Other,
                        "NoOverlap Big-M 超出 SCIP 精确整数范围 / NoOverlap Big-M exceeds SCIP's exact integer range"
                    )
                }
                val firstBefore = addLinearConstraint(
                    "$name-first-before-$i-$j",
                    arrayOf(first.start, second.start, order),
                    doubleArrayOf(1.0, -1.0, bigM),
                    -scip.infinity(),
                    bigM - first.size
                )
                if (firstBefore.failed) return propagate(firstBefore)
                val secondBefore = addLinearConstraint(
                    "$name-second-before-$i-$j",
                    arrayOf(second.start, first.start, order),
                    doubleArrayOf(1.0, -1.0, -bigM),
                    -scip.infinity(),
                    -second.size.toDouble()
                )
                if (secondBefore.failed) return propagate(secondBefore)
                result += firstBefore.value!!
                result += secondBefore.value!!
            }
        }
        return ok(result)
    }

    private fun compileCumulative(cumulative: Cumulative, name: String): Ret<List<Constraint>> {
        val compiled = cumulative.intervals.map { intervals[it.id] }
        if (compiled.any { it == null }) {
            return Failed(ErrorCode.IllegalArgument, "Cumulative 引用了未注册 interval / Cumulative references an unknown interval")
        }
        val nonNullCompiled = compiled.filterNotNull()
        val starts = nonNullCompiled.map { it.start }
        val durations = nonNullCompiled.map { it.size }.toIntArray()
        val demandValues = cumulative.demands.map { constantValue(it)?.toLong() }
        val capacityValue = constantValue(cumulative.capacity)?.toLong()
        if (demandValues.any { it == null } ||
            capacityValue == null ||
            capacityValue < 0L ||
            demandValues.any { it!! < 0L } ||
            demandValues.any { it!! > Int.MAX_VALUE.toLong() } ||
            capacityValue > Int.MAX_VALUE.toLong()
        ) {
            return Failed(
                ErrorCode.Other,
                "SCIP native cumulative 需要固定非负 duration、demand 和 capacity / " +
                    "SCIP native cumulative requires fixed non-negative duration, demand, and capacity"
            )
        }
        val demands = demandValues.map { it!!.toInt() }.toIntArray()
        val capacity = capacityValue.toInt()
        return try {
            val constraint = scip.createConsCumulative(
                name,
                starts.toTypedArray(),
                durations,
                demands,
                capacity
            )
            registerConstraint(name, constraint).map { listOf(it) }
        } catch (error: Throwable) {
            Failed(ErrorCode.OREngineModelingException, "SCIP cumulative 编译失败 / SCIP cumulative compilation failed: ${error.message}")
        }
    }

    private fun compileObjectives(): Ret<Unit> {
        val objective = snapshot.objectives.firstOrNull() ?: return ok(Unit)
        val form = linearForm(objective.expression)
        if (form.failed) return propagate(form)
        form.value!!.variables.forEach { (variable, coefficient) ->
            scip.changeVarObj(variable, coefficient)
        }
        when (snapshot.objectCategory) {
            fuookami.ospf.kotlin.core.model.basic.ObjectCategory.Minimum -> scip.setMinimize()
            fuookami.ospf.kotlin.core.model.basic.ObjectCategory.Maximum -> scip.setMaximize()
        }
        return ok(Unit)
    }

    private fun createIndicator(
        name: String,
        indicator: Variable,
        form: LinearForm,
        lowerBound: Double,
        upperBound: Double
    ): Ret<Constraint> {
        if (lowerBound != -scip.infinity()) {
            return Failed(ErrorCode.Other, "indicator lower bound must be converted to upper-bound form")
        }
        return try {
            val constraint = scip.createConsIndicator(
                name,
                indicator,
                form.variables.keys.toTypedArray(),
                form.variables.values.toDoubleArray(),
                upperBound - form.constant
            )
            registerConstraint(name, constraint)
        } catch (error: Throwable) {
            Failed(ErrorCode.OREngineModelingException, "SCIP indicator 编译失败 / SCIP indicator compilation failed: ${error.message}")
        }
    }

    private fun enforcementVariable(literal: BooleanLiteral, name: String): Ret<Variable> {
        if (literal.constant != null) {
            return Failed(ErrorCode.IllegalArgument, "布尔常量不应请求 solver variable / Boolean constant has no solver variable")
        }
        val variable = literal.variableId?.let(variables::get)
            ?: return Failed(ErrorCode.IllegalArgument, "assumption 变量未注册 / Assumption variable is not registered")
        if (!literal.negated) {
            return ok(variable)
        }
        val negated = auxiliaryBinary("$name-negated")
        val link = addLinearConstraint(
            "$name-negated-link",
            arrayOf(variable, negated),
            doubleArrayOf(1.0, 1.0),
            1.0,
            1.0
        )
        return if (link.failed) propagate(link) else ok(negated)
    }

    private fun auxiliaryBinary(name: String): Variable {
        val variable = scip.createVar(
            "cp-aux-${auxiliaryIndex++}-$name",
            0.0,
            1.0,
            0.0,
            SCIP_Vartype.SCIP_VARTYPE_BINARY
        )
        allVariables += variable
        return variable
    }

    private fun addFormConstraint(
        name: String,
        form: LinearForm,
        lowerBound: Double,
        upperBound: Double
    ): Ret<List<Constraint>> {
        return addLinearConstraint(
            name,
            form.variables.keys.toTypedArray(),
            form.variables.values.toDoubleArray(),
            lowerBound - form.constant,
            upperBound - form.constant
        ).map { listOf(it) }
    }

    private fun addLinearConstraint(
        name: String,
        variables: Array<Variable>,
        coefficients: DoubleArray,
        lowerBound: Double,
        upperBound: Double
    ): Ret<Constraint> {
        return try {
            val constraint = scip.createConsLinear(name, variables, coefficients, lowerBound, upperBound)
            registerConstraint(name, constraint)
        } catch (error: Throwable) {
            Failed(ErrorCode.OREngineModelingException, "SCIP linear constraint 编译失败 / SCIP linear constraint compilation failed: ${error.message}")
        }
    }

    /** 将约束注册为普通约束或 activation superindicator。 / Register a plain constraint or an activation superindicator. */
    private fun registerConstraint(name: String, constraint: Constraint): Ret<Constraint> {
        return try {
            val activation = activeActivation
            if (activation == null) {
                scip.addCons(constraint)
                allConstraints += constraint
                ok(constraint)
            } else {
                val wrapped = scip.createConsSuperindicator(name, activation, constraint)
                scip.addCons(wrapped)
                allConstraints += wrapped
                try {
                    scip.releaseCons(constraint)
                } catch (_: Throwable) {
                    // The wrapper owns the child constraint at the native boundary.
                    // superindicator 在原生边界持有子约束引用。
                }
                ok(wrapped)
            }
        } catch (error: Throwable) {
            Failed(
                ErrorCode.OREngineModelingException,
                "SCIP activation 约束注册失败：${error.message} / " +
                    "SCIP activation constraint registration failed: ${error.message}"
            )
        }
    }

    /** 临时设置当前 activation；嵌套编译结束后恢复外层状态。 / Set a scoped activation for nested compilation. */
    private inline fun <T> withActivation(
        activation: Variable?,
        block: () -> Ret<T>
    ): Ret<T> {
        val previous = activeActivation
        activeActivation = activation
        return try {
            block()
        } finally {
            activeActivation = previous
        }
    }

    /** 创建或复用稳定 activation 变量。 / Create or reuse a stable activation variable. */
    private fun activationFor(
        id: String,
        member: InfeasibilityMember
    ): ScipConstraintProgrammingActivation {
        return activations[id] ?: ScipConstraintProgrammingActivation(
            id = id,
            member = member,
            variable = auxiliaryBinary("activation-$id")
        ).also { activations[id] = it }
    }

    private fun linearForm(expression: ConstraintProgrammingExpression): Ret<LinearForm> {
        return when (expression) {
            is ConstraintProgrammingExpression.Constant -> safeDouble(expression.value, "CP constant")
                .map { LinearForm(emptyMap(), it) }
            is ConstraintProgrammingExpression.Invalid -> Failed(ErrorCode.IllegalArgument, expression.message)
            is ConstraintProgrammingExpression.Variable -> {
                val variable = variables[expression.variableId]
                    ?: return Failed(ErrorCode.IllegalArgument, "CP 表达式变量未注册 / CP expression variable is not registered")
                ok(LinearForm(linkedMapOf(variable to 1.0), 0.0))
            }
            is ConstraintProgrammingExpression.Linear -> {
                val terms = LinkedHashMap<Variable, Double>()
                for (term in expression.terms) {
                    val variable = variables[term.variableId]
                        ?: return Failed(ErrorCode.IllegalArgument, "CP 线性项变量未注册 / CP linear term variable is not registered")
                    val coefficient = safeDouble(term.coefficient, "CP coefficient")
                    if (coefficient.failed) {
                        return propagate(coefficient)
                    }
                    terms[variable] = (terms[variable] ?: 0.0) + coefficient.value!!
                }
                safeDouble(expression.constant, "CP linear constant")
                    .map { constant -> LinearForm(terms, constant) }
            }
        }
    }

    private fun literalForm(literal: BooleanLiteral): Ret<LinearForm> {
        val constant = literal.constant
        if (constant != null) {
            return ok(LinearForm(emptyMap(), if (constant) 1.0 else 0.0))
        }
        val variable = literal.variableId?.let(variables::get)
            ?: return Failed(ErrorCode.IllegalArgument, "布尔文字变量未注册 / Boolean literal variable is not registered")
        return if (literal.negated) {
            ok(LinearForm(linkedMapOf(variable to -1.0), 1.0))
        } else {
            ok(LinearForm(linkedMapOf(variable to 1.0), 0.0))
        }
    }

    private fun asVariable(expression: ConstraintProgrammingExpression): Variable? {
        return (expression as? ConstraintProgrammingExpression.Variable)?.variableId?.let(variables::get)
    }

    private fun constantValue(expression: ConstraintProgrammingExpression): Int64? {
        return (expression as? ConstraintProgrammingExpression.Constant)?.value
    }

    private fun selectorsToForm(selectors: List<Variable>, values: List<Int64>): Ret<LinearForm> {
        val variables = LinkedHashMap<Variable, Double>()
        for ((index, variable) in selectors.withIndex()) {
            val value = safeDouble(values[index], "selector value")
            if (value.failed) {
                return propagate(value)
            }
            variables[variable] = value.value!!
        }
        return ok(LinearForm(variables, 0.0))
    }

    private fun formBounds(form: LinearForm): Pair<Double, Double>? {
        var lower = form.constant
        var upper = form.constant
        for ((variable, coefficient) in form.variables) {
            val definition = snapshot.variables.firstOrNull { variables[it.id] == variable } ?: return null
            val bounds = solverBounds(definition.domain)
            if (bounds.failed) return null
            if (coefficient >= 0.0) {
                lower += coefficient * bounds.value!!.first
                upper += coefficient * bounds.value!!.second
            } else {
                lower += coefficient * bounds.value!!.second
                upper += coefficient * bounds.value!!.first
            }
        }
        return lower to upper
    }

    private fun combine(left: LinearForm, right: LinearForm, leftScale: Double, rightScale: Double): LinearForm {
        val result = LinkedHashMap<Variable, Double>()
        left.variables.forEach { (variable, coefficient) -> result[variable] = coefficient * leftScale }
        right.variables.forEach { (variable, coefficient) -> result[variable] = (result[variable] ?: 0.0) + coefficient * rightScale }
        return LinearForm(result.filterValues { it != 0.0 }, left.constant * leftScale + right.constant * rightScale)
    }

    private fun plusForm(left: LinearForm, right: LinearForm): LinearForm {
        return combine(left, right, 1.0, 1.0)
    }

    private fun negateForm(form: LinearForm): LinearForm {
        return LinearForm(form.variables.mapValues { -it.value }, -form.constant)
    }

    private fun solverBounds(domain: IntegerDomain): Ret<Pair<Double, Double>> {
        return safeDouble(domain.lowerBound, "CP domain lower bound").flatMapResult { lower ->
            safeDouble(domain.upperBound, "CP domain upper bound").map { upper ->
                lower to upper
            }
        }
    }

    private fun safeDouble(value: Int64, context: String): Ret<Double> {
        val long = value.toLong()
        if (kotlin.math.abs(long.toDouble()) > MAX_EXACT_DOUBLE_INTEGER) {
            return Failed(
                ErrorCode.IllegalArgument,
                "$context 超出 SCIP double 精确整数范围 / $context exceeds SCIP's exact integer range"
            )
        }
        return ok(long.toDouble())
    }

    private data class LinearForm(
        val variables: Map<Variable, Double>,
        val constant: Double
    )

    private companion object {
        const val DEFAULT_SPARSE_DOMAIN_LIMIT = 128
        const val DEFAULT_DECOMPOSITION_LIMIT = 256
        const val MAX_EXACT_DOUBLE_INTEGER = 9_007_199_254_740_991.0
    }
}

private inline fun <T> Ret<T>.onFailure(block: (Ret<T>) -> Nothing): Ret<T> {
    if (failed) {
        block(this)
    }
    return this
}

private inline fun <T, U> Ret<T>.flatMapResult(transform: (T) -> Ret<U>): Ret<U> {
    return when (this) {
        is fuookami.ospf.kotlin.utils.functional.Ok -> transform(value)
        is Failed -> Failed(error)
        is Fatal -> Fatal(errors)
    }
}

private fun <T> propagate(result: Ret<*>): Ret<T> {
    return when (result) {
        is Failed -> Failed(result.error)
        is Fatal -> Fatal(result.errors)
        else -> Failed(ErrorCode.ApplicationError, "SCIP CP 编译结果状态无效 / Invalid SCIP CP compiler result state")
    }
}
