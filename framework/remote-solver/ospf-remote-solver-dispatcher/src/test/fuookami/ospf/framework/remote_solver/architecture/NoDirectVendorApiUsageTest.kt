package fuookami.ospf.framework.remote_solver.architecture

import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.absolutePathString
import kotlin.streams.asSequence
import kotlin.test.Test
import kotlin.test.assertTrue

class NoDirectVendorApiUsageTest {
    @Test
    fun nonOspfAdapterModulesShouldNotImportVendorApis() {
        val root = Path.of("src/main/kotlin")
        if (!Files.exists(root)) {
            return
        }
        val violations = Files.walk(root).use { paths ->
            paths.asSequence()
                .filter { Files.isRegularFile(it) }
                .filter { it.toString().endsWith(".kt") }
                .filter { !it.toString().replace('\\', '/').contains("/adapter/ospf/") }
                .flatMap { file ->
                    Files.readAllLines(file).asSequence().mapIndexedNotNull { index, line ->
                        val trimmed = line.trim()
                        val isViolation = trimmed.startsWith("import ") && (
                            trimmed.contains("gurobi.") ||
                                trimmed.contains("scip.")
                            )
                        if (isViolation) {
                            "${file.absolutePathString()}:${index + 1}:$trimmed"
                        } else {
                            null
                        }
                    }
                }
                .toList()
        }

        assertTrue(
            actual = violations.isEmpty(),
            message = buildString {
                append("Vendor solver APIs must not be imported outside adapter/ospf.\n")
                append(violations.joinToString("\n"))
            }
        )
    }
}
