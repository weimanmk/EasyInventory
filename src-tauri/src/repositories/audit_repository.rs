use crate::models::{AuditLogDto, AuditLogFilterRequest};
use rusqlite::{params_from_iter, types::Value};

pub fn list_audit_logs(
    conn: &rusqlite::Connection,
    filter: AuditLogFilterRequest,
) -> anyhow::Result<Vec<AuditLogDto>> {
    let mut sql = "SELECT id, log_time, module, action, target_type, target_id, target_label,
                          result, message, details
                   FROM audit_logs WHERE 1 = 1"
        .to_string();
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(module) = filter.module.filter(|value| !value.is_empty()) {
        sql.push_str(" AND module = ?");
        sql_params.push(Value::Text(module));
    }
    if let Some(action) = filter.action.filter(|value| !value.is_empty()) {
        sql.push_str(" AND action = ?");
        sql_params.push(Value::Text(action));
    }
    if let Some(start) = filter.start_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND log_time >= ?");
        sql_params.push(Value::Text(start));
    }
    if let Some(end) = filter.end_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND log_time <= ?");
        sql_params.push(Value::Text(format!("{end} 23:59:59")));
    }
    sql.push_str(" ORDER BY log_time DESC, id DESC LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), |row| {
        Ok(AuditLogDto {
            id: row.get(0)?,
            log_time: row.get(1)?,
            module: row.get(2)?,
            action: row.get(3)?,
            target_type: row.get(4)?,
            target_id: row.get(5)?,
            target_label: row.get(6)?,
            result: row.get(7)?,
            message: row.get(8)?,
            details: row.get(9)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
