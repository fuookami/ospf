/**
 * 远程求解器启动工厂
 * Remote solver bootstrap factory
 *
 * 提供远程求解器运行时的组装和配置能力。
 * Provides assembly and configuration capabilities for remote solver runtime.
 *
 * 支持多种适配器类型的灵活配置：
 * Supports flexible configuration of multiple adapter types:
 * - 存储适配器：内存、本地文件系统、S3
 *   Storage adapters: in-memory, local filesystem, S3
 * - 事件适配器：内存、Kafka
 *   Event adapters: in-memory, Kafka
 * - 分布式锁适配器：内存、Ktorm (JDBC)
 *   Distributed lock adapters: in-memory, Ktorm (JDBC)
 * - 求解器执行适配器：内存模拟、OSPF 进程内、OSPF 外部进程
 *   Solver execution adapters: in-memory simulation, OSPF in-process, OSPF external process
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryBudgetPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCheckpointPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCostLedgerPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryDistributedLockPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventRetryPolicy
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryNodeStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemorySchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemorySolverExecutionPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTracingPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.CanonicalMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.kafka.KafkaEventPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormBudgetPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormCostLedgerPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormDistributedLockPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormNodeStatePort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormSchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.localfs.LocalFsCheckpointPort
import fuookami.ospf.framework.remote_solver.adapter.localfs.LocalFsObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.localfs.LocalFsSchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.adapter.mirroring.MirroringEventPort
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExecutionBridge
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfInProcessBridge
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfSolverExecutionPort
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverType
import fuookami.ospf.framework.remote_solver.adapter.prometheus.PrometheusMetricsPort
import fuookami.ospf.framework.remote_solver.adapter.s3.S3CheckpointPort
import fuookami.ospf.framework.remote_solver.adapter.s3.S3ObjectStoragePort
import fuookami.ospf.framework.remote_solver.application.RemoteSolverConfig
import fuookami.ospf.framework.remote_solver.application.RemoteSolverApiFacade
import fuookami.ospf.framework.remote_solver.application.RemoteSolverService
import fuookami.ospf.framework.remote_solver.application.SchedulerEngine
import fuookami.ospf.framework.remote_solver.domain.EventSchemaRegistry
import fuookami.ospf.framework.remote_solver.domain.EventSchemaValidationMode
import fuookami.ospf.framework.remote_solver.port.BudgetPort
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.DistributedLockPort
import fuookami.ospf.framework.remote_solver.port.EventPort
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.port.MetricsPort
import fuookami.ospf.framework.remote_solver.port.NodeStatePort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import fuookami.ospf.framework.remote_solver.port.SchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.port.TaskStatePort
import io.minio.MinioClient
import java.nio.file.Path
import java.nio.file.Paths

/**
 * 存储适配器类型
 * Storage adapter type enumeration
 */
enum class StorageAdapterType(val key: String) {
    INMEMORY("inmemory"),
    LOCALFS("localfs"),
    S3("s3");

    companion object {
        fun fromKeyOrNull(key: String?): StorageAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
        }
    }
}

/**
 * 事件适配器类型
 * Event adapter type enumeration
 */
enum class EventAdapterType(val key: String) {
    INMEMORY("inmemory"),
    KAFKA("kafka");

    companion object {
        fun fromKeyOrNull(key: String?): EventAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
        }
    }
}

/**
 * 分布式锁适配器类型
 * Distributed lock adapter type enumeration
 */
enum class DistributedLockAdapterType(val key: String) {
    INMEMORY("inmemory"),
    KTORM("ktorm");

    companion object {
        fun fromKeyOrNull(key: String?): DistributedLockAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return when (key.trim().lowercase()) {
                "jdbc" -> KTORM
                else -> values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
            }
        }
    }
}

/**
 * 调度器审计适配器类型
 * Scheduler audit adapter type enumeration
 */
enum class SchedulerAuditAdapterType(val key: String) {
    INMEMORY("inmemory"),
    LOCALFS("localfs"),
    KTORM("ktorm");

    companion object {
        fun fromKeyOrNull(key: String?): SchedulerAuditAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return when (key.trim().lowercase()) {
                "jdbc" -> KTORM
                else -> values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
            }
        }
    }
}

/**
 * 节点状态适配器类型
 * Node state adapter type enumeration
 */
enum class NodeStateAdapterType(val key: String) {
    INMEMORY("inmemory"),
    KTORM("ktorm");

    companion object {
        fun fromKeyOrNull(key: String?): NodeStateAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return when (key.trim().lowercase()) {
                "jdbc" -> KTORM
                else -> values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
            }
        }
    }
}

/**
 * 预算适配器类型
 * Budget adapter type enumeration
 */
enum class BudgetAdapterType(val key: String) {
    INMEMORY("inmemory"),
    KTORM("ktorm");

    companion object {
        fun fromKeyOrNull(key: String?): BudgetAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return when (key.trim().lowercase()) {
                "jdbc" -> KTORM
                else -> values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
            }
        }
    }
}

/**
 * 任务状态适配器类型
 * Task state adapter type enumeration
 */
enum class TaskStateAdapterType(val key: String) {
    INMEMORY("inmemory"),
    KTORM("ktorm");

    companion object {
        fun fromKeyOrNull(key: String?): TaskStateAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return when (key.trim().lowercase()) {
                "jdbc" -> KTORM
                else -> values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
            }
        }
    }
}

/**
 * 成本账本适配器类型
 * Cost ledger adapter type enumeration
 */
enum class CostLedgerAdapterType(val key: String) {
    INMEMORY("inmemory"),
    KTORM("ktorm");

    companion object {
        fun fromKeyOrNull(key: String?): CostLedgerAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return when (key.trim().lowercase()) {
                "jdbc" -> KTORM
                else -> values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
            }
        }
    }
}

/**
 * 求解器执行适配器类型
 * Solver execution adapter type enumeration
 */
enum class SolverExecutionAdapterType(val key: String) {
    INMEMORY("inmemory"),
    OSPF_INPROCESS("ospf-inprocess"),
    OSPF_EXTERNAL("ospf-external");

    companion object {
        fun fromKeyOrNull(key: String?): SolverExecutionAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return when (key.trim().lowercase()) {
                "ospf" -> OSPF_INPROCESS  // Backward compatibility
                else -> values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
            }
        }
    }
}

/**
 * 指标适配器类型
 * Metrics adapter type enumeration
 */
enum class MetricsAdapterType(val key: String) {
    INMEMORY("inmemory"),
    PROMETHEUS("prometheus");

    companion object {
        fun fromKeyOrNull(key: String?): MetricsAdapterType? {
            if (key.isNullOrBlank()) {
                return null
            }
            return values().firstOrNull { it.key.equals(key.trim(), ignoreCase = true) }
        }
    }
}

/**
 * 远程求解器启动配置选项
 * Remote solver bootstrap options
 *
 * 包含所有适配器配置和运行时参数。
 * Contains all adapter configurations and runtime parameters.
 *
 * @param config 远程求解器配置 / Remote solver configuration
 * @param eventAdapter 事件适配器类型 / Event adapter type
 * @param storageAdapter 存储适配器类型 / Storage adapter type
 * @param solverExecutionAdapter 求解器执行适配器类型 / Solver execution adapter type
 */
