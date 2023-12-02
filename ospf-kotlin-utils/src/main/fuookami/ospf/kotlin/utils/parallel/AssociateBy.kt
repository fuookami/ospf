package fuookami.ospf.kotlin.utils.parallel

import kotlinx.coroutines.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*

suspend inline fun <K, T> Iterable<T>.associateByParallelly(crossinline extractor: Extractor<K, T>): Map<K, T> {
    return this.associateByParallelly(UInt64.ten, extractor)
}

suspend inline fun <K, T> Iterable<T>.associateByParallelly(
    segment: UInt64,
    crossinline extractor: Extractor<K, T>
): Map<K, T> {
    return coroutineScope {
        val promises = ArrayList<Deferred<List<Pair<K, T>>>>()
        val iterator = this@associateByParallelly.iterator()
        while (iterator.hasNext()) {
            val thisSegment = ArrayList<T>()
            var i = UInt64.zero
            while (iterator.hasNext() && i != segment) {
                thisSegment.add(iterator.next())
                ++i
            }
            promises.add(async(Dispatchers.Default) {
                thisSegment.map { Pair(extractor(it), it) }
            })
        }

        promises.flatMap { it.await() }.toMap()
    }
}

suspend inline fun <K, T> Iterable<T>.associateByParallelly(crossinline extractor: TryExtractor<K, T>): Result<Map<K, T>, Error> {
    return this.associateByParallelly(UInt64.ten, extractor)
}

suspend inline fun <K, T> Iterable<T>.associateByParallelly(
    segment: UInt64,
    crossinline extractor: TryExtractor<K, T>
): Result<Map<K, T>, Error> {
    var error: Error? = null

    return try {
        coroutineScope {
            val promises = ArrayList<Deferred<List<Pair<K, T>>>>()
            val iterator = this@associateByParallelly.iterator()
            while (iterator.hasNext()) {
                val thisSegment = ArrayList<T>()
                var i = UInt64.zero
                while (iterator.hasNext() && i != segment) {
                    thisSegment.add(iterator.next())
                    ++i
                }
                promises.add(async(Dispatchers.Default) {
                    thisSegment.mapNotNull {
                        when (val result = extractor(it)) {
                            is Ok -> {
                                Pair(result.value, it)
                            }

                            is Failed -> {
                                error = result.error
                                cancel()
                                null
                            }
                        }
                    }
                })
            }

            Ok(promises.flatMap { it.await() }.toMap())
        }
    } catch (e: CancellationException) {
        error?.let { Failed(it) } ?: Ok(emptyMap())
    }
}

suspend inline fun <K, T> Collection<T>.associateByParallelly(crossinline extractor: Extractor<K, T>): Map<K, T> {
    return (this as Iterable<T>).associateByParallelly(
        UInt64(
            maxOf(
                minOf(
                    Flt64(this.size).log(Flt64.two)!!.toFlt64().floor().toUInt64().toInt(),
                    Runtime.getRuntime().availableProcessors()
                ),
                1
            )
        ),
        extractor
    )
}

suspend inline fun <K, T> Collection<T>.associateByParallelly(
    concurrentAmount: UInt64,
    crossinline extractor: Extractor<K, T>
): Map<K, T> {
    return (this as Iterable<T>).associateByParallelly(UInt64(this.size) / concurrentAmount, extractor)
}

suspend inline fun <K, T> Collection<T>.associateByParallelly(crossinline extractor: TryExtractor<K, T>): Result<Map<K, T>, Error> {
    return (this as Iterable<T>).associateByParallelly(
        UInt64(
            maxOf(
                minOf(
                    Flt64(this.size).log(Flt64.two)!!.toFlt64().floor().toUInt64().toInt(),
                    Runtime.getRuntime().availableProcessors()
                ),
                1
            )
        ),
        extractor
    )
}

suspend inline fun <K, T> Collection<T>.associateByParallelly(
    concurrentAmount: UInt64,
    crossinline extractor: TryExtractor<K, T>
): Result<Map<K, T>, Error> {
    return (this as Iterable<T>).associateByParallelly(UInt64(this.size) / concurrentAmount, extractor)
}

suspend inline fun <K, T> List<T>.associateByParallelly(crossinline extractor: Extractor<K, T>): Map<K, T> {
    return this.associateByParallelly(
        UInt64(
            maxOf(
                minOf(
                    Flt64(this.size).log(Flt64.two)!!.toFlt64().floor().toUInt64().toInt(),
                    Runtime.getRuntime().availableProcessors()
                ),
                1
            )
        ),
        extractor
    )
}

suspend inline fun <K, T> List<T>.associateByParallelly(
    concurrentAmount: UInt64,
    crossinline extractor: Extractor<K, T>
): Map<K, T> {
    return coroutineScope {
        val promises = ArrayList<Deferred<List<Pair<K, T>>>>()
        val segmentAmount = this@associateByParallelly.size / concurrentAmount.toInt()
        var i = 0
        while (i != this@associateByParallelly.size) {
            val j = i
            val k = i + minOf(
                segmentAmount,
                this@associateByParallelly.size - i
            )
            promises.add(async(Dispatchers.Default) {
                this@associateByParallelly.subList(j, k).map { Pair(extractor(it), it) }
            })
            i = k
        }

        promises.flatMap { it.await() }.toMap()
    }
}

suspend inline fun <K, T> List<T>.associateByParallelly(crossinline extractor: TryExtractor<K, T>): Result<Map<K, T>, Error> {
    return this.associateByParallelly(
        UInt64(
            maxOf(
                minOf(
                    Flt64(this.size).log(Flt64.two)!!.toFlt64().floor().toUInt64().toInt(),
                    Runtime.getRuntime().availableProcessors()
                ),
                1
            )
        ),
        extractor
    )
}

suspend inline fun <K, T> List<T>.associateByParallelly(
    concurrentAmount: UInt64,
    crossinline extractor: TryExtractor<K, T>
): Result<Map<K, T>, Error> {
    var error: Error? = null

    return try {
        coroutineScope {
            val promises = ArrayList<Deferred<List<Pair<K, T>>>>()
            val segmentAmount = this@associateByParallelly.size / concurrentAmount.toInt()
            var i = 0
            while (i != this@associateByParallelly.size) {
                val j = i
                val k = i + minOf(
                    segmentAmount,
                    this@associateByParallelly.size - i
                )
                promises.add(async(Dispatchers.Default) {
                    this@associateByParallelly.subList(j, k).mapNotNull {
                        when (val result = extractor(it)) {
                            is Ok -> {
                                Pair(result.value, it)
                            }

                            is Failed -> {
                                error = result.error
                                cancel()
                                null
                            }
                        }
                    }
                })
                i = k
            }

            Ok(promises.flatMap { it.await() }.toMap())
        }
    } catch (e: CancellationException) {
        error?.let { Failed(it) } ?: Ok(emptyMap())
    }
}
