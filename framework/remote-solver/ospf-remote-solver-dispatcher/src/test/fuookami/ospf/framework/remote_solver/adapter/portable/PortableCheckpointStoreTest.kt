package fuookami.ospf.framework.remote_solver.adapter.portable

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlinx.coroutines.runBlocking
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingIncumbent
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingIntervalValue

/**
 * Portable checkpoint store regression tests.
 * portable checkpoint store 回归测试。
 */
class PortableCheckpointStoreTest {
    @Test
    fun roundTripVerifiesDigestAndTenantScope() = runBlocking {
        val store = PortableCheckpointStore(InMemoryObjectStoragePort(SystemClockPort()))
        val envelope = PortableCheckpointEnvelope(
            checkpointId = "cp-1",
            modelFingerprint = "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a",
            createdAtEpochMs = 1L,
            snapshotJson = "{}",
            incumbent = PortableConstraintProgrammingIncumbent(
                valuesById = mapOf("x" to Long.MAX_VALUE),
                intervalsById = mapOf(
                    "job" to PortableConstraintProgrammingIntervalValue(1L, 2L, 3L)
                )
            )
        )
        val ref = store.put("tenant-a", "task-1", envelope)
        assertNotNull(store.get("tenant-a", ref))
        assertEquals(Long.MAX_VALUE, store.get("tenant-a", ref)!!.incumbent!!.valuesById["x"])
        assertNull(store.get("tenant-b", ref))
    }

    @Test
    fun corruptedArtifactIsRejected() = runBlocking {
        val storage = InMemoryObjectStoragePort(SystemClockPort())
        val store = PortableCheckpointStore(storage)
        val ref = storage.put(ObjectRef.of("tenant-a/checkpoint/task/corrupt").path, "{}".encodeToByteArray())
        assertNull(store.get(ref))
    }
}
