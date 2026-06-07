use crate::models::{
    ListOrdersRequest, MonthlyCreditDto, MonthlyCreditFilterRequest, OrderDetailDto, OrderDto,
    OrderItemDto, OrderTotalsDto,
};
use anyhow::Context;
use rusqlite::{params, params_from_iter, types::Value, OptionalExtension};

pub struct NewOrderHeader<'a> {
    pub order_no: &'a str,
    pub order_date: &'a str,
    pub customer_id: i64,
    pub customer_name: &'a str,
    pub customer_address: Option<&'a str>,
    pub remark: Option<&'a str>,
}

pub struct NewOrderItem<'a> {
    pub order_id: i64,
    pub line_type: &'a str,
    pub product_id: Option<i64>,
    pub product_name: Option<&'a str>,
    pub category: Option<&'a str>,
    pub barcode: Option<&'a str>,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub avg_cost: f64,
    pub cost_amount: f64,
    pub profit_amount: f64,
    pub related_product_id: Option<i64>,
    pub rule_id: Option<i64>,
    pub monthly_credit_id: Option<i64>,
    pub remark: Option<&'a str>,
    pub sort_order: i64,
}

pub struct NewMovement<'a> {
    pub date: &'a str,
    pub product_id: i64,
    pub movement_type: &'a str,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub order_id: i64,
    pub order_no: &'a str,
    pub remark: &'a str,
}

pub struct NewMonthlyCredit<'a> {
    pub order_id: i64,
    pub order_no: &'a str,
    pub customer_id: i64,
    pub category: &'a str,
    pub amount: f64,
    pub generated_date: &'a str,
    pub available_month: &'a str,
    pub remark: &'a str,
}

#[derive(Debug, Clone)]
pub struct MonthlyCreditUseRow {
    pub monthly_credit_id: i64,
    pub amount: f64,
}

#[derive(Debug, Clone)]
pub struct RuleRow {
    pub id: i64,
    pub fixed_price: Option<f64>,
    pub threshold_quantity: Option<f64>,
    pub gift_product_id: Option<i64>,
    pub gift_quantity: Option<f64>,
    pub direct_discount_amount: Option<f64>,
    pub monthly_credit_amount: Option<f64>,
    pub credit_category: Option<String>,
}

pub fn get_order_detail(
    conn: &rusqlite::Connection,
    order_id: i64,
) -> anyhow::Result<OrderDetailDto> {
    let order = order_by_id(conn, order_id)?;
    let mut stmt = conn.prepare(
        "SELECT id, line_type, product_id, product_name, category, barcode, quantity, unit_price,
                amount, avg_cost, cost_amount, profit_amount, rule_id, monthly_credit_id, remark, sort_order
         FROM order_items
         WHERE order_id = ?1
         ORDER BY sort_order, id",
    )?;
    let rows = stmt.query_map([order_id], map_order_item)?;
    Ok(OrderDetailDto {
        order,
        items: rows.collect::<Result<Vec<_>, _>>()?,
    })
}

