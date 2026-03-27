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
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTaskStatePort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTracingPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.mock.MockSolverExecutionPort
import fuookami.ospf.framework.remote_solver.application.RemoteSolverApiFacade
import fuookami.ospf.framework.remote_solver.application.RemoteSolverConfig
import fuookami.ospf.framework.remote_solver.application.RemoteSolverService
import fuookami.ospf.framework.remote_solver.application.SchedulerEngine
import fuookami.ospf.framework.remote_solver.domain.EventSchemaRegistry
import fuookami.ospf.framework.remote_solver.domain.EventSchemaValidationMode
import fuookami.ospf.framework.remote_solver.port.BudgetPort
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort
import fuookami.ospf.framework.remote_solver.port.DistributedLockPort
import fuookami.ospf.framework.remote_solver.port.EventPort
import fuookami.ospf.framework.remote_solver.port.MetricsPort
import fuookami.ospf.framework.remote_solver.port.NodeStatePort
import fuookami.ospf.framework.remote_solver.port.SchedulerConfigAuditPort
import fuookami.ospf.framework.remote_solver.port.TaskStatePort
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort

/**
 * In-memory bootstrap for testing purposes.
 *
 * Provides a fully in-memory runtime for RemoteSolverService testing.
 */
object InMemoryRemoteSolverBootstrap {
    fun create(
        config: RemoteSolverConfig = RemoteSolverConfig()
    ): RemoteSolverRuntime {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val eventSchemaRegistry = EventSchemaRegistry.default()

        val eventPort: EventPort = InMemoryEventPort(
            clock = clock,
            idGenerator = idGenerator,
            retryPolicy = InMemoryEventRetryPolicy(),
            schemaRegistry = eventSchemaRegistry,
            schemaValidationMode = EventSchemaValidationMode.LENIENT
        )
        val taskStatePort: TaskStatePort = InMemoryTaskStatePort()
        val nodeStatePort: NodeStatePort = InMemoryNodeStatePort()
        val budgetPort: BudgetPort = InMemoryBudgetPort()
        val costLedgerPort: CostLedgerPort = InMemoryCostLedgerPort()
        val distributedLockPort: DistributedLockPort = InMemoryDistributedLockPort(clock, idGenerator)
        val metricsPort: MetricsPort = InMemoryMetricsPort()
        val tracingPort = InMemoryTracingPort()
        val schedulerConfigAuditPort: SchedulerConfigAuditPort = InMemorySchedulerConfigAuditPort()
        val schedulerEngine = SchedulerEngine(clock)

        val objectStoragePort: ObjectStoragePort = InMemoryObjectStoragePort(clock)
        val checkpointPort: CheckpointPort = InMemoryCheckpointPort(0)

        val solverExecutionPort: SolverExecutionPort = MockSolverExecutionPort(
            clock = clock,
            idGenerator = idGenerator,
            objectStoragePort = objectStoragePort,
            simulatedTotalRuntimeMs = config.complexSolveEstimateMs.coerceAtLeast(config.complexTaskQuantumMs * 4),
            // COMPLEX tasks will take ~4 slices to complete (progressPerSlice = 0.25)
            // This ensures suspend/resume lifecycle events are published
            progressPerSlice = 0.25
        )

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
            config = config,
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
}
