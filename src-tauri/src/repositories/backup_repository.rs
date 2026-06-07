use crate::models::BackupDto;
use crate::utils::now_text;
use rusqlite::params;

pub fn list_backups(conn: &rusqlite::Connection) -> anyhow::Result<Vec<BackupDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, backup_path, backup_type, status, message, created_at
         FROM backup_logs ORDER BY created_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BackupDto {
            id: row.get(0)?,
            backup_path: row.get(1)?,
            backup_type: row.get(2)?,
            status: row.get(3)?,
            message: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn successful_backup_path(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<String> {
    conn.query_row(
        "SELECT backup_path FROM backup_logs WHERE id = ?1 AND status = 'success'",
        [id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn record_backup_event(
    conn: &rusqlite::Connection,
    backup_path: &str,
    backup_type: &str,
    message: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO backup_logs (backup_path, backup_type, status, message, created_at)
         VALUES (?1, ?2, 'success', ?3, ?4)",
        params![backup_path, backup_type, message, now_text()],
    )?;
    Ok(())
}
