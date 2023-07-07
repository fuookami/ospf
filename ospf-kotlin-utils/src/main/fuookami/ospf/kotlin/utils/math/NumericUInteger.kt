package fuookami.ospf.kotlin.utils.math

import kotlinx.serialization.*
import kotlinx.serialization.descriptors.*
import kotlinx.serialization.encoding.*
import fuookami.ospf.kotlin.utils.operator.*

interface NumericUInteger<Self, I>
    : NumericUIntegerNumber<Self, I> where Self : NumericUInteger<Self, I>, I : UIntegerNumber<I>, I : NumberField<I> {

    override fun inc() = this + constants.one

    override fun lg() = log(Flt64(10.0))
    override fun ln() = log(Flt64.e)

    override fun square() = pow(2)
    override fun cubic() = pow(3)

    override fun sqr() = pow(Flt64(1.0 / 2.0))
    override fun cbr() = pow(Flt64(1.0 / 3.0))
}

abstract class NumericUIntegerConstants<Self, I>(
    private val ctor: (I) -> Self,
    private val constants: RealNumberConstants<I>
) : RealNumberConstants<Self> where Self : NumericUInteger<Self, I>, I : UIntegerNumber<I>, I : NumberField<I> {

    override val zero: Self get() = ctor(constants.zero)
    override val one: Self get() = ctor(constants.one)
    override val two: Self get() = ctor(constants.two)
    override val three: Self get() = ctor(constants.three)
    override val ten: Self get() = ctor(constants.ten)
    override val minimum: Self get() = ctor(constants.minimum)
    override val maximum: Self get() = ctor(constants.maximum)
}

object NUInt8Serializer : RealNumberKSerializer<NUInt8> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("NUInt8", PrimitiveKind.INT)
    override val constants = NUInt8

    override fun serialize(encoder: Encoder, value: NUInt8) {
        encoder.encodeInt(value.value.value.toInt())
    }

    override fun deserialize(decoder: Decoder): NUInt8 {
        return NUInt8(UInt8(decoder.decodeInt().toUByte()))
    }
}

@JvmInline
@Serializable(with = NUInt8Serializer::class)
value class NUInt8(val value: UInt8) : NumericUInteger<NUInt8, UInt8> {
    companion object : NumericUIntegerConstants<NUInt8, UInt8>(NUInt8::invoke, UInt8) {
        operator fun invoke(value: UInt8) = NUInt8(value)
    }

    override val constants: RealNumberConstants<NUInt8> get() = NUInt8

    override fun copy(): NUInt8 = NUInt8(value)

    override fun toString() = value.toString()
    fun toString(radix: Int) = value.toString(radix)

    override fun dec(): NUInt8 = NUInt8(value - UInt8.one)

    override fun partialOrd(rhs: NUInt8) = orderOf(value.compareTo(rhs.value))
    override fun partialEq(rhs: NUInt8) = (value.compareTo(rhs.value) == 0)

    override fun reciprocal() = URtn8(UInt8.one, value)
    override fun unaryMinus() = NInt8(-value.toInt8())
    override fun abs() = NUInt8(value.abs())

    override fun plus(rhs: NUInt8) = NUInt8(value + rhs.value)
    override fun minus(rhs: NUInt8) = NInt8(value.toInt8() - rhs.toInt8())
    override fun times(rhs: NUInt8) = NUInt8(value * rhs.value)
    override fun div(rhs: NUInt8) = URtn8(value, rhs.value)
    override fun rem(rhs: NUInt8) = NUInt8(value % rhs.value)
    override fun intDiv(rhs: NUInt8) = NUInt8(value / rhs.value)

    @Throws(IllegalArgumentException::class)
    override fun log(base: FloatingNumber<*>): FloatingNumber<*>? = when (base) {
        is Flt32 -> toFlt32().log(base)
        is Flt64 -> toFlt64().log(base)
        is FltX -> toFltX().log(base)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt8.log: ${base.javaClass}")
    }

    override fun pow(index: Int): URtn8 {
        return if (index >= 1) {
            URtn8(value.pow(index), UInt8.one)
        } else if (index <= -1) {
            URtn8(UInt8.one, value.pow(index))
        } else {
            URtn8.one
        }
    }

    @Throws(IllegalArgumentException::class)
    override fun pow(index: FloatingNumber<*>): FloatingNumber<*> = when (index) {
        is Flt32 -> toFlt32().pow(index)
        is Flt64 -> toFlt64().pow(index)
        is FltX -> toFltX().pow(index)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt8.pow: ${index.javaClass}")
    }

    override fun exp() = toFlt64().exp()

    override fun rangeTo(rhs: NUInt8) = NumericUIntegerRange(copy(), rhs, one, UInt8, UInt8::toNUInt8, NUInt8::toUInt8)
    override infix fun until(rhs: NUInt8) = rangeTo((rhs - constants.one).toNUInt8())

    override fun toInt8() = value.toInt8()
    override fun toInt16() = value.toInt16()
    override fun toInt32() = value.toInt32()
    override fun toInt64() = value.toInt64()
    override fun toIntX() = value.toIntX()

    override fun toUInt8() = value.toUInt8()
    override fun toUInt16() = value.toUInt16()
    override fun toUInt32() = value.toUInt32()
    override fun toUInt64() = value.toUInt64()
    override fun toUIntX() = value.toUIntX()

    override fun toFlt32() = value.toFlt32()
    override fun toFlt64() = value.toFlt64()
    override fun toFltX() = value.toFltX()
}

