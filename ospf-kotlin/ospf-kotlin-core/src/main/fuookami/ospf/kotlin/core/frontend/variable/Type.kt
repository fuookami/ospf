package  fuookami.ospf.kotlin.core.frontend.variable

import fuookami.ospf.kotlin.utils.math.*

sealed interface VariableTypeInterface<T> where T : RealNumber<T>, T : NumberField<T> {
    val name: String
    val shortName: String
    val constants: RealNumberConstants<T>
    val minimum: T
    val maximum: T

    val isBinaryType get() = false
    val isUnsignedType get() = false
    val isIntegerType get() = false
    val isUnsignedIntegerType get() = isIntegerType && isUnsignedType
    val isContinuousType get() = !isIntegerType
    val isUnsignedContinuousType get() = isContinuousType && isUnsignedType
    val isNotBinaryIntegerType get() = !isBinaryType && isIntegerType
}

sealed interface IntegerVariableType<T : IntegerNumber<T>> : VariableTypeInterface<T> {
    override val minimum get() = constants.minimum
    override val maximum get() = constants.maximum

    override val isIntegerType get() = true
}

sealed interface UIntegerVariableType<T : UIntegerNumber<T>> : VariableTypeInterface<T> {
    override val minimum get() = constants.zero
    override val maximum get() = constants.maximum

    override val isUnsignedType get() = true
    override val isIntegerType get() = true
}

sealed interface ContinuesVariableType<T : FloatingNumber<T>> : VariableTypeInterface<T> {
    override val minimum get() = -constants.decimalPrecision.reciprocal()
    override val maximum get() = constants.decimalPrecision.reciprocal()
}

sealed interface UContinuesVariableType<T : FloatingNumber<T>> : VariableTypeInterface<T> {
    override val minimum get() = constants.zero
    override val maximum get() = constants.decimalPrecision.reciprocal()

    override val isUnsignedType get() = true
}

sealed class VariableType<T>(
    override val constants: RealNumberConstants<T>
) : VariableTypeInterface<T> where T : RealNumber<T>, T : NumberField<T>

data object Binary : VariableType<UInt8>(UInt8), UIntegerVariableType<UInt8> {
    override val name = "Binary"
    override val shortName = "bin"
    override val maximum by constants::one

    override val isBinaryType get() = true
    override val isUnsignedType get() = true

    override fun toString(): String = "Binary"
}

data object Ternary : VariableType<UInt8>(UInt8), UIntegerVariableType<UInt8> {
    override val name = "Ternary"
    override val shortName = "ter"
    override val maximum by constants::two

    override fun toString(): String = "Ternary"
}

data object BalancedTernary : VariableType<Int8>(Int8), IntegerVariableType<Int8> {
    override val name = "BalancedTernary"
    override val shortName = "bter"
    override val minimum get() = -constants.one
    override val maximum by constants::one

    override fun toString(): String = "BalancedTernary"
}

data object Percentage : VariableType<Flt64>(Flt64), UContinuesVariableType<Flt64> {
    override val name = "Percentage"
    override val shortName = "pct"
    override val maximum by constants::one

    override fun toString(): String = "Percentage"
}

data object Integer : VariableType<Int64>(Int64), IntegerVariableType<Int64> {
    override val name = "Integer"
    override val shortName = "int"
    override fun toString(): String = "Integer"
}

data object UInteger : VariableType<UInt64>(UInt64), UIntegerVariableType<UInt64> {
    override val name = "UInteger"
    override val shortName = "uint"
    override fun toString(): String = "UInteger"
}

data object Continuous : VariableType<Flt64>(Flt64), ContinuesVariableType<Flt64> {
    override val name = "Continuous"
    override val shortName = "real"
    override fun toString(): String = "Continues"
}

data object UContinuous : VariableType<Flt64>(Flt64), UContinuesVariableType<Flt64> {
    override val name = "UContinuous"
    override val shortName = "ureal"
    override fun toString(): String = "UContinues"
}
