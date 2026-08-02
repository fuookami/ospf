-- Persist ObjectRef ETags for CP payload, result, snapshot, and checkpoint recovery. / 持久化 CP 载荷、结果、快照和 checkpoint 的 ObjectRef ETag。

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_task_state' AND column_name = 'payload_model_etag') THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN payload_model_etag TEXT NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_task_state' AND column_name = 'payload_config_etag') THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN payload_config_etag TEXT NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_task_state' AND column_name = 'payload_snapshot_etag') THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN payload_snapshot_etag TEXT NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_task_state' AND column_name = 'latest_result_checkpoint_etag') THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN latest_result_checkpoint_etag TEXT NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_task_state' AND column_name = 'latest_result_result_etag') THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN latest_result_result_etag TEXT NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_task_state' AND column_name = 'latest_snapshot_etag') THEN
        ALTER TABLE remote_solver_task_state ADD COLUMN latest_snapshot_etag TEXT NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_slice_state' AND column_name = 'checkpoint_etag') THEN
        ALTER TABLE remote_solver_slice_state ADD COLUMN checkpoint_etag TEXT NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'remote_solver_slice_state' AND column_name = 'result_etag') THEN
        ALTER TABLE remote_solver_slice_state ADD COLUMN result_etag TEXT NULL;
    END IF;
END $$;

DELETE FROM remote_solver_migration_history WHERE version = '7';
INSERT INTO remote_solver_migration_history(version, description, applied_at_epoch_ms)
VALUES ('7', 'object_ref_etag_persistence', 0);
