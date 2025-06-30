package fuookami.ospf.kotlin.example.framework_demo.demo4.flight_task_context.model

import fuookami.ospf.kotlin.utils.concept.*

enum class PassengerClass: Indexed {
    First {
        override fun toShortString() = "F"
    },

    Business {
        override fun toShortString() = "B"
    },

    Economy {
        override fun toShortString() = "E"
    };

    override val index: Int get() = this.ordinal

    companion object {
        operator fun invoke(cls: fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure.PassengerClass): PassengerClass {
            return PassengerClass.valueOf(cls.cls)
        }
    }

    abstract fun toShortString(): String
}
