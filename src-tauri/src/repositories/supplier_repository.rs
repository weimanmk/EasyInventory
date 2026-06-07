use crate::models::{
    BatchUpdateResultDto, BatchUpdateSuppliersRequest, ListSuppliersRequest, SupplierDto,
    SupplierPayload,
};
use crate::utils::now_text;
use rusqlite::{params, params_from_iter, types::Value};

pub fn list_suppliers(
    conn: &rusqlite::Connection,
    filter: ListSuppliersRequest,
) -> anyhow::Result<Vec<SupplierDto>> {
    let mut sql = String::from(
        "SELECT id, name, contact, phone, address, is_active, remark
         FROM suppliers WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(keyword) = filter.keyword.filter(|value| !value.trim().is_empty()) {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (name LIKE ? OR contact LIKE ? OR phone LIKE ?)");
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword));
    }
    if let Some(active) = filter.is_active {
        sql.push_str(if active {
            " AND is_active = 1"
        } else {
            " AND is_active = 0"
        });
    }
    sql.push_str(" ORDER BY name LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_supplier)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn create_supplier(
    conn: &rusqlite::Connection,
    payload: SupplierPayload,
    name: &str,
) -> anyhow::Result<SupplierDto> {
    let now = now_text();
    conn.execute(
        "INSERT INTO suppliers (name, contact, phone, address, is_active, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?6)",
        params![
            name,
            payload.contact,
            payload.phone,
            payload.address,
            payload.remark,
            now
        ],
    )?;
    supplier_by_id(conn, conn.last_insert_rowid())
}

pub fn update_supplier(
    conn: &rusqlite::Connection,
    id: i64,
    payload: SupplierPayload,
    name: &str,
) -> anyhow::Result<SupplierDto> {
    conn.execute(
        "UPDATE suppliers
         SET name = ?1, contact = ?2, phone = ?3, address = ?4, remark = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            name,
            payload.contact,
            payload.phone,
            payload.address,
            payload.remark,
            now_text(),
            id
        ],
    )?;
    supplier_by_id(conn, id)
}

pub fn disable_supplier(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    conn.execute(
        "UPDATE suppliers SET is_active = 0, updated_at = ?1 WHERE id = ?2",
        params![now_text(), id],
    )?;
    Ok(true)
}

pub fn batch_update_suppliers(
    conn: &rusqlite::Connection,
    payload: BatchUpdateSuppliersRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    let ids_sql = batch_ids_placeholders(&payload.ids)?;
    let mut sets = Vec::new();
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(contact) = payload.contact {
        append_text_assignment(&mut sets, &mut sql_params, "contact", &contact);
    }
    if let Some(phone) = payload.phone {
        append_text_assignment(&mut sets, &mut sql_params, "phone", &phone);
    }
    if let Some(address) = payload.address {
        append_text_assignment(&mut sets, &mut sql_params, "address", &address);
    }
    if let Some(remark) = payload.remark {
        append_text_assignment(&mut sets, &mut sql_params, "remark", &remark);
    }
    if sets.is_empty() {
        anyhow::bail!("没有要更新的供应商字段");
    }
    sets.push("updated_at = ?".to_string());
    sql_params.push(Value::Text(now_text()));
    for id in payload.ids {
        sql_params.push(Value::Integer(id));
    }
    let sql = format!(
        "UPDATE suppliers SET {} WHERE id IN ({ids_sql})",
        sets.join(", ")
    );
    let affected = conn.execute(&sql, params_from_iter(sql_params.iter()))?;
    Ok(BatchUpdateResultDto {
        affected_count: affected as i64,
    })
}

pub fn supplier_by_id(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<SupplierDto> {
    conn.query_row(
        "SELECT id, name, contact, phone, address, is_active, remark FROM suppliers WHERE id = ?1",
        [id],
        map_supplier,
    )
    .map_err(Into::into)
}

fn batch_ids_placeholders(ids: &[i64]) -> anyhow::Result<String> {
    if ids.is_empty() {
        anyhow::bail!("请选择要批量编辑的记录");
    }
    if ids.iter().any(|id| *id <= 0) {
        anyhow::bail!("批量编辑记录不合法");
    }
    Ok((0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(","))
}

fn append_text_assignment(
    sets: &mut Vec<String>,
    sql_params: &mut Vec<Value>,
    column: &str,
    value: &str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        sets.push(format!("{column} = ?"));
        sql_params.push(Value::Null);
    } else {
        sets.push(format!("{column} = ?"));
        sql_params.push(Value::Text(trimmed.to_string()));
    }
}

fn map_supplier(row: &rusqlite::Row<'_>) -> rusqlite::Result<SupplierDto> {
    Ok(SupplierDto {
        id: row.get(0)?,
        name: row.get(1)?,
        contact: row.get(2)?,
        phone: row.get(3)?,
        address: row.get(4)?,
        is_active: row.get::<_, i64>(5)? == 1,
        remark: row.get(6)?,
    })
}
