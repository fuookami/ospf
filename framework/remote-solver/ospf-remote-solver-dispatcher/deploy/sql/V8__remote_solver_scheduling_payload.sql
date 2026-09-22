-- Persist the complete SolvePayload.scheduling object for restart recovery.
-- 持久化完整 SolvePayload.scheduling，以便重启后无损恢复。

ALTER TABLE remote_solver_task_state
    ADD COLUMN IF NOT EXISTS payload_scheduling_json TEXT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_remote_solver_scheduler_audit_version
    ON remote_solver_scheduler_audit (version);

CREATE UNIQUE INDEX IF NOT EXISTS idx_remote_solver_scheduler_snapshot_version
    ON remote_solver_scheduler_snapshot (version);

DELETE FROM remote_solver_migration_history WHERE version = '8';
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('8', 'remote_solver_scheduling_payload_and_version_uniqueness', 0);
