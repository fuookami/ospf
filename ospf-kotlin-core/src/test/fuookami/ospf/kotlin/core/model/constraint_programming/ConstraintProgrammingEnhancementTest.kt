package fuookami.ospf.kotlin.core.model.constraint_programming

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingCheckpointCodec
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingCheckpointSupport
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingCheckpointSupportEvaluator
import fuookami.ospf.kotlin.core.solver.constraint_programming.FakeConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolveOptions
import fuookami.ospf.kotlin.core.solver.constraint_programming.PortableConstraintProgrammingConflict
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolution
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingUnknownOutput
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.solver.report.ObjectiveId
import fuookami.ospf.kotlin.core.solver.report.SolveHandle
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.IntVar

class ConstraintProgrammingEnhancementTest {
    @Test
    fun automatonFactoryRejectsDuplicateStateAndValueTransitions() {
        val result = ConstraintProgrammingConstraint.automaton(
            expressions = listOf(ConstraintProgrammingExpression.Constant(Int64.zero)),
            initialState = 0,
            finalStates = setOf(1),
            transitions = listOf(
                ConstraintProgrammingConstraint.AutomatonTransition(0, Int64.zero, 1),
                ConstraintProgrammingConstraint.AutomatonTransition(0, Int64.zero, 2)
            )
        )
        assertTrue(result.failed)
    }

    @Test
    fun circuitAutomatonAndReservoirEvaluateExactly() {
        val successors = listOf(IntVar("next-0"), IntVar("next-1"), IntVar("next-2"))
        val circuit = ConstraintProgrammingConstraint.circuit(
            successors.map { ConstraintProgrammingExpression.Variable(it, IntegerDomain.interval(0, 2).value!!) }
        ).value!!
        val circuitValues = mapOf(
            id(successors[0]) to Int64(1),
            id(successors[1]) to Int64(2),
            id(successors[2]) to Int64(0)
        )
        assertEquals(true, circuit.isSatisfied(circuitValues).value)
        assertEquals(false, circuit.isSatisfied(circuitValues + (id(successors[2]) to Int64(1))).value)

        val automaton = ConstraintProgrammingConstraint.automaton(
            expressions = listOf(ConstraintProgrammingExpression.Constant(Int64(0)), ConstraintProgrammingExpression.Constant(Int64(1))),
            initialState = 0,
            finalStates = setOf(2),
            transitions = listOf(
                ConstraintProgrammingConstraint.AutomatonTransition(0, Int64(0), 1),
                ConstraintProgrammingConstraint.AutomatonTransition(1, Int64(1), 2)
            )
        ).value!!
        assertEquals(true, automaton.isSatisfied(emptyMap()).value)

        val reservoir = ConstraintProgrammingConstraint.reservoir(
            events = listOf(
                ConstraintProgrammingConstraint.Reservoir.Event(
                    ConstraintProgrammingExpression.Constant(Int64(1)),
                    ConstraintProgrammingExpression.Constant(Int64(2))
                ),
                ConstraintProgrammingConstraint.Reservoir.Event(
                    ConstraintProgrammingExpression.Constant(Int64(2)),
                    ConstraintProgrammingExpression.Constant(Int64(-1))
                )
            ),
            initialLevel = Int64(0),
            minimumLevel = Int64(0),
            maximumLevel = Int64(2)
        ).value!!
        assertEquals(true, reservoir.isSatisfied(emptyMap()).value)
    }

    @Test
    fun snapshotCodecRoundTripsStableIds() {
        val variable = BinVar("serialized-x")
        val model = ConstraintProgrammingModel("serialized")
        model.registerVariable(variable)
        val expression = ConstraintProgrammingExpression.Variable(variable)
        model.addConstraint(ConstraintProgrammingConstraint.equal(expression, Int64.one).value!!, id = "x-one")
        val snapshot = model.snapshot().value!!
        val encoded = ConstraintProgrammingSnapshotCodec.encode(snapshot)
        assertTrue(encoded is Ok)
        val decoded = ConstraintProgrammingSnapshotCodec.decode(
            encoded.value!!,
            mapOf(id(variable) to variable)
        )
        assertTrue(decoded is Ok)
        assertEquals(snapshot.name, decoded.value!!.name)
        assertEquals(snapshot.variables.map { it.id }, decoded.value!!.variables.map { it.id })
        assertEquals(snapshot.constraints.map { it.id }, decoded.value!!.constraints.map { it.id })
    }

