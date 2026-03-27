/**
 * BootstrapMigrationGuard - 启动迁移守卫
 *
 * BootstrapMigrationGuard - Bootstrap migration guard.
 *
 * 在远程求解器启动前验证数据库迁移是否已完成。
 * 检查 SQL 迁移文件版本与数据库中已应用的迁移版本是否匹配，
 * 防止因缺少必要的数据库迁移而导致运行时错误。
 *
 * Validates database migrations are complete before remote solver bootstrap.
 * Checks if SQL migration file versions match applied migration versions in database,
 * preventing runtime errors caused by missing required database migrations.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import java.nio.file.Files
import java.nio.file.Path
import java.sql.Connection
import java.sql.DriverManager

/**
 * 启动迁移守卫对象
 *
 * Bootstrap migration guard object.
 *
 * 提供迁移验证功能，确保启动前所有必要的数据库迁移已正确应用。
 * 支持多数据库目标验证，并可推断已存在的表结构版本。
 *
 * Provides migration validation functionality, ensuring all necessary
 * database migrations are properly applied before bootstrap.
 * Supports multi-database target validation and can infer existing table structure versions.
 */
object BootstrapMigrationGuard {
    /**
     * 数据库目标配置
     *
     * Database target configuration.
     *
     * 表示一个需要验证迁移的数据库连接目标。
     *
     * Represents a database connection target requiring migration validation.
     *
     * @param name 数据库目标名称（用于日志和错误信息）
     *              Database target name (for logging and error messages)
     * @param url JDBC 连接 URL
     *            JDBC connection URL
     * @param username 用户名，可为 null
     *                 Username, can be null
     * @param password 密码，可为 null
     *                 Password, can be null
     */
    private data class DbTarget(
        val name: String,
        val url: String,
        val username: String?,
        val password: String?
    )

    /** 迁移文件名匹配模式，如 V1__xxx.sql */
    private val migrationFilePattern = Regex("""^V(\d+)__.*\.sql$""")
    /** 数字版本提取模式 */
    private val numericVersionPattern = Regex("""(\d+)""")
    /** 默认 SQL 文件目录 */
    private const val defaultSqlDir = "deploy/sql"
    /** 默认迁移历史表名 */
    private const val defaultHistoryTable = "remote_solver_migration_history"

    /**
     * 验证迁移状态
     *
     * Validates migration status.
     *
     * 根据启动选项和配置属性验证所有数据库目标的迁移状态。
     * 如果启用了迁移检查且存在未应用的迁移，将抛出异常。
     *
     * Validates migration status for all database targets based on
     * bootstrap options and configuration properties.
     * Throws exception if migration check is enabled and unapplied migrations exist.
     *
     * @param options 远程求解器启动选项
     *                 Remote solver bootstrap options
     * @param properties 配置属性映射
     *                   Configuration properties map
     * @throws IllegalArgumentException 存在未应用的迁移时抛出
     *                                  Thrown when unapplied migrations exist
     */
    fun validate(
        options: RemoteSolverBootstrapOptions,
        properties: Map<String, String>
    ) {
        val enabled = parseBoolean(properties["migration.check.enabled"], defaultValue = true)
        if (!enabled) {
            return
        }
        val sqlDir = resolveSqlDir(properties["migration.check.sql-dir"])
        val expectedVersions = resolveExpectedVersions(sqlDir)
        if (expectedVersions.isEmpty()) {
            throw IllegalArgumentException("No migration files found under: $sqlDir")
        }
        val historyTable = resolveHistoryTable(properties["migration.check.history-table"])
        val dbTargets = resolveDbTargets(options)
        if (dbTargets.isEmpty()) {
            return
        }
        dbTargets.forEach { target ->
            validateTarget(target, expectedVersions, historyTable)
        }
    }

    /**
     * 验证单个数据库目标
     *
     * Validates a single database target.
     *
     * 连接到数据库并检查已应用的迁移版本，与期望版本对比。
     *
     * Connects to database and checks applied migration versions,
     * comparing with expected versions.
     *
     * @param target 数据库目标配置
     *               Database target configuration
     * @param expectedVersions 期望的迁移版本集合
     *                          Expected migration version set
     * @param historyTable 迁移历史表名
     *                     Migration history table name
     * @throws IllegalArgumentException 存在缺失的迁移版本时抛出
     *                                  Thrown when missing migration versions exist
     */
    private fun validateTarget(
        target: DbTarget,
        expectedVersions: Set<Int>,
        historyTable: String
    ) {
        DriverManager.getConnection(target.url, target.username, target.password).use { connection ->
            val applied = resolveAppliedVersions(connection, historyTable)
            val missing = (expectedVersions - applied).toSortedSet()
            if (missing.isNotEmpty()) {
                throw IllegalArgumentException(
                    "Missing migrations ${missing.joinToString(prefix = "V", separator = ",V")} " +
                        "for ${target.name} (${target.url}). " +
                        "Please run deploy/scripts/apply-migrations.sh or apply-migrations.ps1 first."
                )
            }
        }
    }

