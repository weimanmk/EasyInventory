use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct CustomerStatementLedgerRow {
    pub record_date: String,
    pub record_type: String,
    pub record_no: String,
    pub description: String,
    pub debit_amount: f64,
    pub credit_amount: f64,
    pub remark: Option<String>,
}

pub fn customer_name(conn: &Connection, customer_id: i64) -> anyhow::Result<String> {
    conn.query_row(
        "SELECT name FROM customers WHERE id = ?1",
        [customer_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn opening_payable(
    conn: &Connection,
    customer_id: i64,
    start_date: &str,
) -> anyhow::Result<f64> {
    conn.query_row(
        "SELECT COALESCE(SUM(customer_payable_amount), 0)
         FROM orders
         WHERE customer_id = ?1 AND order_date < ?2 AND status = 'normal'",
        params![customer_id, start_date],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn opening_paid(conn: &Connection, customer_id: i64, start_date: &str) -> anyhow::Result<f64> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount), 0)
         FROM payment_records
         WHERE customer_id = ?1 AND payment_date < ?2 AND status = 'normal'",
        params![customer_id, start_date],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn period_discount_amount(
    conn: &Connection,
    customer_id: i64,
    start_date: &str,
    end_date: &str,
) -> anyhow::Result<f64> {
    conn.query_row(
        "SELECT COALESCE(SUM(direct_discount_amount + monthly_credit_used), 0)
         FROM orders
         WHERE customer_id = ?1 AND order_date >= ?2 AND order_date <= ?3 AND status = 'normal'",
        params![customer_id, start_date, end_date],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn ledger_rows(
    conn: &Connection,
    customer_id: i64,
    start_date: &str,
    end_date: &str,
) -> anyhow::Result<Vec<CustomerStatementLedgerRow>> {
    let mut stmt = conn.prepare(
        "SELECT record_date, record_type, record_no, description, debit_amount, credit_amount, remark
         FROM (
           SELECT order_date AS record_date,
                  'order' AS record_type,
                  order_no AS record_no,
                  '出库单' AS description,
                  customer_payable_amount AS debit_amount,
                  0.0 AS credit_amount,
                  remark AS remark,
                  id * 2 AS sort_key
           FROM orders
           WHERE customer_id = ?1 AND order_date >= ?2 AND order_date <= ?3 AND status = 'normal'
           UNION ALL
           SELECT payment_date AS record_date,
                  'payment' AS record_type,
                  'PAY' || printf('%06d', id) AS record_no,
                  COALESCE(method, '收款') AS description,
                  0.0 AS debit_amount,
                  amount AS credit_amount,
                  remark AS remark,
                  id * 2 + 1 AS sort_key
           FROM payment_records
           WHERE customer_id = ?1 AND payment_date >= ?2 AND payment_date <= ?3 AND status = 'normal'
         )
         ORDER BY record_date, sort_key",
    )?;
    let rows = stmt
        .query_map(params![customer_id, start_date, end_date], |row| {
            Ok(CustomerStatementLedgerRow {
                record_date: row.get(0)?,
                record_type: row.get(1)?,
                record_no: row.get(2)?,
                description: row.get(3)?,
                debit_amount: row.get(4)?,
                credit_amount: row.get(5)?,
                remark: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
