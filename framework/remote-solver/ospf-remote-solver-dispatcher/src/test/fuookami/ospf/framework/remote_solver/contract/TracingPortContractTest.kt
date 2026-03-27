package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.port.TracingPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

abstract class TracingPortContractTest {
    protected abstract fun createFixture(): TestFixture<TracingPort>

    @Test
    fun inSpanShouldReturnBlockResult() {
        val fixture = createFixture()
        try {
            runSuspend {
                val result = fixture.subject.inSpan("span-a") { 42 }
                assertEquals(42, result)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun inSpanShouldPropagateExceptions() {
        val fixture = createFixture()
        try {
            val exception = assertFailsWith<IllegalStateException> {
                runSuspend {
                    fixture.subject.inSpan("span-b") {
                        throw IllegalStateException("boom")
                    }
                }
            }
            assertEquals("boom", exception.message)
        } finally {
            fixture.close()
        }
    }
}
