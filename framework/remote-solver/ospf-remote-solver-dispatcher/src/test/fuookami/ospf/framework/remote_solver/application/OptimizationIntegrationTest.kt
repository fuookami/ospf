package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.protocol.domain.*
import fuookami.ospf.framework.remote_solver.domain.*
import fuookami.ospf.framework.remote_solver.port.MetricsPort
import kotlinx.coroutines.runBlocking
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * Integration tests for optimization features (RS-018, RS-019, RS-020).
 */
class OptimizationIntegrationTest {

    private val testMetricsPort = object : MetricsPort {
        private val gauges = mutableMapOf<String, Double>()
        private val counters = mutableMapOf<String, Long>()
        private val timings = mutableMapOf<String, Long>()

        override suspend fun increment(name: String, delta: Long, tags: Map<String, String>) {
            counters[name] = (counters[name] ?: 0L) + delta
        }

        override suspend fun gauge(name: String, value: Double, tags: Map<String, String>) {
            gauges[name] = value
        }

        override suspend fun timing(name: String, durationMs: Long, tags: Map<String, String>) {
            timings[name] = durationMs
        }

        fun getGauge(name: String): Double? = gauges[name]
        fun getCounter(name: String): Long? = counters[name]
    }

    // RS-018: TaskFamilyFeature tests

    @Test
    fun taskFamilyFeatureShouldGenerateConsistentSignature() {
        val feature1 = TaskFamilyFeature(
            variableScale = VariableScale.MEDIUM,
            constraintDensity = ConstraintDensity.DENSE,
            convergencePattern = ConvergencePattern.SLOW
        )
        val feature2 = TaskFamilyFeature(
            variableScale = VariableScale.MEDIUM,
            constraintDensity = ConstraintDensity.DENSE,
            convergencePattern = ConvergencePattern.SLOW
        )

        val id1 = TaskFamilyId.fromFeature(feature1)
        val id2 = TaskFamilyId.fromFeature(feature2)

        assertEquals(id1.signature, id2.signature)
    }

    @Test
    fun taskFamilyFeatureShouldGenerateDifferentSignatures() {
        val feature1 = TaskFamilyFeature(
            variableScale = VariableScale.SMALL,
            constraintDensity = ConstraintDensity.SPARSE,
            convergencePattern = ConvergencePattern.FAST
        )
        val feature2 = TaskFamilyFeature(
            variableScale = VariableScale.LARGE,
            constraintDensity = ConstraintDensity.DENSE,
            convergencePattern = ConvergencePattern.SLOW
        )

        val id1 = TaskFamilyId.fromFeature(feature1)
        val id2 = TaskFamilyId.fromFeature(feature2)

        assertFalse { id1.signature == id2.signature }
    }

    @Test
    fun taskFamilyRegistryShouldTrackBestNode() = runBlocking {
        val registry = TaskFamilyRegistry()
        val feature = TaskFamilyFeature(
            variableScale = VariableScale.MEDIUM,
            constraintDensity = ConstraintDensity.MODERATE,
            convergencePattern = ConvergencePattern.MODERATE
        )
        val familyId = TaskFamilyId.fromFeature(feature)

        // Record performance for node1
        val registry1 = registry.record(
            familyId = familyId,
            nodeId = "node1",
            runtimeMs = 1000,
            cost = 0.5,
            success = true,
            timestampEpochMs = System.currentTimeMillis()
        )
        // Record performance for node2 (better)
        val registry2 = registry1.record(
            familyId = familyId,
            nodeId = "node2",
            runtimeMs = 800,
            cost = 0.4,
            success = true,
            timestampEpochMs = System.currentTimeMillis()
        )

        val performance = registry2.getBestNode(familyId, minSamples = 1)
        assertNotNull(performance)
        assertEquals(2, performance.size)
    }

    // RS-019: LearnableScoringModel tests

