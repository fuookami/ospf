package fuookami.ospf.kotlin.core.frontend.model.callback

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.operator.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*

interface CallBackModelPolicy<V> {
    val comparator: ThreeWayComparator<V>

    fun compareObjective(lhs: V?, rhs: V?): Order? {
        return if (lhs != null && rhs == null) {
            Order.Less()
        } else if (lhs == null && rhs != null) {
            Order.Greater()
        } else if (lhs != null && rhs != null) {
            comparator(lhs, rhs)
        } else {
            null
        }
    }

    fun initialSolutions(initialSolutionAmount: UInt64, variableAmount: UInt64): List<Solution> {
        return listOf((UInt64.zero until variableAmount).map { Flt64.zero })
    }
}

class FunctionalCallBackModelPolicy<V>(
    val objectiveComparator: PartialComparator<V>,
    val initialSolutionsGenerator: Extractor<Flt64, Pair<UInt64, UInt64>> = { Flt64.zero }
) : CallBackModelPolicy<V> {
    override val comparator: ThreeWayComparator<V> = { lhs, rhs ->
        if (objectiveComparator(lhs, rhs) == true || objectiveComparator(rhs, lhs) == false) {
            Order.Less(-1)
        } else if (objectiveComparator(lhs, rhs) == false || objectiveComparator(rhs, lhs) == true) {
            Order.Greater(1)
        } else {
            Order.Equal
        }
    }

    override fun compareObjective(lhs: V?, rhs: V?): Order? {
        return if (lhs != null && rhs == null) {
            Order.Less()
        } else if (lhs == null && rhs != null) {
            Order.Greater()
        } else if (lhs != null && rhs != null) {
            if (objectiveComparator(lhs, rhs) == true) {
                Order.Less()
            } else if (objectiveComparator(rhs, lhs) == true) {
                Order.Greater()
            } else {
                Order.Equal
            }
        } else {
            null
        }
    }

    override fun initialSolutions(
        initialSolutionAmount: UInt64,
        variableAmount: UInt64
    ): List<Solution> {
        return (UInt64.zero until initialSolutionAmount).map { solution ->
            (UInt64.zero until variableAmount).map {
                initialSolutionsGenerator(
                    Pair(solution, it)
                )
            }
        }
    }
}