object NUInt16Serializer : RealNumberKSerializer<NUInt16> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("NUInt16", PrimitiveKind.INT)
    override val constants = NUInt16

    override fun serialize(encoder: Encoder, value: NUInt16) {
        encoder.encodeInt(value.value.value.toInt())
    }

    override fun deserialize(decoder: Decoder): NUInt16 {
        return NUInt16(UInt16(decoder.decodeInt().toUShort()))
    }
}

@JvmInline
@Serializable(with = NUInt16Serializer::class)
value class NUInt16(val value: UInt16) : NumericUInteger<NUInt16, UInt16> {
    companion object : NumericUIntegerConstants<NUInt16, UInt16>(NUInt16::invoke, UInt16) {
        operator fun invoke(value: UInt16) = NUInt16(value)
    }

    override val constants: RealNumberConstants<NUInt16> get() = NUInt16

    override fun copy(): NUInt16 = NUInt16(value)

    override fun toString() = value.toString()
    fun toString(radix: Int) = value.toString(radix)

    override fun dec(): NUInt16 = NUInt16(value - UInt16.one)

    override fun partialOrd(rhs: NUInt16) = orderOf(value.compareTo(rhs.value))
    override fun partialEq(rhs: NUInt16) = (value.compareTo(rhs.value) == 0)

    override fun reciprocal() = URtn16(UInt16.one, value)
    override fun unaryMinus() = NInt16(-value.toInt16())
    override fun abs() = NUInt16(value.abs())

    override fun plus(rhs: NUInt16) = NUInt16(value + rhs.value)
    override fun minus(rhs: NUInt16) = NInt16(value.toInt16() - rhs.toInt16())
    override fun times(rhs: NUInt16) = NUInt16(value * rhs.value)
    override fun div(rhs: NUInt16) = URtn16(value, rhs.value)
    override fun rem(rhs: NUInt16) = NUInt16(value % rhs.value)
    override fun intDiv(rhs: NUInt16) = NUInt16(value / rhs.value)

    @Throws(IllegalArgumentException::class)
    override fun log(base: FloatingNumber<*>): FloatingNumber<*>? = when (base) {
        is Flt32 -> toFlt32().log(base)
        is Flt64 -> toFlt64().log(base)
        is FltX -> toFltX().log(base)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt16.log: ${base.javaClass}")
    }

    override fun pow(index: Int): URtn16 {
        return if (index >= 1) {
            URtn16(value.pow(index), UInt16.one)
        } else if (index <= -1) {
            URtn16(UInt16.one, value.pow(index))
        } else {
            URtn16.one
        }
    }

    @Throws(IllegalArgumentException::class)
    override fun pow(index: FloatingNumber<*>): FloatingNumber<*> = when (index) {
        is Flt32 -> toFlt32().pow(index)
        is Flt64 -> toFlt64().pow(index)
        is FltX -> toFltX().pow(index)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt16.pow: ${index.javaClass}")
    }

    override fun exp() = toFlt64().exp()

    override fun rangeTo(rhs: NUInt16) =
        NumericUIntegerRange(copy(), rhs, one, UInt16, UInt16::toNUInt16, NUInt16::toUInt16)

    override infix fun until(rhs: NUInt16) = rangeTo((rhs - constants.one).toNUInt16())

    override fun toInt8() = value.toInt8()
    override fun toInt16() = value.toInt16()
    override fun toInt32() = value.toInt32()
    override fun toInt64() = value.toInt64()
    override fun toIntX() = value.toIntX()

    override fun toUInt8() = value.toUInt8()
    override fun toUInt16() = value.toUInt16()
    override fun toUInt32() = value.toUInt32()
    override fun toUInt64() = value.toUInt64()
    override fun toUIntX() = value.toUIntX()

    override fun toFlt32() = value.toFlt32()
    override fun toFlt64() = value.toFlt64()
    override fun toFltX() = value.toFltX()
}

