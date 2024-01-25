package fuookami.ospf.kotlin.core.frontend.expression

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*

open class ExpressionRange<V>(
    private var _range: ValueRange<V>,
    private val constants: RealNumberConstants<V>
) where V : RealNumber<V>, V : NumberField<V> {
    constructor(constants: RealNumberConstants<V>) : this(
        _range = ValueRange(
            constants.minimum,
            constants.maximum,
            IntervalType.Closed,
            IntervalType.Closed,
            constants
        ),
        constants = constants
    )

    val range by ::_range
    val valueRange: ValueRange<Flt64>
        get() = ValueRange(
            range.lowerBound.toFlt64(),
            range.upperBound.toFlt64(),
            range.lowerInterval,
            range.upperInterval,
            Flt64
        )

    val lowerBound: ValueWrapper<V>?
        get() = if (empty) {
            null
        } else {
            range.lowerBound
        }

    val upperBound: ValueWrapper<V>?
        get() = if (empty) {
            null
        } else {
            range.upperBound
        }

    val lowerInterval by range::lowerInterval
    val upperInterval by range::upperInterval

    val empty by range::empty
    val fixed by range::fixed
    val fixedValue by range::fixedValue

    private var _set: Boolean = false
    internal val set: Boolean = true

    fun set(range: ValueRange<V>) {
        _set = true
        _range = range
    }

    fun intersectWith(range: ValueRange<V>): Boolean {
        _set = true
        _range = _range.intersect(range)
        return !_range.empty
    }

    infix fun ls(value: Invariant<V>): Boolean {
        return if (range.empty) {
            false
        } else {
            intersectWith(
                ValueRange(
                    lowerBound!!,
                    ValueWrapper(value.value(), constants),
                    IntervalType.Closed,
                    IntervalType.Closed,
                    constants
                )
            )
        }
    }

    infix fun leq(value: Invariant<V>): Boolean {
        return ls(value)
    }

    infix fun gr(value: Invariant<V>): Boolean {
        return if (range.empty) {
            false
        } else {
            intersectWith(
                ValueRange(
                    ValueWrapper(value.value(), constants),
                    upperBound!!,
                    IntervalType.Closed,
                    IntervalType.Closed,
                    constants
                )
            )
        }
    }

    infix fun geq(value: Invariant<V>): Boolean {
        return gr(value)
    }

    infix fun eq(value: Invariant<V>): Boolean {
        return if (range.empty) {
            false
        } else {
            intersectWith(
                ValueRange(
                    ValueWrapper(value.value(), constants),
                    ValueWrapper(value.value(), constants),
                    IntervalType.Closed,
                    IntervalType.Closed,
                    constants
                )
            )
        }
    }

    fun intersectWith(lb: Invariant<V>, ub: Invariant<V>): Boolean {
        return if (range.empty) {
            false
        } else {
            intersectWith(
                ValueRange(
                    ValueWrapper(lb.value(), constants),
                    ValueWrapper(ub.value(), constants),
                    IntervalType.Closed,
                    IntervalType.Closed,
                    constants
                )
            )
        }
    }
}

interface Expression {
    /** for lp **/
    var name: String

    /** for opm */
    var displayName: String?

    val discrete: Boolean get() = false

    val range: ExpressionRange<Flt64>
    val lowerBound: Flt64 get() = range.lowerBound?.toFlt64() ?: Flt64.minimum
    val upperBound: Flt64 get() = range.upperBound?.toFlt64() ?: Flt64.maximum

    fun value(tokenList: AbstractTokenList, zeroIfNone: Boolean = false): Flt64?
    fun value(results: List<Flt64>, tokenList: AbstractTokenList, zeroIfNone: Boolean = false): Flt64?
}
