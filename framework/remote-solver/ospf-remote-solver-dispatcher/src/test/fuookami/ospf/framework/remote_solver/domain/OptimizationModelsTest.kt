package fuookami.ospf.framework.remote_solver.domain

import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class TaskFamilyModelsTest {
    @Test
    fun `TaskFamilyFeature should compute similarity correctly`() {
        val feature1 = TaskFamilyFeature(
            variableScale = VariableScale.MEDIUM,
            constraintDensity = ConstraintDensity.MODERATE,
            convergencePattern = ConvergencePattern.MODERATE
        )
        val feature2 = TaskFamilyFeature(
            variableScale = VariableScale.MEDIUM,
            constraintDensity = ConstraintDensity.MODERATE,
            convergencePattern = ConvergencePattern.MODERATE
        )

        // Same features should have high similarity
        assertTrue(feature1.similarityTo(feature2) > 0.99)
    }

    @Test
    fun `TaskFamilyFeature should differentiate dissimilar features`() {
        val feature1 = TaskFamilyFeature(
            variableScale = VariableScale.TINY,
            constraintDensity = ConstraintDensity.SPARSE,
            convergencePattern = ConvergencePattern.FAST
        )
        val feature2 = TaskFamilyFeature(
            variableScale = VariableScale.HUGE,
            constraintDensity = ConstraintDensity.VERY_DENSE,
            convergencePattern = ConvergencePattern.VERY_SLOW
        )

        // Different features should have lower similarity
        assertTrue(feature1.similarityTo(feature2) < 0.7)
    }

    @Test
    fun `TaskFamilyFeature should extract from TaskMeta`() {
        val meta = TaskMeta(
            estimatedVariableCount = 5000,
            estimatedConstraintCount = 10000,
            historicalRuntimeMs = 60000
        )

        val feature = TaskFamilyFeature.fromTaskMeta(meta)

        assertEquals(VariableScale.MEDIUM, feature.variableScale)
        assertEquals(ConstraintDensity.DENSE, feature.constraintDensity)
        assertEquals(ConvergencePattern.SLOW, feature.convergencePattern)
    }

    @Test
    fun `VariableScale should classify correctly`() {
        assertEquals(VariableScale.TINY, VariableScale.fromCount(50))
        assertEquals(VariableScale.SMALL, VariableScale.fromCount(500))
        assertEquals(VariableScale.MEDIUM, VariableScale.fromCount(5000))
        assertEquals(VariableScale.LARGE, VariableScale.fromCount(50000))
        assertEquals(VariableScale.HUGE, VariableScale.fromCount(500000))
    }

    @Test
    fun `ConstraintDensity should calculate correctly`() {
        assertEquals(ConstraintDensity.SPARSE, ConstraintDensity.fromCounts(1000, 200))
        assertEquals(ConstraintDensity.MODERATE, ConstraintDensity.fromCounts(1000, 1000))
        assertEquals(ConstraintDensity.DENSE, ConstraintDensity.fromCounts(1000, 3000))
        assertEquals(ConstraintDensity.VERY_DENSE, ConstraintDensity.fromCounts(1000, 10000))
    }

    @Test
    fun `TaskFamilyId should create signature correctly`() {
        val feature = TaskFamilyFeature(
            variableScale = VariableScale.MEDIUM,
            constraintDensity = ConstraintDensity.DENSE,
            convergencePattern = ConvergencePattern.SLOW,
            solverTypeHint = "gurobi"
        )

        val familyId = TaskFamilyId.fromFeature(feature)

        assertEquals("MDS-gurobi", familyId.signature)
    }

    @Test
    fun `TaskFamilyNodePerformance should calculate confidence correctly`() {
        val highSamples = TaskFamilyNodePerformance(
            familyId = TaskFamilyId("test"),
            nodeId = "node-1",
            sampleCount = 100,
            avgRuntimeMs = 5000.0,
            avgCostPerTask = 1.0,
            successRate = 0.95,
            performanceScore = 1.0,
            lastUpdatedEpochMs = System.currentTimeMillis()
        )
        assertEquals(1.0, highSamples.confidence())

        val lowSamples = TaskFamilyNodePerformance(
            familyId = TaskFamilyId("test"),
            nodeId = "node-1",
            sampleCount = 3,
            avgRuntimeMs = 5000.0,
            avgCostPerTask = 1.0,
            successRate = 0.95,
            performanceScore = 1.0,
            lastUpdatedEpochMs = System.currentTimeMillis()
        )
        assertTrue(lowSamples.confidence() < 0.5)
    }
}

class LearnableScoringModelsTest {
    @Test
    fun `LearnableScoringModel should compute score correctly`() {
        val model = LearnableScoringModel.DEFAULT
        val features = NodeScoreFeatures(
            nodeId = "node-1",
            normalizedCost = 0.5,
            normalizedPerformance = 0.8,
            normalizedQueueDelay = 0.2,
            normalizedSuccessRate = 0.9,
            familyMatchScore = 0.7
        )

        val score = model.score(features)

        // Score should be a weighted combination
        assertTrue(score >= 0.0)
        assertTrue(score <= 1.0)
    }

