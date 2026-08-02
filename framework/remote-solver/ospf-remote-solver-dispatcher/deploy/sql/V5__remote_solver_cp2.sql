-- CP2 protocol and capability persistence. / CP2 协议与能力持久化。

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'remote_solver_node_state'
          AND column_name = 'supported_model_types'
    ) THEN
        ALTER TABLE remote_solver_node_state
            ADD COLUMN supported_model_types TEXT NOT NULL DEFAULT 'LINEAR,QUADRATIC';
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'remote_solver_task_state'
          AND column_name = 'payload_model_format'
    ) THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN payload_model_format TEXT NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'remote_solver_task_state'
          AND column_name = 'latest_result_report_json'
    ) THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN latest_result_report_json TEXT NULL;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_remote_solver_task_state_model_type
    ON remote_solver_task_state (payload_task_meta_target_type);

DELETE FROM remote_solver_migration_history WHERE version = '5';
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('5', 'cp2_capability_and_report_support', 0);