object NUInt32Serializer : RealNumberKSerializer<NUInt32> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("NUInt32", PrimitiveKind.INT)
    override val constants = NUInt32

    override fun serialize(encoder: Encoder, value: NUInt32) {
        encoder.encodeInt(value.value.value.toInt())
    }

    override fun deserialize(decoder: Decoder): NUInt32 {
        return NUInt32(UInt32(decoder.decodeInt().toUInt()))
    }
}

@JvmInline
@Serializable(with = NUInt32Serializer::class)
value class NUInt32(val value: UInt32) : NumericUInteger<NUInt32, UInt32> {
    companion object : NumericUIntegerConstants<NUInt32, UInt32>(NUInt32::invoke, UInt32) {
        operator fun invoke(value: UInt32) = NUInt32(value)
    }

    override val constants: RealNumberConstants<NUInt32> get() = NUInt32

    override fun copy(): NUInt32 = NUInt32(value)

    override fun toString() = value.toString()
    fun toString(radix: Int) = value.toString(radix)

    override fun dec(): NUInt32 = NUInt32(value - UInt32.one)

    override fun partialOrd(rhs: NUInt32) = orderOf(value.compareTo(rhs.value))
    override fun partialEq(rhs: NUInt32) = (value.compareTo(rhs.value) == 0)

    override fun reciprocal() = URtn32(UInt32.one, value)
    override fun unaryMinus() = NInt32(-value.toInt32())
    override fun abs() = NUInt32(value.abs())

    override fun plus(rhs: NUInt32) = NUInt32(value + rhs.value)
    override fun minus(rhs: NUInt32) = NInt32(value.toInt32() - rhs.toInt32())
    override fun times(rhs: NUInt32) = NUInt32(value * rhs.value)
    override fun div(rhs: NUInt32) = URtn32(value, rhs.value)
    override fun rem(rhs: NUInt32) = NUInt32(value % rhs.value)
    override fun intDiv(rhs: NUInt32) = NUInt32(value / rhs.value)

    @Throws(IllegalArgumentException::class)
    override fun log(base: FloatingNumber<*>): FloatingNumber<*>? = when (base) {
        is Flt32 -> toFlt32().log(base)
        is Flt64 -> toFlt64().log(base)
        is FltX -> toFltX().log(base)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt32.log: ${base.javaClass}")
    }

    override fun pow(index: Int): URtn32 {
        return if (index >= 1) {
            URtn32(value.pow(index), UInt32.one)
        } else if (index <= -1) {
            URtn32(UInt32.one, value.pow(index))
        } else {
            URtn32.one
        }
    }

    @Throws(IllegalArgumentException::class)
    override fun pow(index: FloatingNumber<*>): FloatingNumber<*> = when (index) {
        is Flt32 -> toFlt32().pow(index)
        is Flt64 -> toFlt64().pow(index)
        is FltX -> toFltX().pow(index)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt32.pow: ${index.javaClass}")
    }

    override fun exp() = toFlt64().exp()

    override fun rangeTo(rhs: NUInt32) =
        NumericUIntegerRange(copy(), rhs, one, UInt32, UInt32::toNUInt32, NUInt32::toUInt32)

    override infix fun until(rhs: NUInt32) = rangeTo((rhs - constants.one).toNUInt32())

    override fun toInt8() = value.toInt8()
    override fun toInt16() = value.toInt16()
    override fun toInt32() = value.toInt32()
    override fun toInt64() = value.toInt64()
    override fun toIntX() = value.toIntX()

    override fun toUInt8() = value.toUInt8()
    override fun toUInt16() = value.toUInt16()
    override fun toUInt32() = value.toUInt32()
    override fun toUInt64() = value.toUInt64()
    override fun toUIntX() = value.toUIntX()

    override fun toFlt32() = value.toFlt32()
    override fun toFlt64() = value.toFlt64()
    override fun toFltX() = value.toFltX()
}