data class RemoteSolverBootstrapOptions(
    val config: RemoteSolverConfig = RemoteSolverConfig(),
    val eventRetryPolicy: InMemoryEventRetryPolicy = InMemoryEventRetryPolicy(),
    val eventAdapter: EventAdapterType = EventAdapterType.INMEMORY,
    val eventKafkaBootstrapServers: String? = null,
    val eventKafkaClientId: String = "remote-solver-event-port",
    val eventKafkaConsumerPollIntervalMs: Long = 200L,
    val eventKafkaQueryPollTimeoutMs: Long = 1500L,
    val eventKafkaQueryMaxPollRounds: Int = 8,
    val eventKafkaQueryTopics: Set<String> = KafkaEventPort.defaultQueryTopics(),
    val eventSchemaValidationMode: EventSchemaValidationMode = EventSchemaValidationMode.LENIENT,
    val eventMirrorEnabled: Boolean = false,
    val eventMirrorFailOpen: Boolean = true,
    val distributedLockAdapter: DistributedLockAdapterType = DistributedLockAdapterType.INMEMORY,
    val distributedLockKtormUrl: String? = null,
    val distributedLockKtormUsername: String? = null,
    val distributedLockKtormPassword: String? = null,
    val distributedLockKtormTable: String = "remote_solver_lock",
    val nodeStateAdapter: NodeStateAdapterType = NodeStateAdapterType.INMEMORY,
    val nodeStateKtormUrl: String? = null,
    val nodeStateKtormUsername: String? = null,
    val nodeStateKtormPassword: String? = null,
    val nodeStateKtormTable: String = "remote_solver_node_state",
    val budgetAdapter: BudgetAdapterType = BudgetAdapterType.INMEMORY,
    val budgetKtormUrl: String? = null,
    val budgetKtormUsername: String? = null,
    val budgetKtormPassword: String? = null,
    val budgetKtormTable: String = "remote_solver_budget",
    val taskStateAdapter: TaskStateAdapterType = TaskStateAdapterType.INMEMORY,
    val taskStateKtormUrl: String? = null,
    val taskStateKtormUsername: String? = null,
    val taskStateKtormPassword: String? = null,
    val taskStateKtormTaskTable: String = "remote_solver_task_state",
    val taskStateKtormSliceTable: String = "remote_solver_slice_state",
    val costLedgerAdapter: CostLedgerAdapterType = CostLedgerAdapterType.INMEMORY,
    val costLedgerKtormUrl: String? = null,
    val costLedgerKtormUsername: String? = null,
    val costLedgerKtormPassword: String? = null,
    val costLedgerKtormTable: String = "remote_solver_cost_ledger",
    val solverExecutionAdapter: SolverExecutionAdapterType = SolverExecutionAdapterType.OSPF_INPROCESS,
    val solverExecutionOspfSolverType: SolverType = SolverType.AUTO,
    val solverExecutionOspfExternalCommand: String? = null,
    val solverExecutionOspfExternalWorkdir: String? = null,
    val solverExecutionOspfBridgeClass: String? = null,
    val solverExecutionOspfBridgeArgs: Map<String, String> = emptyMap(),
    val storageS3Bucket: String? = null,
    val storageS3Region: String = "us-east-1",
    val storageS3Endpoint: String? = null,
    val storageS3AccessKeyId: String? = null,
    val storageS3SecretAccessKey: String? = null,
    val storageS3PathStyleAccess: Boolean = true,
    val storageS3ObjectPrefix: String = "objects",
    val storageS3CheckpointPrefix: String = "checkpoints",
    val storageAdapter: StorageAdapterType = StorageAdapterType.INMEMORY,
    val schedulerAuditAdapter: SchedulerAuditAdapterType = SchedulerAuditAdapterType.INMEMORY,
    val schedulerAuditLocalFsRoot: Path = Paths.get("target", "remote-solver-audit"),
    val schedulerAuditKtormUrl: String? = null,
    val schedulerAuditKtormUsername: String? = null,
    val schedulerAuditKtormPassword: String? = null,
    val schedulerAuditKtormAuditTable: String = "remote_solver_scheduler_audit",
    val schedulerAuditKtormSnapshotTable: String = "remote_solver_scheduler_snapshot",
    val metricsAdapter: MetricsAdapterType = MetricsAdapterType.INMEMORY,
    val metricsCanonicalEnabled: Boolean = true,
    val checkpointMaxRetainedPerTask: Int = 0,
    val localFsRoot: Path = Paths.get("target", "remote-solver-localfs")
) {
    companion object {
        private fun parseSolverType(rawValue: String): SolverType = when (rawValue.lowercase()) {
            "scip" -> SolverType.SCIP
            "gurobi" -> SolverType.GUROBI
            "heuristic" -> SolverType.AUTO  // heuristic solver no longer supported, fall back to AUTO
            "auto" -> SolverType.AUTO
            else -> SolverType.AUTO
        }

        fun fromAdapterKeys(
            config: RemoteSolverConfig = RemoteSolverConfig(),
            eventRetryPolicy: InMemoryEventRetryPolicy = InMemoryEventRetryPolicy(),
            eventAdapterKey: String? = null,
            eventKafkaBootstrapServers: String? = null,
            eventKafkaClientId: String = "remote-solver-event-port",
            eventKafkaConsumerPollIntervalMs: Long = 200L,
            eventKafkaQueryPollTimeoutMs: Long = 1500L,
            eventKafkaQueryMaxPollRounds: Int = 8,
            eventKafkaQueryTopics: Set<String> = KafkaEventPort.defaultQueryTopics(),
            eventSchemaValidationModeKey: String? = null,
            eventMirrorEnabled: Boolean = false,
            eventMirrorFailOpen: Boolean = true,
            distributedLockAdapterKey: String? = null,
            distributedLockKtormUrl: String? = null,
            distributedLockKtormUsername: String? = null,
            distributedLockKtormPassword: String? = null,
            distributedLockKtormTable: String = "remote_solver_lock",
            nodeStateAdapterKey: String? = null,
            nodeStateKtormUrl: String? = null,
            nodeStateKtormUsername: String? = null,
            nodeStateKtormPassword: String? = null,
            nodeStateKtormTable: String = "remote_solver_node_state",
            budgetAdapterKey: String? = null,
            budgetKtormUrl: String? = null,
            budgetKtormUsername: String? = null,
            budgetKtormPassword: String? = null,
            budgetKtormTable: String = "remote_solver_budget",
            taskStateAdapterKey: String? = null,
            taskStateKtormUrl: String? = null,
            taskStateKtormUsername: String? = null,
            taskStateKtormPassword: String? = null,
            taskStateKtormTaskTable: String = "remote_solver_task_state",
            taskStateKtormSliceTable: String = "remote_solver_slice_state",
            costLedgerAdapterKey: String? = null,
            costLedgerKtormUrl: String? = null,
            costLedgerKtormUsername: String? = null,
            costLedgerKtormPassword: String? = null,
            costLedgerKtormTable: String = "remote_solver_cost_ledger",
            solverExecutionAdapterKey: String? = null,
            solverExecutionOspfSolverType: SolverType = SolverType.AUTO,
            solverExecutionOspfExternalCommand: String? = null,
            solverExecutionOspfExternalWorkdir: String? = null,
            solverExecutionOspfBridgeClass: String? = null,
            solverExecutionOspfBridgeArgs: Map<String, String> = emptyMap(),
            storageS3Bucket: String? = null,
            storageS3Region: String = "us-east-1",
            storageS3Endpoint: String? = null,
            storageS3AccessKeyId: String? = null,
            storageS3SecretAccessKey: String? = null,
            storageS3PathStyleAccess: Boolean = true,
            storageS3ObjectPrefix: String = "objects",
            storageS3CheckpointPrefix: String = "checkpoints",
            storageAdapterKey: String? = null,
            schedulerAuditAdapterKey: String? = null,
            schedulerAuditLocalFsRoot: Path = Paths.get("target", "remote-solver-audit"),
            schedulerAuditKtormUrl: String? = null,
            schedulerAuditKtormUsername: String? = null,
            schedulerAuditKtormPassword: String? = null,
            schedulerAuditKtormAuditTable: String = "remote_solver_scheduler_audit",
            schedulerAuditKtormSnapshotTable: String = "remote_solver_scheduler_snapshot",
            metricsAdapterKey: String? = null,
            metricsCanonicalEnabled: Boolean = true,
            checkpointMaxRetainedPerTask: Int = 0,
            localFsRoot: Path = Paths.get("target", "remote-solver-localfs")
        ): RemoteSolverBootstrapOptions {
            val eventAdapter = resolveEventAdapter(eventAdapterKey)
            val eventSchemaValidationMode = resolveEventSchemaValidationMode(eventSchemaValidationModeKey)
            val distributedLockAdapter = resolveDistributedLockAdapter(distributedLockAdapterKey)
            val nodeStateAdapter = resolveNodeStateAdapter(nodeStateAdapterKey)
            val budgetAdapter = resolveBudgetAdapter(budgetAdapterKey)
            val taskStateAdapter = resolveTaskStateAdapter(taskStateAdapterKey)
            val costLedgerAdapter = resolveCostLedgerAdapter(costLedgerAdapterKey)
            val solverExecutionAdapter = resolveSolverExecutionAdapter(solverExecutionAdapterKey)
            val storageAdapter = resolveStorageAdapter(storageAdapterKey)
            val schedulerAuditAdapter = resolveSchedulerAuditAdapter(schedulerAuditAdapterKey)
            val metricsAdapter = resolveMetricsAdapter(metricsAdapterKey)
            return RemoteSolverBootstrapOptions(
                config = config,
                eventRetryPolicy = eventRetryPolicy,
                eventAdapter = eventAdapter,
                eventKafkaBootstrapServers = eventKafkaBootstrapServers?.trim()?.takeIf { it.isNotEmpty() },
                eventKafkaClientId = eventKafkaClientId.trim().ifEmpty { "remote-solver-event-port" },
                eventKafkaConsumerPollIntervalMs = eventKafkaConsumerPollIntervalMs.coerceAtLeast(50L),
                eventKafkaQueryPollTimeoutMs = eventKafkaQueryPollTimeoutMs.coerceAtLeast(50L),
                eventKafkaQueryMaxPollRounds = eventKafkaQueryMaxPollRounds.coerceAtLeast(1),
                eventKafkaQueryTopics = eventKafkaQueryTopics.map { it.trim() }.filter { it.isNotEmpty() }.toSet()
                    .ifEmpty { KafkaEventPort.defaultQueryTopics() },
                eventSchemaValidationMode = eventSchemaValidationMode,
                eventMirrorEnabled = eventMirrorEnabled,
                eventMirrorFailOpen = eventMirrorFailOpen,
                distributedLockAdapter = distributedLockAdapter,
                distributedLockKtormUrl = distributedLockKtormUrl?.trim()?.takeIf { it.isNotEmpty() },
                distributedLockKtormUsername = distributedLockKtormUsername?.trim()?.takeIf { it.isNotEmpty() },
                distributedLockKtormPassword = distributedLockKtormPassword?.trim()?.takeIf { it.isNotEmpty() },
                distributedLockKtormTable = distributedLockKtormTable.trim().ifEmpty { "remote_solver_lock" },
                nodeStateAdapter = nodeStateAdapter,
                nodeStateKtormUrl = nodeStateKtormUrl?.trim()?.takeIf { it.isNotEmpty() },
                nodeStateKtormUsername = nodeStateKtormUsername?.trim()?.takeIf { it.isNotEmpty() },
                nodeStateKtormPassword = nodeStateKtormPassword?.trim()?.takeIf { it.isNotEmpty() },
                nodeStateKtormTable = nodeStateKtormTable.trim().ifEmpty { "remote_solver_node_state" },
                budgetAdapter = budgetAdapter,
                budgetKtormUrl = budgetKtormUrl?.trim()?.takeIf { it.isNotEmpty() },
                budgetKtormUsername = budgetKtormUsername?.trim()?.takeIf { it.isNotEmpty() },
                budgetKtormPassword = budgetKtormPassword?.trim()?.takeIf { it.isNotEmpty() },
                budgetKtormTable = budgetKtormTable.trim().ifEmpty { "remote_solver_budget" },
                taskStateAdapter = taskStateAdapter,
                taskStateKtormUrl = taskStateKtormUrl?.trim()?.takeIf { it.isNotEmpty() },
                taskStateKtormUsername = taskStateKtormUsername?.trim()?.takeIf { it.isNotEmpty() },
                taskStateKtormPassword = taskStateKtormPassword?.trim()?.takeIf { it.isNotEmpty() },
                taskStateKtormTaskTable = taskStateKtormTaskTable.trim().ifEmpty { "remote_solver_task_state" },
                taskStateKtormSliceTable = taskStateKtormSliceTable.trim().ifEmpty { "remote_solver_slice_state" },
                costLedgerAdapter = costLedgerAdapter,
                costLedgerKtormUrl = costLedgerKtormUrl?.trim()?.takeIf { it.isNotEmpty() },
                costLedgerKtormUsername = costLedgerKtormUsername?.trim()?.takeIf { it.isNotEmpty() },
                costLedgerKtormPassword = costLedgerKtormPassword?.trim()?.takeIf { it.isNotEmpty() },
                costLedgerKtormTable = costLedgerKtormTable.trim().ifEmpty { "remote_solver_cost_ledger" },
                solverExecutionAdapter = solverExecutionAdapter,
                solverExecutionOspfSolverType = solverExecutionOspfSolverType,
                solverExecutionOspfExternalCommand = solverExecutionOspfExternalCommand?.trim()?.takeIf { it.isNotEmpty() },
                solverExecutionOspfExternalWorkdir = solverExecutionOspfExternalWorkdir?.trim()?.takeIf { it.isNotEmpty() },
                solverExecutionOspfBridgeClass = solverExecutionOspfBridgeClass?.trim()?.takeIf { it.isNotEmpty() },
                solverExecutionOspfBridgeArgs = solverExecutionOspfBridgeArgs,
                storageS3Bucket = storageS3Bucket?.trim()?.takeIf { it.isNotEmpty() },
                storageS3Region = storageS3Region.trim().ifEmpty { "us-east-1" },
                storageS3Endpoint = storageS3Endpoint?.trim()?.takeIf { it.isNotEmpty() },
                storageS3AccessKeyId = storageS3AccessKeyId?.trim()?.takeIf { it.isNotEmpty() },
                storageS3SecretAccessKey = storageS3SecretAccessKey?.trim()?.takeIf { it.isNotEmpty() },
                storageS3PathStyleAccess = storageS3PathStyleAccess,
                storageS3ObjectPrefix = storageS3ObjectPrefix.trim().ifEmpty { "objects" },
                storageS3CheckpointPrefix = storageS3CheckpointPrefix.trim().ifEmpty { "checkpoints" },
                storageAdapter = storageAdapter,
                schedulerAuditAdapter = schedulerAuditAdapter,
                schedulerAuditLocalFsRoot = schedulerAuditLocalFsRoot,
                schedulerAuditKtormUrl = schedulerAuditKtormUrl?.trim()?.takeIf { it.isNotEmpty() },
                schedulerAuditKtormUsername = schedulerAuditKtormUsername?.trim()?.takeIf { it.isNotEmpty() },
                schedulerAuditKtormPassword = schedulerAuditKtormPassword?.trim()?.takeIf { it.isNotEmpty() },
                schedulerAuditKtormAuditTable = schedulerAuditKtormAuditTable.trim().ifEmpty { "remote_solver_scheduler_audit" },
                schedulerAuditKtormSnapshotTable = schedulerAuditKtormSnapshotTable.trim().ifEmpty { "remote_solver_scheduler_snapshot" },
                metricsAdapter = metricsAdapter,
                metricsCanonicalEnabled = metricsCanonicalEnabled,
                checkpointMaxRetainedPerTask = normalizeCheckpointRetention(checkpointMaxRetainedPerTask),
                localFsRoot = localFsRoot
            )
        }

        fun fromProperties(
            properties: Map<String, String>,
            config: RemoteSolverConfig = RemoteSolverConfig(),
            eventRetryPolicy: InMemoryEventRetryPolicy? = null
        ): RemoteSolverBootstrapOptions {
            val localFsRoot = properties["storage.localfs.root"]
                ?.trim()
                ?.takeIf { it.isNotEmpty() }
                ?.let { Paths.get(it) }
                ?: Paths.get("target", "remote-solver-localfs")
            val resolvedRetryPolicy = eventRetryPolicy ?: InMemoryEventRetryPolicy.fromProperties(properties)
            val checkpointRetention = parseCheckpointRetention(properties["checkpoint.retention.max-per-task"])
            val mirrorEnabled = parseBoolean(properties["event.mirror.enabled"], defaultValue = false)
            val mirrorFailOpen = parseBoolean(properties["event.mirror.fail-open"], defaultValue = true)
            return fromAdapterKeys(
                config = config,
                eventRetryPolicy = resolvedRetryPolicy,
                eventAdapterKey = properties["event.adapter"],
                eventKafkaBootstrapServers = properties["event.kafka.bootstrap-servers"],
                eventKafkaClientId = properties["event.kafka.client-id"] ?: "remote-solver-event-port",
                eventKafkaConsumerPollIntervalMs = properties["event.kafka.consumer-poll-interval-ms"]
                    ?.trim()
                    ?.toLongOrNull()
                    ?: 200L,
                eventKafkaQueryPollTimeoutMs = properties["event.kafka.query-poll-timeout-ms"]
                    ?.trim()
                    ?.toLongOrNull()
                    ?: 1500L,
                eventKafkaQueryMaxPollRounds = properties["event.kafka.query-max-poll-rounds"]
                    ?.trim()
                    ?.toIntOrNull()
                    ?: 8,
                eventKafkaQueryTopics = parseCsvSet(properties["event.kafka.query-topics"])
                    .ifEmpty { KafkaEventPort.defaultQueryTopics() },
                eventSchemaValidationModeKey = properties["event.schema.registry.mode"],
                eventMirrorEnabled = mirrorEnabled,
                eventMirrorFailOpen = mirrorFailOpen,
                distributedLockAdapterKey = properties["distributed-lock.adapter"] ?: properties["lock.adapter"],
                distributedLockKtormUrl = properties["distributed-lock.ktorm.url"]
                    ?: properties["distributed-lock.jdbc.url"]
                    ?: properties["lock.ktorm.url"]
                    ?: properties["lock.jdbc.url"],
                distributedLockKtormUsername = properties["distributed-lock.ktorm.username"]
                    ?: properties["distributed-lock.jdbc.username"]
                    ?: properties["lock.ktorm.username"]
                    ?: properties["lock.jdbc.username"],
                distributedLockKtormPassword = properties["distributed-lock.ktorm.password"]
                    ?: properties["distributed-lock.jdbc.password"]
                    ?: properties["lock.ktorm.password"]
                    ?: properties["lock.jdbc.password"],
                distributedLockKtormTable = properties["distributed-lock.ktorm.table"]
                    ?: properties["distributed-lock.jdbc.table"]
                    ?: properties["lock.ktorm.table"]
                    ?: properties["lock.jdbc.table"]
                    ?: "remote_solver_lock",
                nodeStateAdapterKey = properties["node-state.adapter"],
                nodeStateKtormUrl = properties["node-state.ktorm.url"] ?: properties["node-state.jdbc.url"] ?: properties["jdbc.url"],
                nodeStateKtormUsername = properties["node-state.ktorm.username"] ?: properties["node-state.jdbc.username"] ?: properties["jdbc.username"],
                nodeStateKtormPassword = properties["node-state.ktorm.password"] ?: properties["node-state.jdbc.password"] ?: properties["jdbc.password"],
                nodeStateKtormTable = properties["node-state.ktorm.table"] ?: properties["node-state.jdbc.table"] ?: "remote_solver_node_state",
                budgetAdapterKey = properties["budget.adapter"],
                budgetKtormUrl = properties["budget.ktorm.url"] ?: properties["budget.jdbc.url"] ?: properties["jdbc.url"],
                budgetKtormUsername = properties["budget.ktorm.username"] ?: properties["budget.jdbc.username"] ?: properties["jdbc.username"],
                budgetKtormPassword = properties["budget.ktorm.password"] ?: properties["budget.jdbc.password"] ?: properties["jdbc.password"],
                budgetKtormTable = properties["budget.ktorm.table"] ?: properties["budget.jdbc.table"] ?: "remote_solver_budget",
                taskStateAdapterKey = properties["task-state.adapter"],
                taskStateKtormUrl = properties["task-state.ktorm.url"] ?: properties["task-state.jdbc.url"] ?: properties["jdbc.url"],
                taskStateKtormUsername = properties["task-state.ktorm.username"] ?: properties["task-state.jdbc.username"] ?: properties["jdbc.username"],
                taskStateKtormPassword = properties["task-state.ktorm.password"] ?: properties["task-state.jdbc.password"] ?: properties["jdbc.password"],
                taskStateKtormTaskTable = properties["task-state.ktorm.task-table"] ?: properties["task-state.jdbc.task-table"] ?: "remote_solver_task_state",
                taskStateKtormSliceTable = properties["task-state.ktorm.slice-table"] ?: properties["task-state.jdbc.slice-table"] ?: "remote_solver_slice_state",
                costLedgerAdapterKey = properties["cost-ledger.adapter"],
                costLedgerKtormUrl = properties["cost-ledger.ktorm.url"] ?: properties["cost-ledger.jdbc.url"] ?: properties["jdbc.url"],
                costLedgerKtormUsername = properties["cost-ledger.ktorm.username"] ?: properties["cost-ledger.jdbc.username"] ?: properties["jdbc.username"],
                costLedgerKtormPassword = properties["cost-ledger.ktorm.password"] ?: properties["cost-ledger.jdbc.password"] ?: properties["jdbc.password"],
                costLedgerKtormTable = properties["cost-ledger.ktorm.table"] ?: properties["cost-ledger.jdbc.table"] ?: "remote_solver_cost_ledger",
                solverExecutionAdapterKey = properties["solver-execution.adapter"],
                solverExecutionOspfSolverType = properties["solver-execution.ospf.solver-type"]
                    ?.trim()
                    ?.let { parseSolverType(it) }
                    ?: SolverType.AUTO,
                solverExecutionOspfExternalCommand = properties["solver-execution.ospf.external.command"]
                    ?: properties["solver-execution.ospf.bridge.arg.command"],
                solverExecutionOspfExternalWorkdir = properties["solver-execution.ospf.external.workdir"]
                    ?: properties["solver-execution.ospf.bridge.arg.workdir"],
                solverExecutionOspfBridgeClass = properties["solver-execution.ospf.bridge-class"],
                solverExecutionOspfBridgeArgs = parseSubProperties(
                    properties = properties,
                    prefix = "solver-execution.ospf.bridge.arg."
                ),
                storageS3Bucket = properties["storage.s3.bucket"],
                storageS3Region = properties["storage.s3.region"] ?: "us-east-1",
                storageS3Endpoint = properties["storage.s3.endpoint"],
                storageS3AccessKeyId = properties["storage.s3.access-key-id"],
                storageS3SecretAccessKey = properties["storage.s3.secret-access-key"],
                storageS3PathStyleAccess = parseBoolean(properties["storage.s3.path-style-access"], defaultValue = true),
                storageS3ObjectPrefix = properties["storage.s3.object-prefix"] ?: "objects",
                storageS3CheckpointPrefix = properties["storage.s3.checkpoint-prefix"] ?: "checkpoints",
                storageAdapterKey = properties["storage.adapter"],
                schedulerAuditAdapterKey = properties["scheduler.audit.adapter"],
                schedulerAuditLocalFsRoot = properties["scheduler.audit.localfs.path"]
                    ?.trim()
                    ?.takeIf { it.isNotEmpty() }
                    ?.let { Paths.get(it) }
                    ?: Paths.get("target", "remote-solver-audit"),
                schedulerAuditKtormUrl = properties["scheduler.audit.ktorm.url"]
                    ?: properties["scheduler.audit.jdbc.url"],
                schedulerAuditKtormUsername = properties["scheduler.audit.ktorm.username"]
                    ?: properties["scheduler.audit.jdbc.username"],
                schedulerAuditKtormPassword = properties["scheduler.audit.ktorm.password"]
                    ?: properties["scheduler.audit.jdbc.password"],
                schedulerAuditKtormAuditTable = properties["scheduler.audit.ktorm.audit-table"]
                    ?: properties["scheduler.audit.jdbc.audit-table"]
                    ?: "remote_solver_scheduler_audit",
                schedulerAuditKtormSnapshotTable = properties["scheduler.audit.ktorm.snapshot-table"]
                    ?: properties["scheduler.audit.jdbc.snapshot-table"]
                    ?: "remote_solver_scheduler_snapshot",
                metricsAdapterKey = properties["metrics.adapter"],
                metricsCanonicalEnabled = parseBoolean(properties["metrics.canonical.enabled"], defaultValue = true),
                checkpointMaxRetainedPerTask = checkpointRetention,
                localFsRoot = localFsRoot
            )
        }

        private fun resolveEventAdapter(adapterKey: String?): EventAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return EventAdapterType.INMEMORY
            }
            return EventAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported event adapter: '$adapterKey'. Supported: ${EventAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveEventSchemaValidationMode(key: String?): EventSchemaValidationMode {
            if (key.isNullOrBlank()) {
                return EventSchemaValidationMode.LENIENT
            }
            return EventSchemaValidationMode.fromKeyOrNull(key)
                ?: throw IllegalArgumentException(
                    "Unsupported event schema registry mode: '$key'. Supported: ${
                        EventSchemaValidationMode.values().joinToString { it.name }
                    }"
                )
        }

        private fun resolveDistributedLockAdapter(adapterKey: String?): DistributedLockAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return DistributedLockAdapterType.INMEMORY
            }
            return DistributedLockAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported distributed-lock adapter: '$adapterKey'. Supported: ${DistributedLockAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveStorageAdapter(adapterKey: String?): StorageAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return StorageAdapterType.INMEMORY
            }
            return StorageAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported storage adapter: '$adapterKey'. Supported: ${StorageAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveSchedulerAuditAdapter(adapterKey: String?): SchedulerAuditAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return SchedulerAuditAdapterType.INMEMORY
            }
            return SchedulerAuditAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported scheduler-audit adapter: '$adapterKey'. Supported: ${SchedulerAuditAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveMetricsAdapter(adapterKey: String?): MetricsAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return MetricsAdapterType.INMEMORY
            }
            return MetricsAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported metrics adapter: '$adapterKey'. Supported: ${MetricsAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveNodeStateAdapter(adapterKey: String?): NodeStateAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return NodeStateAdapterType.INMEMORY
            }
            return NodeStateAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported node-state adapter: '$adapterKey'. Supported: ${NodeStateAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveBudgetAdapter(adapterKey: String?): BudgetAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return BudgetAdapterType.INMEMORY
            }
            return BudgetAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported budget adapter: '$adapterKey'. Supported: ${BudgetAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveTaskStateAdapter(adapterKey: String?): TaskStateAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return TaskStateAdapterType.INMEMORY
            }
            return TaskStateAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported task-state adapter: '$adapterKey'. Supported: ${TaskStateAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveCostLedgerAdapter(adapterKey: String?): CostLedgerAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return CostLedgerAdapterType.INMEMORY
            }
            return CostLedgerAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported cost-ledger adapter: '$adapterKey'. Supported: ${CostLedgerAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun resolveSolverExecutionAdapter(adapterKey: String?): SolverExecutionAdapterType {
            if (adapterKey.isNullOrBlank()) {
                return SolverExecutionAdapterType.INMEMORY
            }
            return SolverExecutionAdapterType.fromKeyOrNull(adapterKey)
                ?: throw IllegalArgumentException(
                    "Unsupported solver-execution adapter: '$adapterKey'. Supported: ${SolverExecutionAdapterType.values().joinToString { it.key }}"
                )
        }

        private fun parseCheckpointRetention(rawValue: String?): Int {
            if (rawValue.isNullOrBlank()) {
                return 0
            }
            val parsed = rawValue.trim().toIntOrNull()
                ?: throw IllegalArgumentException("Invalid checkpoint retention value: '$rawValue'. Expected non-negative integer.")
            return normalizeCheckpointRetention(parsed)
        }

        private fun normalizeCheckpointRetention(value: Int): Int {
            if (value < 0) {
                throw IllegalArgumentException("checkpointMaxRetainedPerTask must be >= 0, but was $value")
            }
            return value
        }

        private fun parseBoolean(rawValue: String?, defaultValue: Boolean): Boolean {
            if (rawValue.isNullOrBlank()) {
                return defaultValue
            }
            return when (rawValue.trim().lowercase()) {
                "true", "1", "yes", "y", "on" -> true
                "false", "0", "no", "n", "off" -> false
                else -> throw IllegalArgumentException("Invalid boolean value: '$rawValue'")
            }
        }

        private fun parseSubProperties(
            properties: Map<String, String>,
            prefix: String
        ): Map<String, String> =
            properties.entries
                .asSequence()
                .filter { it.key.startsWith(prefix) }
                .mapNotNull { entry ->
                    val key = entry.key.removePrefix(prefix).trim()
                    if (key.isEmpty()) {
                        null
                    } else {
                        key to entry.value
                    }
                }
                .toMap()

        private fun parseCsvSet(rawValue: String?): Set<String> {
            if (rawValue.isNullOrBlank()) {
                return emptySet()
            }
            return rawValue.split(',')
                .asSequence()
                .map { it.trim() }
                .filter { it.isNotEmpty() }
                .toSet()
        }
    }
}

