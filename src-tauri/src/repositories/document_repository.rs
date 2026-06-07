use crate::models::{DocumentDto, DocumentFilterRequest};
use crate::utils::now_text;
use rusqlite::{params, params_from_iter, types::Value};

pub fn list_documents(
    conn: &rusqlite::Connection,
    filter: DocumentFilterRequest,
) -> anyhow::Result<Vec<DocumentDto>> {
    let mut sql = String::from(
        "SELECT d.id, d.order_id, d.order_no, d.customer_id, d.customer_name, d.file_path,
                d.file_type, d.printed_at, d.print_count, d.created_at, COALESCE(d.status, 'normal')
         FROM documents d
         JOIN orders o ON o.id = d.order_id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(customer_id) = filter.customer_id {
        sql.push_str(" AND d.customer_id = ?");
        sql_params.push(Value::Integer(customer_id));
    }
    if let Some(start_date) = filter.start_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND o.order_date >= ?");
        sql_params.push(Value::Text(start_date));
    }
    if let Some(end_date) = filter.end_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND o.order_date <= ?");
        sql_params.push(Value::Text(end_date));
    }
    if let Some(order_no) = filter.order_no.filter(|value| !value.is_empty()) {
        sql.push_str(" AND d.order_no LIKE ?");
        sql_params.push(Value::Text(format!("%{order_no}%")));
    }
    if let Some(printed) = filter.printed {
        if printed {
            sql.push_str(" AND d.print_count > 0");
        } else {
            sql.push_str(" AND d.print_count = 0");
        }
    }
    if let Some(status) = filter
        .status
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND COALESCE(d.status, 'normal') = ?");
        sql_params.push(Value::Text(status));
    }
    sql.push_str(" ORDER BY d.created_at DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_document)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn document_by_id(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<DocumentDto> {
    conn.query_row(
        "SELECT id, order_id, order_no, customer_id, customer_name, file_path,
                file_type, printed_at, print_count, created_at, COALESCE(status, 'normal')
         FROM documents WHERE id = ?1",
        [id],
        map_document,
    )
    .map_err(Into::into)
}

pub fn increment_print_count(
    conn: &rusqlite::Connection,
    document_id: i64,
    order_id: i64,
) -> anyhow::Result<()> {
    let now = now_text();
    conn.execute(
        "UPDATE documents SET print_count = print_count + 1, printed_at = ?1 WHERE id = ?2",
        params![now, document_id],
    )?;
    conn.execute(
        "UPDATE orders SET print_count = print_count + 1, updated_at = ?1 WHERE id = ?2",
        params![now_text(), order_id],
    )?;
    Ok(())
}

fn map_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentDto> {
    Ok(DocumentDto {
        id: row.get(0)?,
        order_id: row.get(1)?,
        order_no: row.get(2)?,
        customer_id: row.get(3)?,
        customer_name: row.get(4)?,
        file_path: row.get(5)?,
        file_type: row.get(6)?,
        printed_at: row.get(7)?,
        print_count: row.get(8)?,
        created_at: row.get(9)?,
        status: row.get(10)?,
    })
}
