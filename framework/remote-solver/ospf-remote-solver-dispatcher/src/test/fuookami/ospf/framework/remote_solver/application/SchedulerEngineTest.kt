@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.PreemptionMode
import fuookami.ospf.framework.remote_solver.protocol.domain.ResumeMode
import fuookami.ospf.framework.remote_solver.protocol.domain.SchedulingRequest
import fuookami.ospf.framework.remote_solver.protocol.domain.SchedulingEstimate
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import kotlin.time.Instant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class SchedulerEngineTest {
    @Test
    fun shouldPreferLowerQueueDelayWhenQueueWeightDominates() {
        val now = 1_700_000_000_000L
        val scheduler = SchedulerEngine(
            clock = FixedClockPort(now),
            weights = SchedulerWeights(
                costWeight = 0.0,
                deadlineRiskWeight = 0.0,
                queueDelayWeight = 1.0
            )
        )
        val task = task(deadline = null)

        val congestedNode = node(
            nodeId = "congested",
            performanceScore = 1.0,
            pricePerSecond = 1.0,
            parallelUnits = 4,
            availableUnits = 1
        )
        val idleNode = node(
            nodeId = "idle",
            performanceScore = 1.0,
            pricePerSecond = 1.0,
            parallelUnits = 4,
            availableUnits = 4
        )

        val selected = scheduler.chooseNode(task, listOf(congestedNode, idleNode))
        assertNotNull(selected)
        assertEquals("idle", selected.nodeId.value)
    }

    @Test
    fun shouldPreferDeadlineSatisfiedNodeEvenWhenItIsMoreExpensive() {
        val now = 1_700_000_000_000L
        val scheduler = SchedulerEngine(clock = FixedClockPort(now))
        val task = task(deadline = now + 12_000L)

        val cheapButLate = node(
            nodeId = "cheap-late",
            performanceScore = 1.0,
            pricePerSecond = 0.001
        )
        val expensiveButOnTime = node(
            nodeId = "expensive-on-time",
            performanceScore = 2.0,
            pricePerSecond = 10.0
        )

        val selected = scheduler.chooseNode(task, listOf(cheapButLate, expensiveButOnTime))
        assertNotNull(selected)
        assertEquals("expensive-on-time", selected.nodeId.value)
    }

    @Test
    fun shouldPreferLowestDeadlineRiskWhenNoNodeCanMeetDeadline() {
        val now = 1_700_000_000_000L
        val scheduler = SchedulerEngine(clock = FixedClockPort(now))
        val task = task(deadline = now + 5_000L)

        val cheapButVeryLate = node(
            nodeId = "cheap-risky",
            performanceScore = 0.1,
            pricePerSecond = 0.001
        )
        val expensiveButLessLate = node(
            nodeId = "expensive-less-risky",
            performanceScore = 2.0,
            pricePerSecond = 10.0
        )

        val selected = scheduler.chooseNode(task, listOf(cheapButVeryLate, expensiveButLessLate))
        assertNotNull(selected)
        assertEquals("expensive-less-risky", selected.nodeId.value)
    }

    @Test
    fun risingDeadlineRiskSelectsFastestEligibleNode() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = 1_700_000_010_000L, complexity = TaskComplexity.COMPLEX)
        val slowCheap = node("slow-cheap", performanceScore = 1.0, pricePerSecond = 0.01)
        val fastExpensive = node("fast-expensive", performanceScore = 4.0, pricePerSecond = 10.0)

        val decision = scheduler.explainNodeSelection(
            task = task,
            candidates = listOf(slowCheap, fastExpensive),
            progressSignal = ProgressSignal(deadlineRiskIncreasing = true)
        )

        assertEquals("fast-expensive", decision.selectedNode?.nodeId?.value)
        assertEquals("progress_fast_upgrade", decision.reason)
    }

    @Test
    fun complexTasksUseDeterministicWeightedRoundRobinWithinScoreBand() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null, complexity = TaskComplexity.COMPLEX)
        val first = node("node-a", performanceScore = 1.0, pricePerSecond = 1.0)
        val second = node("node-b", performanceScore = 1.0, pricePerSecond = 1.0)

        val selected = (1..4).map {
            scheduler.chooseNode(task, listOf(first, second))!!.nodeId.value
        }

        assertEquals(listOf("node-a", "node-b", "node-a", "node-b"), selected)
    }

    @Test
    fun preferredNodeIsHeldUntilTheNewScoreClearsMigrationHysteresis() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null)
        val cheap = node("cheap", performanceScore = 1.0, pricePerSecond = 1.0)
        val slightlyCheaper = node("preferred", performanceScore = 1.0, pricePerSecond = 1.1)

        val held = scheduler.chooseNode(
            task,
            listOf(cheap, slightlyCheaper),
            preferredNodeId = NodeId.of("preferred")
        )
        assertEquals("preferred", held?.nodeId?.value)

        val materiallyCheaper = slightlyCheaper.copy(
            profile = slightlyCheaper.profile.copy(pricePerSecond = fuookami.ospf.kotlin.math.algebra.number.Flt64(2.0))
        )
        val migrated = scheduler.chooseNode(
            task,
            listOf(cheap, materiallyCheaper),
            preferredNodeId = NodeId.of("preferred")
        )
        assertEquals("cheap", migrated?.nodeId?.value)
    }

    @Test
    fun selectionExplanationKeepsCapabilityRejectionReason() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val unavailable = node(
            nodeId = "full",
            performanceScore = 1.0,
            pricePerSecond = 1.0,
            availableUnits = 0
        )

        val decision = scheduler.explainNodeSelection(task(deadline = null), listOf(unavailable))

        assertEquals(null, decision.selectedNode)
        assertEquals(listOf(AdmissionReasonCode.NO_AVAILABLE_SLOT), decision.candidates.single().reasons)
    }

    @Test
    fun highRemainingGapPrefersFastNodeAndAuditsUpgrade() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null, complexity = TaskComplexity.COMPLEX).copy(
            latestResult = SolveResult(
                feasible = true,
                optimal = false,
                objectiveValue = 1.0,
                gap = 0.9,
                elapsedMs = 1_000L
            )
        )
        val slowCheap = node("slow-cheap", performanceScore = 1.0, pricePerSecond = 0.01)
        val fastExpensive = node("fast-expensive", performanceScore = 4.0, pricePerSecond = 10.0)

        val decision = scheduler.explainNodeSelection(
            task = task,
            candidates = listOf(slowCheap, fastExpensive),
            progressSignal = ProgressSignal(
                previousGap = 0.9,
                currentGap = 0.9,
                improvement = 0.0,
                noImprovementSlices = 2
            )
        )

        assertEquals("fast-expensive", decision.selectedNode?.nodeId?.value)
        assertEquals("progress_fast_upgrade", decision.reason)
    }

    @Test
    fun lowRemainingGapPrefersCheapNodeAndAuditsDowngrade() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null, complexity = TaskComplexity.COMPLEX).copy(
            latestResult = SolveResult(
                feasible = true,
                optimal = false,
                objectiveValue = 1.0,
                gap = 0.1,
                elapsedMs = 1_000L
            )
        )
        val cheap = node("cheap", performanceScore = 1.0, pricePerSecond = 0.01)
        val fastExpensive = node("fast-expensive", performanceScore = 4.0, pricePerSecond = 10.0)

        val decision = scheduler.explainNodeSelection(
            task = task,
            candidates = listOf(cheap, fastExpensive),
            progressSignal = ProgressSignal(
                previousGap = 0.1,
                currentGap = 0.1,
                improvement = 0.0,
                noImprovementSlices = 2
            )
        )

        assertEquals("cheap", decision.selectedNode?.nodeId?.value)
        assertEquals("progress_cheap_downgrade", decision.reason)
    }

    @Test
    fun absoluteGapAloneDoesNotTriggerProgressPolicy() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null, complexity = TaskComplexity.COMPLEX).copy(
            latestResult = SolveResult(
                feasible = true,
                optimal = false,
                objectiveValue = 1.0,
                gap = 0.9,
                elapsedMs = 1_000L
            )
        )
        val slowCheap = node("slow-cheap", performanceScore = 1.0, pricePerSecond = 0.01)
        val fastExpensive = node("fast-expensive", performanceScore = 4.0, pricePerSecond = 10.0)

        val decision = scheduler.explainNodeSelection(task, listOf(slowCheap, fastExpensive))

        assertEquals("slow-cheap", decision.selectedNode?.nodeId?.value)
        assertEquals("lowest_weighted_score", decision.reason)
    }

    @Test
    fun stickyTaskCannotMigrateBeforeMinimumSliceCount() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null)
        val cheap = node("cheap", performanceScore = 1.0, pricePerSecond = 1.0)
        val preferred = node("preferred", performanceScore = 1.0, pricePerSecond = 2.0)

        val decision = scheduler.explainNodeSelection(
            task = task,
            candidates = listOf(cheap, preferred),
            preferredNodeId = NodeId.of("preferred"),
            minSlicesBeforeMigration = 2,
            slicesExecuted = 1
        )

        assertEquals("preferred", decision.selectedNode?.nodeId?.value)
        assertEquals("held", decision.migrationDecision)
        assertEquals("migration_hysteresis_hold", decision.reason)
    }

    @Test
    fun largeCallerCostEstimateDoesNotFlattenNodeSpecificCostOrdering() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null).copy(
            payload = task(deadline = null).payload.copy(
                scheduling = SchedulingRequest(
                    estimate = SchedulingEstimate(cost = Flt64(1_000_000.0))
                )
            )
        )
        // The IDs intentionally sort opposite to the cost ordering. If the
        // explicit estimate replaced node cost, the tie-breaker would select
        // a-expensive instead of the cheaper z-cheap node.
        val expensive = node("a-expensive", 1.0, 2.0)
        val cheap = node("z-cheap", 1.0, 1.0)

        val decision = scheduler.explainNodeSelection(task, listOf(expensive, cheap))

        assertEquals("z-cheap", decision.selectedNode?.nodeId?.value)
        val costs = decision.candidates.associateBy { it.nodeId }
        assertNotNull(costs["a-expensive"]?.estimatedCost)
        assertNotNull(costs["z-cheap"]?.estimatedCost)
        assertEquals(
            true,
            costs.getValue("z-cheap").estimatedCost!! < costs.getValue("a-expensive").estimatedCost!!
        )
    }

    @Test
    fun opaqueReferenceIsRejectedUnlessNodeExplicitlyAdvertisesUnknown() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val opaqueTask = task(deadline = null).copy(
            payload = SolvePayload(modelData = ModelData.reference(ObjectRef.of(path = "model/opaque")))
        )
        val typedNode = node("typed-only", 1.0, 1.0).copy(
            profile = node("typed-only", 1.0, 1.0).profile.copy(
                supportedModelTypes = setOf(NormalizedModelType.LINEAR)
            )
        )

        val decision = scheduler.explainNodeSelection(opaqueTask, listOf(typedNode))

        assertEquals(null, decision.selectedNode)
        assertEquals(listOf(AdmissionReasonCode.MODEL_UNSUPPORTED), decision.candidates.single().reasons)
    }

    @Test
    fun schedulingModesAreAdmittedOnlyByNodesThatDeclareTheirCapabilities() {
        val scheduler = SchedulerEngine(clock = FixedClockPort(1_700_000_000_000L))
        val task = task(deadline = null).copy(
            payload = SolvePayload(
                modelData = ModelData.reference(ObjectRef.of(path = "model/mode-required")),
                extension = mapOf("modelType" to "LINEAR"),
                scheduling = SchedulingRequest(
                    preemptionMode = PreemptionMode.CONTROLLED_RETURN,
                    resumeMode = ResumeMode.WARM_START
                )
            )
        )
        val nodeWithoutRecovery = node("no-recovery", 1.0, 1.0).copy(
            profile = node("no-recovery", 1.0, 1.0).profile.copy(
                supportsCheckpoint = false,
                supportsWarmStart = false
            )
        )

        val decision = scheduler.explainNodeSelection(task, listOf(nodeWithoutRecovery))

        assertEquals(null, decision.selectedNode)
        assertEquals(
            setOf(AdmissionReasonCode.CHECKPOINT_UNSUPPORTED, AdmissionReasonCode.WARM_START_UNSUPPORTED),
            decision.candidates.single().reasons.toSet()
        )
    }

    private fun task(
        deadline: Long?,
        complexity: TaskComplexity = TaskComplexity.SIMPLE
    ): TaskState =
        TaskState(
            taskId = "task-test",
            requestId = "req-test",
            status = TaskStatus.QUEUED,
            complexity = complexity,
            timeSensitivity = TimeSensitivity.NON_REALTIME,
            priority = 1,
            deadline = deadline,
            payload = SolvePayload(
                modelRef = ObjectRef.of(path = "model/test"),
                extension = mapOf("modelType" to "LINEAR")
            ),
            createdAt = 1_700_000_000_000L,
            updatedAt = 1_700_000_000_000L,
            budgetScope = "budget-test"
        )

    private fun node(
        nodeId: String,
        performanceScore: Double,
        pricePerSecond: Double,
        parallelUnits: Int = 1,
        availableUnits: Int = parallelUnits
    ): NodeState =
        NodeState(
            nodeId = nodeId,
            profile = NodeCapabilityProfile(
                nodeId = nodeId,
                solverType = "test",
                performanceScore = performanceScore,
                pricePerSecond = pricePerSecond,
                minBillingUnitSeconds = 1L,
                supportsInterrupt = true,
                supportsCheckpoint = true,
                supportsWarmStart = true,
                parallelUnits = parallelUnits
            ),
            availableUnits = availableUnits,
            lastHeartbeatEpochMs = 1_700_000_000_000L,
            online = true
        )

    private data class FixedClockPort(private val now: Long) : ClockPort {
        override fun now(): Instant = Instant.fromEpochMilliseconds(now)
    }
}