/**
 * 远程求解器运行时
 * Remote solver runtime
 *
 * 包含所有已组装的端口实例和服务对象。
 * Contains all assembled port instances and service objects.
 *
 * @param service 远程求解器服务 / Remote solver service
 * @param eventPort 事件端口 / Event port
 * @param taskStatePort 任务状态端口 / Task state port
 * @param nodeStatePort 节点状态端口 / Node state port
 * @param objectStoragePort 对象存储端口 / Object storage port
 * @param checkpointPort 检查点端口 / Checkpoint port
 * @param budgetPort 预算端口 / Budget port
 * @param solverExecutionPort 求解器执行端口 / Solver execution port
 */
data class RemoteSolverRuntime(
    val service: RemoteSolverService,
    val eventPort: EventPort,
    val taskStatePort: TaskStatePort,
    val nodeStatePort: NodeStatePort,
    val objectStoragePort: ObjectStoragePort,
    val checkpointPort: CheckpointPort,
    val budgetPort: BudgetPort,
    val costLedgerPort: CostLedgerPort,
    val distributedLockPort: DistributedLockPort,
    val solverExecutionPort: SolverExecutionPort,
    val metricsPort: MetricsPort,
    val schedulerConfigAuditPort: SchedulerConfigAuditPort,
    val apiFacade: RemoteSolverApiFacade
)

