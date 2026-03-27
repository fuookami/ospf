package fuookami.ospf.framework.remote_solver.protocol.domain

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ModelDataEqualsTest {

    @Test
    fun bothRawBytesNullShouldBeEqual() {
        val model1 = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = null
        )
        val model2 = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = null
        )

        assertTrue(model1 == model2, "Both rawBytes null should be equal")
    }

    @Test
    fun oneRawBytesNullOtherNonNullShouldNotBeEqual() {
        val modelWithNull = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = null
        )
        val modelWithData = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = byteArrayOf(1, 2, 3)
        )

        assertFalse(modelWithNull == modelWithData, "null rawBytes should not equal non-null rawBytes")
        assertFalse(modelWithData == modelWithNull, "non-null rawBytes should not equal null rawBytes")
    }

    @Test
    fun bothRawBytesSameContentShouldBeEqual() {
        val model1 = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = byteArrayOf(1, 2, 3)
        )
        val model2 = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = byteArrayOf(1, 2, 3)
        )

        assertTrue(model1 == model2, "Same rawBytes content should be equal")
    }

    @Test
    fun bothRawBytesDifferentContentShouldNotBeEqual() {
        val model1 = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = byteArrayOf(1, 2, 3)
        )
        val model2 = ModelData(
            ref = ObjectRef.of(path = "test/model"),
            rawBytes = byteArrayOf(1, 2, 4)
        )

        assertFalse(model1 == model2, "Different rawBytes content should not be equal")
    }

    @Test
    fun allFieldsNullShouldBeEqual() {
        val model1 = ModelData()
        val model2 = ModelData()

        assertTrue(model1 == model2, "All fields null should be equal")
    }
}
