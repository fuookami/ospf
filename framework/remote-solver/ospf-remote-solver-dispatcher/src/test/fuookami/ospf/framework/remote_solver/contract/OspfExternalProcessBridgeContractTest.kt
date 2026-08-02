package fuookami.ospf.framework.remote_solver.contract

import java.security.MessageDigest
import kotlin.time.Duration
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingSnapshotCodec
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingExpression
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfSolverExecutionPort
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingIncumbent
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProofStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.SerializedSolution
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort

class OspfExternalProcessBridgeContractTest : SolverExecutionPortContractTest() {
    override fun createFixture(): TestFixture<SolverExecutionPort> {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        val bridge = OspfExternalProcessBridge(
            clock = clock,
            idGenerator = idGenerator,
            objectStoragePort = objectStoragePort,
            args = mapOf(
                "command" to "cmd /c \"echo completed=true && echo feasible=true && echo objective=1.0 && echo gap=0.0 && echo elapsedMs=100 && echo checkpointPath=cp.bin && echo resultPath=res.bin\""
            )
        )
        return TestFixture(
            subject = OspfSolverExecutionPort(bridge)
        )
    }

    @Test
    fun malformedCpCheckpointFailsBeforeLaunchingTheExternalProcess() {
        runSuspend {
            val clock = SystemClockPort()
            val idGenerator = UUIDIdGeneratorPort()
            val storage = InMemoryObjectStoragePort(clock)
            val checkpoint = storage.put(
                path = "tenant-test/checkpoint/task-invalid/slice-invalid",
                bytes = "{\"schemaVersion\":\"2.0\"}".encodeToByteArray()
            )
            val bridge = OspfExternalProcessBridge(
                clock = clock,
                idGenerator = idGenerator,
                objectStoragePort = storage,
                args = mapOf("command" to "cmd /c exit 99")
            )
            val handle = bridge.start(
                payload = SolvePayload(
                    modelData = ModelData.raw("{}".encodeToByteArray(), "ospf-cp-snapshot-json"),
                    snapshotRef = checkpoint
                ),
                taskId = "task-invalid",
                sliceId = "slice-invalid",
                nodeId = "node-invalid",
                tenantId = "tenant-test"
            )

            val result = bridge.awaitSliceEnd(handle, quantumMs = 100L)
            assertTrue(result.completed)
            assertEquals(RemoteTerminationReason.BACKEND_FAILURE, result.terminationReason)
            val finalResult = bridge.fetchFinalResult(handle)
            assertEquals(RemoteTerminationReason.BACKEND_FAILURE, finalResult?.terminationReason)
            assertEquals("External CP checkpoint failed integrity or schema validation", finalResult?.message)
        }
    }