class CallBackModel internal constructor(
    override val objectCategory: ObjectCategory = ObjectCategory.Minimum,
    override val tokens: MutableTokenList = ManualAddTokenTokenList(),
    private val _constraints: MutableList<Pair<Extractor<Boolean?, Solution>, String>> = ArrayList(),
    private val _objectiveFunctions: MutableList<Pair<Extractor<Flt64?, Solution>, String>> = ArrayList(),
    private val policy: CallBackModelPolicy<Flt64>
) : CallBackModelInterface {
    companion object {
        private fun dumpObjectiveComparator(category: ObjectCategory): PartialComparator<Flt64> = when (category) {
            ObjectCategory.Maximum -> { lhs, rhs -> lhs gr rhs }
            ObjectCategory.Minimum -> { lhs, rhs -> lhs ls rhs }
        }

        operator fun invoke(
            objectCategory: ObjectCategory = ObjectCategory.Minimum,
            initialSolutionGenerator: Extractor<Flt64, Pair<UInt64, UInt64>> = { Flt64.zero }
        ) = CallBackModel(
            objectCategory = objectCategory,
            policy = FunctionalCallBackModelPolicy(dumpObjectiveComparator(objectCategory), initialSolutionGenerator)
        )

        operator fun invoke(
            objectiveComparator: PartialComparator<Flt64>,
            initialSolutionGenerator: Extractor<Flt64, Pair<UInt64, UInt64>> = { Flt64.zero }
        ) = CallBackModel(policy = FunctionalCallBackModelPolicy(objectiveComparator, initialSolutionGenerator))

        operator fun invoke(
            model: SingleObjectModel<*, *>,
            initialSolutionGenerator: Extractor<Flt64, Pair<UInt64, UInt64>> = { Flt64.zero }
        ): CallBackModel {
            val tokens: MutableTokenList = ManualAddTokenTokenList()
            for (token in model.tokens.tokens) {
                tokens.add(token.variable)
            }
            val constraints = model.constraints.map { constraint ->
                Pair<Extractor<Boolean?, Solution>, String>(
                    { solution: Solution -> constraint.isTrue(solution) },
                    constraint.name
                )
            }.toMutableList()
            val objectiveFunction = model.objectFunction.subObjects.map { objective ->
                Pair<Extractor<Flt64?, Solution>, String>(
                    { solution: Solution ->
                        if (objective.category == model.objectFunction.category) {
                            objective.value(solution)
                        } else {
                            -objective.value(solution)
                        }
                    },
                    objective.name
                )
            }.toMutableList()
            return CallBackModel(
                model.objectFunction.category,
                tokens,
                constraints,
                objectiveFunction,
                FunctionalCallBackModelPolicy(
                    dumpObjectiveComparator(model.objectFunction.category),
                    initialSolutionGenerator
                )
            )
        }
    }

    override val constraints by this::_constraints
    override val objectiveFunctions by this::_objectiveFunctions

    override fun initialSolutions(initialSolutionAmount: UInt64): List<Solution> {
        return policy.initialSolutions(initialSolutionAmount, UInt64(tokens.tokenIndexMap.size))
    }

    override fun compareObjective(lhs: Flt64, rhs: Flt64): Order {
        return policy.comparator(lhs, rhs)
    }

    override fun compareObjective(lhs: Flt64?, rhs: Flt64?): Order? {
        return policy.compareObjective(lhs, rhs)
    }

    override fun addVar(item: AbstractVariableItem<*, *>) {
        tokens.add(item)
    }

    override fun addVars(items: Iterable<AbstractVariableItem<*, *>>) {
        tokens.add(items)
    }

    override fun remove(item: AbstractVariableItem<*, *>) {
        tokens.remove(item)
    }

    override fun addConstraint(
        inequality: Inequality<*, *>,
        name: String?,
        displayName: String?
    ) {
        _constraints.add(
            Pair(
                { solution: Solution -> inequality.isTrue(solution, tokens) },
                name ?: String()
            )
        )
    }

    fun addObject(
        category: ObjectCategory,
        expression: Expression,
        name: String?,
        displayName: String?
    ) {
        _objectiveFunctions.add(
            Pair(
                { solution: Solution ->
                    if (category == objectCategory) {
                        expression.value(solution, tokens)
                    } else {
                        expression.value(solution, tokens)?.let { -it }
                    }
                },
                name ?: String()
            )
        )
    }

    override fun addObject(
        category: ObjectCategory,
        polynomial: Polynomial<*, *, *, *>,
        name: String?,
        displayName: String?
    ) {
        _objectiveFunctions.add(
            Pair(
                { solution: Solution ->
                    if (category == objectCategory) {
                        polynomial.value(solution, tokens)
                    } else {
                        polynomial.value(solution, tokens)?.let { -it }
                    }
                },
                name ?: String()
            )
        )
    }

    override fun <T : RealNumber<T>> addObject(
        category: ObjectCategory,
        constant: T,
        name: String?,
        displayName: String?
    ) {
        _objectiveFunctions.add(
            Pair({ constant.toFlt64() }, name ?: String())
        )
    }

    fun addObject(
        category: ObjectCategory,
        func: Extractor<Flt64?, Solution>,
        name: String? = null,
        displayName: String? = null
    ) {
        _objectiveFunctions.add(
            Pair(
                { solution: Solution ->
                    if (category == objectCategory) {
                        func(solution)
                    } else {
                        func(solution)?.let { -it }
                    }
                },
                name ?: String()
            )
        )
    }

    fun maximize(
        expression: Expression,
        name: String?,
        displayName: String?
    ) {
        addObject(ObjectCategory.Maximum, expression, name, displayName)
    }

    fun maximize(
        func: Extractor<Flt64?, Solution>,
        name: String?,
        displayName: String?
    ) {
        addObject(ObjectCategory.Maximum, func, name, displayName)
    }

    fun minimize(
        expression: Expression,
        name: String?,
        displayName: String?
    ) {
        addObject(ObjectCategory.Minimum, expression, name, displayName)
    }

    fun minimize(
        func: Extractor<Flt64?, Solution>,
        name: String?,
        displayName: String?
    ) {
        addObject(ObjectCategory.Minimum, func, name, displayName)
    }

    override fun setSolution(solution: Solution) {
        tokens.setSolution(solution)
    }

    override fun setSolution(solution: Map<AbstractVariableItem<*, *>, Flt64>) {
        tokens.setSolution(solution)
    }

    override fun clearSolution() {
        tokens.clearSolution()
    }
}

// todo
class MultiObjectCallBackModel