    @Test
    fun stableIdentitySurvivesRebuildRegistrationOrderDuplicateNamesAndCodecRoundTrip() {
        val first = buildStableIdentityModel(reverseRegistration = false)
        val rebuilt = buildStableIdentityModel(reverseRegistration = true)
        try {
            val firstSnapshot = first.model.snapshot().value!!
            val rebuiltSnapshot = rebuilt.model.snapshot().value!!
            assertEquals(identityManifest(firstSnapshot), identityManifest(rebuiltSnapshot))
            assertEquals(
                firstSnapshot.variables.map { it.id }.toSet(),
                rebuiltSnapshot.variables.map { it.id }.toSet()
            )
            assertEquals(
                firstSnapshot.constraints.map { it.id }.toSet(),
                rebuiltSnapshot.constraints.map { it.id }.toSet()
            )
            assertEquals(
                firstSnapshot.objectives.map { it.id }.toSet(),
                rebuiltSnapshot.objectives.map { it.id }.toSet()
            )

            val encoded = ConstraintProgrammingSnapshotCodec.encode(firstSnapshot).value!!
            val decoded = ConstraintProgrammingSnapshotCodec.decode(encoded, rebuilt.variables)
            assertTrue(decoded.ok)
            assertEquals(identityManifest(firstSnapshot), identityManifest(decoded.value!!))
        } finally {
            first.model.close()
            rebuilt.model.close()
        }
    }

    @Test
    fun cancelledSolveCheckpointWithoutIncumbentRestoresAcrossRebuildAndRejectsChangedOrigin() = kotlinx.coroutines.runBlocking {
        val model = buildStableIdentityModel(reverseRegistration = false)
        val rebuilt = buildStableIdentityModel(reverseRegistration = true)
        val changedOrigin = buildStableIdentityModel(reverseRegistration = true, originPrefix = "changed")
        try {
            val handle = SolveHandle.create()
            assertTrue(handle.cancel().ok)
            val result = FakeConstraintProgrammingSolver().solve(
                model.model,
                ConstraintProgrammingSolveOptions(cancellationToken = handle.token)
            )
            val unknown = assertIs<ConstraintProgrammingUnknownOutput>(assertIs<Ok<*, *, *>>(result).value)
            assertEquals(fuookami.ospf.kotlin.core.solver.report.TerminationReason.Cancelled, unknown.terminationReason)

            val captured = ConstraintProgrammingCheckpointCodec.capture(
                snapshot = model.model.snapshot().value!!,
                descriptor = FakeConstraintProgrammingSolver().descriptor,
                checkpointId = "cancelled-without-incumbent",
                createdAtEpochMs = 1L
            )
            assertTrue(captured.ok)
            assertEquals(null, captured.value!!.incumbent)
            val encoded = ConstraintProgrammingCheckpointCodec.encode(captured.value!!)
            assertTrue(encoded.ok)
            val decoded = ConstraintProgrammingCheckpointCodec.decodeCompatible(encoded.value!!)
            assertTrue(decoded.ok)
            assertTrue(
                ConstraintProgrammingCheckpointCodec.restore(
                    decoded.value!!,
                    rebuilt.model.snapshot().value!!,
                    expectedConfigurationFingerprint = decoded.value!!.configurationFingerprint,
                    expectedSolverFingerprint = decoded.value!!.solverFingerprint,
                    allowLegacyConfigurationFingerprint = true
                ).ok
            )
            assertTrue(
                ConstraintProgrammingCheckpointCodec.restore(
                    decoded.value!!,
                    changedOrigin.model.snapshot().value!!,
                    expectedConfigurationFingerprint = null,
                    expectedSolverFingerprint = null
                ).failed
            )
        } finally {
            model.model.close()
            rebuilt.model.close()
            changedOrigin.model.close()
        }
    }

