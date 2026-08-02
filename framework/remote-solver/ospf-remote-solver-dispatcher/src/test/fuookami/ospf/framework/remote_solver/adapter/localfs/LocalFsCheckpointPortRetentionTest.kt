@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.adapter.localfs

import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import java.nio.file.Files
import kotlin.io.path.deleteIfExists
import kotlin.time.Instant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class LocalFsCheckpointPortRetentionTest {
    @Test
    fun metadataAuditFieldsRoundTrip() {
        val tempDir = Files.createTempDirectory("remote-solver-localfs-checkpoint-metadata")
        try {
            val checkpointPort = LocalFsCheckpointPort(tempDir)
            runSuspend {
                val metadata = CheckpointMetadata(
                    taskId = TaskId.of("task-audit"),
                    sliceId = SliceId.of("slice-audit"),
                    ref = ObjectRef.of(
                        path = "checkpoint/task-audit/slice-audit",
                        version = "v7",
                        etag = "sha256:checkpoint"
                    ),
                    createdAt = Instant.fromEpochMilliseconds(1234L),
                    schemaVersion = "2.0",
                    modelFingerprint = "model-sha",
                    configurationFingerprint = "configuration-sha",
                    solverFingerprint = "solver-sha",
                    integritySha256 = "artifact-sha"
                )
                checkpointPort.save(metadata)

                val restored = checkpointPort.latest("task-audit")
                assertNotNull(restored)
                assertEquals(metadata, restored)
            }
        } finally {
            Files.walk(tempDir).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
            }
        }
    }

    @Test
    fun retentionShouldKeepLatestCheckpoints() {
        val tempDir = Files.createTempDirectory("remote-solver-localfs-checkpoint-retention")
        try {
            val checkpointPort = LocalFsCheckpointPort(tempDir, maxRetainedPerTask = 2)
            runSuspend {
                checkpointPort.save(
                    CheckpointMetadata(
                        taskId = "task-1",
                        sliceId = "slice-1",
                        ref = ObjectRef.of(path = "checkpoint/task-1/slice-1", version = "v1"),
                        createdAtEpochMs = 1000L
                    )
                )
                checkpointPort.save(
                    CheckpointMetadata(
                        taskId = "task-1",
                        sliceId = "slice-2",
                        ref = ObjectRef.of(path = "checkpoint/task-1/slice-2", version = "v2"),
                        createdAtEpochMs = 2000L
                    )
                )
                checkpointPort.save(
                    CheckpointMetadata(
                        taskId = "task-1",
                        sliceId = "slice-3",
                        ref = ObjectRef.of(path = "checkpoint/task-1/slice-3", version = "v3"),
                        createdAtEpochMs = 3000L
                    )
                )

                val checkpoints = checkpointPort.list("task-1")
                assertEquals(2, checkpoints.size)
                assertEquals(listOf("slice-2", "slice-3"), checkpoints.map { it.sliceId.value })
            }
        } finally {
            Files.walk(tempDir).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
            }
        }
    }
}
