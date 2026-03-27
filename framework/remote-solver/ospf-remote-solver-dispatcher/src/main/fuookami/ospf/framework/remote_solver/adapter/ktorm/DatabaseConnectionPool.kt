/**
 * 数据库连接池配置模块
 *
 * 本模块提供基于 HikariCP 的生产级数据库连接池配置，
 * 用于优化远程求解器调度器的数据库访问性能。
 *
 * Database Connection Pool Configuration Module
 *
 * This module provides production-grade database connection pool configuration
 * based on HikariCP, used to optimize database access performance for the
 * remote solver dispatcher.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import org.ktorm.database.Database
import java.sql.DriverManager
import javax.sql.DataSource
import com.zaxxer.hikari.HikariConfig
import com.zaxxer.hikari.HikariDataSource

/**
 * 数据库连接池配置对象
 *
 * 用于生产环境的数据库连接池配置，提供优化的连接池管理：
 * - 连接池以提高吞吐量
 * - 连接验证和超时设置
 * - 连接池健康监控指标
 *
 * Database Connection Pool Configuration Object
 *
 * Configuration for production-grade database connection pooling,
 * providing optimized connection pool management:
 * - Connection pooling for better throughput
 * - Connection validation and timeout settings
 * - Metrics for pool health monitoring
 */
object DatabaseConnectionPool {

    /**
     * 创建池化数据库连接
     *
     * 基于配置创建带有 HikariCP 连接池的数据库实例，
     * 适用于高并发生产环境。
     *
     * @param config 连接池配置参数
     * @return 带有池化连接的数据库实例
     *
     * Creates a pooled database connection
     *
     * Creates a Database instance with HikariCP connection pooling
     * based on the provided configuration, suitable for high-concurrency
     * production environments.
     *
     * @param config Pool configuration parameters
     * @return Database instance with pooled connections
     */
    fun createPooledDatabase(config: PoolConfig): Database {
        val hikariConfig = HikariConfig()

        // Connection settings
        // 连接设置
        hikariConfig.jdbcUrl = config.jdbcUrl
        hikariConfig.username = config.username
        hikariConfig.password = config.password
        hikariConfig.driverClassName = config.driverClassName

        // Pool sizing
        // 池大小设置
        hikariConfig.maximumPoolSize = config.maxPoolSize
        hikariConfig.minimumIdle = config.minIdle

        // Timeout settings
        // 超时设置
        hikariConfig.connectionTimeout = config.connectionTimeoutMs
        hikariConfig.idleTimeout = config.idleTimeoutMs
        hikariConfig.maxLifetime = config.maxLifetimeMs

        // Validation
        // 验证设置
        hikariConfig.validationTimeout = config.validationTimeoutMs
        hikariConfig.leakDetectionThreshold = config.leakDetectionThresholdMs

        // Performance optimization
        // 性能优化设置
        hikariConfig.poolName = config.poolName
        hikariConfig.connectionTestQuery = config.validationQuery

        // Prepared statement caching
        // 预编译语句缓存
        hikariConfig.addDataSourceProperty("cachePrepStmts", "true")
        hikariConfig.addDataSourceProperty("prepStmtCacheSize", config.preparedStatementCacheSize.toString())
        hikariConfig.addDataSourceProperty("prepStmtCacheSqlLimit", config.preparedStatementCacheSqlLimit.toString())

        val dataSource: DataSource = HikariDataSource(hikariConfig)

        return Database.connect(dataSource = dataSource)
    }

    /**
     * 创建简单数据库连接
     *
     * 创建不带连接池的数据库连接，适用于开发或测试环境。
     *
     * @param jdbcUrl JDBC 连接 URL
     * @param username 数据库用户名
     * @param password 数据库密码
     * @return 数据库实例
     *
     * Creates a simple database connection (for development/testing)
     *
     * Creates a database connection without connection pooling,
     * suitable for development or testing environments.
     *
     * @param jdbcUrl JDBC URL
     * @param username Database username
     * @param password Database password
     * @return Database instance
     */
    fun createSimpleDatabase(
        jdbcUrl: String,
        username: String = "",
        password: String = ""
    ): Database {
        return Database.connect(
            url = jdbcUrl,
            user = username,
            password = password
        )
    }
}