/**
 * 远程求解器启动工厂
 * Remote solver bootstrap factory
 *
 * 根据配置选项组装并创建远程求解器运行时实例。
 * Assembles and creates remote solver runtime instances based on configuration options.
 */
object RemoteSolverBootstrapFactory {
    fun create(
        properties: Map<String, String>,
        config: RemoteSolverConfig = RemoteSolverConfig(),
        eventRetryPolicy: InMemoryEventRetryPolicy? = null
    ): RemoteSolverRuntime {
        val options = RemoteSolverBootstrapOptions.fromProperties(
            properties = properties,
            config = mergeConfigFromProperties(properties, config),
            eventRetryPolicy = eventRetryPolicy
        )
        BootstrapMigrationGuard.validate(options, properties)
        return create(options)
    }

    fun create(options: RemoteSolverBootstrapOptions = RemoteSolverBootstrapOptions()): RemoteSolverRuntime {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val eventSchemaRegistry = EventSchemaRegistry.default()

        val primaryEventPort: EventPort = when (options.eventAdapter) {
            EventAdapterType.INMEMORY -> InMemoryEventPort(
                clock = clock,
                idGenerator = idGenerator,
                retryPolicy = options.eventRetryPolicy,
                schemaRegistry = eventSchemaRegistry,
                schemaValidationMode = options.eventSchemaValidationMode
            )
            EventAdapterType.KAFKA -> KafkaEventPort(
                clock = clock,
                idGenerator = idGenerator,
                bootstrapServers = options.eventKafkaBootstrapServers
                    ?: throw IllegalArgumentException("event.kafka.bootstrap-servers is required when event.adapter=kafka"),
                clientId = options.eventKafkaClientId,
                consumerPollIntervalMs = options.eventKafkaConsumerPollIntervalMs,
                queryPollTimeoutMs = options.eventKafkaQueryPollTimeoutMs,
                queryMaxPollRounds = options.eventKafkaQueryMaxPollRounds,
                queryTopics = options.eventKafkaQueryTopics,
                schemaRegistry = eventSchemaRegistry,
                schemaValidationMode = options.eventSchemaValidationMode
            )
        }
        val eventPort: EventPort = if (options.eventMirrorEnabled) {
            val mirrorPort: EventPort = when (options.eventAdapter) {
                EventAdapterType.INMEMORY -> InMemoryEventPort(
                    clock = clock,
                    idGenerator = idGenerator,
                    retryPolicy = options.eventRetryPolicy,
                    schemaRegistry = eventSchemaRegistry,
                    schemaValidationMode = options.eventSchemaValidationMode
                )
                EventAdapterType.KAFKA -> KafkaEventPort(
                    clock = clock,
                    idGenerator = idGenerator,
                    bootstrapServers = options.eventKafkaBootstrapServers
                        ?: throw IllegalArgumentException("event.kafka.bootstrap-servers is required when event.adapter=kafka"),
                    clientId = "${options.eventKafkaClientId}-mirror",
                    consumerPollIntervalMs = options.eventKafkaConsumerPollIntervalMs,
                    queryPollTimeoutMs = options.eventKafkaQueryPollTimeoutMs,
                    queryMaxPollRounds = options.eventKafkaQueryMaxPollRounds,
                    queryTopics = options.eventKafkaQueryTopics,
                    schemaRegistry = eventSchemaRegistry,
                    schemaValidationMode = options.eventSchemaValidationMode
                )
            }
            MirroringEventPort(
                primary = primaryEventPort,
                mirror = mirrorPort,
                failOpen = options.eventMirrorFailOpen
            )
        } else {
            primaryEventPort
        }
        val taskStatePort: TaskStatePort = when (options.taskStateAdapter) {
            TaskStateAdapterType.INMEMORY -> InMemoryTaskStatePort()
            TaskStateAdapterType.KTORM -> KtormTaskStatePort(
                jdbcUrl = options.taskStateKtormUrl
                    ?: throw IllegalArgumentException("task-state.ktorm.url is required when task-state.adapter=ktorm"),
                username = options.taskStateKtormUsername,
                password = options.taskStateKtormPassword,
                taskTableName = options.taskStateKtormTaskTable,
                sliceTableName = options.taskStateKtormSliceTable
            )
        }
        val nodeStatePort: NodeStatePort = when (options.nodeStateAdapter) {
            NodeStateAdapterType.INMEMORY -> InMemoryNodeStatePort()
            NodeStateAdapterType.KTORM -> KtormNodeStatePort(
                jdbcUrl = options.nodeStateKtormUrl
                    ?: throw IllegalArgumentException("node-state.ktorm.url is required when node-state.adapter=ktorm"),
                username = options.nodeStateKtormUsername,
                password = options.nodeStateKtormPassword,
                tableName = options.nodeStateKtormTable
            )
        }
        val budgetPort: BudgetPort = when (options.budgetAdapter) {
            BudgetAdapterType.INMEMORY -> InMemoryBudgetPort()
            BudgetAdapterType.KTORM -> KtormBudgetPort(
                jdbcUrl = options.budgetKtormUrl
                    ?: throw IllegalArgumentException("budget.ktorm.url is required when budget.adapter=ktorm"),
                username = options.budgetKtormUsername,
                password = options.budgetKtormPassword,
                tableName = options.budgetKtormTable
            )
        }
        val costLedgerPort: CostLedgerPort = when (options.costLedgerAdapter) {
            CostLedgerAdapterType.INMEMORY -> InMemoryCostLedgerPort()
            CostLedgerAdapterType.KTORM -> KtormCostLedgerPort(
                jdbcUrl = options.costLedgerKtormUrl
                    ?: throw IllegalArgumentException("cost-ledger.ktorm.url is required when cost-ledger.adapter=ktorm"),
                username = options.costLedgerKtormUsername,
                password = options.costLedgerKtormPassword,
                tableName = options.costLedgerKtormTable
            )
        }
        val distributedLockPort: DistributedLockPort = when (options.distributedLockAdapter) {
            DistributedLockAdapterType.INMEMORY -> InMemoryDistributedLockPort(clock, idGenerator)
            DistributedLockAdapterType.KTORM -> KtormDistributedLockPort(
                clock = clock,
                idGenerator = idGenerator,
                jdbcUrl = options.distributedLockKtormUrl
                    ?: throw IllegalArgumentException("distributed-lock.ktorm.url is required when distributed-lock.adapter=ktorm"),
                username = options.distributedLockKtormUsername,
                password = options.distributedLockKtormPassword,
                tableName = options.distributedLockKtormTable
            )
        }
        val rawMetricsPort: MetricsPort = when (options.metricsAdapter) {
            MetricsAdapterType.INMEMORY -> InMemoryMetricsPort()
            MetricsAdapterType.PROMETHEUS -> PrometheusMetricsPort()
        }
        val metricsPort: MetricsPort = if (options.metricsCanonicalEnabled) {
            CanonicalMetricsPort(rawMetricsPort)
        } else {
            rawMetricsPort
        }
        val tracingPort = InMemoryTracingPort()
        val schedulerConfigAuditPort: SchedulerConfigAuditPort = when (options.schedulerAuditAdapter) {
            SchedulerAuditAdapterType.INMEMORY -> InMemorySchedulerConfigAuditPort()
            SchedulerAuditAdapterType.LOCALFS -> LocalFsSchedulerConfigAuditPort(options.schedulerAuditLocalFsRoot)
            SchedulerAuditAdapterType.KTORM -> KtormSchedulerConfigAuditPort(
                jdbcUrl = options.schedulerAuditKtormUrl
                    ?: throw IllegalArgumentException("scheduler.audit.ktorm.url is required when scheduler.audit.adapter=ktorm"),
                username = options.schedulerAuditKtormUsername,
                password = options.schedulerAuditKtormPassword,
                auditTableName = options.schedulerAuditKtormAuditTable,
                snapshotTableName = options.schedulerAuditKtormSnapshotTable
            )
        }
        val schedulerEngine = SchedulerEngine(clock)

        val objectStoragePort: ObjectStoragePort
        val checkpointPort: CheckpointPort
        when (options.storageAdapter) {
            StorageAdapterType.INMEMORY -> {
                objectStoragePort = InMemoryObjectStoragePort(clock)
                checkpointPort = InMemoryCheckpointPort(options.checkpointMaxRetainedPerTask)
            }

            StorageAdapterType.LOCALFS -> {
                val root = options.localFsRoot.toAbsolutePath().normalize()
                objectStoragePort = LocalFsObjectStoragePort(root.resolve("objects"), clock)
                checkpointPort = LocalFsCheckpointPort(
                    root.resolve("checkpoints"),
                    options.checkpointMaxRetainedPerTask
                )
            }

            StorageAdapterType.S3 -> {
                val bucket = options.storageS3Bucket
                    ?: throw IllegalArgumentException("storage.s3.bucket is required when storage.adapter=s3")
                val endpoint = options.storageS3Endpoint
                    ?: "https://s3.${options.storageS3Region}.amazonaws.com"
                val accessKeyId = options.storageS3AccessKeyId
                    ?: throw IllegalArgumentException("storage.s3.access-key-id is required when storage.adapter=s3")
                val secretAccessKey = options.storageS3SecretAccessKey
                    ?: throw IllegalArgumentException("storage.s3.secret-access-key is required when storage.adapter=s3")
                val minioClient = MinioClient.builder()
                    .endpoint(endpoint)
                    .credentials(accessKeyId, secretAccessKey)
                    .region(options.storageS3Region)
                    .build()
                objectStoragePort = S3ObjectStoragePort(
                    minio = minioClient,
                    clock = clock,
                    bucket = bucket,
                    rootPrefix = options.storageS3ObjectPrefix
                )
                checkpointPort = S3CheckpointPort(
                    minio = minioClient,
                    bucket = bucket,
                    rootPrefix = options.storageS3CheckpointPrefix,
                    maxRetainedPerTask = options.checkpointMaxRetainedPerTask
                )
            }
        }

        val solverExecutionPort: SolverExecutionPort = when (options.solverExecutionAdapter) {
            SolverExecutionAdapterType.INMEMORY -> {
                // In-memory solver execution for testing and development
                InMemorySolverExecutionPort(
                    clock = clock,
                    idGenerator = idGenerator,
                    objectStoragePort = objectStoragePort
                )
            }
            SolverExecutionAdapterType.OSPF_INPROCESS -> {
                // Check if custom bridge class is specified
                val bridge = instantiateOspfBridge(
                    className = options.solverExecutionOspfBridgeClass,
                    bridgeArgs = options.solverExecutionOspfBridgeArgs,
                    clock = clock,
                    idGenerator = idGenerator,
                    objectStoragePort = objectStoragePort
                )
                if (bridge != null) {
                    OspfSolverExecutionPort(bridge)
                } else {
                    OspfSolverExecutionPort.withInProcessBridge(
                        clock = clock,
                        idGenerator = idGenerator,
                        objectStoragePort = objectStoragePort,
                        solverType = options.solverExecutionOspfSolverType
                    )
                }
            }
            SolverExecutionAdapterType.OSPF_EXTERNAL -> {
                val command = options.solverExecutionOspfExternalCommand
                    ?: options.solverExecutionOspfBridgeArgs["command"]
                    ?: throw IllegalArgumentException(
                        "solver-execution.ospf.external.command is required when solver-execution.adapter=ospf-external"
                    )
                OspfSolverExecutionPort.withExternalProcessBridge(
                    clock = clock,
                    idGenerator = idGenerator,
                    objectStoragePort = objectStoragePort,
                    command = command,
                    workdir = options.solverExecutionOspfExternalWorkdir
                )
            }
        }
        val service = RemoteSolverService(
            schedulerEngine = schedulerEngine,
            taskStatePort = taskStatePort,
            nodeStatePort = nodeStatePort,
            eventPort = eventPort,
            checkpointPort = checkpointPort,
            budgetPort = budgetPort,
            costLedgerPort = costLedgerPort,
            distributedLockPort = distributedLockPort,
            solverExecutionPort = solverExecutionPort,
            metricsPort = metricsPort,
            tracingPort = tracingPort,
            clock = clock,
            idGenerator = idGenerator,
            config = options.config,
            schedulerConfigAuditPort = schedulerConfigAuditPort
        )

        return RemoteSolverRuntime(
            service = service,
            eventPort = eventPort,
            taskStatePort = taskStatePort,
            nodeStatePort = nodeStatePort,
            objectStoragePort = objectStoragePort,
            checkpointPort = checkpointPort,
            budgetPort = budgetPort,
            costLedgerPort = costLedgerPort,
            distributedLockPort = distributedLockPort,
            solverExecutionPort = solverExecutionPort,
            metricsPort = metricsPort,
            schedulerConfigAuditPort = schedulerConfigAuditPort,
            apiFacade = RemoteSolverApiFacade(service)
        )
    }

