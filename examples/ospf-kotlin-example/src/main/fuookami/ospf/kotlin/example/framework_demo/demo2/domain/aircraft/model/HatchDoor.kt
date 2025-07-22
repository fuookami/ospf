package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*

enum class DoorUbietyType {
    Front,
    AdjacentFront,
    Beside,
    Opposite,
    AdjacentBehind,
    Behind;
}

data class DoorUbiety(
    val type: DoorUbietyType,
    val sameSide: Boolean,
    val position: Position,
    val door: HatchDoor
)

data class HatchDoor(
    val location: DeckLocation,
    val besideBulk: Boolean,
    val noseDoor: Boolean,
    val lateralArm: Quantity<Flt64>,
    val frontArm: Quantity<Flt64>,
    val backArm: Quantity<Flt64>
)
