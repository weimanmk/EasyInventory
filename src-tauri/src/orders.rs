use crate::db;
use crate::models::*;
use crate::utils::{money, next_month, normalize_date, now_text};
use anyhow::{anyhow, Context};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

#[derive(Debug, Clone)]
struct RuleRow {
    id: i64,
    fixed_price: Option<f64>,
    threshold_quantity: Option<f64>,
    gift_product_id: Option<i64>,
    gift_quantity: Option<f64>,
    direct_discount_amount: Option<f64>,
    monthly_credit_amount: Option<f64>,
    credit_category: Option<String>,
}

pub fn preview_quote(
    conn: &Connection,
    request: PreviewQuoteRequest,
) -> anyhow::Result<QuotePreviewDto> {
    if request.quantity <= 0.0 {
        return Err(anyhow!("数量必须大于 0"));
    }
    let product = db::product_by_id(conn, request.product_id)?;
    let rule = active_rule(conn, request.customer_id, request.product_id)?;
    let (unit_price, price_source) = if let Some(manual_price) = request.manual_price {
        (manual_price, "manual".to_string())
    } else if let Some(rule) = &rule {
        if let Some(fixed_price) = rule.fixed_price {
            (fixed_price, "customer_fixed_price".to_string())
        } else if product.default_price > 0.0 {
            (product.default_price, "default_price".to_string())
        } else {
            (0.0, "zero".to_string())
        }
    } else if product.default_price > 0.0 {
        (product.default_price, "default_price".to_string())
    } else {
        (0.0, "zero".to_string())
    };

    let mut messages = Vec::new();
    match price_source.as_str() {
        "manual" => messages.push("手动价".to_string()),
        "customer_fixed_price" => messages.push("客户固定价".to_string()),
        "default_price" => messages.push("默认售价".to_string()),
        _ => messages.push("价格为 0".to_string()),
    }

    let mut gift_preview = None;
    let mut discount_preview = None;
    let mut monthly_preview = None;
    let mut rule_id = None;

    if let Some(rule) = rule {
        rule_id = Some(rule.id);
        if let Some(threshold) = rule.threshold_quantity.filter(|value| *value > 0.0) {
            let times = (request.quantity / threshold).floor();
            if times > 0.0 {
                if let (Some(gift_product_id), Some(gift_quantity)) =
                    (rule.gift_product_id, rule.gift_quantity)
                {
                    if gift_quantity > 0.0 {
                        let gift = db::product_by_id(conn, gift_product_id)?;
                        let quantity = money(times * gift_quantity);
                        gift_preview = Some(GiftPreviewDto {
                            product_id: gift_product_id,
                            product_name: gift.name,
                            quantity,
                        });
                        messages.push(format!("每满 {} 送 {}", threshold, quantity));
                    }
                }
                if let Some(discount) = rule.direct_discount_amount.filter(|value| *value > 0.0) {
                    let amount = money(times * discount);
                    discount_preview = Some(DiscountPreviewDto { amount });
                    messages.push(format!("本单折现 {}", amount));
                }
                if let Some(credit) = rule.monthly_credit_amount.filter(|value| *value > 0.0) {
                    let amount = money(times * credit);
                    let category = rule
                        .credit_category
                        .unwrap_or_else(|| product.category.clone());
                    monthly_preview = Some(MonthlyCreditPreviewDto { amount, category });
                    messages.push(format!(
                        "生成 {} 月费 {}",
                        next_month(&request.order_date),
                        amount
                    ));
                }
            }
        }
    }

    Ok(QuotePreviewDto {
        product_id: request.product_id,
        unit_price: money(unit_price),
        price_source,
        amount: money(request.quantity * unit_price),
        rule_id,
        gift_preview,
        direct_discount_preview: discount_preview,
        monthly_credit_preview: monthly_preview,
        message: messages.join("；"),
    })
}

