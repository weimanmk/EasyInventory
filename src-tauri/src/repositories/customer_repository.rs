use crate::models::{
    BatchUpdateCustomersRequest, BatchUpdateResultDto, CustomerDto, CustomerPayload,
    ListCustomersRequest,
};
use crate::utils::now_text;
use rusqlite::{params, params_from_iter, types::Value};

pub fn list_customers(
    conn: &rusqlite::Connection,
    filter: ListCustomersRequest,
    guest_name: &str,
) -> anyhow::Result<Vec<CustomerDto>> {
    let mut sql = String::from(
        "SELECT id, region, name, address, phone, is_active, remark FROM customers WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(region) = filter
        .region
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND region = ?");
        sql_params.push(Value::Text(region));
    }
    if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (name LIKE ? OR address LIKE ?)");
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
    sql.push_str(" ORDER BY CASE WHEN name = ? THEN 0 ELSE 1 END, region, name LIMIT 1500");
    sql_params.push(Value::Text(guest_name.to_string()));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_customer)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn create_customer(
    conn: &rusqlite::Connection,
    payload: CustomerPayload,
    name: &str,
) -> anyhow::Result<CustomerDto> {
    let now = now_text();
    conn.execute(
        "INSERT INTO customers (region, name, address, phone, is_active, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?6)",
        params![
            payload.region,
            name,
            payload.address,
            payload.phone,
            payload.remark,
            now
        ],
    )?;
    crate::db::customer_by_id(conn, conn.last_insert_rowid())
}

pub fn update_customer(
    conn: &rusqlite::Connection,
    id: i64,
    payload: CustomerPayload,
    name: &str,
) -> anyhow::Result<CustomerDto> {
    conn.execute(
        "UPDATE customers SET region = ?1, name = ?2, address = ?3, phone = ?4, remark = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            payload.region,
            name,
            payload.address,
            payload.phone,
            payload.remark,
            now_text(),
            id
        ],
    )?;
    crate::db::customer_by_id(conn, id)
}

pub fn disable_customer(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    conn.execute(
        "UPDATE customers SET is_active = 0, updated_at = ?1 WHERE id = ?2",
        params![now_text(), id],
    )?;
    Ok(true)
}

pub fn batch_update_customers(
    conn: &rusqlite::Connection,
    payload: BatchUpdateCustomersRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    let ids_sql = batch_ids_placeholders(&payload.ids)?;
    let mut sets = Vec::new();
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(region) = payload.region {
        append_text_assignment(&mut sets, &mut sql_params, "region", &region);
    }
    if let Some(remark) = payload.remark {
        append_text_assignment(&mut sets, &mut sql_params, "remark", &remark);
    }
    if let Some(active) = payload.is_active {
        sets.push("is_active = ?".to_string());
        sql_params.push(Value::Integer(if active { 1 } else { 0 }));
    }
    if sets.is_empty() {
        anyhow::bail!("没有要更新的客户字段");
    }
    sets.push("updated_at = ?".to_string());
    sql_params.push(Value::Text(now_text()));
    for id in payload.ids {
        sql_params.push(Value::Integer(id));
    }
    let sql = format!(
        "UPDATE customers SET {} WHERE id IN ({ids_sql})",
        sets.join(", ")
    );
    let affected = conn.execute(&sql, params_from_iter(sql_params.iter()))?;
    Ok(BatchUpdateResultDto {
        affected_count: affected as i64,
    })
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

fn map_customer(row: &rusqlite::Row<'_>) -> rusqlite::Result<CustomerDto> {
    Ok(CustomerDto {
        id: row.get(0)?,
        region: row.get(1)?,
        name: row.get(2)?,
        address: row.get(3)?,
        phone: row.get(4)?,
        is_active: row.get::<_, i64>(5)? == 1,
        remark: row.get(6)?,
    })
}