    @Test
    fun strictCpSliceWithoutNewCheckpointFailsInsteadOfReusingInputState() {
        runSuspend {
            val clock = SystemClockPort()
            val idGenerator = UUIDIdGeneratorPort()
            val storage = InMemoryObjectStoragePort(clock)
            val model = ConstraintProgrammingModel("bridge-no-checkpoint")
            val snapshot = try {
                ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            } finally {
                model.close()
            }
            val modelFingerprint = sha256(snapshot)
            val configurationFingerprint = sha256(configurationCanonical())
            val artifact = SerializedSolution(
                feasible = true,
                optimal = false,
                elapsed = Duration.ZERO,
                problemStatus = RemoteProblemStatus.FEASIBLE,
                solutionPresence = RemoteSolutionPresence.INCUMBENT,
                proofStatus = RemoteProofStatus.NONE,
                terminationReason = RemoteTerminationReason.TIME_LIMIT,
                schemaVersion = "2.0",
                runId = "task-no-checkpoint",
                attemptId = "slice-no-checkpoint",
                fingerprints = mapOf(
                    "model" to modelFingerprint,
                    "configuration" to configurationFingerprint,
                    "solver" to fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprint()
                ),
                fingerprintSchemas = mapOf(
                    "model" to "1.0",
                    "configuration" to "1.0",
                    "solver" to "2.0"
                )
            )
            val json = Json { encodeDefaults = true; ignoreUnknownKeys = false }
            val digest = sha256(
                json.encodeToString(
                    SerializedSolution.serializer(),
                    artifact.copy(artifactDigest = null)
                )
            )
            val resultFile = java.nio.file.Files.createTempFile("ospf-cp-result-", ".json")
            val workerFile = java.nio.file.Files.createTempFile("ospf-cp-worker-", ".cmd")
            try {
                java.nio.file.Files.writeString(
                    resultFile,
                    json.encodeToString(
                        SerializedSolution.serializer(),
                        artifact.copy(artifactDigest = digest)
                    )
                )
                java.nio.file.Files.writeString(
                    workerFile,
                    "@echo off\r\necho completed=false\r\necho feasible=true\r\necho elapsedMs=0\r\necho resultPath=${resultFile.toAbsolutePath()}\r\n"
                )
                val bridge = OspfExternalProcessBridge(
                    clock = clock,
                    idGenerator = idGenerator,
                    objectStoragePort = storage,
                    args = mapOf(
                        "command" to "cmd /c ${workerFile.toAbsolutePath()}"
                    )
                )
                val handle = bridge.start(
                    payload = SolvePayload(
                        modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json")
                    ),
                    taskId = "task-no-checkpoint",
                    sliceId = "slice-no-checkpoint",
                    nodeId = "node-no-checkpoint",
                    tenantId = "tenant-no-checkpoint"
                )
                val result = bridge.awaitSliceEnd(handle, quantumMs = 100L)
                assertEquals(RemoteTerminationReason.BACKEND_FAILURE, result.terminationReason)
                assertTrue(result.completed)
                assertTrue(result.feasible, result.message ?: "missing result message")
                assertNull(bridge.exportCheckpoint(handle))
            } finally {
                java.nio.file.Files.deleteIfExists(resultFile)
                java.nio.file.Files.deleteIfExists(workerFile)
            }
        }
    }

