package fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.dto

import kotlin.time.*
import kotlinx.datetime.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*

/**
 * 运行心跳
 *
 * @property id                 请求 id
 * @property runTime            运行时间
 * @property estimatedTime      预计运行时间
 * @property optimizedRate      优化率
 * @property time               当前时间
 */
data class RunningHeartBeatDTO(
    val id: String,
    val runTime: Duration,
    val estimatedTime: Duration,
    val optimizedRate: Flt64
) {
    val time: LocalDateTime = Clock.System.now().toLocalDateTime(TimeZone.currentSystemDefault())
}

/**
 * 结束心跳
 *
 * @property id               请求 id
 * @property runTime          运行时间
 * @property code             错误码
 * @property message          错误信息
 * @property time             当前时间
 */
data class FinnishHeartBeatDTO(
    val id: String,
    val runTime: Duration,
    val code: UInt64,
    val message: String
) {
    val time: LocalDateTime = Clock.System.now().toLocalDateTime(TimeZone.currentSystemDefault())

    companion object {
        /**
         * 构建成功结束心跳
         *
         * @param id            请求 id
         * @param runTime       运行时长
         * @return              结束心跳
         */
        operator fun invoke(id: String, runTime: Duration): FinnishHeartBeatDTO {
            return FinnishHeartBeatDTO(id, runTime, UInt64.zero, "")
        }

        /**
         * 构建错误结束心跳
         *
         * @param id            请求 id
         * @param runTime       运行时长
         * @param error         错误
         * @return              结束心跳
         */
        operator fun invoke(id: String, runTime: Duration, error: Error): FinnishHeartBeatDTO {
            return FinnishHeartBeatDTO(id, runTime, error.code.toUInt64(), error.message)
        }

        /**
         * 构建错误结束心跳
         *
         * @param id            请求 id
         * @param error         错误
         * @return              结束心跳
         */
        operator fun invoke(id: String, error: Error): FinnishHeartBeatDTO {
            return FinnishHeartBeatDTO(id, Duration.ZERO, error.code.toUInt64(), error.message)
        }
    }
}
