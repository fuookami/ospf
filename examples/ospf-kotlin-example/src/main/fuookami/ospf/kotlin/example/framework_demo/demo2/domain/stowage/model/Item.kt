package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model

import kotlinx.datetime.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.operator.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*

enum class ItemLocationTag {
    Main,
    Low,
    Bulk,
    Head,
    Tail
}

data class ItemLocation(
    val tags: Set<ItemLocationTag>,
) {
    companion object {
        val head = ItemLocation(setOf(ItemLocationTag.Main, ItemLocationTag.Head))
        val tail = ItemLocation(setOf(ItemLocationTag.Main, ItemLocationTag.Tail))
        val normalMain = ItemLocation(setOf(ItemLocationTag.Main))
        val lowBulk = ItemLocation(setOf(ItemLocationTag.Low, ItemLocationTag.Bulk))
        val lowNotBulk = ItemLocation(setOf(ItemLocationTag.Low))
    }

    operator fun contains(tag: ItemLocationTag): Boolean {
        return tags.contains(tag)
    }

    val main = contains(ItemLocationTag.Main)
    val head = contains(ItemLocationTag.Head)
    val tail = contains(ItemLocationTag.Tail)
    val normalMain = main && !head && !tail
    val specialMain = head || tail
    val low = contains(ItemLocationTag.Low)
    val bulk = contains(ItemLocationTag.Bulk)
    val lowNotBulk = low && !bulk

    fun enabledIn(location: DeckLocation): Try {
        return when (location) {
            DeckLocation.Main -> {
                if (low) {
                    Failed(ErrorCode.ApplicationFailed, "主货舱不能装载下货舱物品")
                } else {
                    ok
                }
            }

            DeckLocation.LowForward, DeckLocation.LowAft -> {
                if (main) {
                    Failed(ErrorCode.ApplicationFailed, "下货舱不能装载主货舱物品")
                } else {
                    ok
                }
            }
        }
    }

    fun enabledIn(location: PositionLocation): Try {
        if (low && location.main) {
            return Failed(ErrorCode.ApplicationFailed, "主货舱不能装载下货舱物品")
        }
        if (main && location.low) {
            return Failed(ErrorCode.ApplicationFailed, "下货舱不能装载主舱物品")
        }
        if (!bulk && location.bulk) {
            return Failed(ErrorCode.ApplicationFailed, "散舱不能装载非散舱物品")
        }
        if (bulk && !location.bulk) {
            return Failed(ErrorCode.ApplicationFailed, "非散舱不能装载散舱物品")
        }
        if (head && !location.head) {
            return Failed(ErrorCode.ApplicationFailed, "非头舱不能装载头舱物品")
        }
        if (tail && !location.tail) {
            return Failed(ErrorCode.ApplicationFailed, "非尾舱不能装载尾舱物品")
        }
        return ok
    }
}

enum class ItemStatus {
    Loaded {
        override val loaded = true
        override val available = false
    },
    AdjustmentNeeded {
        override val loaded = true
        override val available = true
        override val adjustmentNeeded = true
    },
    Preassigned {
        override val stowageNeeded = true
    },
    Optional {
        override val stowageNeeded = true
    },
    Reserved {
        override val available = false
    };

    open val loaded = false
    open val available = true
    open val unavailable get() = !available
    open val stowageNeeded get() = false
    open val adjustmentNeeded get() = false
}

data class ItemCargo(
    val types: Set<CargoType>,
    val priority: CargoPriority
) {
    operator fun contains(cargo: CargoType): Boolean {
        return types.contains(cargo)
    }

    operator fun contains(type: String): Boolean {
        return types.any { it.type == type }
    }

    operator fun contains(code: CargoCode): Boolean {
        return types.any { it.code == code }
    }
}

data class ItemOrder(
    val hardstand: Instant?,
    val reweighed: Instant?,
    val carBoard: CarBoard?,
    val order: UInt8?
) {
    data class CarBoard(
        val car: String,
        val board: String
    ) : PartialOrd<CarBoard>, Ord<CarBoard> {
        override fun partialOrd(rhs: CarBoard): Order {
            when (val result = car ord rhs.car) {
                Order.Equal -> {}

                else -> {
                    return result
                }
            }

            return board ord rhs.board
        }
    }
}

class Item(
    val id: String,
    val destination: IATA,
    val source: FlightNo?,
    val uld: ULD?,
    val weight: Quantity<Flt64>,
    val location: ItemLocation,
    val cargo: ItemCargo,
    val status: ItemStatus,
    val order: ItemOrder?
): ManualIndexed() {
    override fun toString(): String {
        return id
    }
}

typealias ItemPair = Pair<Item, Item>
