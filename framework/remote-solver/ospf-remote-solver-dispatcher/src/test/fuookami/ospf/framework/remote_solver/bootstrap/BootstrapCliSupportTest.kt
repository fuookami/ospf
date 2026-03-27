package fuookami.ospf.framework.remote_solver.bootstrap

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class BootstrapCliSupportTest {
    @Test
    fun parseArgsShouldHandleKeyValuePairsAndFlags() {
        val parsed = BootstrapCliSupport.parseArgs(
            arrayOf("--config", "a.properties", "--port", "18080", "--dry-run")
        )
        assertEquals("a.properties", parsed["config"])
        assertEquals("18080", parsed["port"])
        assertEquals("true", parsed["dry-run"])
    }

    @Test
    fun resolveConfigPathShouldUseProvidedCliPath() {
        val path = BootstrapCliSupport.resolveConfigPath("deploy/config/scheduler.properties")
        assertTrue(path.toString().endsWith("deploy\\config\\scheduler.properties") || path.toString().endsWith("deploy/config/scheduler.properties"))
    }
}

