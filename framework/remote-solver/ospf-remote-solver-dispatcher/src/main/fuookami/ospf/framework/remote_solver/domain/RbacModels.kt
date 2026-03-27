/*
 * Role-Based Access Control (RBAC) Models
 * 基于角色的访问控制(RBAC)模型
 *
 * This file defines the RBAC models for Remote Solver.
 * 本文件定义了远程求解器的RBAC模型。
 *
 * Roles hierarchy:
 * 角色层级：
 * - ADMIN: Full access to all APIs
 *   管理员：对所有API的完全访问权限
 * - OPERATOR: Task management and monitoring
 *   操作员：任务管理和监控
 * - MONITOR_READ: Read-only monitoring access
 *   监控只读：只读监控访问权限
 * - TASK_SUBMITTER: Can submit and manage own tasks
 *   任务提交者：可提交和管理自己的任务
 */
package fuookami.ospf.framework.remote_solver.domain

import fuookami.ospf.framework.remote_solver.port.AuthResult

/**
 * Predefined Roles for Remote Solver
 * 远程求解器预定义角色
 *
 * Roles hierarchy:
 * 角色层级：
 * - ADMIN: Full access to all APIs
 *   管理员：对所有API的完全访问权限
 * - OPERATOR: Task management and monitoring
 *   操作员：任务管理和监控
 * - MONITOR_READ: Read-only monitoring access
 *   监控只读：只读监控访问权限
 * - TASK_SUBMITTER: Can submit and manage own tasks
 *   任务提交者：可提交和管理自己的任务
 *
 * @param displayName Human-readable role name.
 *                     人类可读的角色名称。
 * @param description Detailed description of the role's capabilities.
 *                     角色能力的详细描述。
 * @param priority Priority level for role comparison (higher = more privileged).
 *                  角色比较的优先级级别（越高 = 权限越大）。
 */
enum class RbacRole(
    val displayName: String,
    val description: String,
    val priority: Int
) {
    /**
     * Admin role with full access to all APIs including configuration.
     * 管理员角色，对所有API（包括配置）有完全访问权限。
     */
    ADMIN(
        displayName = "Admin",
        description = "Full access to all APIs including configuration",
        priority = 100
    ),

    /**
     * Operator role for task management, monitoring, and scheduler operations.
     * 操作员角色，用于任务管理、监控和调度器操作。
     */
    OPERATOR(
        displayName = "Operator",
        description = "Task management, monitoring, and scheduler operations",
        priority = 50
    ),

    /**
     * Monitor Read role with read-only access to monitoring APIs.
     * 监控只读角色，对监控API有只读访问权限。
     */
    MONITOR_READ(
        displayName = "Monitor Read",
        description = "Read-only access to monitoring APIs",
        priority = 20
    ),

    /**
     * Task Submitter role that can submit and manage own tasks.
     * 任务提交者角色，可提交和管理自己的任务。
     */
    TASK_SUBMITTER(
        displayName = "Task Submitter",
        description = "Can submit and manage own tasks",
        priority = 10
    );

    companion object {
        /**
         * Parses a role from a string name.
         * 从字符串名称解析角色。
         *
         * @param name The role name to parse.
         *             要解析的角色名称。
         * @return The parsed role, or null if not found.
         *         解析出的角色，如果未找到则返回null。
         */
        fun fromString(name: String): RbacRole? {
            return entries.find { it.name.equals(name, ignoreCase = true) }
        }
    }
}

/**
 * Permission Definitions for API Access Control
 * API访问控制的权限定义
 *
 * Defines specific permissions that can be granted to roles.
 * 定义可授予角色的具体权限。
 *
 * @param displayName Human-readable permission name.
 *                     人类可读的权限名称。
 * @param description Detailed description of the permission.
 *                     权限的详细描述。
 */
