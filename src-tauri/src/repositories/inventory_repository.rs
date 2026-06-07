use crate::models::{InboundRecordDto, ListInboundRecordsRequest};
use rusqlite::{params_from_iter, types::Value, OptionalExtension};

pub fn active_supplier_name(
    conn: &rusqlite::Connection,
    supplier_id: i64,
) -> anyhow::Result<Option<String>> {
    conn.query_row(
        "SELECT name FROM suppliers WHERE id = ?1 AND is_active = 1",
        [supplier_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(Into::into)
}

pub fn list_inbound_records(
    conn: &rusqlite::Connection,
    filter: ListInboundRecordsRequest,
) -> anyhow::Result<Vec<InboundRecordDto>> {
    let mut sql = String::from(
        "SELECT i.id, i.inbound_date, i.product_id, p.name, p.category,
                i.supplier_id, i.supplier_name, i.quantity, i.unit_cost, i.amount, i.remark
         FROM inbound_records i
         JOIN products p ON p.id = i.product_id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(start) = filter.start_date {
        sql.push_str(" AND i.inbound_date >= ?");
        sql_params.push(Value::Text(start));
    }
    if let Some(end) = filter.end_date {
        sql.push_str(" AND i.inbound_date <= ?");
        sql_params.push(Value::Text(end));
    }
    if let Some(product_id) = filter.product_id {
        sql.push_str(" AND i.product_id = ?");
        sql_params.push(Value::Integer(product_id));
    }
    if let Some(category) = filter
        .category
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND p.category = ?");
        sql_params.push(Value::Text(category));
    }
    sql.push_str(" ORDER BY i.inbound_date DESC, i.id DESC LIMIT 500");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_inbound_record)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn map_inbound_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<InboundRecordDto> {
    Ok(InboundRecordDto {
        id: row.get(0)?,
        inbound_date: row.get(1)?,
        product_id: row.get(2)?,
        product_name: row.get(3)?,
        category: row.get(4)?,
        supplier_id: row.get(5)?,
        supplier_name: row.get(6)?,
        quantity: row.get(7)?,
        unit_cost: row.get(8)?,
        amount: row.get(9)?,
        remark: row.get(10)?,
    })
}
