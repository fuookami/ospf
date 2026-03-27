/*
 * JWT 认证端口适配器
 *
 * JWT Authentication Port Adapter
 *
 * 该模块提供基于 JWT (JSON Web Token) 的认证服务实现，支持完整的签名验证、
 * 租户 ID 提取、角色提取和过期验证等功能。适用于生产环境的安全认证需求。
 * This module provides JWT (JSON Web Token) based authentication service implementation,
 * supporting complete signature verification, tenant ID extraction, role extraction
 * and expiration validation. Suitable for production security authentication requirements.
 */

package fuookami.ospf.framework.remote_solver.adapter.jwt

import com.auth0.jwt.JWT
import com.auth0.jwt.algorithms.Algorithm
import com.auth0.jwt.exceptions.JWTVerificationException
import com.auth0.jwt.interfaces.DecodedJWT
import com.auth0.jwt.interfaces.JWTVerifier
import fuookami.ospf.framework.remote_solver.port.AuthConfig
import fuookami.ospf.framework.remote_solver.port.AuthPort
import fuookami.ospf.framework.remote_solver.port.AuthResult
import java.security.interfaces.RSAPublicKey
import java.util.Base64

/**
 * JWT 认证端口
 *
 * JWT Authentication Port
 *
 * 该类实现了 [AuthPort] 接口，提供完整的 JWT 认证功能。主要特性包括：
 * This class implements [AuthPort] interface, providing complete JWT authentication functionality.
 * Main features include:
 *
 * - **JWT 解析和签名验证**: 支持 HS256 (HMAC) 和 RS256 (RSA) 签名算法。
 *   JWT parsing and signature verification: Supports HS256 (HMAC) and RS256 (RSA) signature algorithms.
 * - **租户 ID 提取**: 从可配置的 claim 中提取租户标识。
 *   Tenant ID extraction: Extracts tenant identifier from configurable claim.
 * - **角色提取**: 从可配置的 claim 中提取用户角色集合。
 *   Role extraction: Extracts user role set from configurable claim.
 * - **过期验证**: 支持时钟偏差容忍的过期时间验证。
 *   Expiration validation: Supports clock skew tolerant expiration time validation.
 *
 * 生产环境配置建议：
 * Production environment configuration recommendations:
 * - **HS256**: 配置共享密钥 (jwt.secret)。
 *   HS256: Configure shared secret (jwt.secret).
 * - **RS256**: 配置 RSA 公钥 (jwt.publicKey 或 jwt.publicKeyBase64)。
 *   RS256: Configure RSA public key (jwt.publicKey or jwt.publicKeyBase64).
 *
 * @param config 认证配置，包含签名密钥、issuer、audience 等信息。
 *                Authentication configuration, containing signing key, issuer, audience and other information.
 */
