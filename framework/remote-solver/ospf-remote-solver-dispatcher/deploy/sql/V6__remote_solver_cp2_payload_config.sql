-- Persist inline solver configuration for CP resume. / 持久化 CP 恢复所需的内联求解配置。

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'remote_solver_task_state'
          AND column_name = 'payload_config_json'
    ) THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN payload_config_json TEXT NULL;
    END IF;
END $$;

DELETE FROM remote_solver_migration_history WHERE version = '6';
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('6', 'cp2_inline_solver_config_persistence', 0);
