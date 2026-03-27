# Database Migrations / 数据库迁移

## Overview / 概述

This directory contains SQL migration scripts for the Remote Solver service.
本目录包含 Remote Solver 服务的 SQL 迁移脚本。

## Migration Files / 迁移文件

| Version | File | Description |
|---------|------|-------------|
| V1 | `V1__remote_solver_core.sql` | Core tables (task_state, slice_state, cost_ledger) |
| V2 | `V2__remote_solver_infra.sql` | Infrastructure tables (node_state, budget, lock) |
| V3 | `V3__remote_solver_scheduler_audit.sql` | Scheduler audit tables |
| V4 | `V4__remote_solver_multi_tenant.sql` | Multi-tenant support (tenant_id columns) |

## Applying Migrations / 应用迁移

### Using Shell Script (Linux/macOS)

```bash
./deploy/scripts/apply-migrations.sh "postgresql://user:password@host:port/database"
```

### Using PowerShell (Windows)

```powershell
.\deploy\scripts\apply-migrations.ps1 -DbUrl "postgresql://user:password@host:port/database"
```

### Manual Application / 手动应用

```bash
psql "postgresql://user:password@host:port/database" -f deploy/sql/V1__remote_solver_core.sql
psql "postgresql://user:password@host:port/database" -f deploy/sql/V2__remote_solver_infra.sql
psql "postgresql://user:password@host:port/database" -f deploy/sql/V3__remote_solver_scheduler_audit.sql
psql "postgresql://user:password@host:port/database" -f deploy/sql/V4__remote_solver_multi_tenant.sql
```

## V4 Multi-Tenant Migration / V4 多租户迁移

### What It Does / 迁移内容

V4 adds `tenant_id` column to support multi-tenant data isolation:
V4 添加 `tenant_id` 列以支持多租户数据隔离：

1. `remote_solver_task_state.tenant_id` - Task tenant identification
2. `remote_solver_cost_ledger.tenant_id` - Cost tracking by tenant

### Default Value / 默认值

- Existing tasks will have `tenant_id = 'default'`
- 现有任务将有 `tenant_id = 'default'`

### Rollback / 回滚

If you need to rollback V4 migration:
如需回滚 V4 迁移：

```bash
psql "postgresql://user:password@host:port/database" -f deploy/sql/V4_rollback__remote_solver_multi_tenant.sql
```

**⚠️ Warning:** Rollback will DROP the `tenant_id` column and all tenant data in that column will be lost.
**⚠️ 警告：** 回滚将删除 `tenant_id` 列，该列中的所有租户数据将丢失。

## Important Notes / 注意事项

1. **Idempotency**: All migration scripts are idempotent (can be run multiple times safely).
   **幂等性**：所有迁移脚本都是幂等的（可以安全地多次运行）。

2. **Order**: Migrations must be applied in order (V1 → V2 → V3 → V4).
   **顺序**：迁移必须按顺序应用（V1 → V2 → V3 → V4）。

3. **Backup**: Always backup your database before applying migrations.
   **备份**：应用迁移前请务必备份数据库。

4. **Downtime**: V4 migration is non-blocking but may take time for large tables.
   **停机时间**：V4 迁移是非阻塞的，但对于大表可能需要一些时间。

## Troubleshooting / 故障排除

### Column already exists / 列已存在

If you see "column already exists" errors, it means the migration was partially applied. The scripts use `IF NOT EXISTS` checks, so this is safe to ignore.
如果看到"列已存在"错误，表示迁移已部分应用。脚本使用 `IF NOT EXISTS` 检查，可以安全忽略。

### Index already exists / 索引已存在

Similar to column errors, index creation uses `IF NOT EXISTS` and is safe to ignore.
与列错误类似，索引创建使用 `IF NOT EXISTS`，可以安全忽略。

## Migration History Table / 迁移历史表

All migrations record their application in `remote_solver_migration_history`:
所有迁移都会在 `remote_solver_migration_history` 表中记录：

```sql
SELECT * FROM remote_solver_migration_history ORDER BY version;
```