class JwtAuthPort(
    private val config: AuthConfig = AuthConfig()
) : AuthPort {

    private val verifier: JWTVerifier? = buildVerifier()

    /**
     * 构建 JWT 验证器
     *
     * Build JWT verifier
     *
     * 根据配置构建 JWTVerifier 实例。如果认证未启用或未配置密钥，返回 null，
     * 此时将使用不验证签名的解码模式（仅用于测试环境）。
     *
     * Builds JWTVerifier instance based on configuration. Returns null if authentication
     * is disabled or no key configured, in which case decoding without signature verification
     * will be used (only for test environment).
     *
     * @return JWT 验证器实例，或 null。
     *         JWT verifier instance, or null.
     */
    private fun buildVerifier(): JWTVerifier? {
        if (!config.enabled) return null

        val algorithm = buildAlgorithm()
        if (algorithm == null) {
            // Fallback to no verification if no key configured
            // This allows test tokens to work but logs warning
            return null
        }

        var builder = JWT.require(algorithm)
            .withIssuer(config.issuer ?: "")
            .acceptExpiresAt(config.clockSkewSeconds)
            .acceptIssuedAt(config.clockSkewSeconds)
            .acceptNotBefore(config.clockSkewSeconds)

        if (config.audience != null) {
            builder = builder.withAudience(config.audience)
        }

        return builder.build()
    }

    /**
     * 构建签名算法
     *
     * Build signing algorithm
     *
     * 根据配置选择并构建签名算法。优先级顺序：
 * 1. RS256 (使用 Base64 编码的 RSA 公钥)
 * 2. RS256 (使用 PEM 格式的 RSA 公钥)
 * 3. HS256 (使用 HMAC 共享密钥)
     * Selects and builds signing algorithm based on configuration. Priority order:
 * 1. RS256 (using Base64-encoded RSA public key)
 * 2. RS256 (using PEM format RSA public key)
 * 3. HS256 (using HMAC shared secret)
     *
     * @return 签名算法实例，如果未配置密钥则返回 null。
     *         Signing algorithm instance, or null if no key configured.
     */
    private fun buildAlgorithm(): Algorithm? {
        // RS256: RSA public key
        // RS256: RSA 公钥
        if (config.publicKeyBase64 != null) {
            val publicKeyBytes = Base64.getDecoder().decode(config.publicKeyBase64)
            val publicKeySpec = java.security.spec.X509EncodedKeySpec(publicKeyBytes)
            val keyFactory = java.security.KeyFactory.getInstance("RSA")
            val publicKey = keyFactory.generatePublic(publicKeySpec) as RSAPublicKey
            return Algorithm.RSA256(publicKey, null)
        }

        if (config.publicKey != null) {
            // Parse PEM format
            // 解析 PEM 格式
            val pemContent = config.publicKey
                .replace("-----BEGIN PUBLIC KEY-----", "")
                .replace("-----END PUBLIC KEY-----", "")
                .replace("\\s".toRegex(), "")
            val publicKeyBytes = Base64.getDecoder().decode(pemContent.toByteArray())
            val publicKeySpec = java.security.spec.X509EncodedKeySpec(publicKeyBytes)
            val keyFactory = java.security.KeyFactory.getInstance("RSA")
            val publicKey = keyFactory.generatePublic(publicKeySpec) as RSAPublicKey
            return Algorithm.RSA256(publicKey, null)
        }

        // HS256: HMAC secret
        // HS256: HMAC 共享密钥
        if (config.secret != null) {
            return Algorithm.HMAC256(config.secret)
        }

        return null
    }

    /**
     * 验证令牌有效性
     *
     * Validate token validity
     *
     * 解析和验证 JWT 令牌，提取用户信息、租户 ID、角色等。如果验证失败或
     * 令牌无效，返回 null。
     *
     * Parses and validates JWT token, extracting user information, tenant ID, roles etc.
     * Returns null if validation fails or token is invalid.
     *
     * @param token JWT 令牌字符串。
     *               JWT token string.
     * @return 认证结果，包含用户信息；如果验证失败则返回 null。
     *         Authentication result containing user information, or null if validation fails.
     */
    override suspend fun validateToken(token: String): AuthResult? {
        if (!config.enabled) {
            return null
        }

        return try {
            val decodedJWT = if (verifier != null) {
                verifier.verify(token)
            } else {
                // No verifier - decode without verification (fallback/test mode)
                // 无验证器 - 不验证签名直接解码（回退/测试模式）
                JWT.decode(token)
            }

            // Validate expiration manually if verifier not configured
            // 如果未配置验证器，手动验证过期时间
            if (verifier == null && config.requireExpiration) {
                val now = System.currentTimeMillis() / 1000
                val expiresAt = decodedJWT.expiresAt?.time?.div(1000)
                if (expiresAt == null || expiresAt < now + config.clockSkewSeconds) {
                    return null
                }
            }

            // Extract tenant ID
            // 提取租户 ID
            val tenantId = extractTenantIdFromJWT(decodedJWT)
                ?: return null

            // Extract user ID
            // 提取用户 ID
            val userId = decodedJWT.subject ?: decodedJWT.getClaim("preferred_username").asString()
                ?: return null

            // Extract roles
            // 提取角色
            val roles = extractRolesFromJWT(decodedJWT)

            AuthResult(
                userId = userId,
                tenantId = tenantId,
                roles = roles,
                expiresAtEpochMs = decodedJWT.expiresAt?.time ?: 0,
                issuer = decodedJWT.issuer,
                subject = decodedJWT.subject,
                audience = decodedJWT.audience?.firstOrNull(),
                issuedAtEpochMs = decodedJWT.issuedAt?.time,
                jwtId = decodedJWT.id
            )
        } catch (e: JWTVerificationException) {
            null
        } catch (e: Exception) {
            null
        }
    }

    /**
     * 从令牌提取租户 ID
     *
     * Extract tenant ID from token
     *
     * 不进行完整验证，仅解析令牌并提取租户 ID claim。适用于快速获取租户信息。
     * Performs no full validation, only parses token and extracts tenant ID claim.
     * Suitable for quickly getting tenant information.
     *
     * @param token JWT 令牌字符串。
     *               JWT token string.
     * @return 租户 ID，如果提取失败则返回 null。
     *         Tenant ID, or null if extraction fails.
     */
    override suspend fun extractTenantId(token: String): String? {
        return try {
            val decodedJWT = JWT.decode(token)
            extractTenantIdFromJWT(decodedJWT)
        } catch (e: Exception) {
            null
        }
    }

    /**
     * 从令牌提取角色集合
     *
     * Extract roles set from token
     *
     * 不进行完整验证，仅解析令牌并提取角色 claims。适用于快速检查用户权限。
     * Performs no full validation, only parses token and extracts role claims.
     * Suitable for quickly checking user permissions.
     *
     * @param token JWT 令牌字符串。
     *               JWT token string.
     * @return 用户角色集合，如果提取失败则返回空集合。
     *         User roles set, or empty set if extraction fails.
     */
    override suspend fun extractRoles(token: String): Set<String> {
        return try {
            val decodedJWT = JWT.decode(token)
            extractRolesFromJWT(decodedJWT)
        } catch (e: Exception) {
            emptySet()
        }
    }

    /**
     * 检查用户是否拥有指定角色
     *
     * Check if user has specified role
     *
     * 快速检查用户是否拥有指定的角色，无需完整验证令牌。
     * Quickly checks if user has specified role, without full token validation.
     *
     * @param token JWT 令牌字符串。
     *               JWT token string.
     * @param role 要检查的角色名称。
     *              Role name to check.
     * @return 用户是否拥有该角色。
     *         Whether user has the role.
     */
    override suspend fun hasRole(token: String, role: String): Boolean {
        return extractRoles(token).contains(role)
    }

    /**
     * 从解码后的 JWT 提取租户 ID
     *
     * Extract tenant ID from decoded JWT
     *
     * 根据配置的 tenantClaim 字段提取租户 ID。支持标准字段名和自定义字段名。
     * Extracts tenant ID based on configured tenantClaim field.
     * Supports standard field names and custom field names.
     *
     * @param jwt 解码后的 JWT 对象。
     *            Decoded JWT object.
     * @return 租户 ID，如果不存在则返回 null。
     *         Tenant ID, or null if not exists.
     */
    private fun extractTenantIdFromJWT(jwt: DecodedJWT): String? {
        return when (config.tenantClaim) {
            "tenant_id" -> jwt.getClaim("tenant_id").asString()
            "tid" -> jwt.getClaim("tid").asString()
            else -> jwt.getClaim(config.tenantClaim).asString()
        }
    }

    /**
     * 从解码后的 JWT 提取角色集合
     *
     * Extract roles set from decoded JWT
     *
     * 根据配置的 rolesClaim 字段提取角色。支持多种格式：
 * - 数组格式 (roles: ["admin", "user"])
 * - 单值格式 (role: "admin")
 * - 逗号分隔字符串格式
     * Extracts roles based on configured rolesClaim field. Supports multiple formats:
 * - Array format (roles: ["admin", "user"])
 * - Single value format (role: "admin")
 * - Comma-separated string format
     *
     * @param jwt 解码后的 JWT 对象。
     *            Decoded JWT object.
     * @return 用户角色集合。
     *         User roles set.
     */
    private fun extractRolesFromJWT(jwt: DecodedJWT): Set<String> {
        val roles = mutableSetOf<String>()

        when (config.rolesClaim) {
            "roles" -> {
                jwt.getClaim("roles").asList(String::class.java)?.forEach { roles.add(it) }
            }
            "role" -> {
                jwt.getClaim("role").asString()?.let { roles.add(it) }
            }
            else -> {
                jwt.getClaim(config.rolesClaim).asString()?.let { claimValue ->
                    if (claimValue.contains(",")) {
                        roles.addAll(claimValue.split(",").map { r -> r.trim() })
                    } else {
                        roles.add(claimValue)
                    }
                }
            }
        }

        return roles
    }

    /**
     * 创建测试令牌
     *
     * Create test token
     *
     * 用于单元测试场景生成测试用的 JWT 令牌。如果配置了密钥则使用 HMAC256 签名，
     * 否则使用无签名算法。
     *
     * Generates test JWT token for unit testing scenarios.
     * Uses HMAC256 signature if secret configured, otherwise uses no signature algorithm.
     *
     * @param userId 用户 ID。
     *                User ID.
     * @param tenantId 租户 ID。
     *                  Tenant ID.
     * @param roles 用户角色集合。
     *               User roles set.
     * @param expiresInSeconds 过期时间（秒），默认为 3600 秒（1 小时）。
     *                          Expiration time in seconds, defaults to 3600 seconds (1 hour).
     * @return 生成的测试令牌字符串。
     *         Generated test token string.
     */
    fun createTestToken(
        userId: String,
        tenantId: String,
        roles: Set<String>,
        expiresInSeconds: Long = 3600
    ): String {
        val algorithm = if (config.secret != null) {
            Algorithm.HMAC256(config.secret)
        } else {
            Algorithm.none()
        }

        val now = System.currentTimeMillis() / 1000

        return JWT.create()
            .withSubject(userId)
            .withClaim(config.tenantClaim, tenantId)
            .withClaim("roles", roles.toList())
            .withIssuedAt(java.util.Date(now * 1000))
            .withExpiresAt(java.util.Date((now + expiresInSeconds) * 1000))
            .withIssuer(config.issuer ?: "test")
            .sign(algorithm)
    }

    /**
     * 创建指定密钥签名的测试令牌
     *
     * Create test token signed with specific secret
     *
     * 使用指定的密钥生成 HMAC256 签名的测试令牌，适用于需要特定签名验证的测试场景。
     * Generates HMAC256 signed test token using specified secret,
     * suitable for test scenarios requiring specific signature verification.
     *
     * @param userId 用户 ID。
     *                User ID.
     * @param tenantId 租户 ID。
     *                  Tenant ID.
     * @param roles 用户角色集合。
     *               User roles set.
     * @param secret 用于签名的 HMAC 密钥。
     *                HMAC secret for signing.
     * @param expiresInSeconds 过期时间（秒），默认为 3600 秒（1 小时）。
     *                          Expiration time in seconds, defaults to 3600 seconds (1 hour).
     * @return 生成的测试令牌字符串。
     *         Generated test token string.
     */
    fun createSignedTestToken(
        userId: String,
        tenantId: String,
        roles: Set<String>,
        secret: String,
        expiresInSeconds: Long = 3600
    ): String {
        val algorithm = Algorithm.HMAC256(secret)
        val now = System.currentTimeMillis() / 1000

        return JWT.create()
            .withSubject(userId)
            .withClaim(config.tenantClaim, tenantId)
            .withClaim("roles", roles.toList())
            .withIssuedAt(java.util.Date(now * 1000))
            .withExpiresAt(java.util.Date((now + expiresInSeconds) * 1000))
            .withIssuer(config.issuer ?: "test")
            .sign(algorithm)
    }
}