/**
 * 连接池配置数据类
 *
 * 封装数据库连接池的所有配置参数，包括连接信息、
 * 池大小、超时设置和性能优化参数。
 *
 * Connection Pool Configuration Data Class
 *
 * Encapsulates all configuration parameters for database connection pooling,
 * including connection information, pool sizing, timeout settings,
 * and performance optimization parameters.
 *
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名
 * @param password 数据库密码
 * @param driverClassName 驱动类名，默认为 PostgreSQL 驱动
 * @param poolName 连接池名称
 * @param maxPoolSize 最大连接数
 * @param minIdle 最小空闲连接数
 * @param connectionTimeoutMs 连接超时时间（毫秒）
 * @param idleTimeoutMs 空闲超时时间（毫秒）
 * @param maxLifetimeMs 连接最大生命周期（毫秒）
 * @param validationTimeoutMs 验证超时时间（毫秒）
 * @param leakDetectionThresholdMs 连接泄漏检测阈值（毫秒）
 * @param validationQuery 验证查询语句
 * @param preparedStatementCacheSize 预编译语句缓存大小
 * @param preparedStatementCacheSqlLimit 预编译语句 SQL 限制
 */
data class PoolConfig(
    val jdbcUrl: String,
    val username: String = "",
    val password: String = "",
    val driverClassName: String = "org.postgresql.Driver",
    val poolName: String = "remote-solver-pool",
    val maxPoolSize: Int = 20,
    val minIdle: Int = 5,
    val connectionTimeoutMs: Long = 30000L,
    val idleTimeoutMs: Long = 600000L,
    val maxLifetimeMs: Long = 1800000L,
    val validationTimeoutMs: Long = 5000L,
    val leakDetectionThresholdMs: Long = 0L,
    val validationQuery: String = "SELECT 1",
    val preparedStatementCacheSize: Int = 250,
    val preparedStatementCacheSqlLimit: Int = 2048
) {
    companion object {
        /**
         * 默认生产环境配置
         *
         * 创建适用于一般生产环境的连接池配置。
         *
         * @param jdbcUrl JDBC 连接 URL
         * @param username 数据库用户名
         * @param password 数据库密码
         * @return 默认配置实例
         *
         * Default production configuration
         *
         * Creates a connection pool configuration suitable for
         * general production environments.
         *
         * @param jdbcUrl JDBC URL
         * @param username Database username
         * @param password Database password
         * @return Default configuration instance
         */
        fun default(jdbcUrl: String, username: String, password: String): PoolConfig {
            return PoolConfig(
                jdbcUrl = jdbcUrl,
                username = username,
                password = password,
                maxPoolSize = 20,
                minIdle = 5
            )
        }

        /**
         * 高吞吐量配置
         *
         * 创建适用于高负载场景的连接池配置，
         * 具有更大的连接池容量。
         *
         * @param jdbcUrl JDBC 连接 URL
         * @param username 数据库用户名
         * @param password 数据库密码
         * @return 高吞吐量配置实例
         *
         * High throughput configuration for heavy load
         *
         * Creates a connection pool configuration suitable for
         * high-load scenarios, with larger pool capacity.
         *
         * @param jdbcUrl JDBC URL
         * @param username Database username
         * @param password Database password
         * @return High throughput configuration instance
         */
        fun highThroughput(jdbcUrl: String, username: String, password: String): PoolConfig {
            return PoolConfig(
                jdbcUrl = jdbcUrl,
                username = username,
                password = password,
                maxPoolSize = 50,
                minIdle = 10,
                poolName = "remote-solver-high-throughput"
            )
        }

        /**
         * 开发环境配置
         *
         * 创建适用于开发环境的最小化连接池配置，
         * 具有较小的连接池容量。
         *
         * @param jdbcUrl JDBC 连接 URL
         * @param username 数据库用户名，可选
         * @param password 数据库密码，可选
         * @return 开发环境配置实例
         *
         * Development configuration with minimal pooling
         *
         * Creates a connection pool configuration suitable for
         * development environments, with minimal pool capacity.
         *
         * @param jdbcUrl JDBC URL
         * @param username Database username, optional
         * @param password Database password, optional
         * @return Development configuration instance
         */
        fun development(jdbcUrl: String, username: String = "", password: String = ""): PoolConfig {
            return PoolConfig(
                jdbcUrl = jdbcUrl,
                username = username,
                password = password,
                maxPoolSize = 5,
                minIdle = 1,
                poolName = "remote-solver-dev"
            )
        }
    }
}