pub fn save_order(
    conn: &mut Connection,
    request: SaveOrderRequest,
) -> anyhow::Result<SaveOrderResponse> {
    if request.customer_id <= 0 {
        return Err(anyhow!("必须选择客户"));
    }
    if request.items.is_empty() {
        return Err(anyhow!("必须至少选择一条商品"));
    }
    for item in &request.items {
        if item.quantity <= 0.0 {
            return Err(anyhow!("商品数量必须大于 0"));
        }
        if item.unit_price < 0.0 {
            return Err(anyhow!("商品单价不能小于 0"));
        }
    }

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let order_no = next_order_no(&tx, &request.order_date)?;
    let customer = db::customer_by_id(&tx, request.customer_id)?;
    let address = request
        .customer_address
        .clone()
        .or_else(|| customer.address.clone());
    let now = now_text();
    let totals = OrderTotalsDto::default();

    tx.execute(
        "INSERT INTO orders
         (order_no, order_date, customer_id, customer_name, customer_address,
          product_sales_amount, direct_discount_amount, monthly_credit_used,
          customer_payable_amount, brand_subsidy_amount, cost_amount, gift_cost_amount,
          profit_amount, remark, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 0, 0, 0, 0, 0, ?6, 'normal', ?7, ?7)",
        params![
            order_no,
            normalize_date(&request.order_date),
            request.customer_id,
            customer.name,
            address,
            request.remark,
            now
        ],
    )?;
    let order_id = tx.last_insert_rowid();

    let mut totals = totals;
    let mut sort_order = 1_i64;
    for item in &request.items {
        let product = db::product_by_id(&tx, item.product_id)?;
        let rule = active_rule(&tx, request.customer_id, item.product_id)?;
        let amount = money(item.quantity * item.unit_price);
        let cost_amount = money(product.avg_cost * item.quantity);
        let profit = money(amount - cost_amount);
        totals.product_sales_amount = money(totals.product_sales_amount + amount);
        totals.cost_amount = money(totals.cost_amount + cost_amount);
        totals.profit_amount = money(totals.profit_amount + profit);

        insert_order_item(
            &tx,
            NewOrderItem {
                order_id,
                line_type: "normal",
                product_id: Some(product.id),
                product_name: Some(&product.name),
                category: Some(&product.category),
                barcode: product.barcode.as_deref(),
                quantity: item.quantity,
                unit_price: item.unit_price,
                amount,
                avg_cost: product.avg_cost,
                cost_amount,
                profit_amount: profit,
                related_product_id: None,
                rule_id: rule.as_ref().map(|rule| rule.id),
                monthly_credit_id: None,
                remark: item.remark.as_deref(),
                sort_order,
            },
        )?;
        sort_order += 1;

        insert_movement(
            &tx,
            NewMovement {
                date: &request.order_date,
                product_id: product.id,
                movement_type: "outbound",
                quantity: item.quantity,
                unit_price: item.unit_price,
                amount,
                order_id,
                order_no: &order_no,
                remark: "订单出库",
            },
        )?;
        db::recalc_stock_balance(&tx, product.id)?;

        apply_credit_uses(
            &tx,
            order_id,
            &order_no,
            &mut sort_order,
            &mut totals,
            item.monthly_credit_uses.clone().unwrap_or_default(),
        )?;

        if let Some(rule) = rule {
            apply_rule_effects(
                &tx,
                &mut sort_order,
                &mut totals,
                RuleEffectInput {
                    request: &request,
                    order_id,
                    order_no: &order_no,
                    product: &product,
                    rule: &rule,
                    quantity: item.quantity,
                },
            )?;
        }
    }

    totals.customer_payable_amount = money(
        totals.product_sales_amount - totals.direct_discount_amount - totals.monthly_credit_used,
    );

    tx.execute(
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
            now_text(),
            order_id
        ],
    )?;

    tx.commit()?;
    Ok(SaveOrderResponse {
        order_id,
        order_no,
        document_path: String::new(),
        totals,
    })
}

pub fn get_order_detail(conn: &Connection, order_id: i64) -> anyhow::Result<OrderDetailDto> {
    let order = order_by_id(conn, order_id)?;
    let mut stmt = conn.prepare(
        "SELECT id, line_type, product_id, product_name, category, barcode, quantity, unit_price,
                amount, avg_cost, cost_amount, profit_amount, rule_id, monthly_credit_id, remark, sort_order
         FROM order_items
         WHERE order_id = ?1
         ORDER BY sort_order, id",
    )?;
    let rows = stmt.query_map([order_id], |row| {
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
    })?;
    Ok(OrderDetailDto {
        order,
        items: rows.collect::<Result<Vec<_>, _>>()?,
    })
}