    @Test
    fun snapshotCodecRejectsDuplicateAutomatonTransitions() {
        val model = ConstraintProgrammingModel("duplicate-automaton")
        val automaton = ConstraintProgrammingConstraint.automaton(
            expressions = listOf(ConstraintProgrammingExpression.Constant(Int64.zero)),
            initialState = 0,
            finalStates = setOf(1),
            transitions = listOf(
                ConstraintProgrammingConstraint.AutomatonTransition(0, Int64.zero, 1)
            )
        ).value!!
        model.addConstraint(automaton, id = "automaton")
        val snapshot = model.snapshot().value!!
        val encoded = ConstraintProgrammingSnapshotCodec.encode(snapshot).value!!
        val root = Json.parseToJsonElement(encoded).jsonObject
        val constraints = root["constraints"]!!.jsonArray
        val entry = constraints.first().jsonObject
        val constraint = entry["constraint"]!!.jsonObject
        val transitions = constraint["transitions"]!!.jsonArray
        val duplicate = transitions.first()
        val modifiedConstraint = buildJsonObject {
            constraint.forEach { (key, value) -> put(key, value) }
            put("transitions", JsonArray(transitions + duplicate))
        }
        val modifiedEntry = buildJsonObject {
            entry.forEach { (key, value) ->
                if (key == "constraint") {
                    put(key, modifiedConstraint)
                } else {
                    put(key, value)
                }
            }
        }
        val modifiedRoot = buildJsonObject {
            root.forEach { (key, value) ->
                if (key == "constraints") {
                    put(key, JsonArray(listOf(modifiedEntry)))
                } else {
                    put(key, value)
                }
            }
        }
        val decoded = ConstraintProgrammingSnapshotCodec.decode(modifiedRoot.toString(), emptyMap())
        assertTrue(decoded.failed)
        model.close()
    }

    @Test
    fun checkpointAssessmentUsesPortableSnapshotFallback() {
        val descriptor = FakeConstraintProgrammingSolver().descriptor
        val assessment = ConstraintProgrammingCheckpointSupportEvaluator.assess(descriptor)
        assertEquals(ConstraintProgrammingCheckpointSupport.RebuildFromSnapshot, assessment.support)
        val model = ConstraintProgrammingModel("checkpoint")
        val variable = BinVar("checkpoint-x")
        model.registerVariable(variable)
        val checkpoint = ConstraintProgrammingCheckpointSupportEvaluator.capture(
            model.snapshot().value!!,
            descriptor
        )
        assertTrue(checkpoint is Ok)
        assertEquals("checkpoint", checkpoint.value!!.modelName)
    }

