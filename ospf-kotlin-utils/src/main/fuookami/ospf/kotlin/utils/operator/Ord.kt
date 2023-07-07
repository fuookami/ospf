package fuookami.ospf.kotlin.utils.operator

sealed interface Order {
    val value: Int

    data class Less(override val value: Int = -1) : Order {
        init {
            assert(value < 0)
        }
    }

    object Equal : Order {
        override val value = 0
    }

    data class Greater(override val value: Int = 1) : Order {
        init {
            assert(value > 0)
        }
    }
}

fun orderOf(value: Int): Order {
    return if (value < 0) {
        Order.Less(value)
    } else if (value > 0) {
        Order.Greater(value)
    } else {
        Order.Equal
    }
}

fun <T: Comparable<T>> orderBetween(lhs: T, rhs: T): Order {
    return orderOf(lhs.compareTo(rhs))
}

interface PartialOrd<Self> : PartialEq<Self> {
    infix fun partialOrd(rhs: Self): Order?
}

interface Ord<Self> : PartialOrd<Self>, Eq<Self>, Comparable<Self> {
    infix fun ord(rhs: Self): Order {
        return (this partialOrd rhs)!!
    }

    override fun compareTo(other: Self): Int {
        return (this ord other).value
    }

    infix fun ls(rhs: Self): Boolean {
        return this < rhs
    }

    infix fun leq(rhs: Self): Boolean {
        return this <= rhs
    }

    infix fun gr(rhs: Self): Boolean {
        return this > rhs
    }

    infix fun geq(rhs: Self): Boolean {
        return this >= rhs
    }
}