    @Test
    fun strictCpAllowsAValidCheckpointAndResultPairWithoutAnObjective() {
        runSuspend {
            val clock = SystemClockPort()
            val idGenerator = UUIDIdGeneratorPort()
            val storage = InMemoryObjectStoragePort(clock)
            val model = ConstraintProgrammingModel("bridge-no-objective")
            val snapshot = try {
                ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            } finally {
                model.close()
            }
            val taskId = "task-no-objective"
            val sliceId = "slice-no-objective"
            val tenantId = "tenant-no-objective"
            val modelFingerprint = sha256(snapshot)
            val configurationFingerprint = sha256(configurationCanonical())
            val solverFingerprint = fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprint()
            val checkpoint = PortableCheckpointEnvelope(
                checkpointId = "checkpoint-no-objective",
                modelName = "bridge-no-objective",
                modelFingerprint = modelFingerprint,
                configurationFingerprint = configurationFingerprint,
                solverFingerprint = solverFingerprint,
                runId = taskId,
                attemptId = sliceId,
                createdAtEpochMs = 1L,
                snapshotJson = snapshot,
                incumbent = PortableConstraintProgrammingIncumbent()
            )
            val checkpointFile = java.nio.file.Files.createTempFile("ospf-cp-no-objective-checkpoint-", ".json")
            val resultFile = java.nio.file.Files.createTempFile("ospf-cp-no-objective-result-", ".json")
            val workerFile = java.nio.file.Files.createTempFile("ospf-cp-no-objective-worker-", ".cmd")
            val json = Json { encodeDefaults = true; ignoreUnknownKeys = false }
            try {
                java.nio.file.Files.writeString(checkpointFile, PortableCheckpointCodec.encode(checkpoint))
                val artifact = SerializedSolution(
                    feasible = true,
                    optimal = false,
                    elapsed = Duration.ZERO,
                    problemStatus = RemoteProblemStatus.FEASIBLE,
                    solutionPresence = RemoteSolutionPresence.INCUMBENT,
                    proofStatus = RemoteProofStatus.NONE,
                    terminationReason = RemoteTerminationReason.COMPLETED,
                    schemaVersion = "2.0",
                    runId = taskId,
                    attemptId = sliceId,
                    fingerprints = mapOf(
                        "model" to modelFingerprint,
                        "configuration" to configurationFingerprint,
                        "solver" to solverFingerprint
                    ),
                    fingerprintSchemas = mapOf(
                        "model" to "1.0",
                        "configuration" to "1.0",
                        "solver" to "2.0"
                    )
                )
                val digest = sha256(
                    json.encodeToString(
                        SerializedSolution.serializer(),
                        artifact.copy(artifactDigest = null)
                    )
                )
                java.nio.file.Files.writeString(
                    resultFile,
                    json.encodeToString(
                        SerializedSolution.serializer(),
                        artifact.copy(artifactDigest = digest)
                    )
                )
                java.nio.file.Files.writeString(
                    workerFile,
                    "@echo off\r\necho completed=true\r\necho feasible=true\r\necho elapsedMs=0\r\necho checkpointPath=${checkpointFile.toAbsolutePath()}\r\necho resultPath=${resultFile.toAbsolutePath()}\r\n"
                )
                val bridge = OspfExternalProcessBridge(
                    clock = clock,
                    idGenerator = idGenerator,
                    objectStoragePort = storage,
                    args = mapOf("command" to "cmd /c ${workerFile.toAbsolutePath()}")
                )
                val handle = bridge.start(
                    payload = SolvePayload(
                        modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json")
                    ),
                    taskId = taskId,
                    sliceId = sliceId,
                    nodeId = "node-no-objective",
                    tenantId = tenantId
                )

                val result = bridge.awaitSliceEnd(handle, quantumMs = 100L)
                assertEquals(RemoteTerminationReason.COMPLETED, result.terminationReason)
                assertTrue(result.feasible)
                assertTrue(bridge.exportCheckpoint(handle) != null)
            } finally {
                java.nio.file.Files.deleteIfExists(checkpointFile)
                java.nio.file.Files.deleteIfExists(resultFile)
                java.nio.file.Files.deleteIfExists(workerFile)
            }
        }
    }

