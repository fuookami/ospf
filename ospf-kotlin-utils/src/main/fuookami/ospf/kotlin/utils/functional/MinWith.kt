package fuookami.ospf.kotlin.utils.functional

fun <T> Iterable<T>.minWithComparator(comparator: Comparator<T>): T {
    return this.minOfWith({ lhs, rhs ->
        if (comparator(lhs, rhs)) {
            -1
        } else if (comparator(rhs, lhs)) {
            1
        } else {
            0
        }
    }) { it }
}

fun <T> Iterable<T>.minWithPartialComparator(comparator: PartialComparator<T>): T {
    return this.minOfWith({ lhs, rhs ->
        if (comparator(lhs, rhs) == true) {
            -1
        } else if (comparator(rhs, lhs) == true) {
            1
        } else {
            0
        }
    }) { it }
}

fun <T> Iterable<T>.minWithThreeWayComparator(comparator: ThreeWayComparator<T>): T {
    return this.minOfWith({ lhs, rhs ->
        comparator(lhs, rhs).value
    }) { it }
}

fun <T> Iterable<T>.minWithPartialThreeWayComparator(comparator: PartialThreeWayComparator<T>): T {
    return this.minOfWith({ lhs, rhs ->
        comparator(lhs, rhs)?.value ?: 0
    }) { it }
}