object NUInt64Serializer : RealNumberKSerializer<NUInt64> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("NUInt64", PrimitiveKind.LONG)
    override val constants = NUInt64

    override fun serialize(encoder: Encoder, value: NUInt64) {
        encoder.encodeLong(value.value.value.toLong())
    }

    override fun deserialize(decoder: Decoder): NUInt64 {
        return NUInt64(UInt64(decoder.decodeLong().toULong()))
    }
}

@JvmInline
@Serializable(with = NUInt64Serializer::class)
value class NUInt64(val value: UInt64) : NumericUInteger<NUInt64, UInt64> {
    companion object : NumericUIntegerConstants<NUInt64, UInt64>(NUInt64::invoke, UInt64) {
        operator fun invoke(value: UInt64) = NUInt64(value)
    }

    override val constants: RealNumberConstants<NUInt64> get() = NUInt64

    override fun copy(): NUInt64 = NUInt64(value)

    override fun toString() = value.toString()
    fun toString(radix: Int) = value.toString(radix)

    override fun dec(): NUInt64 = NUInt64(value - UInt64.one)

    override fun partialOrd(rhs: NUInt64) = orderOf(value.compareTo(rhs.value))
    override fun partialEq(rhs: NUInt64) = (value.compareTo(rhs.value) == 0)

    override fun reciprocal() = URtn64(UInt64.one, value)
    override fun unaryMinus() = NInt64(-value.toInt64())
    override fun abs() = NUInt64(value.abs())

    override fun plus(rhs: NUInt64) = NUInt64(value + rhs.value)
    override fun minus(rhs: NUInt64) = NInt64(value.toInt64() - rhs.toInt64())
    override fun times(rhs: NUInt64) = NUInt64(value * rhs.value)
    override fun div(rhs: NUInt64) = URtn64(value, rhs.value)
    override fun rem(rhs: NUInt64) = NUInt64(value % rhs.value)
    override fun intDiv(rhs: NUInt64) = NUInt64(value / rhs.value)

    @Throws(IllegalArgumentException::class)
    override fun log(base: FloatingNumber<*>): FloatingNumber<*>? = when (base) {
        is Flt32 -> toFlt64().log(base)
        is Flt64 -> toFlt64().log(base)
        is FltX -> toFltX().log(base)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt64.log: ${base.javaClass}")
    }

    override fun pow(index: Int): URtn64 {
        return if (index >= 1) {
            URtn64(value.pow(index), UInt64.one)
        } else if (index <= -1) {
            URtn64(UInt64.one, value.pow(index))
        } else {
            URtn64.one
        }
    }

    @Throws(IllegalArgumentException::class)
    override fun pow(index: FloatingNumber<*>): FloatingNumber<*> = when (index) {
        is Flt32 -> toFlt64().pow(index)
        is Flt64 -> toFlt64().pow(index)
        is FltX -> toFltX().pow(index)
        else -> throw IllegalArgumentException("Unknown argument type to NUInt64.pow: ${index.javaClass}")
    }

    override fun exp() = toFlt64().exp()

    override fun rangeTo(rhs: NUInt64) =
        NumericUIntegerRange(copy(), rhs, one, UInt64, UInt64::toNUInt64, NUInt64::toUInt64)

    override infix fun until(rhs: NUInt64) = rangeTo((rhs - constants.one).toNUInt64())

    override fun toInt8() = value.toInt8()
    override fun toInt16() = value.toInt16()
    override fun toInt32() = value.toInt32()
    override fun toInt64() = value.toInt64()
    override fun toIntX() = value.toIntX()

    override fun toUInt8() = value.toUInt8()
    override fun toUInt16() = value.toUInt16()
    override fun toUInt32() = value.toUInt32()
    override fun toUInt64() = value.toUInt64()
    override fun toUIntX() = value.toUIntX()

    override fun toFlt32() = value.toFlt32()
    override fun toFlt64() = value.toFlt64()
    override fun toFltX() = value.toFltX()
}