    @Test
    fun checkpointV2RoundTripsVerifiedIncumbentAndStableIdentity() {
        val model = ConstraintProgrammingModel("checkpoint-v2")
        val variable = IntVar("checkpoint-v2-x")
        val variableId = VariableId("business:x")
        model.registerVariable(
            id = variableId,
            variable = variable,
            domain = IntegerDomain.interval(0, 3).value!!,
            scope = "stable",
            origin = "fixture/business-x"
        )
        val expression = ConstraintProgrammingExpression.Variable(variable)
        model.addConstraint(
            constraint = ConstraintProgrammingConstraint.greaterOrEqual(expression, Int64.zero).value!!,
            id = "business:non-negative",
            scope = "stable",
            origin = "fixture/non-negative"
        )
        model.minimize(
            expression = expression,
            id = ObjectiveId("business:objective"),
            scope = "stable",
            origin = "fixture/objective"
        )
        val snapshot = model.snapshot().value!!
        val incumbent = ConstraintProgrammingSolution(values = mapOf(variableId to Int64.one))
        val captured = ConstraintProgrammingCheckpointCodec.capture(
            snapshot = snapshot,
            descriptor = FakeConstraintProgrammingSolver().descriptor,
            checkpointId = "checkpoint-v2",
            createdAtEpochMs = 1L,
            incumbent = incumbent,
            configurationFingerprint = "configuration-fixture"
        )
        assertTrue(captured.ok)
        val encoded = ConstraintProgrammingCheckpointCodec.encode(captured.value!!)
        assertTrue(encoded.ok)
        val decoded = ConstraintProgrammingCheckpointCodec.decodeCompatible(encoded.value!!)
        assertTrue(decoded.ok)
            val restored = ConstraintProgrammingCheckpointCodec.restore(
                decoded.value!!,
                snapshot,
                expectedConfigurationFingerprint = "configuration-fixture",
                expectedSolverFingerprint = decoded.value!!.solverFingerprint
            )
        assertTrue(restored.ok)
        assertEquals(Int64.one, restored.value!!.incumbent!!.values[variableId])
        assertEquals(emptyList(), restored.value!!.assumptions)
        assertEquals(emptyList(), restored.value!!.conflicts)
        assertEquals("stable", snapshot.variables.single().scope)
        assertEquals("fixture/business-x", snapshot.variables.single().origin)
    }

    @Test
    fun checkpointRestoreIsIdempotentAcrossRepeatedDecodeAndRestore() {
        val model = ConstraintProgrammingModel("checkpoint-idempotent")
        val variable = IntVar("checkpoint-idempotent-x")
        val variableId = VariableId("checkpoint-idempotent:x")
        model.registerVariable(
            id = variableId,
            variable = variable,
            domain = IntegerDomain.interval(0, 2).value!!
        )
        val snapshot = model.snapshot().value!!
        val incumbent = ConstraintProgrammingSolution(values = mapOf(variableId to Int64.one))
        val captured = ConstraintProgrammingCheckpointCodec.capture(
            snapshot = snapshot,
            descriptor = FakeConstraintProgrammingSolver().descriptor,
            checkpointId = "checkpoint-idempotent",
            createdAtEpochMs = 7L,
            incumbent = incumbent,
            parentCheckpointId = "checkpoint-parent",
            configurationFingerprint = "configuration-fixture"
        )
        assertTrue(captured.ok)
        val encoded = ConstraintProgrammingCheckpointCodec.encode(captured.value!!)
        assertTrue(encoded.ok)

        val firstDecoded = ConstraintProgrammingCheckpointCodec.decodeCompatible(encoded.value!!)
        val secondDecoded = ConstraintProgrammingCheckpointCodec.decodeCompatible(encoded.value!!)
        assertTrue(firstDecoded.ok)
        assertTrue(secondDecoded.ok)
        val firstRestore = ConstraintProgrammingCheckpointCodec.restore(
            firstDecoded.value!!,
            snapshot,
            expectedConfigurationFingerprint = "configuration-fixture",
            expectedSolverFingerprint = firstDecoded.value!!.solverFingerprint
        )
        val secondRestore = ConstraintProgrammingCheckpointCodec.restore(
            secondDecoded.value!!,
            snapshot,
            expectedConfigurationFingerprint = "configuration-fixture",
            expectedSolverFingerprint = secondDecoded.value!!.solverFingerprint
        )
        assertTrue(firstRestore.ok)
        assertTrue(secondRestore.ok)
        assertEquals(firstRestore.value, secondRestore.value)
        assertEquals("checkpoint-parent", firstRestore.value!!.envelope.parentCheckpointId)
        assertEquals(firstRestore.value!!.incumbent, secondRestore.value!!.incumbent)
    }

