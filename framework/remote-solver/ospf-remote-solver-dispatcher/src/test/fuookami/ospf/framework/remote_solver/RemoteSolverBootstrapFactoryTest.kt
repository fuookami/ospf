@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventRetryPolicy
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryBudgetPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCostLedgerPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryDistributedLockPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryNodeStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemorySolverExecutionPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.CanonicalMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.mock.MockSolverExecutionPort
import fuookami.ospf.framework.remote_solver.adapter.prometheus.PrometheusMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.kafka.KafkaEventPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormBudgetPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormCostLedgerPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormNodeStatePort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormSchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.mirroring.MirroringEventPort
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExecutionBridge
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfSolverExecutionPort
import fuookami.ospf.framework.remote_solver.adapter.s3.S3CheckpointPort
import fuookami.ospf.framework.remote_solver.adapter.s3.S3ObjectStoragePort
import fuookami.ospf.framework.remote_solver.bootstrap.BudgetAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.CostLedgerAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.DistributedLockAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.EventAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.NodeStateAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.MetricsAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverBootstrapFactory
import fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverBootstrapOptions
import fuookami.ospf.framework.remote_solver.bootstrap.SolverExecutionAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.StorageAdapterType
import fuookami.ospf.framework.remote_solver.bootstrap.SchedulerAuditAdapterType
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.framework.remote_solver.bootstrap.TaskStateAdapterType
import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.EventSchemaValidationMode
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import java.nio.file.Files
import java.sql.DriverManager
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlin.io.path.deleteIfExists
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class RemoteSolverBootstrapFactoryTest {
    @Test
    fun adapterKeysShouldBeParsedCaseInsensitively() {
        val options = RemoteSolverBootstrapOptions.fromAdapterKeys(
            eventAdapterKey = "InMemory",
            distributedLockAdapterKey = "INMEMORY",
            nodeStateAdapterKey = "JDBC",
            budgetAdapterKey = "jDbC",
            taskStateAdapterKey = "jdbc",
            costLedgerAdapterKey = "jdbc",
            solverExecutionAdapterKey = "ospf",
            schedulerAuditAdapterKey = "jdbc",
            storageAdapterKey = "localFs"
        )

        assertEquals(EventAdapterType.INMEMORY, options.eventAdapter)
        assertEquals(DistributedLockAdapterType.INMEMORY, options.distributedLockAdapter)
        assertEquals(NodeStateAdapterType.KTORM, options.nodeStateAdapter)
        assertEquals(BudgetAdapterType.KTORM, options.budgetAdapter)
        assertEquals(TaskStateAdapterType.KTORM, options.taskStateAdapter)
        assertEquals(CostLedgerAdapterType.KTORM, options.costLedgerAdapter)
        assertEquals(SolverExecutionAdapterType.OSPF_INPROCESS, options.solverExecutionAdapter)
        assertEquals(SchedulerAuditAdapterType.KTORM, options.schedulerAuditAdapter)
        assertEquals(StorageAdapterType.LOCALFS, options.storageAdapter)
    }

    @Test
    fun propertiesShouldConfigureAdapterSelectionsAndLocalFsRoot() {
        val tempDir = Files.createTempDirectory("remote-solver-properties-root")
        try {
            val options = RemoteSolverBootstrapOptions.fromProperties(
                mapOf(
                    "event.adapter" to "inmemory",
                    "event.mirror.enabled" to "true",
                    "event.mirror.fail-open" to "false",
                    "distributed-lock.adapter" to "inmemory",
                    "node-state.adapter" to "jdbc",
                    "node-state.ktorm.url" to "jdbc:h2:mem:node_props;MODE=PostgreSQL;DB_CLOSE_DELAY=-1",
                    "budget.adapter" to "jdbc",
                    "budget.ktorm.url" to "jdbc:h2:mem:budget_props;MODE=PostgreSQL;DB_CLOSE_DELAY=-1",
                    "task-state.adapter" to "jdbc",
                    "task-state.ktorm.url" to "jdbc:h2:mem:task_props;MODE=PostgreSQL;DB_CLOSE_DELAY=-1",
                    "cost-ledger.adapter" to "jdbc",
                    "cost-ledger.ktorm.url" to "jdbc:h2:mem:cost_props;MODE=PostgreSQL;DB_CLOSE_DELAY=-1",
                    "solver-execution.adapter" to "ospf",
                    "solver-execution.ospf.simulated-total-runtime-ms" to "15000",
                    "storage.adapter" to "localfs",
                    "storage.localfs.root" to tempDir.toString(),
                    "metrics.adapter" to "prometheus"
                )
            )

            assertEquals(EventAdapterType.INMEMORY, options.eventAdapter)
            assertTrue(options.eventMirrorEnabled)
            assertTrue(!options.eventMirrorFailOpen)
            assertEquals(DistributedLockAdapterType.INMEMORY, options.distributedLockAdapter)
            assertEquals(NodeStateAdapterType.KTORM, options.nodeStateAdapter)
            assertEquals(BudgetAdapterType.KTORM, options.budgetAdapter)
            assertEquals(TaskStateAdapterType.KTORM, options.taskStateAdapter)
            assertEquals(CostLedgerAdapterType.KTORM, options.costLedgerAdapter)
            assertEquals(SolverExecutionAdapterType.OSPF_INPROCESS, options.solverExecutionAdapter)
            assertEquals(StorageAdapterType.LOCALFS, options.storageAdapter)
            assertEquals(tempDir.toAbsolutePath().normalize(), options.localFsRoot.toAbsolutePath().normalize())
            assertEquals(MetricsAdapterType.PROMETHEUS, options.metricsAdapter)
        } finally {
            Files.walk(tempDir).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
            }
        }
    }

    @Test
    fun configuredInProcessBridgeClassShouldBeInstantiable() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                solverExecutionAdapter = SolverExecutionAdapterType.OSPF_INPROCESS,
                solverExecutionOspfBridgeClass =
                    "fuookami.ospf.framework.remote_solver.adapter.ospf.OspfInProcessBridge"
            )
        )

        assertTrue(runtime.solverExecutionPort is OspfSolverExecutionPort)
    }

    @Test
    fun propertiesShouldConfigureS3StorageOptions() {
        val options = RemoteSolverBootstrapOptions.fromProperties(
            mapOf(
                "storage.adapter" to "s3",
                "storage.s3.bucket" to "remote-solver-bucket",
                "storage.s3.region" to "us-east-1",
                "storage.s3.endpoint" to "http://127.0.0.1:9000",
                "storage.s3.access-key-id" to "minioadmin",
                "storage.s3.secret-access-key" to "minioadmin",
                "storage.s3.path-style-access" to "true",
                "storage.s3.object-prefix" to "solver-objects",
                "storage.s3.checkpoint-prefix" to "solver-checkpoints"
            )
        )
        assertEquals(StorageAdapterType.S3, options.storageAdapter)
        assertEquals("remote-solver-bucket", options.storageS3Bucket)
        assertEquals("us-east-1", options.storageS3Region)
        assertEquals("http://127.0.0.1:9000", options.storageS3Endpoint)
        assertEquals("minioadmin", options.storageS3AccessKeyId)
        assertEquals("minioadmin", options.storageS3SecretAccessKey)
        assertTrue(options.storageS3PathStyleAccess)
        assertEquals("solver-objects", options.storageS3ObjectPrefix)
        assertEquals("solver-checkpoints", options.storageS3CheckpointPrefix)
    }

    @Test
    fun propertiesShouldConfigureKafkaEventOptions() {
        val options = RemoteSolverBootstrapOptions.fromProperties(
            mapOf(
                "event.adapter" to "kafka",
                "event.kafka.bootstrap-servers" to "localhost:9092",
                "event.kafka.client-id" to "remote-solver-test",
                "event.kafka.consumer-poll-interval-ms" to "300",
                "event.kafka.query-poll-timeout-ms" to "800",
                "event.kafka.query-max-poll-rounds" to "3",
                "event.kafka.query-topics" to "SolvingRequest,TaskResult",
                "event.schema.registry.mode" to "strict"
            )
        )
        assertEquals(EventAdapterType.KAFKA, options.eventAdapter)
        assertEquals("localhost:9092", options.eventKafkaBootstrapServers)
        assertEquals("remote-solver-test", options.eventKafkaClientId)
        assertEquals(300L, options.eventKafkaConsumerPollIntervalMs)
        assertEquals(800L, options.eventKafkaQueryPollTimeoutMs)
        assertEquals(3, options.eventKafkaQueryMaxPollRounds)
        assertEquals(setOf("SolvingRequest", "TaskResult"), options.eventKafkaQueryTopics)
        assertEquals(EventSchemaValidationMode.STRICT, options.eventSchemaValidationMode)
    }

    @Test
    fun invalidEventSchemaRegistryModeShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapOptions.fromProperties(
                mapOf("event.schema.registry.mode" to "soft")
            )
        }
        assertTrue(error.message?.contains("Unsupported event schema registry mode") == true)
    }

    @Test
    fun propertiesShouldConfigureJdbcDistributedLockOptions() {
        val options = RemoteSolverBootstrapOptions.fromProperties(
            mapOf(
                "distributed-lock.adapter" to "jdbc",
                "distributed-lock.ktorm.url" to "jdbc:postgresql://localhost:5432/remote_solver",
                "distributed-lock.ktorm.username" to "solver_user",
                "distributed-lock.ktorm.password" to "solver_pwd",
                "distributed-lock.ktorm.table" to "rs_lock"
            )
        )
        assertEquals(DistributedLockAdapterType.KTORM, options.distributedLockAdapter)
        assertEquals("jdbc:postgresql://localhost:5432/remote_solver", options.distributedLockKtormUrl)
        assertEquals("solver_user", options.distributedLockKtormUsername)
        assertEquals("solver_pwd", options.distributedLockKtormPassword)
        assertEquals("rs_lock", options.distributedLockKtormTable)
    }

    @Test
    fun propertiesShouldConfigureJdbcNodeStateAndBudgetOptions() {
        val options = RemoteSolverBootstrapOptions.fromProperties(
            mapOf(
                "node-state.adapter" to "jdbc",
                "node-state.ktorm.url" to "jdbc:postgresql://localhost:5432/node_state",
                "node-state.ktorm.username" to "node_user",
                "node-state.ktorm.password" to "node_pwd",
                "node-state.ktorm.table" to "rs_node",
                "budget.adapter" to "jdbc",
                "budget.ktorm.url" to "jdbc:postgresql://localhost:5432/budget",
                "budget.ktorm.username" to "budget_user",
                "budget.ktorm.password" to "budget_pwd",
                "budget.ktorm.table" to "rs_budget",
                "task-state.adapter" to "jdbc",
                "task-state.ktorm.url" to "jdbc:postgresql://localhost:5432/task_state",
                "task-state.ktorm.username" to "task_user",
                "task-state.ktorm.password" to "task_pwd",
                "task-state.ktorm.task-table" to "rs_task",
                "task-state.ktorm.slice-table" to "rs_slice",
                "cost-ledger.adapter" to "jdbc",
                "cost-ledger.ktorm.url" to "jdbc:postgresql://localhost:5432/cost_ledger",
                "cost-ledger.ktorm.username" to "cost_user",
                "cost-ledger.ktorm.password" to "cost_pwd",
                "cost-ledger.ktorm.table" to "rs_cost",
                "solver-execution.adapter" to "ospf",
                "solver-execution.ospf.simulated-total-runtime-ms" to "18000",
                "solver-execution.ospf.bridge-class" to "fuookami.ospf.framework.remote_solver.RemoteSolverBootstrapFactoryTest\$TestOspfBridge",
                "solver-execution.ospf.bridge.arg.mode" to "native",
                "solver-execution.ospf.bridge.arg.endpoint" to "grpc://127.0.0.1:19090"
            )
        )
        assertEquals(NodeStateAdapterType.KTORM, options.nodeStateAdapter)
        assertEquals("jdbc:postgresql://localhost:5432/node_state", options.nodeStateKtormUrl)
        assertEquals("node_user", options.nodeStateKtormUsername)
        assertEquals("node_pwd", options.nodeStateKtormPassword)
        assertEquals("rs_node", options.nodeStateKtormTable)

        assertEquals(BudgetAdapterType.KTORM, options.budgetAdapter)
        assertEquals("jdbc:postgresql://localhost:5432/budget", options.budgetKtormUrl)
        assertEquals("budget_user", options.budgetKtormUsername)
        assertEquals("budget_pwd", options.budgetKtormPassword)
        assertEquals("rs_budget", options.budgetKtormTable)

        assertEquals(TaskStateAdapterType.KTORM, options.taskStateAdapter)
        assertEquals("jdbc:postgresql://localhost:5432/task_state", options.taskStateKtormUrl)
        assertEquals("task_user", options.taskStateKtormUsername)
        assertEquals("task_pwd", options.taskStateKtormPassword)
        assertEquals("rs_task", options.taskStateKtormTaskTable)
        assertEquals("rs_slice", options.taskStateKtormSliceTable)
        assertEquals(CostLedgerAdapterType.KTORM, options.costLedgerAdapter)
        assertEquals("jdbc:postgresql://localhost:5432/cost_ledger", options.costLedgerKtormUrl)
        assertEquals("cost_user", options.costLedgerKtormUsername)
        assertEquals("cost_pwd", options.costLedgerKtormPassword)
        assertEquals("rs_cost", options.costLedgerKtormTable)
        assertEquals(SolverExecutionAdapterType.OSPF_INPROCESS, options.solverExecutionAdapter)
        assertEquals(
            "fuookami.ospf.framework.remote_solver.RemoteSolverBootstrapFactoryTest\$TestOspfBridge",
            options.solverExecutionOspfBridgeClass
        )
        assertEquals("native", options.solverExecutionOspfBridgeArgs["mode"])
        assertEquals("grpc://127.0.0.1:19090", options.solverExecutionOspfBridgeArgs["endpoint"])
    }

    @Test
    fun propertiesShouldConfigureCheckpointRetention() {
        val options = RemoteSolverBootstrapOptions.fromProperties(
            mapOf("checkpoint.retention.max-per-task" to "2")
        )
        assertEquals(2, options.checkpointMaxRetainedPerTask)
    }

    @Test
    fun invalidCheckpointRetentionShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapOptions.fromProperties(
                mapOf("checkpoint.retention.max-per-task" to "-1")
            )
        }
        assertTrue(error.message?.contains("checkpointMaxRetainedPerTask") == true)
    }

    @Test
    fun unknownAdapterKeyShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapOptions.fromAdapterKeys(storageAdapterKey = "mystery")
        }
        assertTrue(error.message?.contains("Unsupported storage adapter") == true)
    }

    @Test
    fun invalidMirrorBooleanShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapOptions.fromProperties(
                mapOf("event.mirror.enabled" to "maybe")
            )
        }
        assertTrue(error.message?.contains("Invalid boolean value") == true)
    }

    @Test
    fun createFromPropertiesWithKtormAndMissingMigrationsShouldFailFast() {
        val jdbcUrl = "jdbc:h2:mem:migration_guard_missing_${System.nanoTime()};MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                properties = mapOf(
                    "node-state.adapter" to "jdbc",
                    "node-state.ktorm.url" to jdbcUrl
                )
            )
        }
        assertTrue(error.message?.contains("Missing migrations") == true)
    }

    @Test
    fun createFromPropertiesWithKtormCanBypassMigrationGuardWhenDisabled() {
        val jdbcUrl = "jdbc:h2:mem:migration_guard_disabled_${System.nanoTime()};MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val runtime = RemoteSolverBootstrapFactory.create(
            properties = mapOf(
                "migration.check.enabled" to "false",
                "node-state.adapter" to "jdbc",
                "node-state.ktorm.url" to jdbcUrl
            )
        )
        assertTrue(runtime.nodeStatePort is KtormNodeStatePort)
    }

    @Test
    fun createFromPropertiesWithSchedulerAuditKtormAndMissingMigrationsShouldFailFast() {
        val jdbcUrl = "jdbc:h2:mem:migration_guard_scheduler_audit_missing_${System.nanoTime()};MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                properties = mapOf(
                    "scheduler.audit.adapter" to "ktorm",
                    "scheduler.audit.ktorm.url" to jdbcUrl
                )
            )
        }
        assertTrue(error.message?.contains("Missing migrations") == true)
    }

    @Test
    fun createFromPropertiesWithSchedulerAuditKtormCanBypassMigrationGuardWhenDisabled() {
        val jdbcUrl = "jdbc:h2:mem:migration_guard_scheduler_audit_disabled_${System.nanoTime()};MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val runtime = RemoteSolverBootstrapFactory.create(
            properties = mapOf(
                "migration.check.enabled" to "false",
                "scheduler.audit.adapter" to "ktorm",
                "scheduler.audit.ktorm.url" to jdbcUrl
            )
        )
        assertTrue(runtime.schedulerConfigAuditPort is KtormSchedulerConfigAuditPort)
    }

    @Test
    fun createFromPropertiesShouldPassWhenMigrationHistoryContainsAllVersions() {
        val jdbcUrl = "jdbc:h2:mem:migration_guard_history_${System.nanoTime()};MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        DriverManager.getConnection(jdbcUrl).use { connection ->
            connection.createStatement().use { stmt ->
                stmt.execute(
                    """
                    CREATE TABLE IF NOT EXISTS remote_solver_migration_history (
                        version VARCHAR(32) PRIMARY KEY,
                        description TEXT NOT NULL,
                        applied_at_epoch_ms BIGINT NOT NULL
                    )
                    """.trimIndent()
                )
                stmt.execute("INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms) VALUES ('1', 'core', 0)")
                stmt.execute("INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms) VALUES ('2', 'infra', 0)")
                stmt.execute("INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms) VALUES ('3', 'scheduler_audit', 0)")
                stmt.execute("INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms) VALUES ('4', 'multi_tenant', 0)")
                stmt.execute("INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms) VALUES ('5', 'cp2', 0)")
                stmt.execute("INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms) VALUES ('6', 'cp2_payload_config', 0)")
                stmt.execute("INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms) VALUES ('7', 'object_ref_etag_persistence', 0)")
            }
        }
        val runtime = RemoteSolverBootstrapFactory.create(
            properties = mapOf(
                "node-state.adapter" to "jdbc",
                "node-state.ktorm.url" to jdbcUrl
            )
        )
        assertTrue(runtime.nodeStatePort is KtormNodeStatePort)
    }

    @Test
    fun ktormDistributedLockAdapterWithoutUrlShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    distributedLockAdapter = DistributedLockAdapterType.KTORM
                )
            )
        }
        assertTrue(error.message?.contains("distributed-lock.ktorm.url") == true)
    }

    @Test
    fun kafkaEventAdapterWithoutBootstrapServersShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    eventAdapter = EventAdapterType.KAFKA
                )
            )
        }
        assertTrue(error.message?.contains("event.kafka.bootstrap-servers") == true)
    }

    @Test
    fun ktormNodeStateAdapterWithoutUrlShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    nodeStateAdapter = NodeStateAdapterType.KTORM
                )
            )
        }
        assertTrue(error.message?.contains("node-state.ktorm.url") == true)
    }

    @Test
    fun ktormBudgetAdapterWithoutUrlShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    budgetAdapter = BudgetAdapterType.KTORM
                )
            )
        }
        assertTrue(error.message?.contains("budget.ktorm.url") == true)
    }

    @Test
    fun ktormTaskStateAdapterWithoutUrlShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    taskStateAdapter = TaskStateAdapterType.KTORM
                )
            )
        }
        assertTrue(error.message?.contains("task-state.ktorm.url") == true)
    }

    @Test
    fun ktormCostLedgerAdapterWithoutUrlShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    costLedgerAdapter = CostLedgerAdapterType.KTORM
                )
            )
        }
        assertTrue(error.message?.contains("cost-ledger.ktorm.url") == true)
    }

    @Test
    fun s3StorageAdapterWithoutBucketShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    storageAdapter = StorageAdapterType.S3
                )
            )
        }
        assertTrue(error.message?.contains("storage.s3.bucket") == true)
    }

    @Test
    fun s3StorageAdapterWithoutAccessKeyShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    storageAdapter = StorageAdapterType.S3,
                    storageS3Bucket = "remote-solver-bucket",
                    storageS3SecretAccessKey = "minioadmin"
                )
            )
        }
        assertTrue(error.message?.contains("storage.s3.access-key-id") == true)
    }

    @Test
    fun s3StorageAdapterWithoutSecretKeyShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    storageAdapter = StorageAdapterType.S3,
                    storageS3Bucket = "remote-solver-bucket",
                    storageS3AccessKeyId = "minioadmin"
                )
            )
        }
        assertTrue(error.message?.contains("storage.s3.secret-access-key") == true)
    }

    @Test
    fun createFromPropertiesShouldApplyRetryPolicy() {
        val runtime = RemoteSolverBootstrapFactory.create(
            properties = mapOf(
                "event.adapter" to "inmemory",
                "event.retry.strategy" to "fixed",
                "event.retry.fixed-delay-ms" to "80"
            )
        )

        runSuspend {
            val attempts = mutableListOf<Long>()
            runtime.eventPort.subscribe("topic-bootstrap-properties-policy", "group-1") { record ->
                attempts.add(System.currentTimeMillis())
                if (record.deliveryAttempt == 0) {
                    runtime.eventPort.nack(record)
                }
            }

            runtime.eventPort.publish("topic-bootstrap-properties-policy", "k", "v".toByteArray())
            waitUntil(1200L) { attempts.size == 2 }

            val elapsed = attempts[1] - attempts[0]
            assertTrue(elapsed >= 50L)
        }
    }

    @Test
    fun createFromPropertiesShouldApplySchedulerLearningConfig() {
        val runtime = RemoteSolverBootstrapFactory.create(
            properties = mapOf(
                "scheduler.performance-learning.enabled" to "false",
                "scheduler.performance-learning.rate" to "0.8",
                "scheduler.simple-task-quantum-ms" to "2000"
            )
        )

        runSuspend {
            runtime.service.registerNode(
                NodeCapabilityProfile(
                    nodeId = "node-config-learning-disabled",
                    solverType = "gurobi",
                    performanceScore = 1.0,
                    pricePerSecond = 0.01,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true,
                    parallelUnits = 1
                )
            )

            val task = runtime.service.submitTask(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/config-learning-disabled")),
                complexity = TaskComplexity.SIMPLE,
                timeSensitivity = TimeSensitivity.NON_REALTIME
            )

            repeat(10) {
                runtime.service.scheduleOnce()
            }

            val finalTask = runtime.service.getTask(task.taskId)
            assertTrue(finalTask != null)
            assertEquals(TaskStatus.COMPLETED, finalTask.status)
            val node = runtime.nodeStatePort.getNode("node-config-learning-disabled")
            assertTrue(node != null)
            assertEquals(1.0, node.profile.performanceScore.toDouble())
        }
    }

    @Test
    fun propertiesShouldConfigureSchedulerHotReloadOptions() {
        val runtime = RemoteSolverBootstrapFactory.create(
            mapOf(
                "scheduler.config.version" to "v-base",
                "scheduler.hot-reload.enabled" to "true",
                "scheduler.audit.adapter" to "inmemory"
            )
        )
        assertEquals("v-base", runtime.service.schedulerConfigVersion())
        runSuspend {
            runtime.service.applySchedulerHotReload(
                changeSet = mapOf("scheduler.simple-task-quantum-ms" to "2100"),
                operator = "bootstrap-test"
            )
        }
    }

    @Test
    fun propertiesShouldConfigureSchedulerAuditKtormOptions() {
        val options = RemoteSolverBootstrapOptions.fromProperties(
            mapOf(
                "scheduler.audit.adapter" to "ktorm",
                "scheduler.audit.ktorm.url" to "jdbc:h2:mem:scheduler_audit_opts;MODE=PostgreSQL;DB_CLOSE_DELAY=-1",
                "scheduler.audit.ktorm.username" to "audit_user",
                "scheduler.audit.ktorm.password" to "audit_pwd",
                "scheduler.audit.ktorm.audit-table" to "rs_scheduler_audit",
                "scheduler.audit.ktorm.snapshot-table" to "rs_scheduler_snapshot"
            )
        )
        assertEquals(SchedulerAuditAdapterType.KTORM, options.schedulerAuditAdapter)
        assertEquals("jdbc:h2:mem:scheduler_audit_opts;MODE=PostgreSQL;DB_CLOSE_DELAY=-1", options.schedulerAuditKtormUrl)
        assertEquals("audit_user", options.schedulerAuditKtormUsername)
        assertEquals("audit_pwd", options.schedulerAuditKtormPassword)
        assertEquals("rs_scheduler_audit", options.schedulerAuditKtormAuditTable)
        assertEquals("rs_scheduler_snapshot", options.schedulerAuditKtormSnapshotTable)
    }

    @Test
    fun schedulerAuditKtormAdapterShouldCreateKtormAuditPort() {
        val dbName = "scheduler_audit_runtime_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                schedulerAuditAdapter = SchedulerAuditAdapterType.KTORM,
                schedulerAuditKtormUrl = jdbcUrl,
                schedulerAuditKtormAuditTable = "rs_scheduler_audit_$dbName",
                schedulerAuditKtormSnapshotTable = "rs_scheduler_snapshot_$dbName"
            )
        )
        assertTrue(runtime.schedulerConfigAuditPort is KtormSchedulerConfigAuditPort)
    }

    @Test
    fun metricsCanonicalEnabledPropertyShouldControlMetricsWrapper() {
        val enabledRuntime = RemoteSolverBootstrapFactory.create(
            properties = mapOf("metrics.canonical.enabled" to "true")
        )
        assertTrue(enabledRuntime.metricsPort is CanonicalMetricsPort)

        val disabledRuntime = RemoteSolverBootstrapFactory.create(
            properties = mapOf("metrics.canonical.enabled" to "false")
        )
        assertTrue(disabledRuntime.metricsPort !is CanonicalMetricsPort)
    }

    @Test
    fun metricsAdapterShouldCreatePrometheusPortWhenConfigured() {
        val runtime = RemoteSolverBootstrapFactory.create(
            properties = mapOf(
                "metrics.adapter" to "prometheus",
                "metrics.canonical.enabled" to "false"
            )
        )
        assertTrue(runtime.metricsPort is PrometheusMetricsPort)
    }

    @Test
    fun metricsAdapterShouldSupportPrometheusWithCanonicalWrapper() {
        val runtime = RemoteSolverBootstrapFactory.create(
            properties = mapOf(
                "metrics.adapter" to "prometheus",
                "metrics.canonical.enabled" to "true"
            )
        )
        assertTrue(runtime.metricsPort is CanonicalMetricsPort)
        runSuspend {
            runtime.metricsPort.increment(
                name = "task.failed",
                tags = mapOf("tenantId" to "tenant-a")
            )
        }
        val content = (runtime.metricsPort as CanonicalMetricsPort).scrape()
        assertTrue(content.contains("remote_solver_task_failed_total"))
        assertTrue(content.contains("tenant_id"))
    }

    @Test
    fun schedulerAuditLocalFsShouldRecoverLatestVersionAcrossRestart() {
        val tempDir = Files.createTempDirectory("remote-solver-audit-recovery")
        try {
            val properties = mapOf(
                "scheduler.config.version" to "v-base",
                "scheduler.hot-reload.enabled" to "true",
                "scheduler.audit.adapter" to "localfs",
                "scheduler.audit.localfs.path" to tempDir.toString()
            )
            val firstRuntime = RemoteSolverBootstrapFactory.create(properties)
            val hotVersion = runSuspend {
                firstRuntime.service.applySchedulerHotReload(
                    changeSet = mapOf("scheduler.simple-task-quantum-ms" to "3300"),
                    operator = "recovery-test"
                ).version
            }

            val secondRuntime = RemoteSolverBootstrapFactory.create(properties)
            val recoveredAudits = secondRuntime.service.listSchedulerConfigAudits()
            assertTrue(recoveredAudits.isNotEmpty())
            assertEquals(hotVersion, secondRuntime.service.schedulerConfigVersion())
            assertEquals(3300L, secondRuntime.service.schedulerRuntimeConfig().simpleTaskQuantumMs)
        } finally {
            Files.walk(tempDir).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
            }
        }
    }

    @Test
    fun invalidSchedulerLearningRateShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                properties = mapOf("scheduler.performance-learning.rate" to "1.2")
            )
        }
        assertTrue(error.message?.contains("scheduler.performance-learning.rate") == true)
    }

    @Test
    fun explicitAdapterSelectionShouldCreateExpectedPortTypes() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                eventAdapter = EventAdapterType.INMEMORY,
                distributedLockAdapter = DistributedLockAdapterType.INMEMORY,
                nodeStateAdapter = NodeStateAdapterType.INMEMORY,
                budgetAdapter = BudgetAdapterType.INMEMORY,
                taskStateAdapter = TaskStateAdapterType.INMEMORY,
                costLedgerAdapter = CostLedgerAdapterType.INMEMORY,
                solverExecutionAdapter = SolverExecutionAdapterType.INMEMORY,
                storageAdapter = StorageAdapterType.INMEMORY
            )
        )

        assertTrue(runtime.eventPort is InMemoryEventPort)
        assertTrue(runtime.distributedLockPort is InMemoryDistributedLockPort)
        assertTrue(runtime.nodeStatePort is InMemoryNodeStatePort)
        assertTrue(runtime.budgetPort is InMemoryBudgetPort)
        assertTrue(runtime.taskStatePort is InMemoryTaskStatePort)
        assertTrue(runtime.costLedgerPort is InMemoryCostLedgerPort)
        assertTrue(runtime.solverExecutionPort is InMemorySolverExecutionPort)
    }

    @Test
    fun runtimeShouldExposeApiFacade() {
        val runtime = RemoteSolverBootstrapFactory.create()
        runSuspend {
            runtime.objectStoragePort.put(
                path = "default/model/runtime-api-facade",
                bytes = Json.encodeToString(
                    SolvePayload.serializer(),
                    SolvePayload(modelData = ModelData(rawBytes = byteArrayOf(), format = "ospf-linear-json"))
                ).encodeToByteArray()
            )
            val response = runtime.apiFacade.submit(
                fuookami.ospf.framework.remote_solver.application.TaskSubmitRequest(
                    payloadRef = ObjectRef.of(path = "model/runtime-api-facade"),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME
                )
            )
            assertTrue(response.accepted)
            assertTrue(response.taskId.isNotBlank())
        }
    }

    @Test
    fun kafkaEventAdapterShouldCreateKafkaPortType() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                eventAdapter = EventAdapterType.KAFKA,
                eventKafkaBootstrapServers = "localhost:9092",
                eventKafkaClientId = "remote-solver-kafka-port-test",
                eventMirrorEnabled = false
            )
        )
        assertTrue(runtime.eventPort is KafkaEventPort)
    }

    @Test
    fun invalidOspfBridgeClassShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    solverExecutionAdapter = SolverExecutionAdapterType.OSPF_INPROCESS,
                    solverExecutionOspfBridgeClass = "not.exists.Bridge"
                )
            )
        }
        assertTrue(error.message?.contains("Failed to load OSPF bridge class") == true)
    }

    @Test
    fun nonBridgeClassShouldThrowClearError() {
        val error = assertFailsWith<IllegalArgumentException> {
            RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    solverExecutionAdapter = SolverExecutionAdapterType.OSPF_INPROCESS,
                    solverExecutionOspfBridgeClass = "java.lang.String"
                )
            )
        }
        assertTrue(error.message?.contains("must implement") == true)
    }

    @Test
    fun ospfBridgeWithMapConstructorShouldReceiveBridgeArgs() {
        TestOspfBridgeWithArgs.lastArgs = emptyMap()
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                solverExecutionAdapter = SolverExecutionAdapterType.OSPF_INPROCESS,
                solverExecutionOspfBridgeClass = "fuookami.ospf.framework.remote_solver.RemoteSolverBootstrapFactoryTest\$TestOspfBridgeWithArgs",
                solverExecutionOspfBridgeArgs = mapOf(
                    "mode" to "native",
                    "solverType" to "gurobi"
                )
            )
        )
        assertTrue(runtime.solverExecutionPort is OspfSolverExecutionPort)
        assertEquals("native", TestOspfBridgeWithArgs.lastArgs["mode"])
        assertEquals("gurobi", TestOspfBridgeWithArgs.lastArgs["solverType"])
    }

    @Test
    fun ospfBridgeArgsWithCommandShouldAutoUseExternalProcessBridge() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                solverExecutionAdapter = SolverExecutionAdapterType.OSPF_INPROCESS,
                solverExecutionOspfBridgeArgs = mapOf(
                    "command" to "powershell -Command \"Write-Output 'completed=true'; Write-Output 'feasible=true'; Write-Output 'objective=1.0'; Write-Output 'gap=0.0'; Write-Output 'elapsedMs=123'; Write-Output 'message=ok'\""
                )
            )
        )
        runSuspend {
            val handle = runtime.solverExecutionPort.start(
                payload = SolvePayload(modelRef = ObjectRef.of(path = "model/bridge-auto")),
                taskId = "task-bridge-auto",
                sliceId = "slice-bridge-auto",
                nodeId = "node-bridge-auto",
                tenantId = "tenant-test"
            )
            val slice = runtime.solverExecutionPort.awaitSliceEnd(handle, 10L)
            assertTrue(slice.completed)
            assertEquals(123L, slice.elapsedMs)
        }
    }

    @Test
    fun ktormNodeAndBudgetAdapterShouldCreateKtormPortTypes() {
        val dbName = "bootstrap_ktorm_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                nodeStateAdapter = NodeStateAdapterType.KTORM,
                nodeStateKtormUrl = jdbcUrl,
                nodeStateKtormTable = "rs_node_$dbName",
                budgetAdapter = BudgetAdapterType.KTORM,
                budgetKtormUrl = jdbcUrl,
                budgetKtormTable = "rs_budget_$dbName",
                taskStateAdapter = TaskStateAdapterType.KTORM,
                taskStateKtormUrl = jdbcUrl,
                taskStateKtormTaskTable = "rs_task_$dbName",
                taskStateKtormSliceTable = "rs_slice_$dbName",
                costLedgerAdapter = CostLedgerAdapterType.KTORM,
                costLedgerKtormUrl = jdbcUrl,
                costLedgerKtormTable = "rs_cost_$dbName",
                solverExecutionAdapter = SolverExecutionAdapterType.OSPF_INPROCESS
            )
        )

        assertTrue(runtime.nodeStatePort is KtormNodeStatePort)
        assertTrue(runtime.budgetPort is KtormBudgetPort)
        assertTrue(runtime.taskStatePort is KtormTaskStatePort)
        assertTrue(runtime.costLedgerPort is KtormCostLedgerPort)
        assertTrue(runtime.solverExecutionPort is OspfSolverExecutionPort)
    }

    @Test
    fun s3StorageAdapterShouldCreateS3PortTypes() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                storageAdapter = StorageAdapterType.S3,
                storageS3Bucket = "remote-solver-bucket",
                storageS3Region = "us-east-1",
                storageS3Endpoint = "http://127.0.0.1:9000",
                storageS3AccessKeyId = "minioadmin",
                storageS3SecretAccessKey = "minioadmin",
                storageS3PathStyleAccess = true,
                storageS3ObjectPrefix = "solver-objects",
                storageS3CheckpointPrefix = "solver-checkpoints"
            )
        )
        assertTrue(runtime.objectStoragePort is S3ObjectStoragePort)
        assertTrue(runtime.checkpointPort is S3CheckpointPort)
    }

    @Test
    fun mirrorEnabledShouldWrapEventPortWithMirroringPort() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                eventAdapter = EventAdapterType.INMEMORY,
                eventMirrorEnabled = true
            )
        )
        assertTrue(runtime.eventPort is MirroringEventPort)
    }

    @Test
    fun localFsStorageAdapterCanRunComplexTask() {
        val tempDir = Files.createTempDirectory("remote-solver-localfs-runtime")
        try {
            val runtime = RemoteSolverBootstrapFactory.create(
                RemoteSolverBootstrapOptions(
                    storageAdapter = StorageAdapterType.LOCALFS,
                    localFsRoot = tempDir,
                    solverExecutionAdapter = SolverExecutionAdapterType.INMEMORY
                )
            )

            runSuspend {
                runtime.service.registerNode(
                    NodeCapabilityProfile(
                        nodeId = "node-factory",
                        solverType = "gurobi",
                        performanceScore = 1.0,
                        pricePerSecond = 0.1,
                        minBillingUnitSeconds = 1L,
                        supportsInterrupt = true,
                        supportsCheckpoint = true,
                        supportsWarmStart = true,
                        parallelUnits = 1
                    )
                )

                val task = runtime.service.submitTask(
                    payload = SolvePayload(modelRef = ObjectRef.of(path = "models/factory")),
                    complexity = TaskComplexity.COMPLEX,
                    timeSensitivity = TimeSensitivity.NON_REALTIME,
                    budgetLimit = Flt64(50.0)
                )

                repeat(10) {
                    runtime.service.scheduleOnce()
                }

                val taskState = runtime.service.getTask(task.taskId)
                assertTrue(taskState != null)
                assertEquals(TaskStatus.COMPLETED, taskState.status)

                val checkpoints = runtime.checkpointPort.list(task.taskId)
                assertTrue(checkpoints.isNotEmpty())
            }
        } finally {
            Files.walk(tempDir).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
            }
        }
    }

    @Test
    fun eventRetryPolicyOptionShouldAffectInMemoryEventPort() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                eventRetryPolicy = InMemoryEventRetryPolicy(
                    maxAttempts = 3,
                    baseDelayMs = 70L,
                    maxDelayMs = 500L
                )
            )
        )

        runSuspend {
            val attempts = mutableListOf<Long>()
            runtime.eventPort.subscribe("topic-bootstrap-policy", "group-1") { record ->
                attempts.add(System.currentTimeMillis())
                if (record.deliveryAttempt == 0) {
                    runtime.eventPort.nack(record)
                }
            }

            runtime.eventPort.publish("topic-bootstrap-policy", "k", "v".toByteArray())
            waitUntil(1000L) { attempts.size == 2 }

            val elapsed = attempts[1] - attempts[0]
            assertTrue(elapsed >= 45L)
        }
    }

    @Test
    fun checkpointRetentionOptionShouldAffectCheckpointPort() {
        val runtime = RemoteSolverBootstrapFactory.create(
            RemoteSolverBootstrapOptions(
                checkpointMaxRetainedPerTask = 1
            )
        )

        runSuspend {
            runtime.checkpointPort.save(
                CheckpointMetadata(
                    taskId = "task-retention",
                    sliceId = "slice-1",
                    ref = ObjectRef.of(path = "checkpoint/task-retention/slice-1", version = "v1"),
                    createdAtEpochMs = 1000L
                )
            )
            runtime.checkpointPort.save(
                CheckpointMetadata(
                    taskId = "task-retention",
                    sliceId = "slice-2",
                    ref = ObjectRef.of(path = "checkpoint/task-retention/slice-2", version = "v2"),
                    createdAtEpochMs = 2000L
                )
            )

            val list = runtime.checkpointPort.list("task-retention")
            assertEquals(1, list.size)
            assertEquals("slice-2", list.first().sliceId.value)
        }
    }

    private fun waitUntil(timeoutMs: Long, pollMs: Long = 10L, condition: () -> Boolean) {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() <= deadline) {
            if (condition()) {
                return
            }
            Thread.sleep(pollMs)
        }
        assertTrue(condition())
    }

    class TestOspfBridge : OspfExecutionBridge {
        override suspend fun start(
            payload: SolvePayload,
            taskId: String,
            sliceId: String,
            nodeId: String,
            tenantId: String
        ): ExecutionHandle =
            throw UnsupportedOperationException("test bridge")

        override suspend fun resume(
            payload: SolvePayload,
            checkpoint: ObjectRef,
            taskId: String,
            sliceId: String,
            nodeId: String,
            tenantId: String
        ): ExecutionHandle =
            throw UnsupportedOperationException("test bridge")

        override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantumMs: Long): SliceResult =
            throw UnsupportedOperationException("test bridge")

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? =
            throw UnsupportedOperationException("test bridge")

        override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? =
            throw UnsupportedOperationException("test bridge")

        override suspend fun stop(handle: ExecutionHandle): Boolean =
            throw UnsupportedOperationException("test bridge")
    }

    class TestOspfBridgeWithArgs(args: Map<String, String>) : OspfExecutionBridge {
        companion object {
            var lastArgs: Map<String, String> = emptyMap()
        }

        init {
            lastArgs = args
        }

        override suspend fun start(
            payload: SolvePayload,
            taskId: String,
            sliceId: String,
            nodeId: String,
            tenantId: String
        ): ExecutionHandle =
            throw UnsupportedOperationException("test bridge")

        override suspend fun resume(
            payload: SolvePayload,
            checkpoint: ObjectRef,
            taskId: String,
            sliceId: String,
            nodeId: String,
            tenantId: String
        ): ExecutionHandle =
            throw UnsupportedOperationException("test bridge")

        override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantumMs: Long): SliceResult =
            throw UnsupportedOperationException("test bridge")

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? =
            throw UnsupportedOperationException("test bridge")

        override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? =
            throw UnsupportedOperationException("test bridge")

        override suspend fun stop(handle: ExecutionHandle): Boolean =
            throw UnsupportedOperationException("test bridge")
    }
}