enum class RbacPermission(
    val displayName: String,
    val description: String
) {
    // Task permissions
    // 任务权限

    /**
     * Permission to submit new solving tasks.
     * 提交新求解任务的权限。
     */
    TASK_SUBMIT("Submit Task", "Submit new solving tasks"),

    /**
     * Permission to view task details and status.
     * 查看任务详情和状态的权限。
     */
    TASK_READ("Read Task", "View task details and status"),

    /**
     * Permission to stop running tasks.
     * 停止运行任务的权限。
     */
    TASK_STOP("Stop Task", "Stop running tasks"),

    /**
     * Permission to resume stopped tasks.
     * 恢复已停止任务的权限。
     */
    TASK_RESUME("Resume Task", "Resume stopped tasks"),

    /**
     * Permission to view task execution timeline.
     * 查看任务执行时间线的权限。
     */
    TASK_TIMELINE("Task Timeline", "View task execution timeline"),

    // Scheduler permissions
    // 调度器权限

    /**
     * Permission to update scheduler configuration via hot reload.
     * 通过热重载更新调度器配置的权限。
     */
    SCHEDULER_HOT_RELOAD("Scheduler Hot Reload", "Update scheduler configuration"),

    /**
     * Permission to rollback scheduler configuration.
     * 回滚调度器配置的权限。
     */
    SCHEDULER_ROLLBACK("Scheduler Rollback", "Rollback scheduler configuration"),

    /**
     * Permission to view scheduler audit logs.
     * 查看调度器审计日志的权限。
     */
    SCHEDULER_AUDIT_READ("Scheduler Audit Read", "View scheduler audit logs"),

    // Monitor permissions
    // 监控权限

    /**
     * Permission to access monitoring dashboard and APIs.
     * 访问监控仪表板和API的权限。
     */
    MONITOR_READ("Monitor Read", "Access monitoring dashboard and APIs"),

    // Admin permissions
    // 管理员权限

    /**
     * Full administrative access to all operations.
     * 对所有操作的完全管理权限。
     */
    ADMIN_ALL("Admin All", "Full administrative access");

    companion object {
        /**
         * Parses a permission from a string name.
         * 从字符串名称解析权限。
         *
         * @param name The permission name to parse.
         *             要解析的权限名称。
         * @return The parsed permission, or null if not found.
         *         解析出的权限，如果未找到则返回null。
         */
        fun fromString(name: String): RbacPermission? {
            return entries.find { it.name.equals(name, ignoreCase = true) }
        }
    }
}

/**
 * Role-to-Permission Mapping
 * 角色到权限的映射
 *
 * Defines which permissions each role has.
 * 定义每个角色拥有的权限。
 */
object RbacPolicy {
    /**
     * Mapping of roles to their permissions.
     * 角色到其权限的映射。
     */
    private val rolePermissions: Map<RbacRole, Set<RbacPermission>> = mapOf(
        RbacRole.ADMIN to RbacPermission.entries.toSet(),
        RbacRole.OPERATOR to setOf(
            RbacPermission.TASK_SUBMIT,
            RbacPermission.TASK_READ,
            RbacPermission.TASK_STOP,
            RbacPermission.TASK_RESUME,
            RbacPermission.TASK_TIMELINE,
            RbacPermission.SCHEDULER_AUDIT_READ,
            RbacPermission.MONITOR_READ
        ),
        RbacRole.MONITOR_READ to setOf(
            RbacPermission.MONITOR_READ,
            RbacPermission.TASK_READ,
            RbacPermission.TASK_TIMELINE
        ),
        RbacRole.TASK_SUBMITTER to setOf(
            RbacPermission.TASK_SUBMIT,
            RbacPermission.TASK_READ,
            RbacPermission.TASK_STOP,
            RbacPermission.TASK_RESUME
        )
    )

    /**
     * Gets all permissions for a role.
     * 获取角色的所有权限。
     *
     * @param role The role to query.
     *             要查询的角色。
     * @return Set of permissions for the role.
     *         角色的权限集合。
     */
    fun getPermissions(role: RbacRole): Set<RbacPermission> {
        return rolePermissions[role] ?: emptySet()
    }

    /**
     * Checks if a role has a specific permission.
     * 检查角色是否拥有特定权限。
     *
     * @param role The role to check.
     *             要检查的角色。
     * @param permission The permission to verify.
     *                   要验证的权限。
     * @return True if the role has the permission.
     *         如果角色拥有权限则返回true。
     */
    fun hasPermission(role: RbacRole, permission: RbacPermission): Boolean {
        return rolePermissions[role]?.contains(permission) == true
    }

    /**
     * Checks if any of the roles have a specific permission.
     * 检查任意角色是否拥有特定权限。
     *
     * @param roles Set of role names to check.
     *              要检查的角色名称集合。
     * @param permission The permission to verify.
     *                   要验证的权限。
     * @return True if any role has the permission.
     *         如果任意角色拥有权限则返回true。
     */
    fun hasAnyPermission(roles: Set<String>, permission: RbacPermission): Boolean {
        return roles.any { roleName ->
            RbacRole.fromString(roleName)?.let { role ->
                hasPermission(role, permission)
            } == true
        }
    }

