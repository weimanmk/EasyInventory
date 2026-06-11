use crate::logger;
use crate::models::{AuditLogDto, AuditLogFilterRequest};
use crate::repositories::audit_repository;
use crate::utils::now_text;
use rusqlite::params;

pub struct AuditEvent<'a> {
    pub module: &'a str,
    pub action: &'a str,
    pub target_type: Option<&'a str>,
    pub target_id: Option<i64>,
    pub target_label: Option<&'a str>,
    pub result: &'a str,
    pub message: Option<&'a str>,
    pub details: Option<&'a str>,
}

pub fn record_audit(conn: &rusqlite::Connection, event: AuditEvent<'_>) -> anyhow::Result<()> {
    let sanitized_details = event.details.map(logger::redact_sensitive_text);
    conn.execute(
        "INSERT INTO audit_logs
         (log_time, module, action, target_type, target_id, target_label, result, message, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            now_text(),
            event.module,
            event.action,
            event.target_type,
            event.target_id,
            event.target_label,
            event.result,
            event.message,
            sanitized_details.as_deref()
        ],
    )?;
    Ok(())
}

pub fn list_audit_logs(
    conn: &rusqlite::Connection,
    filter: Option<AuditLogFilterRequest>,
) -> anyhow::Result<Vec<AuditLogDto>> {
    audit_repository::list_audit_logs(conn, filter.unwrap_or_default())
}
