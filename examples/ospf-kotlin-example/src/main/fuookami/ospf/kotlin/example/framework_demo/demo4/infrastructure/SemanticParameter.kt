package fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure

import kotlinx.serialization.*

@JvmInline
@Serializable
value class IATA(val code: String) {
    init {
        assert(code.length == 3)
    }

    override fun toString() = code
}

@JvmInline
@Serializable
value class ICAO(val code: String) {
    init {
        assert(code.length == 4)
    }

    override fun toString() = code
}

@JvmInline
@Serializable
value class AircraftTypeName(val name: String) {
    override fun toString() = name
}

@JvmInline
@Serializable
value class AircraftTypeCode(val code: String) {
    override fun toString() = code
}

@JvmInline
@Serializable
value class AircraftMinorTypeName(val name: String) {
    override fun toString() = name
}

@JvmInline
@Serializable
value class AircraftMinorTypeCode(val code: String) {
    override fun toString() = code
}

@JvmInline
@Serializable
value class WingAircraftTypeCode(val code: String) {
    override fun toString() = code
}

@JvmInline
@Serializable
value class AircraftRegisterNumber(val no: String) {
    override fun toString() = no
}

@JvmInline
value class PassengerClass(val cls: String) {
    override fun toString() = cls
}