    @Test
    fun checkpointRestoreRejectsUnknownAssumptionsAndConflictMembers() {
        val model = ConstraintProgrammingModel("checkpoint-evidence")
        val variable = IntVar("checkpoint-evidence-x")
        val variableId = VariableId("evidence:x")
        model.registerVariable(
            id = variableId,
            variable = variable,
            domain = IntegerDomain.interval(0, 1).value!!
        )
        val expression = ConstraintProgrammingExpression.Variable(variable)
        val constraintId = ConstraintId("evidence:constraint")
        model.addConstraint(
            constraint = ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!,
            id = constraintId
        )
        val snapshot = model.snapshot().value!!
        val captured = ConstraintProgrammingCheckpointCodec.capture(
            snapshot = snapshot,
            descriptor = FakeConstraintProgrammingSolver().descriptor,
            checkpointId = "checkpoint-evidence",
            createdAtEpochMs = 1L
        ).value!!

        val unknownAssumption = ConstraintProgrammingCheckpointCodec.restore(
            captured.copy(assumptions = listOf("missing-variable")),
            snapshot,
            expectedConfigurationFingerprint = null,
            expectedSolverFingerprint = null
        )
        assertTrue(unknownAssumption.failed)

        val unknownConflictMember = ConstraintProgrammingCheckpointCodec.restore(
            captured.copy(
                conflicts = listOf(
                    PortableConstraintProgrammingConflict(
                        validity = "Verified",
                        minimality = "Irreducible",
                        memberIds = listOf("constraint:missing-constraint")
                    )
                )
            ),
            snapshot,
            expectedConfigurationFingerprint = null,
            expectedSolverFingerprint = null
        )
        assertTrue(unknownConflictMember.failed)
        model.close()
    }

    @Test
    fun checkpointCapturePersistsAndRevalidatesEvidenceState() {
        val model = ConstraintProgrammingModel("checkpoint-capture-evidence")
        val variable = BinVar("checkpoint-capture-x")
        model.registerVariable(variable, IntegerDomain.boolean)
        val variableId = id(variable)
        val expression = ConstraintProgrammingExpression.Variable(variable)
        val constraintId = ConstraintId("checkpoint-capture-constraint")
        model.addConstraint(
            constraint = ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!,
            id = constraintId
        )
        val snapshot = model.snapshot().value!!
        val captured = ConstraintProgrammingCheckpointCodec.capture(
            snapshot = snapshot,
            descriptor = FakeConstraintProgrammingSolver().descriptor,
            checkpointId = "checkpoint-capture-evidence",
            createdAtEpochMs = 1L,
            assumptions = listOf(variableId.value),
            configurationFingerprint = "fixture-config",
            conflicts = listOf(
                PortableConstraintProgrammingConflict(
                    validity = "Verified",
                    minimality = "Irreducible",
                    memberIds = listOf("constraint:${constraintId.value}", "bound:${variableId.value}:Lower")
                )
            ),
            bestBound = "0",
            gap = "0"
        )
        assertTrue(captured.ok)
        assertEquals(listOf(variableId.value), captured.value!!.assumptions)
        assertEquals("0", captured.value!!.bestBound)
        val restored = ConstraintProgrammingCheckpointCodec.restore(
            envelope = captured.value!!,
            snapshot = snapshot,
            expectedConfigurationFingerprint = "fixture-config",
            expectedSolverFingerprint = captured.value!!.solverFingerprint
        )
        assertTrue(restored.ok)
        assertEquals(listOf(variableId.value), restored.value!!.assumptions)
        assertEquals(1, restored.value!!.conflicts.size)
        model.close()
    }

