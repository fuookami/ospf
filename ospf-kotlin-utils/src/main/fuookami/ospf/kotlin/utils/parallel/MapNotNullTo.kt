package fuookami.ospf.kotlin.utils.parallel

import kotlinx.coroutines.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*

suspend inline fun <R, T, C : MutableCollection<in R>> Iterable<T>.mapNotNullToParallelly(
    destination: C,
    crossinline extractor: Extractor<R?, T>
): C {
    return this.mapNotNullToParallelly(UInt64.ten, destination, extractor)
}

suspend inline fun <R, T, C : MutableCollection<in R>> Iterable<T>.mapNotNullToParallelly(
    segment: UInt64,
    destination: C,
    crossinline extractor: Extractor<R?, T>
): C {
    return coroutineScope {
        val promises = ArrayList<Deferred<List<R>>>()
        val iterator = this@mapNotNullToParallelly.iterator()
        while (iterator.hasNext()) {
            val thisSegment = ArrayList<T>()
            var i = UInt64.zero
            while (iterator.hasNext() && i != segment) {
                thisSegment.add(iterator.next())
                ++i
            }
            promises.add(async(Dispatchers.Default) {
                thisSegment.mapNotNull(extractor)
            })
        }

        promises.flatMapTo(destination) { it.await() }
    }
}

suspend inline fun <R, T, C : MutableCollection<in R>> Collection<T>.mapNotNullToParallelly(
    destination: C,
    crossinline extractor: Extractor<R?, T>
): C {
    return this.mapNotNullToParallelly(
        UInt64(
            maxOf(
                minOf(
                    Flt64(this.size).log(Flt64.two)!!.toFlt64().floor().toUInt64().toInt(),
                    Runtime.getRuntime().availableProcessors()
                ),
                1
            )
        ),
        destination,
        extractor
    )
}

suspend inline fun <R, T, C : MutableCollection<in R>> Collection<T>.mapNotNullToParallelly(
    concurrentAmount: UInt64,
    destination: C,
    crossinline extractor: Extractor<R?, T>
): C {
    return (this as Iterable<T>).mapNotNullToParallelly(UInt64(this.size) / concurrentAmount, destination, extractor)
}

suspend inline fun <R, T, C : MutableCollection<in R>> List<T>.mapNotNullToParallelly(
    destination: C,
    crossinline extractor: Extractor<R?, T>
): C {
    return this.mapNotNullToParallelly(
        UInt64(
            maxOf(
                minOf(
                    Flt64(this.size).log(Flt64.two)!!.toFlt64().floor().toUInt64().toInt(),
                    Runtime.getRuntime().availableProcessors()
                ),
                1
            )
        ),
        destination,
        extractor
    )
}

suspend inline fun <R, T, C : MutableCollection<in R>> List<T>.mapNotNullToParallelly(
    concurrentAmount: UInt64,
    destination: C,
    crossinline extractor: Extractor<R?, T>
): C {
    return coroutineScope {
        val promises = ArrayList<Deferred<List<R>>>()
        val segmentAmount = this@mapNotNullToParallelly.size / concurrentAmount.toInt()
        var i = 0
        while (i != this@mapNotNullToParallelly.size) {
            val j = i
            val k = i + minOf(
                segmentAmount,
                this@mapNotNullToParallelly.size - i
            )
            promises.add(async(Dispatchers.Default) {
                this@mapNotNullToParallelly.subList(j, k).mapNotNull(extractor)
            })
            i = k
        }

        promises.flatMapTo(destination) { it.await() }
    }
}