    /**
     * Checks if any of the roles have all specified permissions.
     * 检查任意角色是否拥有所有指定权限。
     *
     * @param roles Set of role names to check.
     *              要检查的角色名称集合。
     * @param permissions Set of permissions to verify.
     *                    要验证的权限集合。
     * @return True if all permissions are covered by the roles.
     *         如果所有权限都被角色覆盖则返回true。
     */
    fun hasAllPermissions(roles: Set<String>, permissions: Set<RbacPermission>): Boolean {
        return permissions.all { permission ->
            hasAnyPermission(roles, permission)
        }
    }

    /**
     * Gets effective permissions for a set of roles.
     * 获取角色集合的有效权限。
     *
     * Combines permissions from all roles into a single set.
     * 将所有角色的权限合并为单一集合。
     *
     * @param roles Set of role names to combine.
     *              要合并的角色名称集合。
     * @return Combined set of permissions.
     *         合并后的权限集合。
     */
    fun getEffectivePermissions(roles: Set<String>): Set<RbacPermission> {
        return roles.flatMap { roleName ->
            RbacRole.fromString(roleName)?.let { role ->
                getPermissions(role)
            } ?: emptySet()
        }.toSet()
    }

    /**
     * Validates that the roles set is not empty and contains valid roles.
     * 验证角色集合不为空且包含有效角色。
     *
     * @param roles Set of role names to validate.
     *              要验证的角色名称集合。
     * @return Pair of (isValid, invalidRoleNames).
     *         (是否有效, 无效角色名称列表)的配对。
     */
    fun validateRoles(roles: Set<String>): Pair<Boolean, List<String>> {
        val invalidRoles = roles.filter { RbacRole.fromString(it) == null }
        return Pair(roles.isNotEmpty() && invalidRoles.isEmpty(), invalidRoles)
    }
}

/**
 * Authorization Context for Request Processing
 * 请求处理的授权上下文
 *
 * Contains the authenticated user's authorization information.
 * 包含已认证用户的授权信息。
 *
 * @param userId The user identifier.
 *               用户标识符。
 * @param tenantId The tenant identifier.
 *                 租户标识符。
 * @param roles Set of role names assigned to the user.
 *              分配给用户的角色名称集合。
 * @param permissions Set of permissions derived from roles.
 *                    从角色派生的权限集合。
 * @param expiresAtEpochMs Expiration timestamp in epoch milliseconds.
 *                          过期时间戳（epoch毫秒）。
 */
data class AuthContext(
    val userId: String,
    val tenantId: String,
    val roles: Set<String>,
    val permissions: Set<RbacPermission>,
    val expiresAtEpochMs: Long
) {
    companion object {
        /**
         * Creates AuthContext from an AuthResult.
         * 从AuthResult创建AuthContext。
         *
         * @param result The authentication result.
         *               认证结果。
         * @return AuthContext with computed permissions.
         *         带有计算权限的AuthContext。
         */
        fun fromAuthResult(result: AuthResult): AuthContext {
            return AuthContext(
                userId = result.userId,
                tenantId = result.tenantId,
                roles = result.roles,
                permissions = RbacPolicy.getEffectivePermissions(result.roles),
                expiresAtEpochMs = result.expiresAtEpochMs
            )
        }
    }

    /**
     * Checks if the context has a specific permission.
     * 检查上下文是否拥有特定权限。
     *
     * @param permission The permission to verify.
     *                   要验证的权限。
     * @return True if the context has the permission.
     *         如果上下文拥有权限则返回true。
     */
    fun hasPermission(permission: RbacPermission): Boolean {
        return permissions.contains(permission)
    }

    /**
     * Checks if the context has any of the specified permissions.
     * 检查上下文是否拥有任意指定权限。
     *
     * @param permissions Set of permissions to verify.
     *                    要验证的权限集合。
     * @return True if the context has any of the permissions.
     *         如果上下文拥有任意权限则返回true。
     */
    fun hasAnyPermission(permissions: Set<RbacPermission>): Boolean {
        return permissions.any { this.permissions.contains(it) }
    }

    /**
     * Checks if the context has all specified permissions.
     * 检查上下文是否拥有所有指定权限。
     *
     * @param permissions Set of permissions to verify.
     *                    要验证的权限集合。
     * @return True if the context has all of the permissions.
     *         如果上下文拥有所有权限则返回true。
     */
    fun hasAllPermissions(permissions: Set<RbacPermission>): Boolean {
        return permissions.all { this.permissions.contains(it) }
    }

    /**
     * Checks if the context has admin role.
     * 检查上下文是否拥有管理员角色。
     *
     * @return True if the user is an admin.
     *         如果用户是管理员则返回true。
     */
    fun isAdmin(): Boolean {
        return roles.any { RbacRole.fromString(it) == RbacRole.ADMIN }
    }
}