    /**
     * 解析已应用的迁移版本
     *
     * Resolves applied migration versions.
     *
     * 从迁移历史表或现有表结构推断已应用的迁移版本。
     *
     * Infers applied migration versions from migration history table
     * or existing table structures.
     *
     * @param connection 数据库连接
     *                   Database connection
     * @param historyTable 迁移历史表名
     *                     Migration history table name
     * @return 已应用的迁移版本集合
     *         Applied migration version set
     */
    private fun resolveAppliedVersions(
        connection: Connection,
        historyTable: String
    ): Set<Int> {
        if (tableExists(connection, historyTable)) {
            val sql = "select version from $historyTable"
            connection.createStatement().use { stmt ->
                stmt.executeQuery(sql).use { rs ->
                    val versions = linkedSetOf<Int>()
                    while (rs.next()) {
                        val raw = rs.getString(1)?.trim().orEmpty()
                        val parsed = raw.toIntOrNull()
                            ?: numericVersionPattern.find(raw)?.groupValues?.get(1)?.toIntOrNull()
                        if (parsed != null) {
                            versions.add(parsed)
                        }
                    }
                    return versions
                }
            }
        }

        // 从现有表结构推断迁移版本
        val inferred = linkedSetOf<Int>()
        if (allTablesExist(connection, listOf("remote_solver_task_state", "remote_solver_slice_state", "remote_solver_cost_ledger"))) {
            inferred.add(1)
        }
        if (allTablesExist(connection, listOf("remote_solver_node_state", "remote_solver_budget", "remote_solver_lock"))) {
            inferred.add(2)
        }
        if (allTablesExist(connection, listOf("remote_solver_scheduler_audit", "remote_solver_scheduler_snapshot"))) {
            inferred.add(3)
        }
        return inferred
    }

    /**
     * 解析数据库目标列表
     *
     * Resolves database target list.
     *
     * 根据启动选项中的适配器类型配置，确定需要验证迁移的数据库目标。
     *
     * Determines database targets requiring migration validation
     * based on adapter type configuration in bootstrap options.
     *
     * @param options 远程求解器启动选项
     *                 Remote solver bootstrap options
     * @return 数据库目标列表，去重后的有效连接配置
     *         Database target list, deduplicated valid connection configurations
     */
    private fun resolveDbTargets(options: RemoteSolverBootstrapOptions): List<DbTarget> {
        val targets = mutableListOf<DbTarget>()
        if (options.distributedLockAdapter == DistributedLockAdapterType.KTORM) {
            targets.add(
                DbTarget(
                    name = "distributed-lock",
                    url = options.distributedLockKtormUrl.orEmpty(),
                    username = options.distributedLockKtormUsername,
                    password = options.distributedLockKtormPassword
                )
            )
        }
        if (options.nodeStateAdapter == NodeStateAdapterType.KTORM) {
            targets.add(
                DbTarget(
                    name = "node-state",
                    url = options.nodeStateKtormUrl.orEmpty(),
                    username = options.nodeStateKtormUsername,
                    password = options.nodeStateKtormPassword
                )
            )
        }
        if (options.budgetAdapter == BudgetAdapterType.KTORM) {
            targets.add(
                DbTarget(
                    name = "budget",
                    url = options.budgetKtormUrl.orEmpty(),
                    username = options.budgetKtormUsername,
                    password = options.budgetKtormPassword
                )
            )
        }
        if (options.taskStateAdapter == TaskStateAdapterType.KTORM) {
            targets.add(
                DbTarget(
                    name = "task-state",
                    url = options.taskStateKtormUrl.orEmpty(),
                    username = options.taskStateKtormUsername,
                    password = options.taskStateKtormPassword
                )
            )
        }
        if (options.costLedgerAdapter == CostLedgerAdapterType.KTORM) {
            targets.add(
                DbTarget(
                    name = "cost-ledger",
                    url = options.costLedgerKtormUrl.orEmpty(),
                    username = options.costLedgerKtormUsername,
                    password = options.costLedgerKtormPassword
                )
            )
        }
        if (options.schedulerAuditAdapter == SchedulerAuditAdapterType.KTORM) {
            targets.add(
                DbTarget(
                    name = "scheduler-audit",
                    url = options.schedulerAuditKtormUrl.orEmpty(),
                    username = options.schedulerAuditKtormUsername,
                    password = options.schedulerAuditKtormPassword
                )
            )
        }
        return targets
            .filter { it.url.isNotBlank() }
            .distinctBy { listOf(it.url, it.username.orEmpty(), it.password.orEmpty()) }
    }