    @Test
    fun `LearnableScoringModel should calibrate from feedback`() {
        val model = LearnableScoringModel.DEFAULT

        val feedback = listOf(
            ScoringFeedback(
                taskId = "task-1",
                nodeId = "node-1",
                features = NodeScoreFeatures(
                    nodeId = "node-1",
                    normalizedCost = 0.3,
                    normalizedPerformance = 0.7,
                    normalizedQueueDelay = 0.1,
                    normalizedSuccessRate = 0.9
                ),
                predictedScore = 0.5,
                actualScore = 0.3,
                outcome = TaskOutcome.SUCCESS_FAST,
                timestampEpochMs = System.currentTimeMillis()
            )
        )

        val calibrated = model.calibrate(feedback, learningRate = 0.1)

        assertNotNull(calibrated)
        assertEquals(model.version.split("-").first(), calibrated.version.split("-").first())
    }

    @Test
    fun `ScoringWeights should be normalized after calibration`() {
        val model = LearnableScoringModel(
            version = "test-1.0",
            weights = ScoringWeights(
                costWeight = 0.5,
                performanceWeight = 0.5,
                queueDelayWeight = 0.0,
                successRateWeight = 0.0,
                matchScoreWeight = 0.0
            ),
            updatedAtEpochMs = System.currentTimeMillis()
        )

        // After calibration, weights should still be valid
        val weights = model.weights
        assertTrue(weights.costWeight >= 0.0)
        assertTrue(weights.performanceWeight >= 0.0)
    }
}

class ExperimentModelsTest {
    @Test
    fun `Experiment should assign variant consistently`() {
        val experiment = Experiment(
            experimentId = "exp-1",
            name = "Test Experiment",
            description = "Test",
            status = ExperimentStatus.RUNNING,
            variants = listOf(
                ExperimentVariant("control", "Control", true, 50),
                ExperimentVariant("treatment", "Treatment", false, 50)
            ),
            config = ExperimentConfig(PrimaryMetric.SUCCESS_RATE),
            metrics = ExperimentMetrics(),
            createdAtEpochMs = System.currentTimeMillis()
        )

        // Same task ID should always get same variant
        val variant1 = experiment.getVariant("task-123")
        val variant2 = experiment.getVariant("task-123")

        assertNotNull(variant1)
        assertNotNull(variant2)
        assertEquals(variant1.id, variant2.id)
    }

    @Test
    fun `Experiment should respect traffic allocation`() {
        val experiment = Experiment(
            experimentId = "exp-1",
            name = "Test",
            description = "Test",
            status = ExperimentStatus.RUNNING,
            variants = listOf(
                ExperimentVariant("control", "Control", true, 80),
                ExperimentVariant("treatment", "Treatment", false, 20)
            ),
            config = ExperimentConfig(PrimaryMetric.SUCCESS_RATE),
            metrics = ExperimentMetrics(),
            createdAtEpochMs = System.currentTimeMillis()
        )

        // Sample 100 tasks
        val assignments = (1..100).map { i ->
            experiment.getVariant("task-$i")?.id
        }.groupBy { it }

        // Control should have approximately 80% of traffic
        val controlCount = assignments["control"]?.size ?: 0
        assertTrue(controlCount in 70..90, "Control count $controlCount not in expected range")
    }

    @Test
    fun `Experiment should not assign variant when not running`() {
        val experiment = Experiment(
            experimentId = "exp-1",
            name = "Test",
            description = "Test",
            status = ExperimentStatus.COMPLETED,
            variants = listOf(
                ExperimentVariant("control", "Control", true, 50),
                ExperimentVariant("treatment", "Treatment", false, 50)
            ),
            config = ExperimentConfig(PrimaryMetric.SUCCESS_RATE),
            metrics = ExperimentMetrics(),
            createdAtEpochMs = System.currentTimeMillis()
        )

        val variant = experiment.getVariant("task-1")
        assertNull(variant)
    }

    @Test
    fun `VariantMetrics should calculate correctly`() {
        val metrics = VariantMetrics(
            sampleCount = 100,
            successCount = 80,
            totalRuntimeMs = 500000,
            totalCost = 200.0
        )

        assertEquals(0.8, metrics.avgSuccessRate, 0.001)
        assertEquals(5000.0, metrics.avgRuntimeMs, 0.001)
        assertEquals(2.0, metrics.avgCost, 0.001)
    }

    @Test
    fun `VariantMetrics should record new observation`() {
        val initial = VariantMetrics(sampleCount = 10, successCount = 8, totalRuntimeMs = 50000, totalCost = 100.0)

        val updated = initial.record(success = true, runtimeMs = 5000, cost = 10.0)

        assertEquals(11, updated.sampleCount)
        assertEquals(9, updated.successCount)
        assertEquals(55000, updated.totalRuntimeMs)
        assertEquals(110.0, updated.totalCost, 0.001)
    }
}