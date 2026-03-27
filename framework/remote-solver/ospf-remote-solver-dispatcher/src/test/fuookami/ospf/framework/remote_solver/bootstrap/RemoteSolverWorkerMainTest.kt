package fuookami.ospf.framework.remote_solver.bootstrap

import java.io.ByteArrayOutputStream
import java.io.PrintStream
import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RemoteSolverWorkerMainTest {
    @Test
    fun workerShouldWriteCheckpointAndResumeFromCheckpointFile() {
        val stateDir = Files.createTempDirectory("remote-worker-state")
        try {
            val first = captureOutput {
                RemoteSolverWorkerMain.main(
                    arrayOf(
                        "--model", "models/test",
                        "--task", "task-w1",
                        "--slice", "slice-1",
                        "--node", "node-1",
                        "--quantum-ms", "1000",
                        "--total-runtime-ms", "2000",
                        "--state-dir", stateDir.toString()
                    )
                )
            }
            assertEquals("false", first["completed"])
            val checkpoint = first["checkpointPath"] ?: ""
            assertTrue(checkpoint.isNotEmpty())
            assertTrue(Files.exists(java.nio.file.Path.of(checkpoint)))

            val second = captureOutput {
                RemoteSolverWorkerMain.main(
                    arrayOf(
                        "--model", "models/test",
                        "--task", "task-w1",
                        "--slice", "slice-2",
                        "--node", "node-1",
                        "--quantum-ms", "1000",
                        "--total-runtime-ms", "2000",
                        "--state-dir", stateDir.toString(),
                        "--checkpoint-in", checkpoint
                    )
                )
            }
            assertEquals("true", second["completed"])
            val resultPath = second["resultPath"] ?: ""
            assertTrue(resultPath.isNotEmpty())
            assertTrue(Files.exists(java.nio.file.Path.of(resultPath)))
        } finally {
            Files.walk(stateDir).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { Files.deleteIfExists(it) }
            }
        }
    }

    private fun captureOutput(block: () -> Unit): Map<String, String> {
        val oldOut = System.out
        val baos = ByteArrayOutputStream()
        try {
            System.setOut(PrintStream(baos))
            block()
        } finally {
            System.setOut(oldOut)
        }
        return baos.toString()
            .lineSequence()
            .map { it.trim() }
            .filter { it.isNotEmpty() && it.contains("=") }
            .associate {
                val idx = it.indexOf('=')
                it.substring(0, idx) to it.substring(idx + 1)
            }
    }
}
