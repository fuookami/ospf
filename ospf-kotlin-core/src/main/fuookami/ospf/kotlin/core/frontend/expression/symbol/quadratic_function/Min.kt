package fuookami.ospf.kotlin.core.frontend.expression.symbol.quadratic_function

import org.apache.logging.log4j.kotlin.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.symbol.*
import fuookami.ospf.kotlin.utils.math.value_range.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*

sealed class AbstractMinFunction(
    protected val polynomials: List<AbstractQuadraticPolynomial<*>>,
    private val exact: Boolean = true,
    override var name: String,
    override var displayName: String? = null
) : QuadraticFunctionSymbol {
    private val logger = logger()

    private val maxmin: RealVar by lazy {
        RealVar("${name}_maxmin")
    }

    private val u: BinVariable1 by lazy {
        val u = BinVariable1("${name}_u", Shape1(polynomials.size))
        for ((i, polynomial) in polynomials.withIndex()) {
            u[i].name = "${u.name}_${polynomial.name.ifEmpty { "$i" }}"
        }
        u
    }

    private val y: AbstractQuadraticPolynomial<*> by lazy {
        val y = QuadraticPolynomial(maxmin, "${name}_y")
        y.range.set(m)
        y
    }

    override val discrete by lazy {
        polynomials.all { it.discrete }
    }

    override val range get() = y.range
    override val lowerBound get() = y.lowerBound
    override val upperBound get() = y.upperBound

    override val category: Category = Linear

    override val dependencies: Set<IntermediateSymbol>
        get() {
            val dependencies = HashSet<IntermediateSymbol>()
            for (polynomial in polynomials) {
                dependencies.addAll(polynomial.dependencies)
            }
            return dependencies
        }
    override val cells get() = y.cells
    override val cached get() = y.cached

    private val possibleRange
        get() = ValueRange(
            polynomials.minOf { it.lowerBound!!.value.unwrap() },
            polynomials.maxOf { it.upperBound!!.value.unwrap() }
        ).value!!
    private var m = possibleRange

    override fun flush(force: Boolean) {
        for (polynomial in polynomials) {
            polynomial.flush(force)
        }
        y.flush(force)
        val newM = possibleRange
        if (m neq newM) {
            maxmin.range.set(m)
            m = newM
        }
    }

    override fun prepare(tokenTable: AbstractTokenTable) {
        for (polynomial in polynomials) {
            polynomial.cells
        }

        if (tokenTable.cachedSolution && tokenTable.cached(this) == false) {
            val values = polynomials.map { it.value(tokenTable) }

            if (values.all { it != null }) {
                val min = values.withIndex().minByOrNull { it.value!! } ?: return

                logger.trace { "Setting MinFunction ${name}.maxmin to ${min.value}" }
                tokenTable.find(maxmin)?.let { token ->
                    token._result = min.value!!
                }

                if (exact) {
                    for (i in polynomials.indices) {
                        if (i == min.index) {
                            logger.trace { "Setting MinFunction ${name}.u[$i] to true" }
                            tokenTable.find(u[i])?.let { token ->
                                token._result = Flt64.one
                            }
                        } else {
                            logger.trace { "Setting MinFunction ${name}.u[$i] to false" }
                            tokenTable.find(u[i])?.let { token ->
                                token._result = Flt64.zero
                            }
                        }
                    }
                }

                tokenTable.cache(this, null, min.value!!)
            }
        }
    }

    override fun register(tokenTable: AbstractMutableTokenTable): Try {
        when (val result = tokenTable.add(maxmin)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (exact) {
            when (val result = tokenTable.add(u)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    override fun register(model: AbstractQuadraticMechanismModel): Try {
        for ((i, polynomial) in polynomials.withIndex()) {
            when (val result = model.addConstraint(
                maxmin leq polynomial,
                "${name}_lb_${polynomial.name.ifEmpty { "$i" }}"
            )) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        if (exact) {
            for ((i, polynomial) in polynomials.withIndex()) {
                when (val result = model.addConstraint(
                    maxmin geq (polynomial - m.upperBound.value.unwrap() * (Flt64.one - u[i])),
                    "${name}_ub_${polynomial.name.ifEmpty { "$i" }}"
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }

            when (val result = model.addConstraint(
                sum(u) eq Flt64.one,
                "${name}_u"
            )) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    override fun toString(): String {
        return displayName ?: name
    }

    override fun value(tokenList: AbstractTokenList, zeroIfNone: Boolean): Flt64? {
        return polynomials.minOf { it.value(tokenList, zeroIfNone) ?: return null }
    }

    override fun value(results: List<Flt64>, tokenList: AbstractTokenList, zeroIfNone: Boolean): Flt64? {
        return polynomials.minOf { it.value(results, tokenList, zeroIfNone) ?: return null }
    }

    override fun calculateValue(tokenTable: AbstractTokenTable, zeroIfNone: Boolean): Flt64? {
        return polynomials.minOf { it.value(tokenTable, zeroIfNone) ?: return null }
    }

    override fun calculateValue(results: List<Flt64>, tokenTable: AbstractTokenTable, zeroIfNone: Boolean): Flt64? {
        return polynomials.minOf { it.value(results, tokenTable, zeroIfNone) ?: return null }
    }
}

class MaxMinFunction(
    polynomials: List<AbstractQuadraticPolynomial<*>>,
    name: String,
    displayName: String? = null
) : AbstractMinFunction(polynomials, true, name, displayName) {
    override fun toRawString(unfold: Boolean): String {
        return "min(${polynomials.joinToString(", ") { it.toRawString(unfold) }})"
    }
}

class MinFunction(
    polynomials: List<AbstractQuadraticPolynomial<*>>,
    name: String,
    displayName: String? = null
) : AbstractMinFunction(polynomials, false, name, displayName) {
    override fun toRawString(unfold: Boolean): String {
        return "maxmin(${polynomials.joinToString(", ") { it.toRawString(unfold) }})"
    }
}