    @Test
    fun checkpointRestoreRejectsVerifiedConflictWithoutMembers() {
        val model = ConstraintProgrammingModel("checkpoint-empty-conflict")
        val variable = BinVar("checkpoint-empty-conflict-x")
        model.registerVariable(variable)
        val snapshot = model.snapshot().value!!
        val captured = ConstraintProgrammingCheckpointCodec.capture(
            snapshot = snapshot,
            descriptor = FakeConstraintProgrammingSolver().descriptor,
            checkpointId = "checkpoint-empty-conflict",
            createdAtEpochMs = 1L
        ).value!!
        val invalid = captured.copy(
            conflicts = listOf(
                PortableConstraintProgrammingConflict(
                    validity = "Verified",
                    minimality = "Irreducible"
                )
            )
        )
        val encoded = ConstraintProgrammingCheckpointCodec.encode(invalid).value!!
        val decoded = ConstraintProgrammingCheckpointCodec.decode(encoded).value!!
        assertTrue(
            ConstraintProgrammingCheckpointCodec.restore(
                decoded,
                snapshot,
                expectedConfigurationFingerprint = null,
                expectedSolverFingerprint = null
            ).failed
        )
        model.close()
    }

    @Test
    fun checkpointRestoreRejectsV2LegacySpoofAndEmbeddedSnapshotMismatch() {
        val model = ConstraintProgrammingModel("checkpoint-spoof")
        val variable = BinVar("checkpoint-spoof-x")
        model.registerVariable(variable)
        val snapshot = model.snapshot().value!!
        val captured = ConstraintProgrammingCheckpointCodec.capture(
            snapshot = snapshot,
            descriptor = FakeConstraintProgrammingSolver().descriptor,
            checkpointId = "checkpoint-spoof",
            createdAtEpochMs = 1L
        ).value!!
        assertTrue(
            ConstraintProgrammingCheckpointCodec.restore(
                captured.copy(checkpointId = "legacy-v1"),
                snapshot,
                expectedConfigurationFingerprint = null,
                expectedSolverFingerprint = null
            ).failed
        )
        assertTrue(
            ConstraintProgrammingCheckpointCodec.restore(
                captured.copy(snapshotJson = "{}"),
                snapshot,
                expectedConfigurationFingerprint = null,
                expectedSolverFingerprint = null
            ).failed
        )
        model.close()
    }

    @Test
    fun checkpointV2ReadsLegacySnapshotOnlyEnvelope() {
        val model = ConstraintProgrammingModel("legacy-checkpoint")
        val variable = BinVar("legacy-x")
        model.registerVariable(variable)
        val snapshotJson = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
        val legacy = buildJsonObject {
            put("schema", 1)
            put("modelName", "legacy-checkpoint")
            put("solverId", "legacy-solver")
            put("snapshotJson", snapshotJson)
        }.toString()
        val decoded = ConstraintProgrammingCheckpointCodec.decodeCompatible(legacy)
        assertTrue(decoded.ok)
        assertEquals("legacy-checkpoint", decoded.value!!.modelName)
        assertEquals("1.0", decoded.value!!.identitySchemaVersion)
        assertEquals(model.snapshot().value!!.identityNamespace, decoded.value!!.identityNamespace)
        assertEquals(null, decoded.value!!.solverFingerprint)
    }

    @Test
    fun malformedV2CheckpointWithLegacySchemaMarkerCannotDowngrade() {
        val model = ConstraintProgrammingModel("malformed-v2")
        model.registerVariable(BinVar("malformed-v2-x"))
        val snapshot = model.snapshot().value!!
        val captured = ConstraintProgrammingCheckpointCodec.capture(
            snapshot = snapshot,
            descriptor = FakeConstraintProgrammingSolver().descriptor,
            checkpointId = "malformed-v2",
            createdAtEpochMs = 1L
        ).value!!
        val encoded = ConstraintProgrammingCheckpointCodec.encode(captured).value!!
        val malformed = buildJsonObject {
            Json.parseToJsonElement(encoded).jsonObject.forEach { (key, value) -> put(key, value) }
            put("schema", 1)
            put("integritySha256", "broken")
        }.toString()

        assertTrue(ConstraintProgrammingCheckpointCodec.decodeCompatible(malformed).failed)
        model.close()
    }