pub fn list_orders(
    conn: &rusqlite::Connection,
    filter: ListOrdersRequest,
) -> anyhow::Result<Vec<OrderDto>> {
    let mut sql = String::from(
        "SELECT id, order_no, order_date, customer_id, customer_name, customer_address,
                product_sales_amount, direct_discount_amount, monthly_credit_used,
                customer_payable_amount, brand_subsidy_amount, cost_amount, gift_cost_amount,
                profit_amount, remark, document_path, print_count, status
         FROM orders WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(start) = filter.start_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND order_date >= ?");
        sql_params.push(Value::Text(start));
    }
    if let Some(end) = filter.end_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND order_date <= ?");
        sql_params.push(Value::Text(end));
    }
    if let Some(customer_id) = filter.customer_id {
        sql.push_str(" AND customer_id = ?");
        sql_params.push(Value::Integer(customer_id));
    }
    if let Some(order_no) = filter.order_no.filter(|value| !value.is_empty()) {
        sql.push_str(" AND order_no LIKE ?");
        sql_params.push(Value::Text(format!("%{order_no}%")));
    }
    if let Some(status) = filter.status.filter(|value| !value.is_empty()) {
        sql.push_str(" AND status = ?");
        sql_params.push(Value::Text(status));
    }
    sql.push_str(" ORDER BY order_date DESC, id DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_order)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn list_monthly_credits(
    conn: &rusqlite::Connection,
    filter: MonthlyCreditFilterRequest,
) -> anyhow::Result<Vec<MonthlyCreditDto>> {
    let mut sql = String::from(
        "SELECT m.id, m.source_order_id, m.source_order_no, m.customer_id, c.name, m.category,
                m.amount, m.used_amount, m.remaining_amount, m.generated_date, m.available_month,
                m.status, m.remark
         FROM monthly_credits m
         JOIN customers c ON c.id = m.customer_id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(customer_id) = filter.customer_id {
        sql.push_str(" AND m.customer_id = ?");
        sql_params.push(Value::Integer(customer_id));
    }
    if let Some(category) = filter.category.filter(|value| !value.is_empty()) {
        sql.push_str(" AND m.category = ?");
        sql_params.push(Value::Text(category));
    }
    if let Some(status) = filter.status.filter(|value| !value.is_empty()) {
        sql.push_str(" AND m.status = ?");
        sql_params.push(Value::Text(status));
    }
    if let Some(start) = filter.start_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND m.generated_date >= ?");
        sql_params.push(Value::Text(start));
    }
    if let Some(end) = filter.end_date.filter(|value| !value.is_empty()) {
        sql.push_str(" AND m.generated_date <= ?");
        sql_params.push(Value::Text(end));
    }
    if let Some(month) = filter.available_month.filter(|value| !value.is_empty()) {
        sql.push_str(" AND m.available_month = ?");
        sql_params.push(Value::Text(month));
    }
    sql.push_str(" ORDER BY m.generated_date DESC, m.id DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_credit)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn monthly_credit_by_id(
    conn: &rusqlite::Connection,
    id: i64,
) -> anyhow::Result<MonthlyCreditDto> {
    conn.query_row(
        "SELECT m.id, m.source_order_id, m.source_order_no, m.customer_id, c.name, m.category,
                m.amount, m.used_amount, m.remaining_amount, m.generated_date,
                m.available_month, m.status, m.remark
         FROM monthly_credits m
         JOIN customers c ON c.id = m.customer_id
         WHERE m.id = ?1",
        [id],
        map_credit,
    )
    .with_context(|| format!("月费记录不存在：{id}"))
}

pub fn active_rule(
    conn: &rusqlite::Connection,
    customer_id: i64,
    product_id: i64,
) -> anyhow::Result<Option<RuleRow>> {
    conn.query_row(
        "SELECT id, fixed_price, threshold_quantity, gift_product_id, gift_quantity,
                direct_discount_amount, monthly_credit_amount, credit_category
         FROM customer_product_rules
         WHERE customer_id = ?1 AND product_id = ?2 AND is_active = 1
         LIMIT 1",
        params![customer_id, product_id],
        |row| {
            Ok(RuleRow {
                id: row.get(0)?,
                fixed_price: row.get(1)?,
                threshold_quantity: row.get(2)?,
                gift_product_id: row.get(3)?,
                gift_quantity: row.get(4)?,
                direct_discount_amount: row.get(5)?,
                monthly_credit_amount: row.get(6)?,
                credit_category: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn next_order_no(conn: &rusqlite::Connection, order_date: &str) -> anyhow::Result<String> {
    let date = crate::utils::normalize_date(order_date);
    let prefix = date.replace('-', "");
    let like = format!("{prefix}%");
    let max_no: Option<String> = conn.query_row(
        "SELECT MAX(order_no) FROM orders WHERE order_no LIKE ?1",
        [like],
        |row| row.get(0),
    )?;
    let next = max_no
        .and_then(|value| value[prefix.len()..].parse::<i64>().ok())
        .unwrap_or(0)
        + 1;
    Ok(format!("{prefix}{next:03}"))
}

pub fn create_order_header(
    conn: &rusqlite::Connection,
    header: NewOrderHeader<'_>,
) -> anyhow::Result<i64> {
    let now = crate::utils::now_text();
    conn.execute(
        "INSERT INTO orders
         (order_no, order_date, customer_id, customer_name, customer_address,
          product_sales_amount, direct_discount_amount, monthly_credit_used,
          customer_payable_amount, brand_subsidy_amount, cost_amount, gift_cost_amount,
          profit_amount, remark, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 0, 0, 0, 0, 0, ?6, 'normal', ?7, ?7)",
        params![
            header.order_no,
            crate::utils::normalize_date(header.order_date),
            header.customer_id,
            header.customer_name,
            header.customer_address,
            header.remark,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_order_totals(
    conn: &rusqlite::Connection,
    order_id: i64,
    totals: &OrderTotalsDto,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE orders SET
          product_sales_amount = ?1,
          direct_discount_amount = ?2,
          monthly_credit_used = ?3,
          customer_payable_amount = ?4,
          brand_subsidy_amount = ?5,
          cost_amount = ?6,
          gift_cost_amount = ?7,
          profit_amount = ?8,
          updated_at = ?9
         WHERE id = ?10",
        params![
            totals.product_sales_amount,
            totals.direct_discount_amount,
            totals.monthly_credit_used,
            totals.customer_payable_amount,
            totals.brand_subsidy_amount,
            totals.cost_amount,
            totals.gift_cost_amount,
            totals.profit_amount,
            crate::utils::now_text(),
            order_id
        ],
    )?;
    Ok(())
}

pub fn create_order_item(
    conn: &rusqlite::Connection,
    item: NewOrderItem<'_>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO order_items
         (order_id, line_type, product_id, product_name, category, barcode, quantity,
          unit_price, amount, avg_cost, cost_amount, profit_amount, related_product_id,
          rule_id, monthly_credit_id, remark, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            item.order_id,
            item.line_type,
            item.product_id,
            item.product_name,
            item.category,
            item.barcode,
            item.quantity,
            item.unit_price,
            item.amount,
            item.avg_cost,
            item.cost_amount,
            item.profit_amount,
            item.related_product_id,
            item.rule_id,
            item.monthly_credit_id,
            item.remark,
            item.sort_order
        ],
    )?;
    Ok(())
}

pub fn create_inventory_movement(
    conn: &rusqlite::Connection,
    movement: NewMovement<'_>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount,
          source_type, source_id, source_no, remark, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'order', ?7, ?8, ?9, ?10)",
        params![
            crate::utils::normalize_date(movement.date),
            movement.product_id,
            movement.movement_type,
            movement.quantity,
            movement.unit_price,
            movement.amount,
            movement.order_id,
            movement.order_no,
            movement.remark,
            crate::utils::now_text()
        ],
    )?;
    Ok(())
}

pub fn apply_monthly_credit_use(
    conn: &rusqlite::Connection,
    credit_id: i64,
    used_amount: f64,
    remaining_amount: f64,
    status: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE monthly_credits
         SET used_amount = ?1, remaining_amount = ?2, status = ?3, updated_at = ?4
         WHERE id = ?5",
        params![
            used_amount,
            remaining_amount,
            status,
            crate::utils::now_text(),
            credit_id
        ],
    )?;
    Ok(())
}

pub fn create_monthly_credit(
    conn: &rusqlite::Connection,
    credit: NewMonthlyCredit<'_>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO monthly_credits
         (source_order_id, source_order_no, customer_id, category, amount,
          used_amount, remaining_amount, generated_date, available_month, status, remark,
          created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?5, ?6, ?7, 'pending', ?8, ?9, ?9)",
        params![
            credit.order_id,
            credit.order_no,
            credit.customer_id,
            credit.category,
            credit.amount,
            crate::utils::normalize_date(credit.generated_date),
            credit.available_month,
            credit.remark,
            crate::utils::now_text()
        ],
    )?;
    Ok(())
}

pub fn refresh_credit_statuses(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    let current_month = chrono::Local::now().format("%Y-%m").to_string();
    conn.execute(
        "UPDATE monthly_credits
         SET status = 'available', updated_at = ?1
         WHERE status = 'pending' AND available_month <= ?2 AND remaining_amount > 0",
        params![crate::utils::now_text(), current_month],
    )?;
    Ok(())
}

pub fn update_monthly_credit_status(
    conn: &rusqlite::Connection,
    id: i64,
    status: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE monthly_credits SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, crate::utils::now_text(), id],
    )?;
    Ok(())
}

pub fn order_movement_product_ids(
    conn: &rusqlite::Connection,
    order_id: i64,
) -> anyhow::Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT product_id
         FROM inventory_movements
         WHERE source_type = 'order' AND source_id = ?1 AND product_id IS NOT NULL",
    )?;
    let rows = stmt.query_map([order_id], |row| row.get::<_, i64>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn order_credit_uses(
    conn: &rusqlite::Connection,
    order_id: i64,
) -> anyhow::Result<Vec<MonthlyCreditUseRow>> {
    let mut stmt = conn.prepare(
        "SELECT monthly_credit_id, amount
         FROM order_items
         WHERE order_id = ?1 AND line_type = 'credit' AND monthly_credit_id IS NOT NULL",
    )?;
    let rows = stmt.query_map([order_id], |row| {
        Ok(MonthlyCreditUseRow {
            monthly_credit_id: row.get(0)?,
            amount: row.get::<_, f64>(1)?.abs(),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn restore_monthly_credit_use(
    conn: &rusqlite::Connection,
    credit_id: i64,
    amount: f64,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE monthly_credits
         SET used_amount = MAX(used_amount - ?1, 0),
             remaining_amount = remaining_amount + ?1,
             status = CASE
                WHEN status = 'voided' THEN status
                WHEN available_month <= strftime('%Y-%m', 'now', 'localtime') THEN 'available'
                ELSE 'pending'
             END,
             updated_at = ?2
         WHERE id = ?3",
        params![amount, crate::utils::now_text(), credit_id],
    )?;
    Ok(())
}

pub fn void_credits_generated_by_order(
    conn: &rusqlite::Connection,
    order_id: i64,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE monthly_credits
         SET status = 'voided', remaining_amount = 0, updated_at = ?1
         WHERE source_order_id = ?2",
        params![crate::utils::now_text(), order_id],
    )?;
    Ok(())
}

pub fn delete_order_movements(conn: &rusqlite::Connection, order_id: i64) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM inventory_movements WHERE source_type = 'order' AND source_id = ?1",
        [order_id],
    )?;
    Ok(())
}

pub fn mark_order_voided(
    conn: &rusqlite::Connection,
    order_id: i64,
    remark: Option<&str>,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE orders SET status = 'voided', remark = ?1, updated_at = ?2 WHERE id = ?3",
        params![remark, crate::utils::now_text(), order_id],
    )?;
    Ok(())
}

pub fn mark_order_documents_voided(
    conn: &rusqlite::Connection,
    order_id: i64,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE documents SET status = 'voided' WHERE order_id = ?1",
        [order_id],
    )?;
    Ok(())
}

