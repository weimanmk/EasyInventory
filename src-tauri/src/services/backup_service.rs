use crate::db;
use crate::models::{BackupDto, RestoreBackupResultDto};
use crate::repositories::backup_repository;
use crate::services::audit_service::{record_audit, AuditEvent};
use std::path::{Path, PathBuf};

pub fn list_backups(conn: &rusqlite::Connection) -> anyhow::Result<Vec<BackupDto>> {
    backup_repository::list_backups(conn)
}

pub fn restore_backup(
    conn: &rusqlite::Connection,
    backup_id: i64,
    db_path: &Path,
    backups_dir: &Path,
) -> anyhow::Result<RestoreBackupResultDto> {
    if backup_id <= 0 {
        anyhow::bail!("备份记录不合法");
    }
    let backup_path_text = backup_repository::successful_backup_path(conn, backup_id)?;
    let backup_path = PathBuf::from(&backup_path_text);
    if !backup_path.exists() {
        anyhow::bail!("备份文件不存在");
    }

    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let snapshot_path = backups_dir.join(format!("pre_restore_{stamp}.db"));
    db::restore_database_file(db_path, &backup_path, &snapshot_path)?;

    Ok(RestoreBackupResultDto {
        restored_backup_path: backup_path.to_string_lossy().to_string(),
        pre_restore_backup_path: snapshot_path.to_string_lossy().to_string(),
        message: "数据库恢复完成，请重新打开应用确认数据".to_string(),
    })
}

pub fn finalize_restore(
    conn: &rusqlite::Connection,
    backup_id: i64,
    result: &RestoreBackupResultDto,
) -> anyhow::Result<()> {
    db::init_schema(conn)?;
    db::seed_settings(conn)?;
    db::ensure_guest_customer(conn)?;
    backup_repository::record_backup_event(
        conn,
        &result.pre_restore_backup_path,
        "pre_restore",
        "恢复前自动快照",
    )?;
    backup_repository::record_backup_event(
        conn,
        &result.restored_backup_path,
        "restore",
        "已从该备份恢复数据库",
    )?;
    record_audit(
        conn,
        AuditEvent {
            module: "backup",
            action: "restore",
            target_type: Some("backup_logs"),
            target_id: Some(backup_id),
            target_label: Some("数据库恢复"),
            result: "success",
            message: Some("数据库恢复完成"),
            details: Some(&format!("preRestore={}", result.pre_restore_backup_path)),
        },
    )?;
    Ok(())
}