class NUIntXSerializer : RealNumberKSerializer<NUIntX> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("NUIntX", PrimitiveKind.STRING)
    override val constants = NUIntX

    override fun serialize(encoder: Encoder, value: NUIntX) {
        encoder.encodeString(value.value.toString(10))
    }

    override fun deserialize(decoder: Decoder): NUIntX {
        return NUIntX(UIntX(decoder.decodeString()))
    }
}

@JvmInline
@Serializable(with = NUIntXSerializer::class)
value class NUIntX(val value: UIntX) : NumericUInteger<NUIntX, UIntX> {
    companion object : NumericUIntegerConstants<NUIntX, UIntX>(NUIntX::invoke, UIntX) {
        operator fun invoke(value: UIntX) = NUIntX(value)
    }

    override val constants: RealNumberConstants<NUIntX> get() = NUIntX

    override fun copy(): NUIntX = NUIntX(value)

    override fun toString() = value.toString()
    fun toString(radix: Int): String = value.toString(radix)

    override fun dec(): NUIntX = NUIntX(value - UIntX.one)

    override fun partialOrd(rhs: NUIntX) = orderOf(value.compareTo(rhs.value))
    override fun partialEq(rhs: NUIntX) = (value.compareTo(rhs.value) == 0)

    override fun reciprocal() = URtnX(UIntX.one, value)
    override fun unaryMinus() = NIntX(-value.toIntX())
    override fun abs() = NUIntX(value.abs())

    override fun plus(rhs: NUIntX) = NUIntX(value + rhs.value)
    override fun minus(rhs: NUIntX) = NIntX(value.toIntX() - rhs.toIntX())
    override fun times(rhs: NUIntX) = NUIntX(value * rhs.value)
    override fun div(rhs: NUIntX) = URtnX(value, rhs.value)
    override fun rem(rhs: NUIntX) = NUIntX(value % rhs.value)
    override fun intDiv(rhs: NUIntX) = NUIntX(value / rhs.value)

    override fun square() = pow(2)
    override fun cubic() = pow(3)

    @Throws(IllegalArgumentException::class)
    override fun log(base: FloatingNumber<*>): FloatingNumber<*>? = when (base) {
        is Flt32 -> toFltX().log(base)
        is Flt64 -> toFltX().log(base)
        is FltX -> toFltX().log(base)
        else -> throw IllegalArgumentException("Unknown argument type to NUIntX.log: ${base.javaClass}")
    }

    override fun lg() = log(FltX(10.0))
    override fun ln() = log(FltX.e)

    override fun pow(index: Int): URtnX {
        return if (index >= 1) {
            URtnX(value.pow(index), UIntX.one)
        } else if (index <= -1) {
            URtnX(UIntX.one, value.pow(index))
        } else {
            URtnX.one
        }
    }

    @Throws(IllegalArgumentException::class)
    override fun pow(index: FloatingNumber<*>): FloatingNumber<*> = when (index) {
        is Flt32 -> toFltX().pow(index)
        is Flt64 -> toFltX().pow(index)
        is FltX -> toFltX().pow(index)
        else -> throw IllegalArgumentException("Unknown argument type to NUIntX.pow: ${index.javaClass}")
    }

    override fun sqr() = pow(FltX(1.0 / 2.0))
    override fun cbr() = pow(FltX(1.0 / 3.0))
    override fun exp() = toFltX().exp()

    override fun rangeTo(rhs: NUIntX) = NumericUIntegerRange(copy(), rhs, one, UIntX, UIntX::toNUIntX, NUIntX::toUIntX)
    override infix fun until(rhs: NUIntX) = rangeTo((rhs - constants.one).toNUIntX())

    override fun toInt8() = value.toInt8()
    override fun toInt16() = value.toInt16()
    override fun toInt32() = value.toInt32()
    override fun toInt64() = value.toInt64()
    override fun toIntX() = value.toIntX()

    override fun toUInt8() = value.toUInt8()
    override fun toUInt16() = value.toUInt16()
    override fun toUInt32() = value.toUInt32()
    override fun toUInt64() = value.toUInt64()
    override fun toUIntX() = value.toUIntX()

    override fun toFlt32() = value.toFlt32()
    override fun toFlt64() = value.toFlt64()
    override fun toFltX() = value.toFltX()
}
