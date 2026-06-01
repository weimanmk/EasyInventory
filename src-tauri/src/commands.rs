use crate::app::AppState;
use crate::db;
use crate::excel;
use crate::logger;
use crate::models::*;
use crate::orders;
use crate::reports;
use crate::utils::{money, normalize_date, now_text};
use rusqlite::{params, OptionalExtension};
use tauri::State;

fn ok<T: serde::Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse::ok(data)
}

fn fail<T: serde::Serialize>(err: anyhow::Error) -> ApiResponse<T> {
    let message = err.to_string();
    let chain = err
        .chain()
        .skip(1)
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(" | caused by: ");
    if chain.is_empty() {
        logger::error("command", &message);
    } else {
        logger::error("command", format!("{message} | {chain}"));
    }
    ApiResponse::err("INTERNAL_ERROR", message)
}

#[tauri::command]
pub fn get_app_status(state: State<AppState>) -> ApiResponse<AppStatusDto> {
    ok(state.app_status())
}

#[tauri::command]
pub fn write_client_log(payload: ClientLogRequest) -> ApiResponse<bool> {
    let module = format!(
        "client:{}",
        payload
            .module
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("app")
    );
    let mut message = payload.message;
    if let Some(details) = payload.details.filter(|value| !value.trim().is_empty()) {
        message.push_str(" | details: ");
        message.push_str(&details);
    }
    match payload.level.trim().to_ascii_uppercase().as_str() {
        "ERROR" => logger::error(&module, message),
        "WARN" | "WARNING" => logger::warn(&module, message),
        _ => logger::info(&module, message),
    }
    ok(true)
}

