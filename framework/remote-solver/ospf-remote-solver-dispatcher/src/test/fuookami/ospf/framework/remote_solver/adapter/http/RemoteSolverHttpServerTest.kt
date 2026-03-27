@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.adapter.http

import fuookami.ospf.framework.remote_solver.adapter.prometheus.PrometheusMetricsPort
import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RemoteSolverHttpServerTest {
    @Test
    fun submitGetStopResumeFlowShouldWork() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"

            val submitBody = """
                {"payloadRef":"models/http-test","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}
            """.trimIndent()
            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString(submitBody))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            assertTrue(submitResponse.body().contains("\"code\":\"OK\""))
            val taskId = extractField(submitResponse.body(), "taskId")
            assertTrue(taskId.isNotBlank())

            val getResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId"))
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, getResponse.statusCode())
            assertTrue(getResponse.body().contains("\"code\":\"OK\""))
            assertTrue(getResponse.body().contains("\"status\":\"${TaskStatus.QUEUED}\""))

            val stopResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId/stop"))
                    .header("Content-Type", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"reason":"http-stop"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, stopResponse.statusCode())
            assertTrue(stopResponse.body().contains("\"code\":\"OK\""))
            assertTrue(stopResponse.body().contains("\"status\":\"${TaskStatus.STOPPED}\""))

            val resumeResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId/resume"))
                    .POST(HttpRequest.BodyPublishers.ofString("{}"))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, resumeResponse.statusCode())
            assertTrue(resumeResponse.body().contains("\"code\":\"OK\""))
            assertTrue(resumeResponse.body().contains("\"status\":\"${TaskStatus.QUEUED}\""))

            runSuspend {
                val stored = runtime.service.getTask(taskId)
                assertTrue(stored != null)
                assertEquals(TaskStatus.QUEUED, stored.status)
            }
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun invalidSubmitShouldReturn400() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"  "}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(400, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"INVALID_ARGUMENT\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun malformedJsonShouldReturn400() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/x""""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(400, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"INVALID_ARGUMENT\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun submitWithUnknownJsonFieldsShouldStillSucceed() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(
                        HttpRequest.BodyPublishers.ofString(
                            """{"payloadRef":"models/unknown-fields","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME","unknownA":"x","unknownB":123}"""
                        )
                    )
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"OK\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun resumeFailedTaskShouldReturn409WithUnifiedErrorCode() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(
                        HttpRequest.BodyPublishers.ofString(
                            """{"payloadRef":"models/http-failed-resume","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}"""
                        )
                    )
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            val taskId = extractField(submitResponse.body(), "taskId")
            assertTrue(taskId.isNotBlank())

            runSuspend {
                val task = runtime.service.getTask(taskId)
                assertTrue(task != null)
                runtime.taskStatePort.upsertTask(
                    task.copy(
                        status = TaskStatus.FAILED,
                        updatedAt = kotlin.time.Instant.fromEpochMilliseconds(System.currentTimeMillis())
                    )
                )
            }

            val resumeResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId/resume"))
                    .POST(HttpRequest.BodyPublishers.ofString("{}"))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(409, resumeResponse.statusCode())
            assertTrue(resumeResponse.body().contains("\"code\":\"INVALID_TASK_STATE_TRANSITION\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun schedulerHotReloadEndpointsShouldWork() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = fuookami.ospf.framework.remote_solver.application.RemoteSolverConfig(
                schedulerConfigVersion = "v-base",
                schedulerHotReloadEnabled = true
            )
        )
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"

            val hotReloadResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/scheduler/config/hot-reload"))
                    .header("Content-Type", "application/json")
                    .POST(
                        HttpRequest.BodyPublishers.ofString(
                            """{"operator":"http-test","changeSet":{"scheduler.simple-task-quantum-ms":"3200"}}"""
                        )
                    )
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, hotReloadResponse.statusCode())
            assertTrue(hotReloadResponse.body().contains("\"code\":\"OK\""))
            assertTrue(hotReloadResponse.body().contains("\"operator\":\"http-test\""))

            val auditListResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/scheduler/config/audits?limit=10"))
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, auditListResponse.statusCode())
            assertTrue(auditListResponse.body().contains("\"code\":\"OK\""))
            assertTrue(auditListResponse.body().contains("\"operator\":\"http-test\""))

            val rollbackResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/scheduler/config/rollback"))
                    .header("Content-Type", "application/json")
                    .POST(
                        HttpRequest.BodyPublishers.ofString(
                            """{"operator":"http-test","targetVersion":"v-base"}"""
                        )
                    )
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, rollbackResponse.statusCode())
            assertTrue(rollbackResponse.body().contains("\"code\":\"OK\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun taskTimelineEndpointShouldReturnReplayReport() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"

            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(
                        HttpRequest.BodyPublishers.ofString(
                            """{"payloadRef":"models/http-timeline","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}"""
                        )
                    )
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            val taskId = extractField(submitResponse.body(), "taskId")
            assertTrue(taskId.isNotBlank())

            val timelineResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId/timeline?limit=20"))
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, timelineResponse.statusCode())
            assertTrue(timelineResponse.body().contains("\"code\":\"OK\""))
            assertTrue(timelineResponse.body().contains("\"taskId\":\"$taskId\""))
            assertTrue(timelineResponse.body().contains("\"events\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun submitWithIllegalTenantShouldReturn400() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(
                        HttpRequest.BodyPublishers.ofString(
                            """{"tenantId":"tenant/a","payloadRef":"models/x"}"""
                        )
                    )
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(400, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"INVALID_ARGUMENT\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun tenantAuthShouldRequireTenantHeaderWhenEnabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            tenantAuthEnabled = true,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/x","tenantId":"tenant-a"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(403, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"TENANT_AUTH_REQUIRED\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun tenantAuthShouldRejectTenantMismatch() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            tenantAuthEnabled = true,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-Id", "tenant-a")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/x","tenantId":"tenant-b"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(403, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"TENANT_MISMATCH\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun tenantAuthShouldDenyCrossTenantTaskRead() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            tenantAuthEnabled = true,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-Id", "tenant-a")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/x"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            val taskId = extractField(submitResponse.body(), "taskId")
            assertTrue(taskId.isNotBlank())

            val readResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId"))
                    .header("X-Tenant-Id", "tenant-b")
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(403, readResponse.statusCode())
            assertTrue(readResponse.body().contains("\"code\":\"TENANT_ACCESS_DENIED\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun tenantAuthShouldDenyCrossTenantStopAndNotChangeTaskStatus() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            tenantAuthEnabled = true,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"

            // Submit task as tenant-a
            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-Id", "tenant-a")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/stop-test"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            val taskId = extractField(submitResponse.body(), "taskId")
            assertTrue(taskId.isNotBlank())

            // Verify initial status is QUEUED
            runSuspend {
                val task = runtime.service.getTask(taskId)
                assertTrue(task != null)
                assertEquals(TaskStatus.QUEUED, task.status)
            }

            // Attempt stop as tenant-b (cross-tenant)
            val stopResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId/stop"))
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-Id", "tenant-b")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"reason":"cross-tenant-stop"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(403, stopResponse.statusCode())
            assertTrue(stopResponse.body().contains("\"code\":\"TENANT_ACCESS_DENIED\""))

            // Verify task status is unchanged (not STOPPED)
            runSuspend {
                val task = runtime.service.getTask(taskId)
                assertTrue(task != null)
                assertEquals(TaskStatus.QUEUED, task.status, "Task status should not change after cross-tenant stop denial")
            }
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun tenantAuthShouldDenyCrossTenantResumeAndNotChangeTaskStatus() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            tenantAuthEnabled = true,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"

            // Submit and stop task as tenant-a
            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-Id", "tenant-a")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/resume-test"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            val taskId = extractField(submitResponse.body(), "taskId")
            assertTrue(taskId.isNotBlank())

            // Stop task as tenant-a
            val stopResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId/stop"))
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-Id", "tenant-a")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"reason":"owner-stop"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, stopResponse.statusCode())

            // Verify task is STOPPED
            runSuspend {
                val task = runtime.service.getTask(taskId)
                assertTrue(task != null)
                assertEquals(TaskStatus.STOPPED, task.status)
            }

            // Attempt resume as tenant-b (cross-tenant)
            val resumeResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId/resume"))
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-Id", "tenant-b")
                    .POST(HttpRequest.BodyPublishers.ofString("""{}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(403, resumeResponse.statusCode())
            assertTrue(resumeResponse.body().contains("\"code\":\"TENANT_ACCESS_DENIED\""))

            // Verify task status is still STOPPED (not resumed)
            runSuspend {
                val task = runtime.service.getTask(taskId)
                assertTrue(task != null)
                assertEquals(TaskStatus.STOPPED, task.status, "Task status should not change after cross-tenant resume denial")
            }
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun metricsEndpointShouldExposePrometheusContentWhenEnabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val metrics = PrometheusMetricsPort()
        runSuspend {
            metrics.increment("remote_solver_test_counter", tags = mapOf("tenant_id" to "tenant-a"))
        }
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            metricsPort = metrics,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/metrics"))
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, response.statusCode())
            assertTrue(response.body().contains("remote_solver_test_counter"))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun monitorOverviewShouldReturn401WhenUnauthenticated() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/monitor/overview?limit=100"))
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(401, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"UNAUTHENTICATED\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun monitorOverviewShouldReturn403WhenRoleMissing() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/monitor/overview?limit=100"))
                    .header("X-User-Id", "monitor-user")
                    .header("X-User-Roles", "ops")
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(403, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"MONITOR_ACCESS_DENIED\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun monitorPageShouldRedirectToLoginWhenUnauthenticated() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            monitorLoginUrl = "/auth/login",
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/monitor"))
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(302, response.statusCode())
            assertEquals("/auth/login", response.headers().firstValue("location").orElse(""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun monitorOverviewEndpointShouldReturnSchedulerAndNodeStats() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "monitor-node-1",
                    solverType = "gurobi",
                    performanceScore = 1.3,
                    pricePerSecond = 0.12,
                    minBillingUnitSeconds = 1,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 4
                )
            )
            runtime.apiFacade.submit(
                fuookami.ospf.framework.remote_solver.application.TaskSubmitRequest(
                    payloadRef = ObjectRef.of(path = "models/monitor-task")
                )
            )
        }
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/monitor/overview?limit=100"))
                    .header("X-User-Id", "monitor-user")
                    .header("X-User-Roles", "monitor_read")
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, response.statusCode())
            assertTrue(response.body().contains("\"code\":\"OK\""))
            assertTrue(response.body().contains("\"schedulerConfigVersion\""))
            assertTrue(response.body().contains("\"monitor-node-1\""))
            assertTrue(response.body().contains("\"queueDepth\""))
            assertTrue(response.body().contains("\"recentTasks\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun monitorPageShouldServeDashboardHtmlForAdminRole() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/monitor"))
                    .header("X-User-Id", "admin-user")
                    .header("X-User-Roles", "admin")
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, response.statusCode())
            assertTrue(response.body().contains("<title>Remote Solver Monitor</title>"))
            assertTrue(response.body().contains("fetch('/api/v1/monitor/overview?limit=300'"))
            assertTrue(response.body().contains("Task Status Distribution"))
            assertTrue(response.body().contains("Node Filter"))
            assertTrue(response.body().contains("Recent Tasks"))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun monitorOverviewShouldClassifyOnlineStaleAndOfflineNodes() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = fuookami.ospf.framework.remote_solver.application.RemoteSolverConfig(
                nodeHeartbeatTimeoutMs = 10L
            )
        )
        runSuspend {
            val now = runtime.service.clockPort().nowEpochMs()
            val profileOnline = NodeCapabilityProfile(
                nodeId = "node-online",
                solverType = "gurobi",
                performanceScore = 1.0,
                pricePerSecond = 0.1,
                minBillingUnitSeconds = 1,
                supportsInterrupt = true,
                supportsCheckpoint = true,
                supportsWarmStart = true,
                parallelUnits = 2
            )
            val profileStale = profileOnline.copy(nodeId = NodeId.of("node-stale"))
            val profileOffline = profileOnline.copy(nodeId = NodeId.of("node-offline"))
            runtime.nodeStatePort.upsertNode(
                NodeState(
                    nodeId = "node-online",
                    profile = profileOnline,
                    availableUnits = 2,
                    lastHeartbeatEpochMs = now + 60000L,
                    online = true
                )
            )
            runtime.nodeStatePort.upsertNode(
                NodeState(
                    nodeId = "node-stale",
                    profile = profileStale,
                    availableUnits = 2,
                    lastHeartbeatEpochMs = now - 1000L,
                    online = true
                )
            )
            runtime.nodeStatePort.upsertNode(
                NodeState(
                    nodeId = "node-offline",
                    profile = profileOffline,
                    availableUnits = 2,
                    lastHeartbeatEpochMs = now - 1000L,
                    online = false
                )
            )
        }
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/monitor/overview?limit=100"))
                    .header("X-User-Id", "monitor-user")
                    .header("X-User-Roles", "monitor_read")
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, response.statusCode())
            assertTrue(response.body().contains("\"nodeId\":\"node-online\""))
            assertTrue(response.body().contains("\"nodeId\":\"node-stale\""))
            assertTrue(response.body().contains("\"nodeId\":\"node-offline\""))
            assertTrue(response.body().contains("\"health\":\"ONLINE\""))
            assertTrue(response.body().contains("\"health\":\"STALE\""))
            assertTrue(response.body().contains("\"health\":\"OFFLINE\""))
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun successResponseShouldContainTraceIdField() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"

            // Test submit task success response
            val submitResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .header("X-Trace-Id", "trace-123")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/trace-test"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, submitResponse.statusCode())
            assertTrue(submitResponse.body().contains("\"traceId\":\"trace-123\""), "Success response should contain traceId from header")

            // Test get task success response
            val taskId = extractField(submitResponse.body(), "taskId")
            val getResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks/$taskId"))
                    .header("X-Trace-Id", "trace-get-456")
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, getResponse.statusCode())
            assertTrue(getResponse.body().contains("\"traceId\":\"trace-get-456\""), "GET response should contain traceId")

            // Test health endpoint success response
            val healthResponse = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/health"))
                    .header("X-Trace-Id", "trace-health-789")
                    .GET()
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, healthResponse.statusCode())
            assertTrue(healthResponse.body().contains("\"traceId\":\"trace-health-789\""), "Health response should contain traceId")
        } finally {
            server.stop(0)
        }
    }

    @Test
    fun successResponseShouldUseRequestIdAsFallbackTraceId() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val server = RemoteSolverHttpServer(
            apiFacade = runtime.apiFacade,
            host = "127.0.0.1",
            port = 0
        )
        server.start()
        try {
            val client = HttpClient.newBuilder().build()
            val baseUrl = "http://127.0.0.1:${server.port()}"

            // Test with X-Request-Id but no X-Trace-Id
            val response = client.send(
                HttpRequest.newBuilder()
                    .uri(URI.create("$baseUrl/api/v1/tasks"))
                    .header("Content-Type", "application/json")
                    .header("X-Request-Id", "request-fallback-123")
                    .POST(HttpRequest.BodyPublishers.ofString("""{"payloadRef":"models/trace-fallback"}"""))
                    .build(),
                HttpResponse.BodyHandlers.ofString()
            )
            assertEquals(200, response.statusCode())
            assertTrue(response.body().contains("\"traceId\":\"request-fallback-123\""), "Should use X-Request-Id as fallback traceId")
        } finally {
            server.stop(0)
        }
    }

    private fun extractField(json: String, field: String): String {
        val regex = """"$field"\s*:\s*"((?:\\.|[^"\\])*)"""".toRegex()
        return regex.find(json)?.groupValues?.get(1)
            ?.replace("\\\"", "\"")
            ?.replace("\\\\", "\\")
            ?: ""
    }
}