    private fun mergeConfigFromProperties(
        properties: Map<String, String>,
        base: RemoteSolverConfig
    ): RemoteSolverConfig {
        val resolved = base.copy(
            schedulerConfigVersion = properties["scheduler.config.version"]
                ?.trim()
                ?.takeIf { it.isNotEmpty() }
                ?: base.schedulerConfigVersion,
            schedulerHotReloadEnabled = parseBooleanProperty(
                properties = properties,
                key = "scheduler.hot-reload.enabled",
                fallback = base.schedulerHotReloadEnabled
            ),
            simpleTaskQuantumMs = parseLongProperty(
                properties = properties,
                key = "scheduler.simple-task-quantum-ms",
                fallback = base.simpleTaskQuantumMs,
                minValue = 1L
            ),
            complexTaskQuantumMs = parseLongProperty(
                properties = properties,
                key = "scheduler.complex-task-quantum-ms",
                fallback = base.complexTaskQuantumMs,
                minValue = 1L
            ),
            complexTaskQuantumMinMs = parseLongProperty(
                properties = properties,
                key = "scheduler.complex-task-quantum-min-ms",
                fallback = base.complexTaskQuantumMinMs,
                minValue = 1L
            ),
            complexTaskQuantumMaxMs = parseLongProperty(
                properties = properties,
                key = "scheduler.complex-task-quantum-max-ms",
                fallback = base.complexTaskQuantumMaxMs,
                minValue = 1L
            ),
            complexSolveEstimateMs = parseLongProperty(
                properties = properties,
                key = "scheduler.complex-solve-estimate-ms",
                fallback = base.complexSolveEstimateMs,
                minValue = 1L
            ),
            complexCheckpointEstimateMs = parseLongProperty(
                properties = properties,
                key = "scheduler.complex-checkpoint-estimate-ms",
                fallback = base.complexCheckpointEstimateMs,
                minValue = 0L
            ),
            complexQuantumAlpha = parseDoubleProperty(
                properties = properties,
                key = "scheduler.complex-quantum-alpha",
                fallback = base.complexQuantumAlpha,
                minValue = 0.0
            ),
            complexQuantumBeta = parseDoubleProperty(
                properties = properties,
                key = "scheduler.complex-quantum-beta",
                fallback = base.complexQuantumBeta,
                minValue = 0.0
            ),
            complexQuantumPricePenalty = parseDoubleProperty(
                properties = properties,
                key = "scheduler.complex-quantum-price-penalty",
                fallback = base.complexQuantumPricePenalty,
                minValue = 0.0
            ),
            maxSchedulingBatch = parseIntProperty(
                properties = properties,
                key = "scheduler.max-scheduling-batch",
                fallback = base.maxSchedulingBatch,
                minValue = 1
            ),
            nodeHeartbeatTimeoutMs = parseLongProperty(
                properties = properties,
                key = "scheduler.node-heartbeat-timeout-ms",
                fallback = base.nodeHeartbeatTimeoutMs,
                minValue = 1L
            ),
            sliceTimeoutGraceMs = parseLongProperty(
                properties = properties,
                key = "scheduler.slice-timeout-grace-ms",
                fallback = base.sliceTimeoutGraceMs,
                minValue = 0L
            ),
            dispatchLockTtlMs = parseLongProperty(
                properties = properties,
                key = "scheduler.dispatch-lock-ttl-ms",
                fallback = base.dispatchLockTtlMs,
                minValue = 1L
            ),
            complexTaskVariableThreshold = parseIntProperty(
                properties = properties,
                key = "scheduler.complex-task-variable-threshold",
                fallback = base.complexTaskVariableThreshold,
                minValue = 1
            ),
            complexTaskConstraintThreshold = parseIntProperty(
                properties = properties,
                key = "scheduler.complex-task-constraint-threshold",
                fallback = base.complexTaskConstraintThreshold,
                minValue = 1
            ),
            complexTaskHistoricalRuntimeThresholdMs = parseLongProperty(
                properties = properties,
                key = "scheduler.complex-task-historical-runtime-threshold-ms",
                fallback = base.complexTaskHistoricalRuntimeThresholdMs,
                minValue = 1L
            ),
            complexUrgencyWeight = parseDoubleProperty(
                properties = properties,
                key = "scheduler.complex-urgency-weight",
                fallback = base.complexUrgencyWeight,
                minValue = 0.0
            ),
            complexWaitingAgeWeight = parseDoubleProperty(
                properties = properties,
                key = "scheduler.complex-waiting-age-weight",
                fallback = base.complexWaitingAgeWeight,
                minValue = 0.0
            ),
            complexProgressNeedWeight = parseDoubleProperty(
                properties = properties,
                key = "scheduler.complex-progress-need-weight",
                fallback = base.complexProgressNeedWeight,
                minValue = 0.0
            ),
            complexCostSensitivityWeight = parseDoubleProperty(
                properties = properties,
                key = "scheduler.complex-cost-sensitivity-weight",
                fallback = base.complexCostSensitivityWeight,
                minValue = 0.0
            ),
            performanceLearningEnabled = parseBooleanProperty(
                properties = properties,
                key = "scheduler.performance-learning.enabled",
                fallback = base.performanceLearningEnabled
            ),
            performanceLearningRate = parseDoubleInRangeProperty(
                properties = properties,
                key = "scheduler.performance-learning.rate",
                fallback = base.performanceLearningRate,
                minValue = 0.0,
                maxValue = 1.0
            ),
            performanceScoreMin = parseDoubleProperty(
                properties = properties,
                key = "scheduler.performance-learning.score-min",
                fallback = base.performanceScoreMin,
                minValue = 0.0
            ),
            performanceScoreMax = parseDoubleProperty(
                properties = properties,
                key = "scheduler.performance-learning.score-max",
                fallback = base.performanceScoreMax,
                minValue = 0.0
            )
        )
        if (resolved.complexTaskQuantumMaxMs < resolved.complexTaskQuantumMinMs) {
            throw IllegalArgumentException(
                "scheduler.complex-task-quantum-max-ms must be >= scheduler.complex-task-quantum-min-ms"
            )
        }
        if (resolved.performanceScoreMax < resolved.performanceScoreMin) {
            throw IllegalArgumentException(
                "scheduler.performance-learning.score-max must be >= scheduler.performance-learning.score-min"
            )
        }
        return resolved
    }

