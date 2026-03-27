package fuookami.ospf.framework.remote_solver.bootstrap

import kotlin.reflect.KFunction
import kotlin.reflect.full.declaredFunctions
import kotlin.reflect.jvm.isAccessible
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class RemoteSolverApiMainTest {
    @Test
    fun resolvePortShouldPreferCliPortOverProperty() {
        val port = invokeResolvePort(
            args = arrayOf("--port", "19090"),
            properties = mapOf("api.http.port" to "18081")
        )
        assertEquals(19090, port)
    }

    @Test
    fun resolvePortShouldFallbackToPropertyPort() {
        val port = invokeResolvePort(
            args = emptyArray(),
            properties = mapOf("api.http.port" to "18082")
        )
        assertEquals(18082, port)
    }

    @Test
    fun resolvePortShouldRejectOutOfRangePort() {
        assertFailsWith<IllegalArgumentException> {
            invokeResolvePort(
                args = arrayOf("--port", "70000"),
                properties = emptyMap()
            )
        }
    }

    private fun invokeResolvePort(args: Array<String>, properties: Map<String, String>): Int {
        val function: KFunction<*> = RemoteSolverApiMain::class.declaredFunctions
            .first { it.name == "resolvePort" }
        function.isAccessible = true
        return try {
            function.call(RemoteSolverApiMain, args, properties) as Int
        } catch (e: Exception) {
            val cause = e.cause
            if (cause is IllegalArgumentException) {
                throw cause
            }
            throw e
        }
    }
}

