package fuookami.ospf.kotlin.core.frontend.expression.polynomial

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.operator.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*

private typealias ExprSymbol<Cell, C> = Symbol<Cell, C>

sealed interface Polynomial<Self : Polynomial<Self, M, Cell, C>, M : Monomial<M, Cell, C>, Cell : MonomialCell<Cell, C>, C : Category>
    : Expression, Copyable<Self>, Neg<Self>,
    Plus<Flt64, Self>, Minus<Flt64, Self>, Times<Flt64, Self>, Div<Flt64, Self> {
    val category: C
    val monomials: List<M>
    val constant: Flt64
    override val discrete: Boolean get() = monomials.all { it.discrete } && constant.round() eq constant
    val cells: List<Cell>
    val cached: Boolean

    override fun unaryMinus(): Self {
        return this.times(-Flt64.one)
    }

    operator fun <T : RealNumber<T>> plus(rhs: T): Self {
        return this.plus(rhs.toFlt64())
    }

    operator fun plus(rhs: AbstractVariableItem<*, *>): Self

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("plusVariables")
    operator fun plus(rhs: Iterable<AbstractVariableItem<*, *>>): Self
    operator fun plus(rhs: ExprSymbol<Cell, C>): Self

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("plusSymbols")
    operator fun plus(rhs: Iterable<Symbol<Cell, C>>): Self
    operator fun plus(rhs: M): Self
    operator fun plus(rhs: Polynomial<*, M, Cell, C>): Self

    operator fun <T : RealNumber<T>> minus(rhs: T): Self {
        return this.minus(rhs.toFlt64())
    }

    operator fun minus(rhs: AbstractVariableItem<*, *>): Self

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("minusVariables")
    operator fun minus(rhs: Iterable<AbstractVariableItem<*, *>>): Self
    operator fun minus(rhs: ExprSymbol<Cell, C>): Self

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("minusSymbols")
    operator fun minus(rhs: Iterable<Symbol<Cell, C>>): Self
    operator fun minus(rhs: M): Self
    operator fun minus(rhs: Polynomial<*, M, Cell, C>): Self

    operator fun <T : RealNumber<T>> times(rhs: T): Self {
        return this.times(rhs.toFlt64())
    }

    operator fun <T : RealNumber<T>> div(rhs: T): Self {
        return this.div(rhs.toFlt64())
    }

    fun toMutable(): MutablePolynomial<*, M, Cell, C>
    fun asMutable(): MutablePolynomial<*, M, Cell, C>? {
        return null
    }

    fun copy(name: String, displayName: String?): Self
    fun flush(force: Boolean = false)

    fun toRawString(unfold: Boolean = false): String {
        return if (monomials.isEmpty()) {
            "$constant"
        } else if (constant neq Flt64.zero) {
            "${monomials.joinToString(" + ") { it.toRawString(unfold) }} + $constant"
        } else {
            monomials.joinToString(" + ") { it.toRawString(unfold) }
        }
    }

    override fun value(tokenList: AbstractTokenList, zeroIfNone: Boolean): Flt64? {
        var ret = Flt64.zero
        for (monomial in monomials) {
            val thisValue = monomial.value(tokenList, zeroIfNone)
                ?: return null
            ret += thisValue
        }
        return ret
    }

    fun value(tokenTable: AbstractTokenTable<Cell, C>, zeroIfNone: Boolean = false): Flt64? {
        return value(tokenTable.tokenList, zeroIfNone)
    }

    override fun value(results: List<Flt64>, tokenList: AbstractTokenList, zeroIfNone: Boolean): Flt64? {
        var ret = Flt64.zero
        for (monomial in monomials) {
            val thisValue = monomial.value(results, tokenList, zeroIfNone)
                ?: return null
            ret += thisValue
        }
        return ret
    }

    fun value(results: List<Flt64>, tokenTable: AbstractTokenTable<Cell, C>, zeroIfNone: Boolean = false): Flt64? {
        return value(results, tokenTable.tokenList, zeroIfNone)
    }
}

sealed interface MutablePolynomial<Self : MutablePolynomial<Self, M, Cell, C>, M : Monomial<M, Cell, C>, Cell : MonomialCell<Cell, C>, C : Category>
    : Polynomial<Self, M, Cell, C>, PlusAssign<Flt64>, MinusAssign<Flt64>, TimesAssign<Flt64>, DivAssign<Flt64> {
    operator fun <T : RealNumber<T>> plusAssign(rhs: T) {
        this.plusAssign(rhs.toFlt64())
    }

    operator fun plusAssign(rhs: AbstractVariableItem<*, *>)

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("plusAssignVariables")
    operator fun plusAssign(rhs: Iterable<AbstractVariableItem<*, *>>)
    operator fun plusAssign(rhs: ExprSymbol<Cell, C>)

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("plusAssignSymbols")
    operator fun plusAssign(rhs: Iterable<Symbol<Cell, C>>)
    operator fun plusAssign(rhs: M)
    operator fun plusAssign(rhs: Polynomial<*, M, Cell, C>)

    operator fun <T : RealNumber<T>> minusAssign(rhs: T) {
        this.minusAssign(rhs.toFlt64())
    }

    operator fun minusAssign(rhs: AbstractVariableItem<*, *>)

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("minusAssignVariables")
    operator fun minusAssign(rhs: Iterable<AbstractVariableItem<*, *>>)
    operator fun minusAssign(rhs: ExprSymbol<Cell, C>)

    @Suppress("INAPPLICABLE_JVM_NAME")
    @JvmName("minusAssignSymbols")
    operator fun minusAssign(rhs: Iterable<Symbol<Cell, C>>)
    operator fun minusAssign(rhs: M)
    operator fun minusAssign(rhs: Polynomial<*, M, Cell, C>)

    operator fun <T : RealNumber<T>> timesAssign(rhs: T) {
        this.timesAssign(rhs.toFlt64())
    }

    operator fun <T : RealNumber<T>> divAssign(rhs: T) {
        this.divAssign(rhs.toFlt64())
    }
}

internal fun possibleRange(
    monomials: List<Monomial<*, *, *>>,
    constant: Flt64
): ValueRange<Flt64> {
    return if (monomials.isEmpty()) {
        ValueRange(
            constant,
            constant,
            IntervalType.Closed,
            IntervalType.Closed,
            Flt64
        )
    } else {
        var ret = monomials[0].range.range
        for (i in 1 until monomials.size) {
            ret += monomials[i].range.range
        }
        ret += constant
        ret
    }
}

internal fun <Cell : MonomialCell<Cell, C>, C : Category> cells(
    monomials: List<Monomial<*, Cell, C>>,
    constant: Flt64,
    ctor: Extractor<Cell, Flt64>
): List<Cell> {
    val cells = ArrayList<Cell>()
    var totalConstant = constant
    for (monomial in monomials) {
        val thisCells = monomial.cells
        for (cell in thisCells) {
            if (cell.isConstant) {
                totalConstant += cell.constant!!
            } else {
                var sameCell = cells.find { it == cell }
                if (sameCell != null) {
                    sameCell += cell
                } else {
                    cells.add(cell.copy())
                }
            }
        }
    }
    cells.sortBy { it.hashCode() }
    cells.add(ctor(totalConstant))
    return cells
}