    @Test
    fun learnableScoringModelShouldComputeScore() {
        val model = LearnableScoringModel.DEFAULT

        val features = NodeScoreFeatures(
            nodeId = "node1",
            normalizedCost = 0.3,
            normalizedPerformance = 0.2,
            normalizedQueueDelay = 0.1,
            normalizedSuccessRate = 0.05,
            familyMatchScore = 0.8
        )

        val score = model.score(features)
        assertTrue { score in 0.0..1.0 }
    }

    @Test
    fun learnableScoringModelShouldCalibrate() {
        val model = LearnableScoringModel.DEFAULT

        val features = NodeScoreFeatures(
            nodeId = "node1",
            normalizedCost = 0.3,
            normalizedPerformance = 0.2,
            normalizedQueueDelay = 0.1,
            normalizedSuccessRate = 0.05,
            familyMatchScore = 0.8
        )

        val feedback = listOf(
            ScoringFeedback(
                taskId = "task1",
                nodeId = "node1",
                features = features,
                predictedScore = 0.5,
                actualScore = 0.6,
                outcome = TaskOutcome.SUCCESS_NORMAL,
                timestampEpochMs = System.currentTimeMillis()
            )
        )

        val calibrated = model.calibrate(feedback, learningRate = 0.1)
        assertNotNull(calibrated)
        assertTrue { calibrated.version != model.version }
    }

    @Test
    fun scoringModelConfigShouldProvideInitialModel() {
        val defaultConfig = ScoringModelConfig(modelType = ScoringModelConfig.ModelType.DEFAULT)
        assertEquals(LearnableScoringModel.DEFAULT.weights.costWeight, defaultConfig.getInitialModel().weights.costWeight)

        val costOptimizedConfig = ScoringModelConfig(modelType = ScoringModelConfig.ModelType.COST_OPTIMIZED)
        assertTrue { costOptimizedConfig.getInitialModel().weights.costWeight > defaultConfig.getInitialModel().weights.costWeight }

        val perfOptimizedConfig = ScoringModelConfig(modelType = ScoringModelConfig.ModelType.PERFORMANCE_OPTIMIZED)
        assertTrue { perfOptimizedConfig.getInitialModel().weights.performanceWeight > defaultConfig.getInitialModel().weights.performanceWeight }
    }

    // RS-020: A/B Experiment tests

    @Test
    fun experimentShouldAssignVariantConsistently() {
        val experiment = Experiment(
            experimentId = "exp-001",
            name = "Test Experiment",
            description = "Test",
            status = ExperimentStatus.RUNNING,
            variants = listOf(
                ExperimentVariant(
                    id = "control",
                    name = "Control",
                    isControl = true,
                    trafficPercentage = 50
                ),
                ExperimentVariant(
                    id = "treatment",
                    name = "Treatment",
                    isControl = false,
                    trafficPercentage = 50
                )
            ),
            config = ExperimentConfig(
                primaryMetric = PrimaryMetric.SUCCESS_RATE,
                minSampleSize = 100,
                significanceLevel = 0.05
            ),
            metrics = ExperimentMetrics(),
            createdAtEpochMs = System.currentTimeMillis()
        )

        // Same task ID should always get same variant
        val variant1 = experiment.getVariant("task-123")
        val variant2 = experiment.getVariant("task-123")
        assertEquals(variant1?.id, variant2?.id)
    }

    @Test
    fun experimentShouldNotAssignVariantWhenNotRunning() {
        val experiment = Experiment(
            experimentId = "exp-001",
            name = "Test Experiment",
            description = "Test",
            status = ExperimentStatus.PAUSED,
            variants = listOf(
                ExperimentVariant(
                    id = "control",
                    name = "Control",
                    isControl = true,
                    trafficPercentage = 100
                )
            ),
            config = ExperimentConfig(
                primaryMetric = PrimaryMetric.SUCCESS_RATE
            ),
            metrics = ExperimentMetrics(),
            createdAtEpochMs = System.currentTimeMillis()
        )

        val variant = experiment.getVariant("task-123")
        assertEquals(null, variant)
    }

