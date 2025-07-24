package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model

import fuookami.ospf.kotlin.utils.operator.*

enum class CargoCode {
    BAL,
    ELD,
    FKI,
    CCD,
    ICE,
    AVI,
    AOG,
    ELI,
    ELM,
    MAG,
    HUB,
    PER,
    YYI,
    MAT,
    RRM,
    BIG,
    OHG,
    RFG,
    RFL,
    RFS,
    ROX,
    YYE,
    HWJ,
    RRY,
    RRW,
    CVV,
    Crush,
    Stiff,
    Empty,
    Virtual
}

data class CargoType(
    val code: CargoCode?,
    val type: String
) {
    companion object {
        private val cache: MutableMap<String, CargoType> = HashMap()

        operator fun invoke(code: CargoCode): CargoType {
            return cache.getOrPut(code.name) {
                CargoType(
                    code = code,
                    type = code.name
                )
            }
        }

        operator fun invoke(name: String): CargoType {
            return cache.getOrPut(name) {
                CargoType(
                    code = CargoCode.entries.firstOrNull { it.name == name },
                    type = name,
                )
            }
        }
    }
}

enum class CargoPriorityCategory {
    High,
    Normal,
    Low
}

enum class CargoPriorityType {
    HighTransfer {
        override val category = CargoPriorityCategory.High
        override val transfer = true
    },

    CCD {
        override val category = CargoPriorityCategory.High
    },

    MediumTransfer {
        override val category = CargoPriorityCategory.High
        override val transfer = true
    },

    HUB {
        override val category = CargoPriorityCategory.High
    },

    LowTransfer {
        override val category = CargoPriorityCategory.High
        override val transfer = true
    },

    NormalWithoutOrder {
        override val category = CargoPriorityCategory.Normal
    },

    NormalWithOrder {
        override val category = CargoPriorityCategory.Normal
    },

    SameACTransfer {
        override val category = CargoPriorityCategory.Low
    };

    abstract val category: CargoPriorityCategory
    open val transfer: Boolean = false
}

infix fun CargoPriorityType.ord(rhs: CargoPriorityType): Order {
    return orderOf(this.category.ordinal compareTo rhs.ordinal)
}
