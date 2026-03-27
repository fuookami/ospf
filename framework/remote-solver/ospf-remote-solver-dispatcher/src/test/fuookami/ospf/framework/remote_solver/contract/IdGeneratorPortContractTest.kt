package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

abstract class IdGeneratorPortContractTest {
    protected abstract fun createFixture(): TestFixture<IdGeneratorPort>

    @Test
    fun generatedIdShouldContainPrefix() {
        val fixture = createFixture()
        try {
            val id = fixture.subject.newId("task")
            assertTrue(id.startsWith("task-"))
            assertTrue(id.length > "task-".length)
        } finally {
            fixture.close()
        }
    }

    @Test
    fun generatedIdsShouldBeUniqueInBatch() {
        val fixture = createFixture()
        try {
            val ids = (1..200).map { fixture.subject.newId("slice") }.toSet()
            assertEquals(200, ids.size)
        } finally {
            fixture.close()
        }
    }
}
