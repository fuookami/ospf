package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*

class Formula(
    val aircraftModel: AircraftModel,
    val lip: Quantity<Flt64>,
    val chord: Quantity<Flt64>,
    val standardDatum: Quantity<Flt64>,
    val forceDistanceCoefficient: Flt64,
    val doiCorrection: Quantity<Flt64>,
) {
    fun balancedArm(mac: MAC): Quantity<Flt64> {
        return mac.mac * chord + lip
    }

    @JvmName("balancedArm")
    fun balancedArm(index: Quantity<Flt64>, totalWeight: Quantity<Flt64>): Quantity<Flt64> {
        return (index - doiCorrection) * forceDistanceCoefficient / aircraftModel.gravity(totalWeight) + standardDatum
    }

    @JvmName("balancedArmPolynomial")
    fun balancedArm(index: Quantity<LinearPolynomial>, totalWeight: Quantity<Flt64>): Quantity<LinearPolynomial> {
        return (index - doiCorrection) * forceDistanceCoefficient / aircraftModel.gravity(totalWeight) + standardDatum
    }

    fun arm(index: Quantity<Flt64>, weight: Quantity<Flt64>): Quantity<Flt64> {
        return index * forceDistanceCoefficient / aircraftModel.gravity(weight) + standardDatum
    }

    fun index(mac: MAC, totalWeight: Quantity<Flt64>): Quantity<Flt64> {
        return totalWeight * (balancedArm(mac) - standardDatum) / forceDistanceCoefficient + doiCorrection
    }

    fun index(weight: Quantity<Flt64>, arm: Quantity<Flt64>): Quantity<Flt64> {
        return aircraftModel.gravity(weight) * (arm - standardDatum) / forceDistanceCoefficient
    }

    fun index(weight: Quantity<Flt64>, position: Position): Quantity<Flt64> {
        return index(weight, position.coordinate.longitudinalArm)
    }

    fun mac(balancedArm: Quantity<Flt64>): MAC {
        return MAC(((balancedArm - lip) / chord).value)
    }

    fun mac(index: Quantity<Flt64>, totalWeight: Quantity<Flt64>): MAC {
        return mac(balancedArm(index, totalWeight))
    }

    fun mac(index: Quantity<LinearPolynomial>, totalWeight: Quantity<Flt64>): LinearPolynomial {
        return ((balancedArm(index, totalWeight) - lip) / chord).value
    }
}
