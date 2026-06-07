use crate::models::{CustomerAnalysisRowDto, ProductRankingRowDto};
use crate::utils::money;
use rusqlite::{params_from_iter, types::Value};

pub fn product_ranking(
    conn: &rusqlite::Connection,
    where_sql: &str,
    params: &[Value],
    rank_expr: &str,
    limit: i64,
) -> anyhow::Result<Vec<ProductRankingRowDto>> {
    let sql = format!(
        "SELECT
            oi.product_id,
            COALESCE(NULLIF(TRIM(oi.product_name), ''), '未命名商品') AS product_name,
            COALESCE(NULLIF(TRIM(oi.category), ''), '未分类') AS category,
            COUNT(DISTINCT CASE WHEN oi.line_type = 'normal' THEN o.id END) AS order_count,
            COALESCE(SUM(CASE WHEN oi.line_type = 'normal' THEN oi.quantity ELSE 0 END), 0) AS sales_quantity,
            COALESCE(SUM(CASE WHEN oi.line_type = 'normal' THEN oi.amount ELSE 0 END), 0) AS sales_amount,
            COALESCE(SUM(CASE WHEN oi.line_type = 'normal' THEN oi.cost_amount ELSE 0 END), 0) AS cost_amount,
            COALESCE(SUM(CASE WHEN oi.line_type IN ('normal', 'gift') THEN oi.profit_amount ELSE 0 END), 0) AS profit_amount,
            COALESCE(SUM(CASE WHEN oi.line_type = 'gift' THEN oi.quantity ELSE 0 END), 0) AS gift_quantity,
            COALESCE(SUM(CASE WHEN oi.line_type = 'gift' THEN oi.cost_amount ELSE 0 END), 0) AS gift_cost_amount
         FROM order_items oi
         JOIN orders o ON o.id = oi.order_id
         WHERE {where_sql}
         GROUP BY oi.product_id, product_name, category
         ORDER BY {rank_expr} DESC, sales_amount DESC, product_name
         LIMIT {limit}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok(ProductRankingRowDto {
            product_id: row.get(0)?,
            product_name: row.get(1)?,
            category: row.get(2)?,
            order_count: row.get(3)?,
            sales_quantity: money(row.get::<_, f64>(4)?),
            sales_amount: money(row.get::<_, f64>(5)?),
            cost_amount: money(row.get::<_, f64>(6)?),
            profit_amount: money(row.get::<_, f64>(7)?),
            gift_quantity: money(row.get::<_, f64>(8)?),
            gift_cost_amount: money(row.get::<_, f64>(9)?),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn customer_analysis_rows(
    conn: &rusqlite::Connection,
    metric_where: &str,
    order_balance_where: &str,
    payment_balance_where: &str,
    params: &[Value],
    rank_expr: &str,
    limit: i64,
) -> anyhow::Result<Vec<CustomerAnalysisRowDto>> {
    let sql = format!(
        "SELECT
            c.id,
            c.name,
            c.region,
            COALESCE(m.order_count, 0) AS order_count,
            COALESCE(m.sales_amount, 0) AS sales_amount,
            COALESCE(m.cost_amount, 0) AS cost_amount,
            COALESCE(m.profit_amount, 0) AS profit_amount,
            COALESCE(b.payable_amount, 0) - COALESCE(p.paid_amount, 0) AS balance_amount,
            m.recent_order_date
         FROM customers c
         LEFT JOIN (
           SELECT
             o.customer_id,
             COUNT(DISTINCT o.id) AS order_count,
             COALESCE(SUM(CASE WHEN oi.line_type = 'normal' THEN oi.amount ELSE 0 END), 0) AS sales_amount,
             COALESCE(SUM(CASE WHEN oi.line_type = 'normal' THEN oi.cost_amount ELSE 0 END), 0) AS cost_amount,
             COALESCE(SUM(CASE WHEN oi.line_type IN ('normal', 'gift') THEN oi.profit_amount ELSE 0 END), 0) AS profit_amount,
             MAX(o.order_date) AS recent_order_date
           FROM orders o
           JOIN order_items oi ON oi.order_id = o.id
           WHERE {metric_where}
           GROUP BY o.customer_id
         ) m ON m.customer_id = c.id
         LEFT JOIN (
           SELECT customer_id, COALESCE(SUM(customer_payable_amount), 0) AS payable_amount
           FROM orders
           WHERE {order_balance_where}
           GROUP BY customer_id
         ) b ON b.customer_id = c.id
         LEFT JOIN (
           SELECT customer_id, COALESCE(SUM(amount), 0) AS paid_amount
           FROM payment_records
           WHERE {payment_balance_where}
           GROUP BY customer_id
         ) p ON p.customer_id = c.id
         WHERE c.is_active = 1
           AND (
             COALESCE(m.order_count, 0) > 0
             OR ABS(COALESCE(b.payable_amount, 0) - COALESCE(p.paid_amount, 0)) > 0.0001
           )
         ORDER BY {rank_expr} DESC, sales_amount DESC, c.name
         LIMIT {limit}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok(CustomerAnalysisRowDto {
            customer_id: row.get(0)?,
            customer_name: row.get(1)?,
            region: row.get(2)?,
            order_count: row.get(3)?,
            sales_amount: money(row.get::<_, f64>(4)?),
            cost_amount: money(row.get::<_, f64>(5)?),
            profit_amount: money(row.get::<_, f64>(6)?),
            balance_amount: money(row.get::<_, f64>(7)?),
            recent_order_date: row.get(8)?,
            average_repurchase_days: None,
            favorite_products: String::new(),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn customer_order_dates(
    conn: &rusqlite::Connection,
    where_sql: &str,
    params: &[Value],
) -> anyhow::Result<Vec<String>> {
    let sql = format!(
        "SELECT DISTINCT o.order_date
         FROM orders o
         WHERE {where_sql}
         ORDER BY o.order_date"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        row.get::<_, String>(0)
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn customer_favorite_product_rows(
    conn: &rusqlite::Connection,
    where_sql: &str,
    params: &[Value],
) -> anyhow::Result<Vec<(String, f64)>> {
    let sql = format!(
        "SELECT
            COALESCE(NULLIF(TRIM(oi.product_name), ''), '未命名商品') AS product_name,
            COALESCE(SUM(oi.quantity), 0) AS quantity,
            COALESCE(SUM(oi.amount), 0) AS sales_amount
         FROM order_items oi
         JOIN orders o ON o.id = oi.order_id
         WHERE {where_sql}
         GROUP BY oi.product_id, product_name
         ORDER BY quantity DESC, sales_amount DESC, product_name
         LIMIT 3"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
