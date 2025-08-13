package fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.dto

import kotlin.time.*
import kotlinx.datetime.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*

data class RunningHeartBeatDTO(
    val id: String,
    val runTime: Duration,
    val estimatedTime: Duration,
    val optimizedRate: Flt64
) {
    val time: LocalDateTime = Clock.System.now().toLocalDateTime(TimeZone.currentSystemDefault())
}

data class FinnishHeartBeatDTO(
    val id: String,
    val runTime: Duration,
    val code: UInt64,
    val message: String
) {
    val time: LocalDateTime = Clock.System.now().toLocalDateTime(TimeZone.currentSystemDefault())

    companion object {
        operator fun invoke(
            id: String,
            runTime: Duration
        ): FinnishHeartBeatDTO {
            return FinnishHeartBeatDTO(
                id = id,
                runTime = runTime,
                code = UInt64.zero,
                message = ""
            )
        }

        operator fun invoke(
            id: String,
            runTime: Duration,
            error: Error
        ): FinnishHeartBeatDTO {
            return FinnishHeartBeatDTO(
                id = id,
                runTime = runTime,
                code = error.code.toUInt64(),
                message = error.message
            )
        }

        operator fun invoke(
            id: String,
            error: Error
        ): FinnishHeartBeatDTO {
            return FinnishHeartBeatDTO(
                id = id,
                runTime = Duration.ZERO,
                code = error.code.toUInt64(),
                message = error.message
            )
        }
    }
}