    private fun parseLongProperty(
        properties: Map<String, String>,
        key: String,
        fallback: Long,
        minValue: Long
    ): Long {
        val rawValue = properties[key]?.trim()?.takeIf { it.isNotEmpty() } ?: return fallback
        val parsed = rawValue.toLongOrNull()
            ?: throw IllegalArgumentException("Invalid long value for '$key': '$rawValue'")
        return parsed.coerceAtLeast(minValue)
    }

    private fun parseIntProperty(
        properties: Map<String, String>,
        key: String,
        fallback: Int,
        minValue: Int
    ): Int {
        val rawValue = properties[key]?.trim()?.takeIf { it.isNotEmpty() } ?: return fallback
        val parsed = rawValue.toIntOrNull()
            ?: throw IllegalArgumentException("Invalid int value for '$key': '$rawValue'")
        return parsed.coerceAtLeast(minValue)
    }

    private fun parseDoubleProperty(
        properties: Map<String, String>,
        key: String,
        fallback: Double,
        minValue: Double
    ): Double {
        val rawValue = properties[key]?.trim()?.takeIf { it.isNotEmpty() } ?: return fallback
        val parsed = rawValue.toDoubleOrNull()
            ?: throw IllegalArgumentException("Invalid double value for '$key': '$rawValue'")
        return parsed.coerceAtLeast(minValue)
    }

