package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

abstract class ObjectStoragePortContractTest {
    protected abstract fun createFixture(): TestFixture<ObjectStoragePort>

    @Test
    fun putAndGetByVersionRef() {
        val fixture = createFixture()
        try {
            runSuspend {
                val expected = "hello-storage".toByteArray()
                val ref = fixture.subject.put("models/task-a", expected)
                val actual = fixture.subject.get(ref)

                assertTrue(actual != null)
                assertTrue(actual.contentEquals(expected))
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun latestReferenceResolvesToNewestVersion() {
        val fixture = createFixture()
        try {
            runSuspend {
                val oldRef = fixture.subject.put("models/task-b", "old".toByteArray())
                val newRef = fixture.subject.put("models/task-b", "new".toByteArray())

                assertTrue(oldRef.version != null)
                assertTrue(newRef.version != null)
                assertFalse(oldRef.version == newRef.version)

                val latest = fixture.subject.get(ObjectRef.of(path = "models/task-b"))
                assertTrue(latest != null)
                assertEquals("new", latest.decodeToString())
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun deleteRemovesObject() {
        val fixture = createFixture()
        try {
            runSuspend {
                val ref = fixture.subject.put("models/task-c", "payload".toByteArray())
                assertTrue(fixture.subject.exists(ref))

                val removed = fixture.subject.delete(ref)
                assertTrue(removed)
                assertFalse(fixture.subject.exists(ref))
                assertTrue(fixture.subject.get(ref) == null)
            }
        } finally {
            fixture.close()
        }
    }
}