#[tauri::command]
pub fn list_products(
    state: State<AppState>,
    filter: Option<ListProductsRequest>,
) -> ApiResponse<Vec<ProductDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let filter = filter.unwrap_or(ListProductsRequest {
            category: None,
            keyword: None,
            only_low_stock: None,
            only_in_stock: None,
            is_active: Some(true),
        });
        logger::info("product", format!("list_products filter={filter:?}"));
        let mut sql = String::from(
            "SELECT p.id, p.name, p.category, p.barcode, p.default_price, p.safety_stock, p.unit,
                    COALESCE(s.current_stock, 0), COALESCE(s.avg_cost, 0), COALESCE(s.stock_value, 0),
                    p.is_active, p.remark
             FROM products p
             LEFT JOIN stock_balances s ON s.product_id = p.id
             WHERE 1 = 1",
        );
        if let Some(category) = filter
            .category
            .filter(|value| !value.is_empty() && value != "全部")
        {
            sql.push_str(&format!(" AND p.category = '{}'", escape_sql(&category)));
        }
        if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
            let keyword = escape_sql(&keyword);
            sql.push_str(&format!(
                " AND (p.name LIKE '%{keyword}%' OR p.barcode LIKE '%{keyword}%')"
            ));
        }
        if filter.only_low_stock.unwrap_or(false) {
            sql.push_str(" AND COALESCE(s.current_stock, 0) <= p.safety_stock");
        }
        if filter.only_in_stock.unwrap_or(false) {
            sql.push_str(" AND COALESCE(s.current_stock, 0) > 0");
        }
        if let Some(active) = filter.is_active {
            sql.push_str(if active {
                " AND p.is_active = 1"
            } else {
                " AND p.is_active = 0"
            });
        }
        sql.push_str(" ORDER BY p.category, p.name LIMIT 1000");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], db::map_product)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    if let Ok(items) = &result {
        logger::info(
            "product",
            format!("list_products result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_product(state: State<AppState>, payload: ProductPayload) -> ApiResponse<ProductDto> {
    logger::info("product", format!("create_product payload={payload:?}"));
    let result = (|| {
        if payload.name.trim().is_empty() || payload.category.trim().is_empty() {
            anyhow::bail!("商品名称和类别必填");
        }
        let conn = state.connection()?;
        let now = now_text();
        conn.execute(
            "INSERT INTO products
             (name, category, barcode, default_price, safety_stock, unit, is_active, remark, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?8)",
            params![
                payload.name.trim(),
                payload.category.trim(),
                payload.barcode,
                payload.default_price.unwrap_or(0.0),
                payload.safety_stock.unwrap_or(0.0),
                payload.unit,
                payload.remark,
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
             VALUES (?1, 0, 0, 0, ?2)",
            params![id, now_text()],
        )?;
        db::product_by_id(&conn, id)
    })();
    if let Ok(product) = &result {
        logger::info(
            "product",
            format!(
                "create_product success id={} name={} category={} barcode={:?}",
                product.id, product.name, product.category, product.barcode
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_product(
    state: State<AppState>,
    id: i64,
    payload: ProductPayload,
) -> ApiResponse<ProductDto> {
    logger::info(
        "product",
        format!("update_product id={id} payload={payload:?}"),
    );
    let result = (|| {
        let conn = state.connection()?;
        conn.execute(
            "UPDATE products SET name = ?1, category = ?2, barcode = ?3, default_price = ?4,
             safety_stock = ?5, unit = ?6, remark = ?7, updated_at = ?8 WHERE id = ?9",
            params![
                payload.name.trim(),
                payload.category.trim(),
                payload.barcode,
                payload.default_price.unwrap_or(0.0),
                payload.safety_stock.unwrap_or(0.0),
                payload.unit,
                payload.remark,
                now_text(),
                id
            ],
        )?;
        db::product_by_id(&conn, id)
    })();
    if let Ok(product) = &result {
        logger::info(
            "product",
            format!(
                "update_product success id={} name={} category={} barcode={:?}",
                product.id, product.name, product.category, product.barcode
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_product(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        conn.execute(
            "UPDATE products SET is_active = 0, updated_at = ?1 WHERE id = ?2",
            params![now_text(), id],
        )?;
        Ok(true)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn find_product_by_barcode(
    state: State<AppState>,
    barcode: String,
) -> ApiResponse<Option<ProductDto>> {
    let result = (|| {
        let barcode = barcode.trim();
        if barcode.is_empty() {
            return Ok(None);
        }
        let conn = state.connection()?;
        product_by_barcode(&conn, barcode)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_customers(
    state: State<AppState>,
    filter: Option<ListCustomersRequest>,
) -> ApiResponse<Vec<CustomerDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let filter = filter.unwrap_or(ListCustomersRequest {
            region: None,
            keyword: None,
            is_active: Some(true),
        });
        logger::info("customer", format!("list_customers filter={filter:?}"));
        let mut sql = String::from(
            "SELECT id, region, name, address, phone, is_active, remark FROM customers WHERE 1 = 1",
        );
        if let Some(region) = filter
            .region
            .filter(|value| !value.is_empty() && value != "全部")
        {
            sql.push_str(&format!(" AND region = '{}'", escape_sql(&region)));
        }
        if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
            let keyword = escape_sql(&keyword);
            sql.push_str(&format!(
                " AND (name LIKE '%{keyword}%' OR address LIKE '%{keyword}%')"
            ));
        }
        if let Some(active) = filter.is_active {
            sql.push_str(if active {
                " AND is_active = 1"
            } else {
                " AND is_active = 0"
            });
        }
        sql.push_str(
            " ORDER BY CASE WHEN name = '散客' THEN 0 ELSE 1 END, region, name LIMIT 1500",
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(CustomerDto {
                id: row.get(0)?,
                region: row.get(1)?,
                name: row.get(2)?,
                address: row.get(3)?,
                phone: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
                remark: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    if let Ok(items) = &result {
        logger::info(
            "customer",
            format!("list_customers result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_customer(
    state: State<AppState>,
    payload: CustomerPayload,
) -> ApiResponse<CustomerDto> {
    logger::info("customer", format!("create_customer payload={payload:?}"));
    let result = (|| {
        let conn = state.connection()?;
        create_customer_record(&conn, payload)
    })();
    if let Ok(customer) = &result {
        logger::info(
            "customer",
            format!(
                "create_customer success id={} name={} region={:?}",
                customer.id, customer.name, customer.region
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_customer(
    state: State<AppState>,
    id: i64,
    payload: CustomerPayload,
) -> ApiResponse<CustomerDto> {
    logger::info(
        "customer",
        format!("update_customer id={id} payload={payload:?}"),
    );
    let result = (|| {
        let conn = state.connection()?;
        update_customer_record(&conn, id, payload)
    })();
    if let Ok(customer) = &result {
        logger::info(
            "customer",
            format!(
                "update_customer success id={} name={} region={:?}",
                customer.id, customer.name, customer.region
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_customer(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        if db::is_guest_customer(&conn, id)? {
            anyhow::bail!("散客是系统默认客户，不能删除");
        }
        conn.execute(
            "UPDATE customers SET is_active = 0, updated_at = ?1 WHERE id = ?2",
            params![now_text(), id],
        )?;
        Ok(true)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_suppliers(
    state: State<AppState>,
    filter: Option<ListSuppliersRequest>,
) -> ApiResponse<Vec<SupplierDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let filter = filter.unwrap_or(ListSuppliersRequest {
            keyword: None,
            is_active: Some(true),
        });
        let mut sql = String::from(
            "SELECT id, name, contact, phone, address, is_active, remark
             FROM suppliers WHERE 1 = 1",
        );
        if let Some(keyword) = filter.keyword.filter(|value| !value.trim().is_empty()) {
            let keyword = escape_sql(&keyword);
            sql.push_str(&format!(
                " AND (name LIKE '%{keyword}%' OR contact LIKE '%{keyword}%' OR phone LIKE '%{keyword}%')"
            ));
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
        let rows = stmt.query_map([], map_supplier)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_supplier(
    state: State<AppState>,
    payload: SupplierPayload,
) -> ApiResponse<SupplierDto> {
    let result = (|| {
        if payload.name.trim().is_empty() {
            anyhow::bail!("供应商名称必填");
        }
        let conn = state.connection()?;
        let now = now_text();
        conn.execute(
            "INSERT INTO suppliers (name, contact, phone, address, is_active, remark, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?6)",
            params![
                payload.name.trim(),
                payload.contact,
                payload.phone,
                payload.address,
                payload.remark,
                now
            ],
        )?;
        supplier_by_id(&conn, conn.last_insert_rowid())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_supplier(
    state: State<AppState>,
    id: i64,
    payload: SupplierPayload,
) -> ApiResponse<SupplierDto> {
    let result = (|| {
        if payload.name.trim().is_empty() {
            anyhow::bail!("供应商名称必填");
        }
        let conn = state.connection()?;
        conn.execute(
            "UPDATE suppliers
             SET name = ?1, contact = ?2, phone = ?3, address = ?4, remark = ?5, updated_at = ?6
             WHERE id = ?7",
            params![
                payload.name.trim(),
                payload.contact,
                payload.phone,
                payload.address,
                payload.remark,
                now_text(),
                id
            ],
        )?;
        supplier_by_id(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_supplier(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        disable_supplier_record(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_inbound(
    state: State<AppState>,
    payload: CreateInboundRequest,
) -> ApiResponse<CreateInboundResponse> {
    let result = (|| {
        let mut conn = state.connection()?;
        create_inbound_record(&mut conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_inbound_records(
    state: State<AppState>,
    filter: Option<ListInboundRecordsRequest>,
) -> ApiResponse<Vec<InboundRecordDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let filter = filter.unwrap_or(ListInboundRecordsRequest {
            start_date: None,
            end_date: None,
            product_id: None,
            category: None,
        });
        let mut sql = String::from(
            "SELECT i.id, i.inbound_date, i.product_id, p.name, p.category,
                    i.supplier_id, i.supplier_name, i.quantity, i.unit_cost, i.amount, i.remark
             FROM inbound_records i
             JOIN products p ON p.id = i.product_id
             WHERE 1 = 1",
        );
        if let Some(start) = filter.start_date {
            sql.push_str(&format!(" AND i.inbound_date >= '{}'", escape_sql(&start)));
        }
        if let Some(end) = filter.end_date {
            sql.push_str(&format!(" AND i.inbound_date <= '{}'", escape_sql(&end)));
        }
        if let Some(product_id) = filter.product_id {
            sql.push_str(&format!(" AND i.product_id = {product_id}"));
        }
        if let Some(category) = filter
            .category
            .filter(|value| !value.is_empty() && value != "全部")
        {
            sql.push_str(&format!(" AND p.category = '{}'", escape_sql(&category)));
        }
        sql.push_str(" ORDER BY i.inbound_date DESC, i.id DESC LIMIT 500");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
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
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn preview_quote(
    state: State<AppState>,
    payload: PreviewQuoteRequest,
) -> ApiResponse<QuotePreviewDto> {
    let result = (|| {
        let conn = state.connection()?;
        orders::preview_quote(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_order(
    state: State<AppState>,
    payload: SaveOrderRequest,
) -> ApiResponse<SaveOrderResponse> {
    let result = (|| {
        let mut conn = state.connection()?;
        let mut response = orders::save_order(&mut conn, payload)?;
        response.document_path = reports::export_order_document(&state, response.order_id)?;
        logger::info(
            "order",
            format!(
                "保存订单成功：{}，单据：{}",
                response.order_no, response.document_path
            ),
        );
        Ok(response)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_order(state: State<AppState>, id: i64) -> ApiResponse<OrderDetailDto> {
    let result = (|| {
        let conn = state.connection()?;
        orders::get_order_detail(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_orders(
    state: State<AppState>,
    filter: Option<ListOrdersRequest>,
) -> ApiResponse<Vec<OrderDto>> {
    let result = (|| {
        let conn = state.connection()?;
        orders::list_orders(
            &conn,
            filter.unwrap_or(ListOrdersRequest {
                start_date: None,
                end_date: None,
                customer_id: None,
                order_no: None,
                status: Some("normal".to_string()),
            }),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_order_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_document(&state, order_id)
        .map(ok)
        .unwrap_or_else(fail)
}

#[tauri::command]
pub fn print_order_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    let result = (|| {
        let path = reports::export_order_document(&state, order_id)?;
        open::that(&path)?;
        Ok(path)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn print_order_document_with_options(
    state: State<AppState>,
    order_id: i64,
    payload: Option<PrintRequest>,
) -> ApiResponse<PrintStatusDto> {
    let result = (|| {
        let path = reports::export_order_document(&state, order_id)?;
        let printer_name = payload.and_then(|value| value.printer_name);
        let message = if let Some(printer) = printer_name
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            open::that(&path)?;
            format!("已打开单据文件，请在关联程序中选择打印机：{printer}")
        } else {
            open::that(&path)?;
            "已打开单据文件，请在关联程序中确认打印".to_string()
        };
        Ok(PrintStatusDto {
            file_path: path,
            printer_name,
            message,
        })
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_order(
    state: State<AppState>,
    id: i64,
    payload: Option<VoidOrderRequest>,
) -> ApiResponse<OrderDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        let order = orders::void_order(&mut conn, id, payload.and_then(|value| value.reason))?;
        logger::warn("order", format!("订单已作废：{}", order.order_no));
        Ok(order)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_customer_product_rules(
    state: State<AppState>,
    filter: Option<RuleFilterRequest>,
) -> ApiResponse<Vec<CustomerProductRuleDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let filter = filter.unwrap_or(RuleFilterRequest {
            customer_id: None,
            product_id: None,
            category: None,
            keyword: None,
            is_active: None,
            rule_type: None,
        });
        logger::info(
            "rule",
            format!("list_customer_product_rules filter={filter:?}"),
        );
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
        if let Some(customer_id) = filter.customer_id {
            sql.push_str(&format!(" AND r.customer_id = {customer_id}"));
        }
        if let Some(product_id) = filter.product_id {
            sql.push_str(&format!(" AND r.product_id = {product_id}"));
        }
        if let Some(category) = filter
            .category
            .filter(|value| !value.is_empty() && value != "全部")
        {
            sql.push_str(&format!(" AND p.category = '{}'", escape_sql(&category)));
        }
        if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
            let keyword = escape_sql(&keyword);
            sql.push_str(&format!(
                " AND (c.name LIKE '%{keyword}%' OR p.name LIKE '%{keyword}%')"
            ));
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
        let rows = stmt.query_map([], |row| {
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
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    if let Ok(items) = &result {
        logger::info(
            "rule",
            format!("list_customer_product_rules result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_customer_product_rule(
    state: State<AppState>,
    payload: SaveCustomerProductRuleRequest,
) -> ApiResponse<i64> {
    logger::info(
        "rule",
        format!("save_customer_product_rule payload={payload:?}"),
    );
    let result = (|| {
        let conn = state.connection()?;
        save_customer_product_rule_record(&conn, payload)
    })();
    if let Ok(id) = &result {
        logger::info(
            "rule",
            format!("save_customer_product_rule success id={id}"),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_customer_product_rule(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        disable_customer_product_rule_record(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn delete_customer_product_rule(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        delete_customer_product_rule_record(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_monthly_credits(
    state: State<AppState>,
    filter: Option<MonthlyCreditFilterRequest>,
) -> ApiResponse<Vec<MonthlyCreditDto>> {
    let result = (|| {
        let conn = state.connection()?;
        orders::list_monthly_credits(
            &conn,
            filter.unwrap_or(MonthlyCreditFilterRequest {
                customer_id: None,
                category: None,
                status: None,
                start_date: None,
                end_date: None,
                available_month: None,
            }),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_available_monthly_credits(
    state: State<AppState>,
    customer_id: i64,
    category: String,
    order_date: String,
) -> ApiResponse<Vec<MonthlyCreditDto>> {
    let result = (|| {
        let conn = state.connection()?;
        orders::available_monthly_credits(&conn, customer_id, category, order_date)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn close_monthly_credit(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        orders::close_or_void_credit(&conn, id, "closed")?;
        Ok(true)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_monthly_credit(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        orders::close_or_void_credit(&conn, id, "voided")?;
        Ok(true)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_customer_balances(
    state: State<AppState>,
    filter: Option<CustomerBalanceFilterRequest>,
) -> ApiResponse<Vec<CustomerBalanceDto>> {
    let result = (|| {
        let conn = state.connection()?;
        customer_balances(
            &conn,
            filter.unwrap_or(CustomerBalanceFilterRequest {
                region: None,
                keyword: None,
                only_unpaid: None,
            }),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_payment_records(
    state: State<AppState>,
    filter: Option<PaymentFilterRequest>,
) -> ApiResponse<Vec<PaymentRecordDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let filter = filter.unwrap_or(PaymentFilterRequest {
            customer_id: None,
            start_date: None,
            end_date: None,
            status: Some("normal".to_string()),
        });
        let mut sql = String::from(
            "SELECT p.id, p.payment_date, p.customer_id, c.name, p.amount, p.method,
                    p.related_order_id, p.status, p.remark, p.created_at
             FROM payment_records p
             JOIN customers c ON c.id = p.customer_id
             WHERE 1 = 1",
        );
        if let Some(customer_id) = filter.customer_id {
            sql.push_str(&format!(" AND p.customer_id = {customer_id}"));
        }
        if let Some(start_date) = filter.start_date.filter(|value| !value.is_empty()) {
            sql.push_str(&format!(
                " AND p.payment_date >= '{}'",
                escape_sql(&start_date)
            ));
        }
        if let Some(end_date) = filter.end_date.filter(|value| !value.is_empty()) {
            sql.push_str(&format!(
                " AND p.payment_date <= '{}'",
                escape_sql(&end_date)
            ));
        }
        if let Some(status) = filter
            .status
            .filter(|value| !value.is_empty() && value != "全部")
        {
            sql.push_str(&format!(" AND p.status = '{}'", escape_sql(&status)));
        }
        sql.push_str(" ORDER BY p.payment_date DESC, p.id DESC LIMIT 1000");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], map_payment_record)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_payment(
    state: State<AppState>,
    payload: CreatePaymentRequest,
) -> ApiResponse<PaymentRecordDto> {
    let result = (|| {
        let conn = state.connection()?;
        create_payment_record(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_payment(state: State<AppState>, id: i64) -> ApiResponse<PaymentRecordDto> {
    let result = (|| {
        let conn = state.connection()?;
        conn.execute(
            "UPDATE payment_records SET status = 'voided', updated_at = ?1 WHERE id = ?2",
            params![now_text(), id],
        )?;
        payment_by_id(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_daily_profit_summary(
    state: State<AppState>,
    date: String,
) -> ApiResponse<DailyProfitSummary> {
    let result = (|| {
        let conn = state.connection()?;
        reports::daily_profit_summary(&conn, &date)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_profit_analytics(
    state: State<AppState>,
    request: ProfitAnalyticsRequest,
) -> ApiResponse<ProfitAnalyticsResponse> {
    let result = (|| {
        let conn = state.connection()?;
        reports::get_profit_analytics(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_profit_records(
    state: State<AppState>,
    filter: Option<ProfitFilterRequest>,
) -> ApiResponse<Vec<OrderDto>> {
    let result = (|| {
        let conn = state.connection()?;
        reports::list_profit_records(
            &conn,
            filter.unwrap_or(ProfitFilterRequest {
                start_date: None,
                end_date: None,
                customer_id: None,
                category: None,
            }),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_inventory_report(
    state: State<AppState>,
    filter: Option<InventoryReportRequest>,
) -> ApiResponse<Vec<InventoryReportRowDto>> {
    let result = (|| {
        let conn = state.connection()?;
        reports::list_inventory_report(
            &conn,
            filter.unwrap_or(InventoryReportRequest {
                start_date: None,
                end_date: None,
                category: None,
                keyword: None,
            }),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_documents(
    state: State<AppState>,
    filter: Option<DocumentFilterRequest>,
) -> ApiResponse<Vec<DocumentDto>> {
    let result = (|| {
        let conn = state.connection()?;
        reports::list_documents(
            &conn,
            filter.unwrap_or(DocumentFilterRequest {
                customer_id: None,
                start_date: None,
                end_date: None,
                order_no: None,
                printed: None,
                status: None,
            }),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_document(state: State<AppState>, document_id: i64) -> ApiResponse<String> {
    let result = (|| {
        let conn = state.connection()?;
        reports::open_document(&conn, document_id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_document(&state, order_id)
        .map(ok)
        .unwrap_or_else(fail)
}

#[tauri::command]
pub fn print_document(
    state: State<AppState>,
    document_id: i64,
    payload: Option<PrintRequest>,
) -> ApiResponse<PrintStatusDto> {
    let result = (|| {
        let conn = state.connection()?;
        reports::print_document(
            &conn,
            document_id,
            payload.and_then(|value| value.printer_name),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_data(state: State<AppState>, payload: ExportDataRequest) -> ApiResponse<String> {
    let result = reports::export_data(&state, payload);
    if let Ok(path) = &result {
        logger::info("export", format!("导出成功：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_exports_folder(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let path = state.exports_dir();
        open::that(&path)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_logs_folder(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let path = state.logs_dir();
        open::that(&path)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn import_excel(state: State<AppState>, file_path: String) -> ApiResponse<ImportResult> {
    let result = excel::import_excel_file(&state, &file_path);
    if let Ok(result) = &result {
        logger::info(
            "import",
            format!(
                "Excel导入完成：商品 {}，客户 {}，流水 {}，利润行 {}",
                result.product_count,
                result.customer_count,
                result.movement_count,
                result.profit_count
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_import_status(
    state: State<AppState>,
    _import_id: Option<String>,
) -> ApiResponse<Option<ImportResult>> {
    ok(state.import_result())
}

#[tauri::command]
pub fn create_backup(state: State<AppState>) -> ApiResponse<String> {
    let result = db::create_backup_file(&state, "manual");
    if let Ok(path) = &result {
        logger::info("backup", format!("手动备份成功：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_backups(state: State<AppState>) -> ApiResponse<Vec<BackupDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, backup_path, backup_type, status, message, created_at
             FROM backup_logs ORDER BY created_at DESC LIMIT 100",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupDto {
                id: row.get(0)?,
                backup_path: row.get(1)?,
                backup_type: row.get(2)?,
                status: row.get(3)?,
                message: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_backup_folder(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let path = state.backups_dir();
        open::that(&path)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_settings(state: State<AppState>) -> ApiResponse<Vec<SettingDto>> {
    let result = (|| {
        let conn = state.connection()?;
        let mut stmt =
            conn.prepare("SELECT key, COALESCE(value, '') FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok(SettingDto {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, payload: SaveSettingsRequest) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        save_settings_record(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_printers() -> ApiResponse<Vec<String>> {
    reports::list_system_printers().map(ok).unwrap_or_else(fail)
}

fn escape_sql(value: &str) -> String {
    value.replace('\'', "''")
}

fn create_customer_record(
    conn: &rusqlite::Connection,
    payload: CustomerPayload,
) -> anyhow::Result<CustomerDto> {
    let name = payload.name.trim();
    if name.is_empty() {
        anyhow::bail!("客户名称必填");
    }
    if name == db::GUEST_CUSTOMER_NAME {
        let id = db::ensure_guest_customer(conn)?;
        return db::customer_by_id(conn, id);
    }

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
    db::customer_by_id(conn, conn.last_insert_rowid())
}

fn update_customer_record(
    conn: &rusqlite::Connection,
    id: i64,
    payload: CustomerPayload,
) -> anyhow::Result<CustomerDto> {
    let name = payload.name.trim();
    if name.is_empty() {
        anyhow::bail!("客户名称必填");
    }

    let is_guest = db::is_guest_customer(conn, id)?;
    if is_guest && name != db::GUEST_CUSTOMER_NAME {
        anyhow::bail!("散客是系统默认客户，名称不能修改");
    }
    if !is_guest && name == db::GUEST_CUSTOMER_NAME {
        anyhow::bail!("散客是系统默认客户，不能重复创建");
    }

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
    db::customer_by_id(conn, id)
}

fn disable_supplier_record(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    conn.execute(
        "UPDATE suppliers SET is_active = 0, updated_at = ?1 WHERE id = ?2",
        params![now_text(), id],
    )?;
    Ok(true)
}

fn create_inbound_record(
    conn: &mut rusqlite::Connection,
    payload: CreateInboundRequest,
) -> anyhow::Result<CreateInboundResponse> {
    if payload.product_id <= 0 || payload.quantity <= 0.0 || payload.unit_cost < 0.0 {
        anyhow::bail!("入库商品、数量和进货价不合法");
    }
    let tx = conn.transaction()?;
    let amount = money(payload.quantity * payload.unit_cost);
    let now = now_text();
    let supplier_name = match payload.supplier_id {
        Some(id) => tx
            .query_row(
                "SELECT name FROM suppliers WHERE id = ?1 AND is_active = 1",
                [id],
                |row| row.get::<_, String>(0),
            )
            .optional()?,
        None => None,
    };
    if payload.supplier_id.is_some() && supplier_name.is_none() {
        anyhow::bail!("供应商不存在或已停用");
    }
    tx.execute(
        "INSERT INTO inbound_records
         (inbound_date, product_id, supplier_id, supplier_name, quantity, unit_cost, amount, remark, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            normalize_date(&payload.inbound_date),
            payload.product_id,
            payload.supplier_id,
            supplier_name,
            payload.quantity,
            payload.unit_cost,
            amount,
            payload.remark,
            now
        ],
    )?;
    let inbound_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, source_id, remark, created_at)
         VALUES (?1, ?2, 'inbound', ?3, ?4, ?5, 'inbound', ?6, ?7, ?8)",
        params![
            normalize_date(&payload.inbound_date),
            payload.product_id,
            payload.quantity,
            payload.unit_cost,
            amount,
            inbound_id,
            "入库",
            now
        ],
    )?;
    let (current_stock, avg_cost) = db::recalc_stock_balance(&tx, payload.product_id)?;
    tx.commit()?;
    Ok(CreateInboundResponse {
        inbound_id,
        product_id: payload.product_id,
        current_stock,
        avg_cost,
    })
}

fn create_payment_record(
    conn: &rusqlite::Connection,
    payload: CreatePaymentRequest,
) -> anyhow::Result<PaymentRecordDto> {
    if payload.customer_id <= 0 || payload.amount <= 0.0 {
        anyhow::bail!("收款客户和金额不合法");
    }
    let customer_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM customers WHERE id = ?1 AND is_active = 1",
        [payload.customer_id],
        |row| row.get(0),
    )?;
    if !customer_exists {
        anyhow::bail!("客户不存在或已停用");
    }
    if let Some(order_id) = payload.related_order_id {
        let valid_order: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM orders WHERE id = ?1 AND customer_id = ?2 AND status = 'normal'",
            params![order_id, payload.customer_id],
            |row| row.get(0),
        )?;
        if !valid_order {
            anyhow::bail!("关联订单不存在或不属于该客户");
        }
    }
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

fn save_customer_product_rule_record(
    conn: &rusqlite::Connection,
    payload: SaveCustomerProductRuleRequest,
) -> anyhow::Result<i64> {
    if payload.customer_id <= 0 || payload.product_id <= 0 {
        anyhow::bail!("客户和商品必填");
    }
    if payload.threshold_quantity.unwrap_or(1.0) <= 0.0 {
        anyhow::bail!("每满数量必须大于 0");
    }
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

fn disable_customer_product_rule_record(
    conn: &rusqlite::Connection,
    id: i64,
) -> anyhow::Result<bool> {
    conn.execute(
        "UPDATE customer_product_rules SET is_active = 0, updated_at = ?1 WHERE id = ?2",
        params![now_text(), id],
    )?;
    Ok(true)
}

fn delete_customer_product_rule_record(
    conn: &rusqlite::Connection,
    id: i64,
) -> anyhow::Result<bool> {
    conn.execute("DELETE FROM customer_product_rules WHERE id = ?1", [id])?;
    Ok(true)
}

fn save_settings_record(
    conn: &rusqlite::Connection,
    payload: SaveSettingsRequest,
) -> anyhow::Result<bool> {
    if let Some(value) = payload.daily_auto_backup {
        db::set_setting(
            conn,
            "daily_auto_backup",
            if value { "true" } else { "false" },
        )?;
    }
    if let Some(value) = payload.default_print_template {
        db::set_setting(conn, "default_print_template", &value)?;
    }
    if let Some(value) = payload.default_export_format {
        db::set_setting(conn, "default_export_format", &value)?;
    }
    if let Some(value) = payload.default_printer {
        db::set_setting(conn, "default_printer", &value)?;
    }
    Ok(true)
}

fn supplier_by_id(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<SupplierDto> {
    conn.query_row(
        "SELECT id, name, contact, phone, address, is_active, remark FROM suppliers WHERE id = ?1",
        [id],
        map_supplier,
    )
    .map_err(Into::into)
}

fn product_by_barcode(
    conn: &rusqlite::Connection,
    barcode: &str,
) -> anyhow::Result<Option<ProductDto>> {
    conn.query_row(
        "SELECT p.id, p.name, p.category, p.barcode, p.default_price, p.safety_stock, p.unit,
                COALESCE(s.current_stock, 0), COALESCE(s.avg_cost, 0), COALESCE(s.stock_value, 0),
                p.is_active, p.remark
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         WHERE p.barcode = ?1 AND p.is_active = 1
         ORDER BY p.id LIMIT 1",
        [barcode],
        db::map_product,
    )
    .optional()
    .map_err(Into::into)
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

fn customer_balances(
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
    if let Some(region) = filter
        .region
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(&format!(" AND c.region = '{}'", escape_sql(&region)));
    }
    if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
        let keyword = escape_sql(&keyword);
        sql.push_str(&format!(
            " AND (c.name LIKE '%{keyword}%' OR c.address LIKE '%{keyword}%')"
        ));
    }
    if filter.only_unpaid.unwrap_or(false) {
        sql.push_str(" AND COALESCE(o.total_payable, 0) - COALESCE(p.total_paid, 0) > 0");
    }
    sql.push_str(" ORDER BY balance DESC, c.name LIMIT 1500");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_customer_balance)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::Connection;
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    fn memory_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn create_customer_record_reuses_fixed_guest_customer() {
        let conn = memory_conn();
        let guest_id = db::ensure_guest_customer(&conn).unwrap();

        let customer = create_customer_record(
            &conn,
            CustomerPayload {
                region: Some("其他地区".to_string()),
                name: db::GUEST_CUSTOMER_NAME.to_string(),
                address: Some("临时地址".to_string()),
                phone: None,
                remark: None,
            },
        )
        .unwrap();
        let guest_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM customers WHERE name = ?1",
                [db::GUEST_CUSTOMER_NAME],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(customer.id, guest_id);
        assert_eq!(customer.name, db::GUEST_CUSTOMER_NAME);
        assert_eq!(guest_count, 1);
    }

    #[test]
    fn update_customer_record_rejects_regular_customer_renamed_to_guest() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('默认', '普通客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        let customer_id = conn.last_insert_rowid();

        let result = update_customer_record(
            &conn,
            customer_id,
            CustomerPayload {
                region: Some("默认".to_string()),
                name: db::GUEST_CUSTOMER_NAME.to_string(),
                address: None,
                phone: None,
                remark: None,
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不能重复创建"));
    }

    #[test]
    fn supplier_crud_helpers_disable_and_reload_supplier() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO suppliers (name, contact, phone, address, is_active, remark, created_at, updated_at)
             VALUES ('供应商A', '张三', '13800000000', '地址A', 1, '备注A', ?1, ?1)",
            [&now],
        )
        .unwrap();
        let supplier_id = conn.last_insert_rowid();

        let supplier = supplier_by_id(&conn, supplier_id).unwrap();
        assert!(supplier.is_active);

        disable_supplier_record(&conn, supplier_id).unwrap();
        let disabled = supplier_by_id(&conn, supplier_id).unwrap();
        assert!(!disabled.is_active);
    }

    #[test]
    fn create_payment_rejects_invalid_order_customer_pair() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('收款', '客户A', 1, ?1, ?1), ('收款', '客户B', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES ('20260601001', '2026-06-01', 1, '客户A', 100, 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let result = create_payment_record(
            &conn,
            CreatePaymentRequest {
                payment_date: "2026-06-02".to_string(),
                customer_id: 2,
                amount: 50.0,
                method: Some("现金".to_string()),
                related_order_id: Some(1),
                remark: None,
            },
        );
        let payment_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM payment_records", [], |row| row.get(0))
            .unwrap();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("关联订单"));
        assert_eq!(payment_count, 0);
    }

    #[test]
    fn create_inbound_rejects_disabled_supplier() {
        let mut conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('入库商品', '入库', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (name, is_active, created_at, updated_at)
             VALUES ('停用供应商', 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let result = create_inbound_record(
            &mut conn,
            CreateInboundRequest {
                inbound_date: "2026-06-01".to_string(),
                product_id: 1,
                supplier_id: Some(1),
                quantity: 5.0,
                unit_cost: 3.0,
                remark: None,
            },
        );
        let inbound_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM inbound_records", [], |row| row.get(0))
            .unwrap();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("供应商不存在或已停用"));
        assert_eq!(inbound_count, 0);
    }

    #[test]
    fn customer_product_rule_lifecycle_disables_old_active_rule_and_deletes_draft_rule() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('规则', '规则客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('规则商品', '规则类', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let first_id = save_customer_product_rule_record(
            &conn,
            SaveCustomerProductRuleRequest {
                id: None,
                customer_id: 1,
                product_id: 1,
                fixed_price: Some(9.0),
                threshold_quantity: None,
                gift_product_id: None,
                gift_quantity: None,
                direct_discount_amount: None,
                monthly_credit_amount: None,
                credit_category: None,
                is_active: true,
                remark: Some("第一条".to_string()),
            },
        )
        .unwrap();
        let second_id = save_customer_product_rule_record(
            &conn,
            SaveCustomerProductRuleRequest {
                id: None,
                customer_id: 1,
                product_id: 1,
                fixed_price: Some(8.0),
                threshold_quantity: None,
                gift_product_id: None,
                gift_quantity: None,
                direct_discount_amount: None,
                monthly_credit_amount: None,
                credit_category: None,
                is_active: true,
                remark: Some("第二条".to_string()),
            },
        )
        .unwrap();
        let draft_id = save_customer_product_rule_record(
            &conn,
            SaveCustomerProductRuleRequest {
                id: None,
                customer_id: 1,
                product_id: 1,
                fixed_price: Some(7.0),
                threshold_quantity: None,
                gift_product_id: None,
                gift_quantity: None,
                direct_discount_amount: None,
                monthly_credit_amount: None,
                credit_category: None,
                is_active: false,
                remark: Some("草稿".to_string()),
            },
        )
        .unwrap();

        disable_customer_product_rule_record(&conn, second_id).unwrap();
        delete_customer_product_rule_record(&conn, draft_id).unwrap();

        let first_active: i64 = conn
            .query_row(
                "SELECT is_active FROM customer_product_rules WHERE id = ?1",
                [first_id],
                |row| row.get(0),
            )
            .unwrap();
        let second_active: i64 = conn
            .query_row(
                "SELECT is_active FROM customer_product_rules WHERE id = ?1",
                [second_id],
                |row| row.get(0),
            )
            .unwrap();
        let draft_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM customer_product_rules WHERE id = ?1",
                [draft_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(first_active, 0);
        assert_eq!(second_active, 0);
        assert_eq!(draft_count, 0);
    }

    #[test]
    fn save_settings_updates_known_keys_only() {
        let conn = memory_conn();
        db::seed_settings(&conn).unwrap();

        save_settings_record(
            &conn,
            SaveSettingsRequest {
                daily_auto_backup: Some(false),
                default_print_template: None,
                default_export_format: Some("xlsx".to_string()),
                default_printer: Some("测试打印机".to_string()),
            },
        )
        .unwrap();

        assert_eq!(
            db::setting(&conn, "daily_auto_backup").unwrap().as_deref(),
            Some("false")
        );
        assert_eq!(
            db::setting(&conn, "default_export_format")
                .unwrap()
                .as_deref(),
            Some("xlsx")
        );
        assert_eq!(
            db::setting(&conn, "default_printer").unwrap().as_deref(),
            Some("测试打印机")
        );
        assert_eq!(
            db::setting(&conn, "default_print_template")
                .unwrap()
                .as_deref(),
            Some("excel")
        );
    }

    #[test]
    fn concurrent_inbounds_keep_stock_balance_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("concurrent-inbounds.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.busy_timeout(Duration::from_secs(10)).unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('并发入库商品', '压力', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
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
                    create_inbound_record(
                        &mut conn,
                        CreateInboundRequest {
                            inbound_date: "2026-06-01".to_string(),
                            product_id: 1,
                            supplier_id: None,
                            quantity: 1.0,
                            unit_cost: 5.0,
                            remark: None,
                        },
                    )
                    .unwrap();
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }

        let conn = Connection::open(&db_path).unwrap();
        let (stock, avg_cost): (f64, f64) = conn
            .query_row(
                "SELECT current_stock, avg_cost FROM stock_balances WHERE product_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let inbound_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM inbound_records", [], |row| row.get(0))
            .unwrap();

        assert_eq!(inbound_count, worker_count as i64);
        assert_eq!(stock, worker_count as f64);
        assert_eq!(avg_cost, 5.0);
    }

    #[test]
    fn concurrent_payments_keep_customer_balance_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("concurrent-payments.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.busy_timeout(Duration::from_secs(10)).unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('压力', '并发收款客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES ('20260601001', '2026-06-01', 1, '并发收款客户', 1000, 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();
        drop(conn);

        let worker_count = 20;
        let barrier = Arc::new(Barrier::new(worker_count));
        let handles = (0..worker_count)
            .map(|_| {
                let db_path = db_path.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let conn = Connection::open(db_path).unwrap();
                    conn.busy_timeout(Duration::from_secs(10)).unwrap();
                    barrier.wait();
                    create_payment_record(
                        &conn,
                        CreatePaymentRequest {
                            payment_date: "2026-06-02".to_string(),
                            customer_id: 1,
                            amount: 1.0,
                            method: Some("现金".to_string()),
                            related_order_id: None,
                            remark: None,
                        },
                    )
                    .unwrap();
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }

        let conn = Connection::open(&db_path).unwrap();
        let balances = customer_balances(
            &conn,
            CustomerBalanceFilterRequest {
                region: None,
                keyword: None,
                only_unpaid: Some(true),
            },
        )
        .unwrap();

        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].total_payable, 1000.0);
        assert_eq!(balances[0].total_paid, worker_count as f64);
        assert_eq!(balances[0].balance, 1000.0 - worker_count as f64);
    }

    #[test]
    fn barcode_scan_returns_only_active_exact_match() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('停用商品', '扫码', 'SCAN001', 10, 0, 0, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('扫码商品', '扫码', 'SCAN001', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let product = product_by_barcode(&conn, "SCAN001").unwrap().unwrap();
        assert_eq!(product.name, "扫码商品");
        assert!(product.is_active);
        assert!(product_by_barcode(&conn, "MISS").unwrap().is_none());
    }

    #[test]
    fn customer_balance_subtracts_normal_payments_and_ignores_voided_payments() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('欠款', '客户A', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES
             ('20260601001', '2026-06-01', 1, '客户A', 120, 'normal', ?1, ?1),
             ('20260601002', '2026-06-01', 1, '客户A', 50, 'voided', ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO payment_records
             (payment_date, customer_id, amount, status, created_at, updated_at)
             VALUES
             ('2026-06-02', 1, 70, 'normal', ?1, ?1),
             ('2026-06-03', 1, 30, 'voided', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let rows = customer_balances(
            &conn,
            CustomerBalanceFilterRequest {
                region: Some("欠款".to_string()),
                keyword: Some("客户A".to_string()),
                only_unpaid: Some(true),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].total_payable, 120.0);
        assert_eq!(rows[0].total_paid, 70.0);
        assert_eq!(rows[0].balance, 50.0);
    }

    #[test]
    fn supplier_mapping_keeps_contact_fields() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO suppliers (name, contact, phone, address, is_active, remark, created_at, updated_at)
             VALUES ('供应商A', '张三', '13800000000', '地址A', 1, '备注A', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let supplier = supplier_by_id(&conn, 1).unwrap();
        assert_eq!(supplier.name, "供应商A");
        assert_eq!(supplier.contact.as_deref(), Some("张三"));
        assert_eq!(supplier.phone.as_deref(), Some("13800000000"));
        assert!(supplier.is_active);
    }
}