    private fun parseDoubleInRangeProperty(
        properties: Map<String, String>,
        key: String,
        fallback: Double,
        minValue: Double,
        maxValue: Double
    ): Double {
        val value = parseDoubleProperty(properties, key, fallback, minValue)
        if (value > maxValue) {
            throw IllegalArgumentException("Invalid double value for '$key': '$value'. Expected <= $maxValue")
        }
        return value
    }

    private fun parseBooleanProperty(
        properties: Map<String, String>,
        key: String,
        fallback: Boolean
    ): Boolean {
        val rawValue = properties[key]?.trim()?.takeIf { it.isNotEmpty() } ?: return fallback
        return when (rawValue.lowercase()) {
            "true", "1", "yes", "y", "on" -> true
            "false", "0", "no", "n", "off" -> false
            else -> throw IllegalArgumentException("Invalid boolean value for '$key': '$rawValue'")
        }
    }

    private fun instantiateOspfBridge(
        className: String?,
        bridgeArgs: Map<String, String>,
        clock: ClockPort,
        idGenerator: IdGeneratorPort,
        objectStoragePort: ObjectStoragePort
    ): OspfExecutionBridge? {
        if (className.isNullOrBlank()) {
            val command = bridgeArgs["command"]?.trim()
            if (!command.isNullOrEmpty()) {
                return OspfExternalProcessBridge(
                    clock = clock,
                    idGenerator = idGenerator,
                    objectStoragePort = objectStoragePort,
                    args = bridgeArgs
                )
            }
            return null
        }
        val clazz = try {
            Class.forName(className)
        } catch (e: Exception) {
            throw IllegalArgumentException("Failed to load OSPF bridge class '$className'", e)
        }
        val constructors = listOf(
            listOf(ClockPort::class.java, IdGeneratorPort::class.java, ObjectStoragePort::class.java, Map::class.java),
            listOf(ClockPort::class.java, IdGeneratorPort::class.java, ObjectStoragePort::class.java),
            listOf(Map::class.java),
            emptyList()
        )
        val instance = constructors.asSequence()
            .mapNotNull { signature ->
                try {
                    val ctor = clazz.getDeclaredConstructor(*signature.toTypedArray())
                    ctor.isAccessible = true
                    when (signature.size) {
                        4 -> ctor.newInstance(clock, idGenerator, objectStoragePort, bridgeArgs)
                        3 -> ctor.newInstance(clock, idGenerator, objectStoragePort)
                        1 -> ctor.newInstance(bridgeArgs)
                        0 -> ctor.newInstance()
                        else -> null
                    }
                } catch (_: NoSuchMethodException) {
                    null
                } catch (e: Exception) {
                    throw IllegalArgumentException("Failed to instantiate OSPF bridge class '$className'", e)
                }
            }
            .firstOrNull()
            ?: throw IllegalArgumentException(
                "Failed to instantiate OSPF bridge class '$className': no supported constructor found"
            )
        if (instance !is OspfExecutionBridge) {
            throw IllegalArgumentException(
                "OSPF bridge class '$className' must implement ${OspfExecutionBridge::class.qualifiedName}"
            )
        }
        return instance
    }
}