pub fn order_by_id(conn: &rusqlite::Connection, order_id: i64) -> anyhow::Result<OrderDto> {
    conn.query_row(
        "SELECT id, order_no, order_date, customer_id, customer_name, customer_address,
                product_sales_amount, direct_discount_amount, monthly_credit_used,
                customer_payable_amount, brand_subsidy_amount, cost_amount, gift_cost_amount,
                profit_amount, remark, document_path, print_count, status
         FROM orders WHERE id = ?1",
        [order_id],
        map_order,
    )
    .with_context(|| format!("订单不存在：{order_id}"))
}

fn map_order(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrderDto> {
    Ok(OrderDto {
        id: row.get(0)?,
        order_no: row.get(1)?,
        order_date: row.get(2)?,
        customer_id: row.get(3)?,
        customer_name: row.get(4)?,
        customer_address: row.get(5)?,
        totals: OrderTotalsDto {
            product_sales_amount: row.get(6)?,
            direct_discount_amount: row.get(7)?,
            monthly_credit_used: row.get(8)?,
            customer_payable_amount: row.get(9)?,
            brand_subsidy_amount: row.get(10)?,
            cost_amount: row.get(11)?,
            gift_cost_amount: row.get(12)?,
            profit_amount: row.get(13)?,
        },
        remark: row.get(14)?,
        document_path: row.get(15)?,
        print_count: row.get(16)?,
        status: row.get(17)?,
    })
}

fn map_order_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrderItemDto> {
    Ok(OrderItemDto {
        id: row.get(0)?,
        line_type: row.get(1)?,
        product_id: row.get(2)?,
        product_name: row.get(3)?,
        category: row.get(4)?,
        barcode: row.get(5)?,
        quantity: row.get(6)?,
        unit_price: row.get(7)?,
        amount: row.get(8)?,
        avg_cost: row.get(9)?,
        cost_amount: row.get(10)?,
        profit_amount: row.get(11)?,
        rule_id: row.get(12)?,
        monthly_credit_id: row.get(13)?,
        remark: row.get(14)?,
        sort_order: row.get(15)?,
    })
}

fn map_credit(row: &rusqlite::Row<'_>) -> rusqlite::Result<MonthlyCreditDto> {
    Ok(MonthlyCreditDto {
        id: row.get(0)?,
        source_order_id: row.get(1)?,
        source_order_no: row.get(2)?,
        customer_id: row.get(3)?,
        customer_name: row.get(4)?,
        category: row.get(5)?,
        amount: row.get(6)?,
        used_amount: row.get(7)?,
        remaining_amount: row.get(8)?,
        generated_date: row.get(9)?,
        available_month: row.get(10)?,
        status: row.get(11)?,
        remark: row.get(12)?,
    })
}