    /**
     * 解析期望的迁移版本集合
     *
     * Resolves expected migration version set.
     *
     * 从 SQL 文件目录中扫描迁移文件并提取版本号。
     *
     * Scans migration files from SQL directory and extracts version numbers.
     *
     * @param sqlDir SQL 文件目录路径
     *                SQL file directory path
     * @return 期望的迁移版本集合
     *         Expected migration version set
     */
    private fun resolveExpectedVersions(sqlDir: Path): Set<Int> =
        Files.list(sqlDir).use { stream ->
            val versions = linkedSetOf<Int>()
            stream
                .filter { Files.isRegularFile(it) }
                .forEach { path ->
                    val name = path.fileName.toString()
                    val version = migrationFilePattern.find(name)?.groupValues?.get(1)?.toIntOrNull()
                    if (version != null) {
                        versions.add(version)
                    }
                }
            versions
        }

    /**
     * 解析 SQL 文件目录路径
     *
     * Resolves SQL file directory path.
     *
     * 将配置的路径字符串转换为绝对规范化路径。
     *
     * Converts configured path string to absolute normalized path.
     *
     * @param path 配置的路径字符串，可为 null
     *             Configured path string, can be null
     * @return SQL 文件目录的绝对规范化路径
     *         Absolute normalized path of SQL file directory
     */
    private fun resolveSqlDir(path: String?): Path {
        val resolved = path?.trim()?.takeIf { it.isNotEmpty() } ?: defaultSqlDir
        return Path.of(resolved).toAbsolutePath().normalize()
    }

    /**
     * 解析迁移历史表名
     *
     * Resolves migration history table name.
     *
     * 验证并返回有效的迁移历史表名。
     *
     * Validates and returns valid migration history table name.
     *
     * @param tableName 配置的表名，可为 null
     *                   Configured table name, can be null
     * @return 迁移历史表名
     *         Migration history table name
     * @throws IllegalArgumentException 表名格式无效时抛出
     *                                  Thrown when table name format is invalid
     */
    private fun resolveHistoryTable(tableName: String?): String {
        val resolved = tableName?.trim()?.takeIf { it.isNotEmpty() } ?: defaultHistoryTable
        require(Regex("""^[A-Za-z0-9_]+$""").matches(resolved)) {
            "migration.check.history-table must match [A-Za-z0-9_]+, but was '$resolved'"
        }
        return resolved
    }

    /**
     * 检查表是否存在
     *
     * Checks if table exists.
     *
     * 通过数据库元数据检查指定表是否存在。
     * 尝试多种大小写组合以兼容不同数据库系统。
     *
     * Checks if specified table exists via database metadata.
     * Tries multiple case combinations for compatibility with different database systems.
     *
     * @param connection 数据库连接
     *                   Database connection
     * @param tableName 表名
     *                  Table name
     * @return 表是否存在
     *         Whether table exists
     */
    private fun tableExists(connection: Connection, tableName: String): Boolean {
        val candidates = linkedSetOf(
            tableName,
            tableName.lowercase(),
            tableName.uppercase()
        )
        candidates.forEach { candidate ->
            connection.metaData.getTables(null, null, candidate, arrayOf("TABLE")).use { rs ->
                if (rs.next()) {
                    return true
                }
            }
        }
        return false
    }

    /**
     * 检查所有表是否存在
     *
     * Checks if all tables exist.
     *
     * 验证指定的所有表是否都存在于数据库中。
     *
     * Validates all specified tables exist in database.
     *
     * @param connection 数据库连接
     *                   Database connection
     * @param tableNames 表名列表
     *                   Table name list
     * @return 所有表是否都存在
     *         Whether all tables exist
     */
    private fun allTablesExist(connection: Connection, tableNames: List<String>): Boolean =
        tableNames.all { tableExists(connection, it) }

    /**
     * 解析布尔值
     *
     * Parses boolean value.
     *
     * 将字符串值转换为布尔值，支持多种常见格式。
     *
     * Converts string value to boolean, supporting multiple common formats.
     *
     * @param raw 原始字符串值，可为 null
     *            Raw string value, can be null
     * @param defaultValue 默认值
     *                     Default value
     * @return 解析后的布尔值
     *         Parsed boolean value
     * @throwsIllegalArgumentException 值格式无效时抛出
     *                                 Thrown when value format is invalid
     */
    private fun parseBoolean(raw: String?, defaultValue: Boolean): Boolean {
        val normalized = raw?.trim()?.takeIf { it.isNotEmpty() } ?: return defaultValue
        return when (normalized.lowercase()) {
            "true", "1", "yes", "y", "on" -> true
            "false", "0", "no", "n", "off" -> false
            else -> throw IllegalArgumentException("Invalid boolean value: '$normalized'")
        }
    }
}