    @Test
    fun experimentRegistryShouldManageExperiments() = runBlocking {
        val registry = ExperimentRegistry()

        val experiment = Experiment(
            experimentId = "exp-001",
            name = "Test Experiment",
            description = "Test",
            status = ExperimentStatus.RUNNING,
            variants = listOf(
                ExperimentVariant(
                    id = "control",
                    name = "Control",
                    isControl = true,
                    trafficPercentage = 50,
                    metrics = VariantMetrics()
                ),
                ExperimentVariant(
                    id = "treatment",
                    name = "Treatment",
                    isControl = false,
                    trafficPercentage = 50,
                    metrics = VariantMetrics()
                )
            ),
            config = ExperimentConfig(
                primaryMetric = PrimaryMetric.SUCCESS_RATE
            ),
            metrics = ExperimentMetrics(),
            createdAtEpochMs = System.currentTimeMillis()
        )

        val updatedRegistry = registry.copy(
            experiments = mapOf("exp-001" to experiment)
        )

        val active = updatedRegistry.getActiveExperiments()
        assertEquals(1, active.size)
        assertEquals("exp-001", active.first().experimentId)

        // Record outcome
        val afterOutcome = updatedRegistry.recordOutcome(
            experimentId = "exp-001",
            variantId = "control",
            success = true,
            runtimeMs = 1000,
            cost = 0.5
        )

        val updatedExperiment = afterOutcome.experiments["exp-001"]
        val controlVariant = updatedExperiment?.variants?.find { it.id == "control" }
        assertEquals(1, controlVariant?.metrics?.sampleCount)
    }

    @Test
    fun variantMetricsShouldCalculateCorrectly() {
        val metrics = VariantMetrics()

        // Record some outcomes
        val updated = metrics
            .record(success = true, runtimeMs = 1000, cost = 0.5)
            .record(success = true, runtimeMs = 1200, cost = 0.6)
            .record(success = false, runtimeMs = 500, cost = 0.3)

        assertEquals(3, updated.sampleCount)
        assertEquals(2, updated.successCount)
        assertEquals(2.0 / 3.0, updated.avgSuccessRate, 0.001)
        assertEquals((1000.0 + 1200.0 + 500.0) / 3.0, updated.avgRuntimeMs, 0.001)
        assertEquals((0.5 + 0.6 + 0.3) / 3.0, updated.avgCost, 0.001)
    }

    @Test
    fun schedulingDecisionCacheShouldCacheDecisions() {
        val cache = SchedulingDecisionCache(
            metricsPort = testMetricsPort,
            config = CacheConfig(enabled = true, ttlMs = 60000L)
        )

        val task = TaskState(
            taskId = "task-001",
            requestId = "req-001",
            tenantId = "tenant-001",
            status = TaskStatus.QUEUED,
            complexity = TaskComplexity.SIMPLE,
            timeSensitivity = TimeSensitivity.NON_REALTIME,
            priority = 0,
            deadline = null,
            payload = SolvePayload(
                modelRef = ObjectRef.of(path = "models/test"),
                taskMeta = TaskMeta(timeLimit = null)
            ),
            budgetScope = "test",
            createdAt = System.currentTimeMillis(),
            updatedAt = System.currentTimeMillis()
        )

        val nodes = listOf(
            NodeState(
                nodeId = "node-001",
                profile = NodeCapabilityProfile(
                    nodeId = "node-001",
                    solverType = "GUROBI",
                    parallelUnits = 4,
                    performanceScore = 1.0,
                    pricePerSecond = 0.001,
                    minBillingUnitSeconds = 1L,
                    supportsInterrupt = true,
                    supportsCheckpoint = true,
                    supportsWarmStart = true
                ),
                availableUnits = 4,
                lastHeartbeatEpochMs = System.currentTimeMillis(),
                online = true
            )
        )

        // First call - cache miss
        val cachedMiss = cache.getCachedNode(task, nodes)
        assertEquals(null, cachedMiss)

        // Cache the decision
        cache.cacheDecision(task, "node-001")

        // Second call - cache hit
        val cachedHit = cache.getCachedNode(task, nodes)
        assertEquals("node-001", cachedHit)

        val stats = cache.getStats()
        assertEquals(1L, stats.hits)
        assertEquals(1L, stats.misses)
    }
}
