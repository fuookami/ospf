package fuookami.ospf.kotlin.utils.meta_programming

import kotlin.reflect.*

fun <T, U : Any> lazyDelegate(lazyFunc: () -> U): LazyDelegate<T, U> {
    return LazyDelegate(lazyFunc)
}

@JvmName("selfLazyDelegate")
fun <T, U : Any> lazyDelegate(lazyFunc: (T) -> U): SelfLazyDelegate<T, U> {
    return SelfLazyDelegate(lazyFunc)
}

fun <T, U : Any> lazyDelegate(lazyKProperty: KProperty1<T, U>): SelfLazyDelegate<T, U> {
    return SelfLazyDelegate { lazyKProperty(it) }
}

class LazyDelegate<T, U : Any>(
    val lazyFunc: () -> U
) {
    lateinit var range: U

    operator fun getValue(self: T, property: KProperty<*>): U {
        if (!::range.isInitialized) {
            range = lazyFunc()
        }
        return range
    }

    operator fun setValue(self: T, property: KProperty<*>, value: U) {
        if (!::range.isInitialized) {
            range = lazyFunc()
        }
        range = value
    }
}

class SelfLazyDelegate<T, U : Any>(
    val lazyFunc: (T) -> U
) {
    lateinit var range: U

    operator fun getValue(self: T, property: KProperty<*>): U {
        if (!::range.isInitialized) {
            range = lazyFunc(self)
        }
        return range
    }

    operator fun setValue(self: T, property: KProperty<*>, value: U) {
        if (!::range.isInitialized) {
            range = lazyFunc(self)
        }
        range = value
    }
}
