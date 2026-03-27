-- Remote Solver multi-tenant rollback script. / Remote Solver 多租户回滚脚本。
-- Target: PostgreSQL/H2 compatibility. / 目标：兼容 PostgreSQL 与 H2。
-- Removes tenant_id columns added in V4. / 移除 V4 添加的 tenant_id 列。
-- CAUTION: This will DROP tenant_id columns. Ensure data is backed up. / 警告：这将删除 tenant_id 列，请确保数据已备份。

CREATE TABLE IF NOT EXISTS remote_solver_migration_history (
    version VARCHAR(32) PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at_epoch_ms BIGINT NOT NULL
);

-- Drop tenant_id index from task_state
DROP INDEX IF EXISTS idx_remote_solver_task_state_tenant;

-- Drop tenant_id column from task_state
ALTER TABLE remote_solver_task_state DROP COLUMN IF EXISTS tenant_id;

-- Drop tenant_id index from cost_ledger
DROP INDEX IF EXISTS idx_remote_solver_cost_ledger_tenant;

-- Drop tenant_id column from cost_ledger
ALTER TABLE remote_solver_cost_ledger DROP COLUMN IF EXISTS tenant_id;

-- Remove migration record
DELETE FROM remote_solver_migration_history WHERE version = '4';

-- Record rollback
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('4_rollback', 'multi_tenant_tenant_id_rollback', EXTRACT(EPOCH FROM NOW())::BIGINT * 1000);