    @Test
    fun legacyCheckpointMigrationCanBeReadAgainAsV2() {
        val model = ConstraintProgrammingModel("legacy-migration")
        model.registerVariable(BinVar("legacy-migration-x"))
        val snapshotJson = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
        val legacy = buildJsonObject {
            put("schema", 1)
            put("modelName", "legacy-migration")
            put("snapshotJson", snapshotJson)
        }.toString()

        val decodedLegacy = ConstraintProgrammingCheckpointCodec.decodeCompatible(legacy)
        assertTrue(decodedLegacy.ok)
        val migrated = ConstraintProgrammingCheckpointCodec.encode(decodedLegacy.value!!)
        assertTrue(migrated.ok)
        val decodedMigrated = ConstraintProgrammingCheckpointCodec.decodeCompatible(migrated.value!!)
        assertTrue(decodedMigrated.ok)
        assertEquals("v2", decodedMigrated.value!!.sourceFormat)
        assertEquals("legacy-v1-migrated", decodedMigrated.value!!.checkpointId)
        assertTrue(decodedMigrated.value!!.migratedFromLegacy)
        assertTrue(
            ConstraintProgrammingCheckpointCodec.restore(
                envelope = decodedMigrated.value!!,
                snapshot = model.snapshot().value!!,
                expectedConfigurationFingerprint = null,
                expectedSolverFingerprint = null
            ).ok
        )
        model.close()
    }

    private fun id(variable: BinVar): VariableId {
        return VariableId("${variable.identifier}:${variable.index}")
    }

    private fun id(variable: IntVar): VariableId {
        return VariableId("${variable.identifier}:${variable.index}")
    }

    private data class StableIdentityFixture(
        val model: ConstraintProgrammingModel,
        val variables: Map<VariableId, IntVar>
    )

    private fun buildStableIdentityModel(
        reverseRegistration: Boolean,
        originPrefix: String = "fixture"
    ): StableIdentityFixture {
        val model = ConstraintProgrammingModel("stable-identity")
        val x = IntVar("duplicate-display")
        val y = IntVar("duplicate-display")
        val definitions = listOf(
            VariableId("stable:x") to x,
            VariableId("stable:y") to y
        )
        val ordered = if (reverseRegistration) definitions.asReversed() else definitions
        ordered.forEach { (variableId, variable) ->
            model.registerVariable(
                id = variableId,
                variable = variable,
                domain = IntegerDomain.interval(0, 1).value!!,
                scope = "stable",
                origin = "$originPrefix/${variableId.value}"
            )
        }
        val xExpression = ConstraintProgrammingExpression.Variable(x)
        val yExpression = ConstraintProgrammingExpression.Variable(y)
        val constraints = listOf(
            ConstraintId("stable:x-lower") to ConstraintProgrammingConstraint.greaterOrEqual(xExpression, Int64.zero).value!!,
            ConstraintId("stable:y-lower") to ConstraintProgrammingConstraint.greaterOrEqual(yExpression, Int64.zero).value!!
        )
        val orderedConstraints = if (reverseRegistration) constraints.asReversed() else constraints
        orderedConstraints.forEach { (constraintId, constraint) ->
            model.addConstraint(
                constraint = constraint,
                id = constraintId,
                name = "duplicate-display",
                scope = "stable",
                origin = "$originPrefix/${constraintId.value}"
            )
        }
        model.minimize(
            expression = xExpression,
            id = ObjectiveId("stable:objective"),
            name = "duplicate-display",
            scope = "stable",
            origin = "$originPrefix/stable:objective"
        )
        return StableIdentityFixture(model, definitions.toMap())
    }

    private fun identityManifest(snapshot: ConstraintProgrammingModelSnapshot): List<String> {
        return buildList {
            snapshot.variables.forEach {
                add("variable|${it.id}|${it.name}|${it.scope}|${it.origin}")
            }
            snapshot.constraints.forEach {
                add("constraint|${it.id}|${it.name}|${it.scope}|${it.origin}")
            }
            snapshot.objectives.forEach {
                add("objective|${it.id}|${it.name}|${it.category}|${it.scope}|${it.origin}")
            }
        }.sorted()
    }
}
