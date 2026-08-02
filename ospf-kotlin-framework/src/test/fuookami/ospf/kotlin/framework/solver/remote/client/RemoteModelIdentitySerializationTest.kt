package fuookami.ospf.kotlin.framework.solver.remote.client

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import fuookami.ospf.kotlin.framework.solver.remote.domain.SerializedLinearModel

/**
 * 线性/二次模型身份 DTO 回归测试。 / Linear/quadratic model identity DTO regression tests.
 */
class RemoteModelIdentitySerializationTest {
    private val json = Json {
        ignoreUnknownKeys = false
    }

    /**
     * 验证跨仓库模型 fixture 保留稳定身份元数据。 /
     * Verify that the cross-repository model fixture preserves stable identity metadata.
     */
    @Test
    fun canonicalLinearModelFixturePreservesIdentityMetadata() {
        val fixture = checkNotNull(javaClass.getResource("/fixtures/remote-linear-model-v2.json"))
            .readText()
        val model = json.decodeFromString(SerializedLinearModel.serializer(), fixture)

        assertEquals("fixture-model", model.identityNamespace)
        assertEquals("1.0", model.identitySchemaVersion)
        assertEquals("variable:x", model.variables.single().identityId)
        assertEquals("STABLE", model.variables.single().identityScope)
        assertEquals("demand", model.variables.single().identityOriginKind)
        assertEquals("constraint:capacity", model.constraints.single().identityId)
        assertEquals("capacity", model.constraints.single().identityOriginKind)
        assertEquals("objective:cost", model.objective.identityId)
        assertEquals("total", model.objective.identityOriginKey)

        val roundTripped = json.decodeFromString(
            SerializedLinearModel.serializer(),
            json.encodeToString(SerializedLinearModel.serializer(), model)
        )
        assertEquals(model, roundTripped)
    }
}
