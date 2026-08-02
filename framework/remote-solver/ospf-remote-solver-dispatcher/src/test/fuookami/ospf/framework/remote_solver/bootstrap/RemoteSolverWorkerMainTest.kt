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

    @Test
    fun cpWorkerShouldUseRealSnapshotExecutorAndReturnBackendFailureForInvalidSnapshot() {
        val stateDir = Files.createTempDirectory("remote-worker-cp-state")
        val modelPath = stateDir.resolve("invalid.snapshot.json")
        Files.writeString(modelPath, "not-json")
        try {
            val output = captureOutput {
                RemoteSolverWorkerMain.main(
                    arrayOf(
                        "--model", modelPath.toString(),
                        "--model-format", "ospf-cp-snapshot-json",
                        "--task", "task-cp",
                        "--slice", "slice-1",
                        "--tenant-id", "tenant-a",
                        "--quantum-ms", "10",
                        "--state-dir", stateDir.toString()
                    )
                )
            }
            assertEquals("false", output["completed"])
            assertEquals("false", output["feasible"])
            assertEquals("BACKEND_FAILURE", output["terminationReason"])
            assertEquals("2.0", output["schemaVersion"])
        } finally {
            Files.walk(stateDir).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { Files.deleteIfExists(it) }
            }
        }
    }

    @Test
    fun cpWorkerShouldRejectCorruptedExplicitCheckpointInsteadOfColdStarting() {
        val stateDir = Files.createTempDirectory("remote-worker-corrupt-checkpoint")
        val modelPath = stateDir.resolve("invalid.snapshot.json")
        val checkpointPath = stateDir.resolve("corrupt.checkpoint.json")
        Files.writeString(modelPath, "not-json")
        Files.writeString(checkpointPath, "{\"schema\":1,\"snapshotJson\":\"not-json\"}")
        try {
            val output = captureOutput {
                RemoteSolverWorkerMain.main(
                    arrayOf(
                        "--model", modelPath.toString(),
                        "--model-format", "ospf-cp-snapshot-json",
                        "--task", "task-cp-corrupt-checkpoint",
                        "--slice", "slice-1",
                        "--tenant-id", "tenant-a",
                        "--quantum-ms", "10",
                        "--state-dir", stateDir.toString(),
                        "--checkpoint-in", checkpointPath.toString()
                    )
                )
            }
            assertEquals("false", output["completed"])
            assertEquals("BACKEND_FAILURE", output["terminationReason"])
            assertEquals("2.0", output["schemaVersion"])
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
