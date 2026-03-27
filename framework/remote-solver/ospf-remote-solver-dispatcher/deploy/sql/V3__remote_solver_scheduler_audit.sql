-- Remote Solver scheduler-audit schema baseline. / Remote Solver 调度审计表基线脚本。
-- Target: PostgreSQL/H2 compatibility. / 目标：兼容 PostgreSQL 与 H2。

CREATE TABLE IF NOT EXISTS remote_solver_migration_history (
    version VARCHAR(32) PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at_epoch_ms BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_solver_scheduler_audit (
    version VARCHAR(256) NOT NULL,
    previous_version VARCHAR(256) NOT NULL,
    operator VARCHAR(256) NOT NULL,
    effective_at_epoch_ms BIGINT NOT NULL,
    rollback_from_version VARCHAR(256) NULL,
    change_set_encoded TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_remote_solver_scheduler_audit_effective_at
    ON remote_solver_scheduler_audit (effective_at_epoch_ms);

CREATE TABLE IF NOT EXISTS remote_solver_scheduler_snapshot (
    version VARCHAR(256) NOT NULL,
    snapshot_properties TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_remote_solver_scheduler_snapshot_version
    ON remote_solver_scheduler_snapshot (version);

DELETE FROM remote_solver_migration_history WHERE version = '3';
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('3', 'remote_solver_scheduler_audit', 0);