pub fn list_orders(conn: &Connection, filter: ListOrdersRequest) -> anyhow::Result<Vec<OrderDto>> {
    let mut sql = String::from(
        "SELECT id, order_no, order_date, customer_id, customer_name, customer_address,
                product_sales_amount, direct_discount_amount, monthly_credit_used,
                customer_payable_amount, brand_subsidy_amount, cost_amount, gift_cost_amount,
                profit_amount, remark, document_path, print_count, status
         FROM orders WHERE 1 = 1",
    );
    if let Some(start) = filter.start_date {
        sql.push_str(&format!(" AND order_date >= '{}'", escape_sql(&start)));
    }
    if let Some(end) = filter.end_date {
        sql.push_str(&format!(" AND order_date <= '{}'", escape_sql(&end)));
    }
    if let Some(customer_id) = filter.customer_id {
        sql.push_str(&format!(" AND customer_id = {customer_id}"));
    }
    if let Some(order_no) = filter.order_no {
        sql.push_str(&format!(" AND order_no LIKE '%{}%'", escape_sql(&order_no)));
    }
    if let Some(status) = filter.status {
        sql.push_str(&format!(" AND status = '{}'", escape_sql(&status)));
    }
    sql.push_str(" ORDER BY order_date DESC, id DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_order)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn list_monthly_credits(
    conn: &Connection,
    filter: MonthlyCreditFilterRequest,
) -> anyhow::Result<Vec<MonthlyCreditDto>> {
    refresh_credit_statuses(conn)?;
    let mut sql = String::from(
        "SELECT m.id, m.source_order_id, m.source_order_no, m.customer_id, c.name, m.category,
                m.amount, m.used_amount, m.remaining_amount, m.generated_date, m.available_month,
                m.status, m.remark
         FROM monthly_credits m
         JOIN customers c ON c.id = m.customer_id
         WHERE 1 = 1",
    );
    if let Some(customer_id) = filter.customer_id {
        sql.push_str(&format!(" AND m.customer_id = {customer_id}"));
    }
    if let Some(category) = filter.category {
        sql.push_str(&format!(" AND m.category = '{}'", escape_sql(&category)));
    }
    if let Some(status) = filter.status {
        sql.push_str(&format!(" AND m.status = '{}'", escape_sql(&status)));
    }
    if let Some(start) = filter.start_date {
        sql.push_str(&format!(
            " AND m.generated_date >= '{}'",
            escape_sql(&start)
        ));
    }
    if let Some(end) = filter.end_date {
        sql.push_str(&format!(" AND m.generated_date <= '{}'", escape_sql(&end)));
    }
    if let Some(month) = filter.available_month {
        sql.push_str(&format!(
            " AND m.available_month = '{}'",
            escape_sql(&month)
        ));
    }
    sql.push_str(" ORDER BY m.generated_date DESC, m.id DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_credit)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn available_monthly_credits(
    conn: &Connection,
    customer_id: i64,
    category: String,
    order_date: String,
) -> anyhow::Result<Vec<MonthlyCreditDto>> {
    refresh_credit_statuses(conn)?;
    let order_month = normalize_date(&order_date)[..7].to_string();
    let filter = MonthlyCreditFilterRequest {
        customer_id: Some(customer_id),
        category: Some(category),
        status: Some("available".to_string()),
        start_date: None,
        end_date: None,
        available_month: None,
    };
    let credits = list_monthly_credits(conn, filter)?
        .into_iter()
        .filter(|credit| credit.remaining_amount > 0.0 && credit.available_month <= order_month)
        .collect();
    Ok(credits)
}

pub fn close_or_void_credit(conn: &Connection, id: i64, status: &str) -> anyhow::Result<()> {
    if !matches!(status, "closed" | "voided") {
        return Err(anyhow!("不支持的月费状态"));
    }
    conn.execute(
        "UPDATE monthly_credits SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now_text(), id],
    )?;
    Ok(())
}

pub fn void_order(
    conn: &mut Connection,
    order_id: i64,
    reason: Option<String>,
) -> anyhow::Result<OrderDto> {
    let tx = conn.transaction()?;
    let order = order_by_id(&tx, order_id)?;
    if order.status == "voided" {
        return Err(anyhow!("订单已作废"));
    }

    let mut product_stmt = tx.prepare(
        "SELECT DISTINCT product_id
         FROM inventory_movements
         WHERE source_type = 'order' AND source_id = ?1 AND product_id IS NOT NULL",
    )?;
    let product_ids = product_stmt
        .query_map([order_id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(product_stmt);

    let mut credit_stmt = tx.prepare(
        "SELECT id, monthly_credit_id, amount
         FROM order_items
         WHERE order_id = ?1 AND line_type = 'credit' AND monthly_credit_id IS NOT NULL",
    )?;
    let credit_uses = credit_stmt
        .query_map([order_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, f64>(2)?.abs(),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(credit_stmt);

    for (_, credit_id, amount) in credit_uses {
        tx.execute(
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
            params![amount, now_text(), credit_id],
        )?;
    }

    tx.execute(
        "UPDATE monthly_credits
         SET status = 'voided', remaining_amount = 0, updated_at = ?1
         WHERE source_order_id = ?2",
        params![now_text(), order_id],
    )?;
    tx.execute(
        "DELETE FROM inventory_movements WHERE source_type = 'order' AND source_id = ?1",
        [order_id],
    )?;

    for product_id in product_ids {
        db::recalc_stock_balance(&tx, product_id)?;
    }

    let remark = match (
        order.remark,
        reason.filter(|value| !value.trim().is_empty()),
    ) {
        (Some(existing), Some(reason)) => Some(format!("{existing}；作废原因：{reason}")),
        (None, Some(reason)) => Some(format!("作废原因：{reason}")),
        (existing, None) => existing,
    };
    tx.execute(
        "UPDATE orders SET status = 'voided', remark = ?1, updated_at = ?2 WHERE id = ?3",
        params![remark, now_text(), order_id],
    )?;
    tx.execute(
        "UPDATE documents SET status = 'voided' WHERE order_id = ?1",
        [order_id],
    )?;

    let updated = order_by_id(&tx, order_id)?;
    tx.commit()?;
    Ok(updated)
}

fn active_rule(
    conn: &Connection,
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

fn next_order_no(conn: &Connection, order_date: &str) -> anyhow::Result<String> {
    let date = normalize_date(order_date);
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

struct NewOrderItem<'a> {
    order_id: i64,
    line_type: &'a str,
    product_id: Option<i64>,
    product_name: Option<&'a str>,
    category: Option<&'a str>,
    barcode: Option<&'a str>,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    avg_cost: f64,
    cost_amount: f64,
    profit_amount: f64,
    related_product_id: Option<i64>,
    rule_id: Option<i64>,
    monthly_credit_id: Option<i64>,
    remark: Option<&'a str>,
    sort_order: i64,
}

struct NewMovement<'a> {
    date: &'a str,
    product_id: i64,
    movement_type: &'a str,
    quantity: f64,
    unit_price: f64,
    amount: f64,
    order_id: i64,
    order_no: &'a str,
    remark: &'a str,
}

struct RuleEffectInput<'a> {
    request: &'a SaveOrderRequest,
    order_id: i64,
    order_no: &'a str,
    product: &'a ProductDto,
    rule: &'a RuleRow,
    quantity: f64,
}

fn insert_order_item(conn: &Connection, item: NewOrderItem<'_>) -> anyhow::Result<()> {
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

fn insert_movement(conn: &Connection, movement: NewMovement<'_>) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount,
          source_type, source_id, source_no, remark, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'order', ?7, ?8, ?9, ?10)",
        params![
            normalize_date(movement.date),
            movement.product_id,
            movement.movement_type,
            movement.quantity,
            movement.unit_price,
            movement.amount,
            movement.order_id,
            movement.order_no,
            movement.remark,
            now_text()
        ],
    )?;
    Ok(())
}

fn apply_credit_uses(
    tx: &Transaction<'_>,
    order_id: i64,
    order_no: &str,
    sort_order: &mut i64,
    totals: &mut OrderTotalsDto,
    uses: Vec<MonthlyCreditUseRequest>,
) -> anyhow::Result<()> {
    for credit_use in uses {
        if credit_use.amount <= 0.0 {
            continue;
        }
        let credit: MonthlyCreditDto = tx.query_row(
            "SELECT m.id, m.source_order_id, m.source_order_no, m.customer_id, c.name, m.category,
                    m.amount, m.used_amount, m.remaining_amount, m.generated_date,
                    m.available_month, m.status, m.remark
             FROM monthly_credits m
             JOIN customers c ON c.id = m.customer_id
             WHERE m.id = ?1",
            [credit_use.monthly_credit_id],
            map_credit,
        )?;
        if credit.status != "available"
            || credit.remaining_amount + f64::EPSILON < credit_use.amount
        {
            return Err(anyhow!("月费额度不可用或余额不足"));
        }
        let used = money(credit.used_amount + credit_use.amount);
        let remaining = money(credit.remaining_amount - credit_use.amount);
        let status = if remaining <= 0.0 {
            "used_up"
        } else {
            "available"
        };
        tx.execute(
            "UPDATE monthly_credits
             SET used_amount = ?1, remaining_amount = ?2, status = ?3, updated_at = ?4
             WHERE id = ?5",
            params![used, remaining, status, now_text(), credit.id],
        )?;
        let remark = format!("使用月费 {}，来源 {}", credit_use.amount, order_no);
        insert_order_item(
            tx,
            NewOrderItem {
                order_id,
                line_type: "credit",
                product_id: None,
                product_name: Some("月费抵扣"),
                category: Some(&credit.category),
                barcode: None,
                quantity: 1.0,
                unit_price: -credit_use.amount,
                amount: -credit_use.amount,
                avg_cost: 0.0,
                cost_amount: 0.0,
                profit_amount: 0.0,
                related_product_id: None,
                rule_id: None,
                monthly_credit_id: Some(credit.id),
                remark: Some(&remark),
                sort_order: *sort_order,
            },
        )?;
        *sort_order += 1;
        totals.monthly_credit_used = money(totals.monthly_credit_used + credit_use.amount);
    }
    Ok(())
}

fn apply_rule_effects(
    tx: &Transaction<'_>,
    sort_order: &mut i64,
    totals: &mut OrderTotalsDto,
    input: RuleEffectInput<'_>,
) -> anyhow::Result<()> {
    let request = input.request;
    let order_id = input.order_id;
    let order_no = input.order_no;
    let product = input.product;
    let rule = input.rule;
    let quantity = input.quantity;
    let Some(threshold) = rule.threshold_quantity.filter(|value| *value > 0.0) else {
        return Ok(());
    };
    let times = (quantity / threshold).floor();
    if times <= 0.0 {
        return Ok(());
    }

    if let (Some(gift_product_id), Some(gift_quantity)) = (rule.gift_product_id, rule.gift_quantity)
    {
        let total_gift_qty = money(times * gift_quantity);
        if total_gift_qty > 0.0 {
            let gift = db::product_by_id(tx, gift_product_id)?;
            let gift_cost = money(gift.avg_cost * total_gift_qty);
            insert_order_item(
                tx,
                NewOrderItem {
                    order_id,
                    line_type: "gift",
                    product_id: Some(gift.id),
                    product_name: Some(&gift.name),
                    category: Some(&gift.category),
                    barcode: gift.barcode.as_deref(),
                    quantity: total_gift_qty,
                    unit_price: 0.0,
                    amount: 0.0,
                    avg_cost: gift.avg_cost,
                    cost_amount: gift_cost,
                    profit_amount: -gift_cost,
                    related_product_id: Some(product.id),
                    rule_id: Some(rule.id),
                    monthly_credit_id: None,
                    remark: Some("赠品"),
                    sort_order: *sort_order,
                },
            )?;
            *sort_order += 1;
            insert_movement(
                tx,
                NewMovement {
                    date: &request.order_date,
                    product_id: gift.id,
                    movement_type: "gift_outbound",
                    quantity: total_gift_qty,
                    unit_price: 0.0,
                    amount: 0.0,
                    order_id,
                    order_no,
                    remark: "买赠赠品出库",
                },
            )?;
            db::recalc_stock_balance(tx, gift.id)?;
            totals.gift_cost_amount = money(totals.gift_cost_amount + gift_cost);
            totals.cost_amount = money(totals.cost_amount + gift_cost);
            totals.profit_amount = money(totals.profit_amount - gift_cost);
        }
    }

    if let Some(discount) = rule.direct_discount_amount.filter(|value| *value > 0.0) {
        let amount = money(times * discount);
        insert_order_item(
            tx,
            NewOrderItem {
                order_id,
                line_type: "discount",
                product_id: None,
                product_name: Some("本单折现"),
                category: Some(&product.category),
                barcode: None,
                quantity: 1.0,
                unit_price: -amount,
                amount: -amount,
                avg_cost: 0.0,
                cost_amount: 0.0,
                profit_amount: -amount,
                related_product_id: Some(product.id),
                rule_id: Some(rule.id),
                monthly_credit_id: None,
                remark: Some("规则折现"),
                sort_order: *sort_order,
            },
        )?;
        *sort_order += 1;
        totals.direct_discount_amount = money(totals.direct_discount_amount + amount);
        totals.profit_amount = money(totals.profit_amount - amount);
    }

    if let Some(credit) = rule.monthly_credit_amount.filter(|value| *value > 0.0) {
        let amount = money(times * credit);
        let category = rule
            .credit_category
            .clone()
            .unwrap_or_else(|| product.category.clone());
        tx.execute(
            "INSERT INTO monthly_credits
             (source_order_id, source_order_no, customer_id, category, amount,
              used_amount, remaining_amount, generated_date, available_month, status, remark,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?5, ?6, ?7, 'pending', ?8, ?9, ?9)",
            params![
                order_id,
                order_no,
                request.customer_id,
                category,
                amount,
                normalize_date(&request.order_date),
                next_month(&request.order_date),
                "规则生成月费",
                now_text()
            ],
        )?;
        totals.brand_subsidy_amount = money(totals.brand_subsidy_amount + amount);
    }

    Ok(())
}

fn refresh_credit_statuses(conn: &Connection) -> anyhow::Result<()> {
    let current_month = chrono::Local::now().format("%Y-%m").to_string();
    conn.execute(
        "UPDATE monthly_credits
         SET status = 'available', updated_at = ?1
         WHERE status = 'pending' AND available_month <= ?2 AND remaining_amount > 0",
        params![now_text(), current_month],
    )?;
    Ok(())
}

fn order_by_id(conn: &Connection, order_id: i64) -> anyhow::Result<OrderDto> {
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

fn escape_sql(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::sync::{Arc, Barrier};
    use std::time::{Duration, Instant};

    #[test]
    fn next_month_rolls_year() {
        assert_eq!(crate::utils::next_month("2026-12-15"), "2027-01");
    }

    #[test]
    fn quote_discount_math_is_stable() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::seed_settings(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('A', '统一', 12, 10, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('B', '统一', 0, 10, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('溪南', '客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customer_product_rules
             (customer_id, product_id, fixed_price, threshold_quantity, gift_product_id, gift_quantity,
              direct_discount_amount, monthly_credit_amount, credit_category, is_active, created_at, updated_at)
             VALUES (1, 1, 10, 10, 2, 1, 5, 20, '统一', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        let quote = preview_quote(
            &conn,
            PreviewQuoteRequest {
                customer_id: 1,
                product_id: 1,
                quantity: 25.0,
                manual_price: None,
                order_date: "2026-05-30".to_string(),
            },
        )
        .unwrap();
        assert_eq!(quote.unit_price, 10.0);
        assert_eq!(quote.gift_preview.unwrap().quantity, 2.0);
        assert_eq!(quote.direct_discount_preview.unwrap().amount, 10.0);
        assert_eq!(quote.monthly_credit_preview.unwrap().amount, 40.0);
    }

    #[test]
    fn void_order_rolls_back_stock_and_credit_use() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::seed_settings(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('A', '统一', 10, 10, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('溪南', '客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES ('2026-05-01', 1, 'inbound', 10, 3, 30, 'test', 'test', ?1)",
            [&now],
        )
        .unwrap();
        db::recalc_stock_balance(&conn, 1).unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount, customer_payable_amount,
              remark, status, created_at, updated_at)
             VALUES ('20260401001', '2026-04-01', 1, '客户', 10, 10, NULL, 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO monthly_credits
             (source_order_id, source_order_no, customer_id, category, amount, used_amount, remaining_amount,
              generated_date, available_month, status, remark, created_at, updated_at)
             VALUES (1, '20260401001', 1, '统一', 5, 0, 5, '2026-04-01', '2026-05', 'available', NULL, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let response = save_order(
            &mut conn,
            SaveOrderRequest {
                order_date: "2026-05-30".to_string(),
                customer_id: 1,
                customer_address: None,
                remark: None,
                items: vec![SaveOrderItemRequest {
                    product_id: 1,
                    quantity: 2.0,
                    unit_price: 10.0,
                    remark: None,
                    monthly_credit_uses: Some(vec![MonthlyCreditUseRequest {
                        monthly_credit_id: 1,
                        amount: 5.0,
                    }]),
                }],
            },
        )
        .unwrap();
        let stock_after_order: f64 = conn
            .query_row(
                "SELECT current_stock FROM stock_balances WHERE product_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_after_order, 8.0);

        void_order(&mut conn, response.order_id, Some("测试作废".to_string())).unwrap();

        let stock_after_void: f64 = conn
            .query_row(
                "SELECT current_stock FROM stock_balances WHERE product_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let credit: (f64, f64, String) = conn
            .query_row(
                "SELECT used_amount, remaining_amount, status FROM monthly_credits WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM orders WHERE id = ?1",
                [response.order_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stock_after_void, 10.0);
        assert_eq!(credit, (0.0, 5.0, "available".to_string()));
        assert_eq!(status, "voided");
    }

    #[test]
    fn list_orders_filters_by_date_customer_and_status() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('筛选', '客户A', 1, ?1, ?1), ('筛选', '客户B', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES
             ('20260601001', '2026-06-01', 1, '客户A', 10, 'normal', ?1, ?1),
             ('20260602001', '2026-06-02', 1, '客户A', 20, 'voided', ?1, ?1),
             ('20260603001', '2026-06-03', 2, '客户B', 30, 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let rows = list_orders(
            &conn,
            ListOrdersRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-02".to_string()),
                customer_id: Some(1),
                order_no: Some("202606".to_string()),
                status: Some("normal".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].order_no, "20260601001");
        assert_eq!(rows[0].customer_name, "客户A");
    }

    #[test]
    fn monthly_credit_lifecycle_filters_and_status_changes() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('月费', '月费客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES ('20260501001', '2026-05-01', 1, '月费客户', 100, 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO monthly_credits
             (source_order_id, source_order_no, customer_id, category, amount, used_amount, remaining_amount,
              generated_date, available_month, status, remark, created_at, updated_at)
             VALUES
             (1, '20260501001', 1, '饮料', 30, 0, 30, '2026-05-01', '2026-06', 'pending', NULL, ?1, ?1),
             (1, '20260501001', 1, '饮料', 20, 0, 20, '2026-05-01', '2026-07', 'pending', NULL, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let available =
            available_monthly_credits(&conn, 1, "饮料".to_string(), "2026-06-15".to_string())
                .unwrap();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].remaining_amount, 30.0);

        close_or_void_credit(&conn, 1, "closed").unwrap();
        close_or_void_credit(&conn, 2, "voided").unwrap();

        let closed = list_monthly_credits(
            &conn,
            MonthlyCreditFilterRequest {
                customer_id: Some(1),
                category: Some("饮料".to_string()),
                status: Some("closed".to_string()),
                start_date: None,
                end_date: None,
                available_month: None,
            },
        )
        .unwrap();
        let voided = list_monthly_credits(
            &conn,
            MonthlyCreditFilterRequest {
                customer_id: Some(1),
                category: Some("饮料".to_string()),
                status: Some("voided".to_string()),
                start_date: None,
                end_date: None,
                available_month: None,
            },
        )
        .unwrap();

        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].status, "closed");
        assert_eq!(voided.len(), 1);
        assert_eq!(voided[0].status, "voided");
    }

    #[test]
    fn concurrent_orders_keep_unique_order_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("concurrent-orders.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.busy_timeout(Duration::from_secs(10)).unwrap();
        db::init_schema(&conn).unwrap();
        db::seed_settings(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('并发商品', '压力', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('压力', '并发客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES ('2026-06-01', 1, 'inbound', 1000, 5, 5000, 'test', '并发测试库存', ?1)",
            [&now],
        )
        .unwrap();
        db::recalc_stock_balance(&conn, 1).unwrap();
        drop(conn);

        let worker_count = 16;
        let barrier = Arc::new(Barrier::new(worker_count));
        let handles = (0..worker_count)
            .map(|_| {
                let db_path = db_path.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let mut conn = Connection::open(db_path).unwrap();
                    conn.busy_timeout(Duration::from_secs(10)).unwrap();
                    barrier.wait();
                    save_order(
                        &mut conn,
                        SaveOrderRequest {
                            order_date: "2026-06-01".to_string(),
                            customer_id: 1,
                            customer_address: None,
                            remark: None,
                            items: vec![SaveOrderItemRequest {
                                product_id: 1,
                                quantity: 1.0,
                                unit_price: 10.0,
                                remark: None,
                                monthly_credit_uses: None,
                            }],
                        },
                    )
                    .unwrap()
                    .order_no
                })
            })
            .collect::<Vec<_>>();

        let mut order_numbers = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        order_numbers.sort();
        order_numbers.dedup();

        let conn = Connection::open(&db_path).unwrap();
        let order_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))
            .unwrap();
        let distinct_count: i64 = conn
            .query_row("SELECT COUNT(DISTINCT order_no) FROM orders", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(order_count, worker_count as i64);
        assert_eq!(distinct_count, worker_count as i64);
        assert_eq!(order_numbers.len(), worker_count);
    }

    #[test]
    fn large_order_save_finishes_under_three_seconds() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::seed_settings(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('大单商品', '性能', 12, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('性能', '大单客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES ('2026-06-01', 1, 'inbound', 10000, 5, 50000, 'test', '大单测试库存', ?1)",
            [&now],
        )
        .unwrap();
        db::recalc_stock_balance(&conn, 1).unwrap();

        let request = SaveOrderRequest {
            order_date: "2026-06-01".to_string(),
            customer_id: 1,
            customer_address: None,
            remark: Some("100 行订单性能测试".to_string()),
            items: (0..100)
                .map(|index| SaveOrderItemRequest {
                    product_id: 1,
                    quantity: 1.0,
                    unit_price: 12.0,
                    remark: Some(format!("行{index}")),
                    monthly_credit_uses: None,
                })
                .collect(),
        };

        let started = Instant::now();
        let response = save_order(&mut conn, request).unwrap();
        let elapsed = started.elapsed();
        let item_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM order_items WHERE order_id = ?1 AND line_type = 'normal'",
                [response.order_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(item_count, 100);
        assert_eq!(response.totals.customer_payable_amount, 1200.0);
        assert!(
            elapsed < Duration::from_secs(3),
            "100 行订单保存耗时 {:?}",
            elapsed
        );
    }

    #[test]
    fn high_volume_order_listing_stays_under_two_seconds() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let tx = conn.transaction().unwrap();
        let now = now_text();
        tx.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('高量', '高量客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        for index in 0..10_000 {
            tx.execute(
                "INSERT INTO orders
                 (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
                 VALUES (?1, ?2, 1, '高量客户', 10, 'normal', ?3, ?3)",
                params![
                    format!("202606{index:05}"),
                    if index % 2 == 0 {
                        "2026-06-01"
                    } else {
                        "2026-06-02"
                    },
                    now
                ],
            )
            .unwrap();
        }
        tx.commit().unwrap();

        let started = Instant::now();
        let rows = list_orders(
            &conn,
            ListOrdersRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                customer_id: Some(1),
                order_no: Some("202606".to_string()),
                status: Some("normal".to_string()),
            },
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert_eq!(rows.len(), 500);
        assert!(
            elapsed < Duration::from_secs(2),
            "万级订单列表查询耗时 {:?}",
            elapsed
        );
    }
}
