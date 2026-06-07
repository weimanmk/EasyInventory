use crate::models::{
    CreatePaymentRequest, CustomerBalanceDto, CustomerBalanceFilterRequest, PaymentFilterRequest,
    PaymentRecordDto,
};
use crate::utils::{money, normalize_date, now_text};
use rusqlite::{params, params_from_iter, types::Value};

pub fn list_customer_balances(
    conn: &rusqlite::Connection,
    filter: CustomerBalanceFilterRequest,
) -> anyhow::Result<Vec<CustomerBalanceDto>> {
    let mut sql = String::from(
        "SELECT c.id, c.name, c.region,
                COALESCE(o.total_payable, 0),
                COALESCE(p.total_paid, 0),
                COALESCE(o.total_payable, 0) - COALESCE(p.total_paid, 0) AS balance,
                o.last_order_date,
                p.last_payment_date
         FROM customers c
         LEFT JOIN (
           SELECT customer_id, SUM(customer_payable_amount) AS total_payable, MAX(order_date) AS last_order_date
           FROM orders WHERE status = 'normal' GROUP BY customer_id
         ) o ON o.customer_id = c.id
         LEFT JOIN (
           SELECT customer_id, SUM(amount) AS total_paid, MAX(payment_date) AS last_payment_date
           FROM payment_records WHERE status = 'normal' GROUP BY customer_id
         ) p ON p.customer_id = c.id
         WHERE c.is_active = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(region) = filter
        .region
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND c.region = ?");
        sql_params.push(Value::Text(region));
    }
    if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (c.name LIKE ? OR c.address LIKE ?)");
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword));
    }
    if filter.only_unpaid.unwrap_or(false) {
        sql.push_str(" AND COALESCE(o.total_payable, 0) - COALESCE(p.total_paid, 0) > 0");
    }
    sql.push_str(" ORDER BY balance DESC, c.name LIMIT 1500");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_customer_balance)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn list_payment_records(
    conn: &rusqlite::Connection,
    filter: PaymentFilterRequest,
) -> anyhow::Result<Vec<PaymentRecordDto>> {
    let mut sql = String::from(
        "SELECT p.id, p.payment_date, p.customer_id, c.name, p.amount, p.method,
                p.related_order_id, p.status, p.remark, p.created_at
         FROM payment_records p
         JOIN customers c ON c.id = p.customer_id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(customer_id) = filter.customer_id {
        sql.push_str(" AND p.customer_id = ?");
        sql_params.push(Value::Integer(customer_id));
    }
    if let Some(start_date) = filter.start_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND p.payment_date >= ?");
        sql_params.push(Value::Text(start_date));
    }
    if let Some(end_date) = filter.end_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND p.payment_date <= ?");
        sql_params.push(Value::Text(end_date));
    }
    if let Some(status) = filter
        .status
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND p.status = ?");
        sql_params.push(Value::Text(status));
    }
    sql.push_str(" ORDER BY p.payment_date DESC, p.id DESC LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_payment_record)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn active_customer_exists(
    conn: &rusqlite::Connection,
    customer_id: i64,
) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) > 0 FROM customers WHERE id = ?1 AND is_active = 1",
        [customer_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn normal_order_belongs_to_customer(
    conn: &rusqlite::Connection,
    order_id: i64,
    customer_id: i64,
) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) > 0 FROM orders WHERE id = ?1 AND customer_id = ?2 AND status = 'normal'",
        params![order_id, customer_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn create_payment(
    conn: &rusqlite::Connection,
    payload: CreatePaymentRequest,
) -> anyhow::Result<PaymentRecordDto> {
    let now = now_text();
    conn.execute(
        "INSERT INTO payment_records
         (payment_date, customer_id, amount, method, related_order_id, status, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'normal', ?6, ?7, ?7)",
        params![
            normalize_date(&payload.payment_date),
            payload.customer_id,
            money(payload.amount),
            payload.method,
            payload.related_order_id,
            payload.remark,
            now
        ],
    )?;
    payment_by_id(conn, conn.last_insert_rowid())
}

pub fn void_payment(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<PaymentRecordDto> {
    conn.execute(
        "UPDATE payment_records SET status = 'voided', updated_at = ?1 WHERE id = ?2",
        params![now_text(), id],
    )?;
    payment_by_id(conn, id)
}

fn payment_by_id(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<PaymentRecordDto> {
    conn.query_row(
        "SELECT p.id, p.payment_date, p.customer_id, c.name, p.amount, p.method,
                p.related_order_id, p.status, p.remark, p.created_at
         FROM payment_records p
         JOIN customers c ON c.id = p.customer_id
         WHERE p.id = ?1",
        [id],
        map_payment_record,
    )
    .map_err(Into::into)
}

fn map_payment_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<PaymentRecordDto> {
    Ok(PaymentRecordDto {
        id: row.get(0)?,
        payment_date: row.get(1)?,
        customer_id: row.get(2)?,
        customer_name: row.get(3)?,
        amount: row.get(4)?,
        method: row.get(5)?,
        related_order_id: row.get(6)?,
        status: row.get(7)?,
        remark: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn map_customer_balance(row: &rusqlite::Row<'_>) -> rusqlite::Result<CustomerBalanceDto> {
    Ok(CustomerBalanceDto {
        customer_id: row.get(0)?,
        customer_name: row.get(1)?,
        region: row.get(2)?,
        total_payable: row.get(3)?,
        total_paid: row.get(4)?,
        balance: row.get(5)?,
        last_order_date: row.get(6)?,
        last_payment_date: row.get(7)?,
    })
}
