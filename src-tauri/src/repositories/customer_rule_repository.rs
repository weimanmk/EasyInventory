use crate::models::{CustomerProductRuleDto, RuleFilterRequest, SaveCustomerProductRuleRequest};
use crate::utils::now_text;
use rusqlite::{params, params_from_iter, types::Value, OptionalExtension};

pub fn list_customer_product_rules(
    conn: &rusqlite::Connection,
    filter: RuleFilterRequest,
) -> anyhow::Result<Vec<CustomerProductRuleDto>> {
    let mut sql = String::from(
        "SELECT r.id, r.customer_id, c.name, r.product_id, p.name, p.category,
                r.fixed_price, r.threshold_quantity, r.gift_product_id, gp.name, r.gift_quantity,
                r.direct_discount_amount, r.monthly_credit_amount, r.credit_category,
                r.is_active, r.remark
         FROM customer_product_rules r
         JOIN customers c ON c.id = r.customer_id
         JOIN products p ON p.id = r.product_id
         LEFT JOIN products gp ON gp.id = r.gift_product_id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(customer_id) = filter.customer_id {
        sql.push_str(" AND r.customer_id = ?");
        sql_params.push(Value::Integer(customer_id));
    }
    if let Some(product_id) = filter.product_id {
        sql.push_str(" AND r.product_id = ?");
        sql_params.push(Value::Integer(product_id));
    }
    if let Some(category) = filter
        .category
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND p.category = ?");
        sql_params.push(Value::Text(category));
    }
    if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (c.name LIKE ? OR p.name LIKE ?)");
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword));
    }
    if let Some(active) = filter.is_active {
        sql.push_str(if active {
            " AND r.is_active = 1"
        } else {
            " AND r.is_active = 0"
        });
    }
    if let Some(rule_type) = filter.rule_type {
        match rule_type.as_str() {
            "fixed" => sql.push_str(" AND r.fixed_price IS NOT NULL"),
            "gift" => sql.push_str(" AND r.gift_product_id IS NOT NULL"),
            "discount" => sql.push_str(" AND r.direct_discount_amount IS NOT NULL"),
            "credit" => sql.push_str(" AND r.monthly_credit_amount IS NOT NULL"),
            _ => {}
        }
    }
    sql.push_str(" ORDER BY c.name, p.category, p.name LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_customer_rule)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn save_customer_product_rule(
    conn: &rusqlite::Connection,
    payload: SaveCustomerProductRuleRequest,
) -> anyhow::Result<i64> {
    let now = now_text();
    if payload.is_active {
        conn.execute(
            "UPDATE customer_product_rules
             SET is_active = 0, updated_at = ?1
             WHERE customer_id = ?2 AND product_id = ?3 AND (?4 IS NULL OR id != ?4)",
            params![now, payload.customer_id, payload.product_id, payload.id],
        )?;
    }
    let id = if let Some(id) = payload.id {
        conn.execute(
            "UPDATE customer_product_rules SET
             customer_id = ?1, product_id = ?2, fixed_price = ?3, threshold_quantity = ?4,
             gift_product_id = ?5, gift_quantity = ?6, direct_discount_amount = ?7,
             monthly_credit_amount = ?8, credit_category = ?9, is_active = ?10,
             remark = ?11, updated_at = ?12 WHERE id = ?13",
            params![
                payload.customer_id,
                payload.product_id,
                payload.fixed_price,
                payload.threshold_quantity,
                payload.gift_product_id,
                payload.gift_quantity,
                payload.direct_discount_amount,
                payload.monthly_credit_amount,
                payload.credit_category,
                if payload.is_active { 1 } else { 0 },
                payload.remark,
                now,
                id
            ],
        )?;
        id
    } else {
        conn.execute(
            "INSERT INTO customer_product_rules
             (customer_id, product_id, fixed_price, threshold_quantity, gift_product_id,
              gift_quantity, direct_discount_amount, monthly_credit_amount, credit_category,
              is_active, remark, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                payload.customer_id,
                payload.product_id,
                payload.fixed_price,
                payload.threshold_quantity,
                payload.gift_product_id,
                payload.gift_quantity,
                payload.direct_discount_amount,
                payload.monthly_credit_amount,
                payload.credit_category,
                if payload.is_active { 1 } else { 0 },
                payload.remark,
                now
            ],
        )?;
        conn.last_insert_rowid()
    };
    Ok(id)
}

pub fn disable_customer_product_rule(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    conn.execute(
        "UPDATE customer_product_rules SET is_active = 0, updated_at = ?1 WHERE id = ?2",
        params![now_text(), id],
    )?;
    Ok(true)
}

pub fn delete_customer_product_rule(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    conn.execute("DELETE FROM customer_product_rules WHERE id = ?1", [id])?;
    Ok(true)
}

pub fn lookup_import_customer_id(conn: &rusqlite::Connection, name: &str) -> anyhow::Result<i64> {
    conn.query_row(
        "SELECT id FROM customers WHERE name = ?1 AND is_active = 1",
        [name.trim()],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| anyhow::anyhow!("客户不存在或已停用：{}", name.trim()))
}

pub fn lookup_import_product_id(
    conn: &rusqlite::Connection,
    name: &str,
    category: Option<&str>,
) -> anyhow::Result<i64> {
    let name = name.trim();
    let mut sql = "SELECT id FROM products WHERE name = ?1 AND is_active = 1".to_string();
    let mut sql_params = vec![Value::Text(name.to_string())];
    if let Some(category) = category.filter(|value| !value.trim().is_empty()) {
        sql.push_str(" AND category = ?");
        sql_params.push(Value::Text(category.trim().to_string()));
    }
    sql.push_str(" ORDER BY id LIMIT 2");
    let mut stmt = conn.prepare(&sql)?;
    let ids = stmt
        .query_map(params_from_iter(sql_params.iter()), |row| {
            row.get::<_, i64>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    match ids.as_slice() {
        [id] => Ok(*id),
        [] => anyhow::bail!("商品不存在或已停用：{name}"),
        _ => anyhow::bail!("商品名称重复，请填写类别：{name}"),
    }
}

fn map_customer_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<CustomerProductRuleDto> {
    Ok(CustomerProductRuleDto {
        id: row.get(0)?,
        customer_id: row.get(1)?,
        customer_name: row.get(2)?,
        product_id: row.get(3)?,
        product_name: row.get(4)?,
        category: row.get(5)?,
        fixed_price: row.get(6)?,
        threshold_quantity: row.get(7)?,
        gift_product_id: row.get(8)?,
        gift_product_name: row.get(9)?,
        gift_quantity: row.get(10)?,
        direct_discount_amount: row.get(11)?,
        monthly_credit_amount: row.get(12)?,
        credit_category: row.get(13)?,
        is_active: row.get::<_, i64>(14)? == 1,
        remark: row.get(15)?,
    })
}