    @Test
    fun strictCpAllowsBackendFailureCheckpointToRecomputeObjective() {
        runSuspend {
            val clock = SystemClockPort()
            val idGenerator = UUIDIdGeneratorPort()
            val storage = InMemoryObjectStoragePort(clock)
            val model = ConstraintProgrammingModel("bridge-backend-failure-objective")
            var variableId: String? = null
            val snapshot = try {
                val variable = IntVar("objective-x")
                assertTrue(model.registerVariable(variable, IntegerDomain.interval(0, 1).value!!).ok)
                assertTrue(model.minimize(ConstraintProgrammingExpression.Variable(variable)).ok)
                val modelSnapshot = model.snapshot().value!!
                variableId = modelSnapshot.variables.single().id.value
                ConstraintProgrammingSnapshotCodec.encode(modelSnapshot).value!!
            } finally {
                model.close()
            }
            val resolvedVariableId = variableId!!
            val taskId = "task-backend-failure-objective"
            val sliceId = "slice-backend-failure-objective"
            val tenantId = "tenant-backend-failure-objective"
            val modelFingerprint = sha256(snapshot)
            val configurationFingerprint = sha256(configurationCanonical())
            val solverFingerprint = fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprint()
            val checkpoint = PortableCheckpointEnvelope(
                checkpointId = "checkpoint-backend-failure-objective",
                modelName = "bridge-backend-failure-objective",
                modelFingerprint = modelFingerprint,
                configurationFingerprint = configurationFingerprint,
                solverFingerprint = solverFingerprint,
                runId = taskId,
                attemptId = sliceId,
                createdAtEpochMs = 1L,
                snapshotJson = snapshot,
                incumbent = PortableConstraintProgrammingIncumbent(
                    valuesById = mapOf(resolvedVariableId to 0L),
                    objective = "0"
                )
            )
            val checkpointFile = java.nio.file.Files.createTempFile("ospf-cp-backend-failure-checkpoint-", ".json")
            val resultFile = java.nio.file.Files.createTempFile("ospf-cp-backend-failure-result-", ".json")
            val workerFile = java.nio.file.Files.createTempFile("ospf-cp-backend-failure-worker-", ".cmd")
            val json = Json { encodeDefaults = true; ignoreUnknownKeys = false }
            try {
                java.nio.file.Files.writeString(checkpointFile, PortableCheckpointCodec.encode(checkpoint))
                val artifact = SerializedSolution(
                    feasible = true,
                    optimal = false,
                    variableValuesById = mapOf(resolvedVariableId to 0L),
                    objectiveValue = null,
                    objectiveValueInt64 = null,
                    elapsed = Duration.ZERO,
                    problemStatus = RemoteProblemStatus.FEASIBLE,
                    solutionPresence = RemoteSolutionPresence.INCUMBENT,
                    proofStatus = RemoteProofStatus.NONE,
                    terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
                    schemaVersion = "2.0",
                    runId = taskId,
                    attemptId = sliceId,
                    fingerprints = mapOf(
                        "model" to modelFingerprint,
                        "configuration" to configurationFingerprint,
                        "solver" to solverFingerprint
                    ),
                    fingerprintSchemas = mapOf(
                        "model" to "1.0",
                        "configuration" to "1.0",
                        "solver" to "2.0"
                    )
                )
                val digest = sha256(
                    json.encodeToString(
                        SerializedSolution.serializer(),
                        artifact.copy(artifactDigest = null)
                    )
                )
                java.nio.file.Files.writeString(
                    resultFile,
                    json.encodeToString(
                        SerializedSolution.serializer(),
                        artifact.copy(artifactDigest = digest)
                    )
                )
                java.nio.file.Files.writeString(
                    workerFile,
                    "@echo off\r\necho completed=true\r\necho feasible=true\r\necho elapsedMs=0\r\necho checkpointPath=${checkpointFile.toAbsolutePath()}\r\necho resultPath=${resultFile.toAbsolutePath()}\r\n"
                )
                val bridge = OspfExternalProcessBridge(
                    clock = clock,
                    idGenerator = idGenerator,
                    objectStoragePort = storage,
                    args = mapOf("command" to "cmd /c ${workerFile.toAbsolutePath()}")
                )
                val handle = bridge.start(
                    payload = SolvePayload(
                        modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json")
                    ),
                    taskId = taskId,
                    sliceId = sliceId,
                    nodeId = "node-backend-failure-objective",
                    tenantId = tenantId
                )

                val result = bridge.awaitSliceEnd(handle, quantumMs = 100L)
                assertEquals(RemoteTerminationReason.BACKEND_FAILURE, result.terminationReason)
                assertTrue(result.feasible)
                assertTrue(bridge.exportCheckpoint(handle) != null)
            } finally {
                java.nio.file.Files.deleteIfExists(checkpointFile)
                java.nio.file.Files.deleteIfExists(resultFile)
                java.nio.file.Files.deleteIfExists(workerFile)
            }
        }
    }

    private fun configurationCanonical(): String {
        return buildString {
            appendCanonical("timeLimitMs", null)
            appendCanonical("solutionLimit", null)
            appendCanonical("mipGapTolerance", null)
            appendCanonical("threads", "8")
        }
    }

    private fun StringBuilder.appendCanonical(name: String, value: String?) {
        append(name.length).append(':').append(name)
            .append(value?.length ?: -1).append(':').append(value ?: "")
    }

    private fun sha256(value: String): String {
        return MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }
}
