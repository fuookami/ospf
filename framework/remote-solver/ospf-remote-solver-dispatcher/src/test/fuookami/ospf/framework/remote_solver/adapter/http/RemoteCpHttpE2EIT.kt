@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.adapter.http

import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverBootstrapFactory
import fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverBootstrapOptions
import fuookami.ospf.framework.remote_solver.bootstrap.SolverExecutionAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.StorageAdapterType
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SerializedSolution
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverType
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverTypeName
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingExpression
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingSnapshotCodec
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64

/**
 * HTTP/object-storage/dispatcher/calculator CP integration test.
 * HTTP、对象存储、dispatcher、calculator 的 CP 集成测试。
 */
class RemoteCpHttpE2EIT {
    @Test
    fun submitsScopedPayloadAndReturnsScipArtifact() = runBlocking {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                solverExecutionAdapter = SolverExecutionAdapterType.OSPF_INPROCESS,
                solverExecutionOspfSolverType = SolverType.SCIP,
                storageAdapter = StorageAdapterType.INMEMORY
            )
        )
        runtime.service.registerNode(
            NodeCapabilityProfile(
                nodeId = NodeId.of("cp-e2e-node"),
                solverType = SolverTypeName.of("scip"),
                performanceScore = Flt64.one,
                pricePerSecond = Flt64.one,
                minBillingUnit = 1.seconds,
                supportsInterrupt = true,
                supportsCheckpoint = true,
                supportsWarmStart = true,
                parallelUnits = 1,
                supportedModelTypes = setOf(NormalizedModelType.CP)
            )
        )

        val snapshotJson = buildSnapshot()
        val payload = SolvePayload(
            modelData = ModelData.raw(
                bytes = snapshotJson.encodeToByteArray(),
                format = "ospf-cp-snapshot-json"
            )
        )
        runtime.objectStoragePort.put(
            path = "tenant-a/payload/e2e",
            bytes = Json { encodeDefaults = true }
                .encodeToString(SolvePayload.serializer(), payload)
                .encodeToByteArray()
        )

        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newHttpClient()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(
                        HttpRequest.BodyPublishers.ofString(
                            """{"tenantId":"tenant-a","payloadRef":"payload/e2e","complexity":"SIMPLE"}"""
                        )
                    )
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            val submitData = Json.parseToJsonElement(submitResponse.body()).jsonObject
                .getValue("data").jsonObject
            val taskId = submitData.getValue("taskId").jsonPrimitive.content
            assertTrue(taskId.isNotBlank())

            runtime.service.runUntilIdle(maxRounds = 10)

            val taskResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId"))
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, taskResponse.statusCode())
            val taskData = Json.parseToJsonElement(taskResponse.body()).jsonObject
                .getValue("data").jsonObject
            assertEquals(TaskStatus.COMPLETED.name, taskData.getValue("status").jsonPrimitive.content)
            assertEquals("tenant-a", taskData.getValue("tenantId").jsonPrimitive.content)
            val resultPath = taskData.getValue("latestResultPath").jsonPrimitive.content
            assertTrue(resultPath.startsWith("tenant-a/result/$taskId/"))

            val resultRef = ObjectRef.of(path = resultPath)
            val artifactBytes = runtime.objectStoragePort.get(resultRef)
            assertNotNull(artifactBytes)
            val artifact = Json.decodeFromString(
                SerializedSolution.serializer(),
                artifactBytes.decodeToString()
            )
            assertEquals(taskId, artifact.runId)
            assertEquals(0L, artifact.variableValuesById.values.single())
            assertEquals(0L, artifact.objectiveValueInt64)
            assertEquals("scip-runtime-2", artifact.fingerprintSchemas["solver"])
            assertTrue(artifact.artifactDigest?.isNotBlank() == true)
        } finally {
            server.stop(0)
        }
    }

    private fun buildSnapshot(): String {
        val model = ConstraintProgrammingModel("remote-cp-e2e", ObjectCategory.Minimum)
        return try {
            val variable = IntVar("remote-cp-value")
            model.registerVariable(variable, IntegerDomain.interval(0, 2).value!!)
            model.minimize(ConstraintProgrammingExpression.Variable(variable))
            ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
        } finally {
            model.close()
        }
    }
}
