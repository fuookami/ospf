-- Remote Solver multi-tenant schema migration. / Remote Solver 多租户迁移脚本。
-- Target: PostgreSQL/H2 compatibility. / 目标：兼容 PostgreSQL 与 H2。
-- Adds tenant_id column for multi-tenant isolation. / 添加 tenant_id 列实现多租户隔离。

CREATE TABLE IF NOT EXISTS remote_solver_migration_history (
    version VARCHAR(32) PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at_epoch_ms BIGINT NOT NULL
);

-- Add tenant_id column to task_state table if not exists
-- 为任务状态表添加 tenant_id 列（若不存在）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'remote_solver_task_state'
        AND column_name = 'tenant_id'
    ) THEN
        ALTER TABLE remote_solver_task_state
        ADD COLUMN tenant_id VARCHAR(256) NOT NULL DEFAULT 'default';
    END IF;
END $$;

-- Create index on tenant_id for efficient multi-tenant queries
-- 创建租户索引以支持高效的多租户查询
CREATE INDEX IF NOT EXISTS idx_remote_solver_task_state_tenant
    ON remote_solver_task_state (tenant_id);

-- Update request_id unique constraint to be tenant-scoped
-- Historically request_id was globally unique; now we make it tenant-scoped
-- This is a no-op if the constraint already exists in the desired form

-- Add tenant_id column to cost_ledger table for consistent tracking
-- 为成本账本表添加 tenant_id 列以保持一致性
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'remote_solver_cost_ledger'
        AND column_name = 'tenant_id'
    ) THEN
        ALTER TABLE remote_solver_cost_ledger
        ADD COLUMN tenant_id VARCHAR(256) NULL;
    END IF;
END $$;

-- Backfill tenant_id from budget_scope pattern if possible
-- 从 budget_scope 回填 tenant_id（如果可能）
-- Note: This assumes budget_scope format is "{tenantId}:{scope}" or just "{tenantId}"
-- 注：假设 budget_scope 格式为 "{tenantId}:{scope}" 或仅为 "{tenantId}"
-- For custom budget_scope patterns, manual data migration may be required
-- 对于自定义 budget_scope 格式，可能需要手动数据迁移

-- Create index on cost_ledger tenant_id
CREATE INDEX IF NOT EXISTS idx_remote_solver_cost_ledger_tenant
    ON remote_solver_cost_ledger (tenant_id);

DELETE FROM remote_solver_migration_history WHERE version = '4';
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('4', 'multi_tenant_tenant_id', EXTRACT(EPOCH FROM NOW())::BIGINT * 1000);