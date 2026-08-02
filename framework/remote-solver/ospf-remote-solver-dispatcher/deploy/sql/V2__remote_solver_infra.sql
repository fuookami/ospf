-- Remote Solver infrastructure schema baseline. / Remote Solver 基础设施表基线脚本。
-- Target: PostgreSQL/H2 compatibility. / 目标：兼容 PostgreSQL 与 H2。

CREATE TABLE IF NOT EXISTS remote_solver_migration_history (
    version VARCHAR(32) PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at_epoch_ms BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_solver_node_state (
    node_id VARCHAR(256) PRIMARY KEY,
    solver_type VARCHAR(128) NOT NULL,
    performance_score DOUBLE PRECISION NOT NULL,
    price_per_second DOUBLE PRECISION NOT NULL,
    min_billing_unit_seconds BIGINT NOT NULL,
    supports_interrupt BOOLEAN NOT NULL,
    supports_checkpoint BOOLEAN NOT NULL,
    supports_warm_start BOOLEAN NOT NULL,
    parallel_units INT NOT NULL,
    license_cost_per_slice DOUBLE PRECISION NOT NULL,
    supported_model_types TEXT NOT NULL DEFAULT 'LINEAR,QUADRATIC',
    available_units INT NOT NULL,
    last_heartbeat_epoch_ms BIGINT NOT NULL,
    online BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_solver_budget (
    scope VARCHAR(256) PRIMARY KEY,
    limit_value DOUBLE PRECISION NOT NULL,
    consumed DOUBLE PRECISION NOT NULL,
    updated_at_epoch_ms BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_solver_lock (
    lock_key VARCHAR(512) PRIMARY KEY,
    owner_id VARCHAR(256) NOT NULL,
    lease_id VARCHAR(256) NOT NULL,
    expires_at_epoch_ms BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_remote_solver_lock_expires
    ON remote_solver_lock (expires_at_epoch_ms);

DELETE FROM remote_solver_migration_history WHERE version = '2';
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('2', 'remote_solver_infra', 0);
