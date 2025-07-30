package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.crew.model

import fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure.*

enum class CrewManRankClass(val no: CrewManRankNo) {
    PrivateAttendant(CrewManRankNo("S002")),
    Attendant(CrewManRankNo("S001")),
    Maintainer(CrewManRankNo("M001")),
    CoMaintainer(CrewManRankNo("M002"))
}

data class CrewManRank(
    val cls: CrewManRankClass?,
    val no: CrewManRankNo,
    val name: String,
    val displayName: String? = null
) {
    companion object {
        private val pool = HashMap<CrewManRankNo, CrewManRank>()
        val values by pool::values

        operator fun invoke(cls: CrewManRankClass): CrewManRank? {
            return pool[cls.no]
        }

        operator fun invoke(no: CrewManRankNo): CrewManRank? {
            return pool[no]
        }
    }

    init {
        pool[no] = this
    }

    override fun toString(): String {
        return displayName ?: name
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is CrewManRank) return false

        if (cls != other.cls) return false
        if (no != other.no) return false

        return true
    }

    override fun hashCode(): Int {
        return cls?.hashCode() ?: no.hashCode()
    }
}
