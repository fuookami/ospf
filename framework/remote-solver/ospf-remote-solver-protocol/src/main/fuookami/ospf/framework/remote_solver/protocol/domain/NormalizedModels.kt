/**
 * 标准化模型类型
 * Normalized model type
 *
 * 用于优化模型的类型分类。
 * Type classification for optimization models.
 *
 * 由 ModelData 使用，用于区分线性模型和二次模型。
 * Used by ModelData to distinguish between linear and quadratic models.
 */
package fuookami.ospf.framework.remote_solver.protocol.domain

/**
 * 标准化模型类型枚举
 * Normalized model type enumeration
 */
enum class NormalizedModelType {
    /** 线性模型 / Linear model */
    LINEAR,

    /** 二次模型 / Quadratic model */
    QUADRATIC,

    /** 未知类型 / Unknown type */
    UNKNOWN
}