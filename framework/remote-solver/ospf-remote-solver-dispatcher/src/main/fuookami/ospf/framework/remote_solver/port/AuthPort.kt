/*
 * 认证端口接口
 *
 * Authentication Port Interface
 *
 * 该接口定义了认证和授权的核心抽象，支持 JWT 令牌验证、租户隔离和角色访问控制。
 * This interface defines the core abstraction for authentication and authorization,
 * supporting JWT token validation, tenant isolation, and role-based access control.
 *
 * 实现要求：
 * Implementation requirements:
 * - 支持 HMAC (HS256) 或 RSA (RS256) 签名验证
 *   Support HMAC (HS256) or RSA (RS256) signature verification
 * - 从令牌中提取租户 ID 和角色信息
 *   Extract tenant ID and role information from tokens
 * - 处理令牌过期和时钟偏差
 *   Handle token expiration and clock skew
 *
 * 配置参数：
 * Configuration parameters:
 * - auth.jwt.enabled: 启用 JWT 认证（默认：false）/ Enable JWT authentication (default: false)
 * - auth.jwt.secret: HMAC 签名验证的共享密钥 / HMAC secret key for signature verification
 * - auth.jwt.issuer: 预期的令牌颁发者 / Expected token issuer
 * - auth.jwt.tenant-claim: 租户 ID 的声明名称（默认：tenant_id）/ Claim name for tenant ID (default: tenant_id)
 * - auth.jwt.roles-claim: 角色的声明名称（默认：roles）/ Claim name for roles (default: roles)
 */
package fuookami.ospf.framework.remote_solver.port

/**
 * 认证端口接口
 *
 * Authentication Port Interface
 *
 * 提供认证和授权功能的端口接口。
 * Port interface providing authentication and authorization capabilities.
 */
interface AuthPort {
    /**
     * 验证 JWT 令牌并返回认证结果
     *
     * Validates a JWT token and returns authentication result.
     *
     * @param token JWT 令牌字符串（不含 "Bearer " 前缀）
     *              The JWT token string (without "Bearer " prefix)
     * @return 认证成功时返回 AuthResult，令牌无效时返回 null
     *         AuthResult if token is valid, null otherwise
     */
    suspend fun validateToken(token: String): AuthResult?

    /**
     * 从 JWT 令牌中提取租户 ID（不进行完整验证）
     *
     * Extracts tenant ID from a JWT token without full validation.
     *
     * 用于请求路由中的快速租户识别。
     * Useful for quick tenant identification in request routing.
     *
     * @param token JWT 令牌字符串 / The JWT token string
     * @return 租户 ID（如存在），否则返回 null
     *         Tenant ID if present, null otherwise
     */
    suspend fun extractTenantId(token: String): String?

    /**
     * 从 JWT 令牌中提取角色（不进行完整验证）
     *
     * Extracts roles from a JWT token without full validation.
     *
     * @param token JWT 令牌字符串 / The JWT token string
     * @return 角色名称集合，无角色或无效时返回空集合
     *         Set of role names, empty set if none or invalid
     */
    suspend fun extractRoles(token: String): Set<String>

    /**
     * 检查令牌是否具有指定角色
     *
     * Checks if a token has a specific role.
     *
     * @param token JWT 令牌字符串 / The JWT token string
     * @param role 要检查的角色名称 / The role name to check
     * @return 如果令牌具有该角色则返回 true，否则返回 false
     *         true if token has the role, false otherwise
     */
    suspend fun hasRole(token: String, role: String): Boolean
}

/**
 * 认证结果
 *
 * Authentication Result
 *
 * 包含用户身份和权限信息的认证结果数据类。
 * Data class containing user identity and permissions from authentication.
 *
 * @param userId 用户唯一标识符 / Unique user identifier
 * @param tenantId 租户唯一标识符 / Unique tenant identifier
 * @param roles 用户拥有的角色集合 / Set of roles assigned to the user
 * @param expiresAtEpochMs 令牌过期时间的毫秒级时间戳 / Token expiration timestamp in milliseconds
 * @param issuer 令牌颁发者（可选）/ Token issuer (optional)
 * @param subject 令牌主题（可选）/ Token subject (optional)
 * @param audience 令牌受众（可选）/ Token audience (optional)
 * @param issuedAtEpochMs 令牌颁发时间的毫秒级时间戳（可选）/ Token issued-at timestamp in milliseconds (optional)
 * @param jwtId 令牌唯一标识符（可选）/ Unique JWT identifier (optional)
 */
data class AuthResult(
    val userId: String,
    val tenantId: String,
    val roles: Set<String>,
    val expiresAtEpochMs: Long,
    val issuer: String? = null,
    val subject: String? = null,
    val audience: String? = null,
    val issuedAtEpochMs: Long? = null,
    val jwtId: String? = null
)

/**
 * 认证配置
 *
 * Authentication Configuration
 *
 * 配置 JWT 认证行为的参数数据类。
 * Data class for configuring JWT authentication behavior.
 *
 * HS256 (HMAC) 签名配置：
 * For HS256 (HMAC) signatures:
 * - 将 secret 设置为共享密钥 / Set secret to the shared secret key
 *
 * RS256 (RSA) 签名配置：
 * For RS256 (RSA) signatures:
 * - 将 publicKeyBase64 设置为 Base64 编码的 X509 公钥 / Set publicKeyBase64 to the Base64-encoded X509 public key
 * - 或将 publicKey 设置为 PEM 格式的公钥 / Or set publicKey to the PEM-formatted public key
 *
 * @param enabled 是否启用认证（默认：false）/ Whether authentication is enabled (default: false)
 * @param secret HMAC 签名的共享密钥（可选）/ HMAC secret key for signatures (optional)
 * @param publicKeyBase64 Base64 编码的公钥（可选）/ Base64-encoded public key (optional)
 * @param publicKey PEM 格式的公钥（可选）/ PEM-formatted public key (optional)
 * @param issuer 预期的令牌颁发者（可选）/ Expected token issuer (optional)
 * @param audience 预期的令牌受众（可选）/ Expected token audience (optional)
 * @param tenantClaim 租户 ID 的声明名称（默认：tenant_id）/ Claim name for tenant ID (default: tenant_id)
 * @param rolesClaim 角色的声明名称（默认：roles）/ Claim name for roles (default: roles)
 * @param clockSkewSeconds 允许的时钟偏差秒数（默认：60）/ Allowed clock skew in seconds (default: 60)
 * @param requireExpiration 是否要求令牌有过期时间（默认：true）/ Whether token expiration is required (default: true)
 * @param requireIssuer 是否要求令牌有颁发者（默认：false）/ Whether token issuer is required (default: false)
 */
data class AuthConfig(
    val enabled: Boolean = false,
    val secret: String? = null,
    val publicKeyBase64: String? = null,
    val publicKey: String? = null,
    val issuer: String? = null,
    val audience: String? = null,
    val tenantClaim: String = "tenant_id",
    val rolesClaim: String = "roles",
    val clockSkewSeconds: Long = 60L,
    val requireExpiration: Boolean = true,
    val requireIssuer: Boolean = false
)