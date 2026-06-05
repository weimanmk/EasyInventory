use crate::app::AppState;
use crate::db;
use crate::excel;
use crate::generalization;
use crate::logger;
use crate::models::*;
use crate::orders;
use crate::reports;
use crate::utils::{money, normalize_date, now_text, safe_file_name};
use calamine::{open_workbook_auto, Data, Reader};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
pub fn batch_update_products(
    state: State<AppState>,
    payload: BatchUpdateProductsRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        batch_update_products_record(&conn, payload)
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
        let guest_name = db::guest_customer_name(&conn)?;
        sql.push_str(&format!(
            " ORDER BY CASE WHEN name = '{}' THEN 0 ELSE 1 END, region, name LIMIT 1500",
            escape_sql(&guest_name)
        ));
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
            let guest_name = db::guest_customer_name(&conn)?;
            anyhow::bail!("{guest_name}是系统默认客户，不能删除");
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
pub fn batch_update_customers(
    state: State<AppState>,
    payload: BatchUpdateCustomersRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        batch_update_customers_record(&conn, payload)
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
pub fn batch_update_suppliers(
    state: State<AppState>,
    payload: BatchUpdateSuppliersRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        batch_update_suppliers_record(&conn, payload)
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
        let audit_conn = state.connection()?;
        record_audit(
            &audit_conn,
            AuditEvent {
                module: "order",
                action: "save",
                target_type: Some("orders"),
                target_id: Some(response.order_id),
                target_label: Some(&response.order_no),
                result: "success",
                message: Some("订单保存成功"),
                details: Some(&format!("documentPath={}", response.document_path)),
            },
        )?;
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
pub fn export_order_pdf_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_pdf_document(&state, order_id)
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
        record_audit(
            &conn,
            AuditEvent {
                module: "order",
                action: "void",
                target_type: Some("orders"),
                target_id: Some(order.id),
                target_label: Some(&order.order_no),
                result: "success",
                message: Some("订单已作废"),
                details: Some(&format!("customer={}", order.customer_name)),
            },
        )?;
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
        let id = save_customer_product_rule_record(&conn, payload)?;
        record_audit(
            &conn,
            AuditEvent {
                module: "rule",
                action: "save",
                target_type: Some("customer_product_rules"),
                target_id: Some(id),
                target_label: Some("客户商品规则"),
                result: "success",
                message: Some("客户商品规则已保存"),
                details: None,
            },
        )?;
        Ok(id)
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
        let disabled = disable_customer_product_rule_record(&conn, id)?;
        record_audit(
            &conn,
            AuditEvent {
                module: "rule",
                action: "disable",
                target_type: Some("customer_product_rules"),
                target_id: Some(id),
                target_label: Some("客户商品规则"),
                result: "success",
                message: Some("客户商品规则已停用"),
                details: None,
            },
        )?;
        Ok(disabled)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn delete_customer_product_rule(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        let deleted = delete_customer_product_rule_record(&conn, id)?;
        record_audit(
            &conn,
            AuditEvent {
                module: "rule",
                action: "delete",
                target_type: Some("customer_product_rules"),
                target_id: Some(id),
                target_label: Some("客户商品规则"),
                result: "success",
                message: Some("客户商品规则已删除"),
                details: None,
            },
        )?;
        Ok(deleted)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn preview_customer_product_rule_import(
    state: State<AppState>,
    file_path: String,
) -> ApiResponse<CustomerProductRuleImportPreviewDto> {
    let result = (|| {
        let conn = state.connection()?;
        preview_customer_product_rule_import_record(&conn, &file_path)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn import_customer_product_rules(
    state: State<AppState>,
    file_path: String,
) -> ApiResponse<CustomerProductRuleImportResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        import_customer_product_rules_record(&conn, &file_path)
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
pub fn get_customer_statement(
    state: State<AppState>,
    request: CustomerStatementRequest,
) -> ApiResponse<CustomerStatementDto> {
    let result = (|| {
        let conn = state.connection()?;
        reports::customer_statement(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_customer_statement_pdf(
    state: State<AppState>,
    request: CustomerStatementRequest,
) -> ApiResponse<String> {
    reports::export_customer_statement_pdf_document(&state, request)
        .map(ok)
        .unwrap_or_else(fail)
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
pub fn get_product_ranking(
    state: State<AppState>,
    request: ProductRankingRequest,
) -> ApiResponse<Vec<ProductRankingRowDto>> {
    let result = (|| {
        let conn = state.connection()?;
        reports::product_ranking(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_customer_analysis(
    state: State<AppState>,
    request: CustomerAnalysisRequest,
) -> ApiResponse<CustomerAnalysisDto> {
    let result = (|| {
        let conn = state.connection()?;
        reports::customer_analysis(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_supplier_purchase_ledger(
    state: State<AppState>,
    filter: Option<SupplierPurchaseLedgerRequest>,
) -> ApiResponse<SupplierPurchaseLedgerDto> {
    let result = (|| {
        let conn = state.connection()?;
        reports::supplier_purchase_ledger(
            &conn,
            filter.unwrap_or(SupplierPurchaseLedgerRequest {
                start_date: None,
                end_date: None,
                supplier_id: None,
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
pub fn export_document_pdf(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_pdf_document(&state, order_id)
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
pub fn run_data_self_check(state: State<AppState>) -> ApiResponse<DataSelfCheckDto> {
    let result = (|| {
        let conn = state.connection()?;
        run_data_self_check_record(&conn, |path| Path::new(path).exists())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_data_self_check(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let conn = state.connection()?;
        let check = run_data_self_check_record(&conn, |path| Path::new(path).exists())?;
        let path = state.exports_dir().join(format!(
            "data_self_check_{}.txt",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        write_self_check_export(&path, &check)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_diagnostic_summary(state: State<AppState>) -> ApiResponse<DiagnosticSummaryDto> {
    let result = diagnostic_summary(&state);
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_diagnostic_package(state: State<AppState>) -> ApiResponse<DiagnosticPackageDto> {
    let result = export_diagnostic_package_record(&state);
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn import_excel(state: State<AppState>, file_path: String) -> ApiResponse<ImportResult> {
    let result = (|| {
        let backup_path = db::create_backup_file(&state, "pre_legacy_import")?;
        logger::info(
            "import",
            format!("历史兼容 Excel 导入前已自动备份：{backup_path}"),
        );
        excel::import_excel_file(&state, &file_path)
    })();
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
pub fn restore_backup(
    state: State<AppState>,
    backup_id: i64,
) -> ApiResponse<RestoreBackupResultDto> {
    let result = restore_backup_record(&state, backup_id);
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_inventory_adjustment(
    state: State<AppState>,
    payload: CreateInventoryAdjustmentRequest,
) -> ApiResponse<InventoryAdjustmentDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        create_inventory_adjustment_record(&mut conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_inventory_adjustments(
    state: State<AppState>,
    filter: Option<InventoryAdjustmentFilterRequest>,
) -> ApiResponse<Vec<InventoryAdjustmentDto>> {
    let result = (|| {
        let conn = state.connection()?;
        inventory_adjustments(&conn, filter.unwrap_or_default())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_inventory_adjustment(
    state: State<AppState>,
    id: i64,
    payload: Option<VoidRecordRequest>,
) -> ApiResponse<InventoryAdjustmentDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        void_inventory_adjustment_record(&mut conn, id, payload.and_then(|value| value.reason))
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_stocktake(
    state: State<AppState>,
    payload: CreateStocktakeRequest,
) -> ApiResponse<StocktakeRecordDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        create_stocktake_record(&mut conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_stocktakes(
    state: State<AppState>,
    filter: Option<StocktakeFilterRequest>,
) -> ApiResponse<Vec<StocktakeRecordDto>> {
    let result = (|| {
        let conn = state.connection()?;
        stocktakes(&conn, filter.unwrap_or_default())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_stocktake(
    state: State<AppState>,
    id: i64,
    payload: Option<VoidRecordRequest>,
) -> ApiResponse<StocktakeRecordDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        void_stocktake_record(&mut conn, id, payload.and_then(|value| value.reason))
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_audit_logs(
    state: State<AppState>,
    filter: Option<AuditLogFilterRequest>,
) -> ApiResponse<Vec<AuditLogDto>> {
    let result = (|| {
        let conn = state.connection()?;
        audit_logs(&conn, filter.unwrap_or_default())
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
pub fn get_setup_status(state: State<AppState>) -> ApiResponse<SetupStatusDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::setup_status(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn complete_setup(state: State<AppState>, request: CompleteSetupRequest) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::complete_setup(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_merchant_profile(state: State<AppState>) -> ApiResponse<MerchantProfileDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::merchant_profile(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_merchant_profile(
    state: State<AppState>,
    profile: MerchantProfileDto,
) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_merchant_profile(&conn, profile)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_term_settings(state: State<AppState>) -> ApiResponse<TermSettingsDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::term_settings(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_term_settings(state: State<AppState>, terms: TermSettingsDto) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_term_settings(&conn, terms)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_feature_flags(state: State<AppState>) -> ApiResponse<FeatureFlagsDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::feature_flags(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_feature_flags(state: State<AppState>, flags: FeatureFlagsDto) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_feature_flags(&conn, flags)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_industry_templates() -> ApiResponse<Vec<IndustryTemplateDto>> {
    ok(generalization::industry_templates())
}

#[tauri::command]
pub fn apply_industry_template(
    state: State<AppState>,
    request: ApplyIndustryTemplateRequest,
) -> ApiResponse<IndustryTemplateDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::apply_industry_template(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_document_templates(state: State<AppState>) -> ApiResponse<Vec<DocumentTemplateDto>> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::document_templates(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn apply_document_template(state: State<AppState>, template_id: String) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::apply_document_template(&conn, template_id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn preview_generic_import(
    state: State<AppState>,
    request: GenericImportRequest,
) -> ApiResponse<GenericImportPreviewDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::preview_generic_import(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn preview_generic_import_headers(
    request: GenericImportHeaderRequest,
) -> ApiResponse<GenericImportHeadersDto> {
    let result = generalization::preview_generic_import_headers(request);
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn confirm_generic_import(
    state: State<AppState>,
    request: GenericImportRequest,
) -> ApiResponse<GenericImportResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::confirm_generic_import(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_generic_import_report(
    state: State<AppState>,
    request: GenericImportReportRequest,
) -> ApiResponse<String> {
    let result = {
        let title = safe_file_name(if request.title.trim().is_empty() {
            "通用导入报告"
        } else {
            request.title.trim()
        });
        let path = state.exports_dir().join(format!(
            "{}_{}.xlsx",
            title,
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        generalization::export_generic_import_report(&path, request)
    };
    if let Ok(path) = &result {
        logger::info("import", format!("通用导入报告已导出：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn download_import_template(
    state: State<AppState>,
    import_type: String,
) -> ApiResponse<String> {
    let result = (|| {
        let title = match import_type.as_str() {
            "products" => "通用商品导入模板",
            "customers" => "通用客户导入模板",
            "initial_stock" => "通用期初库存导入模板",
            other => anyhow::bail!("不支持的通用导入模板类型：{other}"),
        };
        let path = state.exports_dir().join(format!(
            "{}_{}.xlsx",
            safe_file_name(title),
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        generalization::export_generic_import_template(&path, &import_type)
    })();
    if let Ok(path) = &result {
        logger::info("import", format!("通用导入模板已导出：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_import_mapping(state: State<AppState>, mapping: ImportMappingDto) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_import_mapping(&conn, mapping)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_import_mappings(
    state: State<AppState>,
    import_type: Option<String>,
) -> ApiResponse<Vec<ImportMappingDto>> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::list_import_mappings(&conn, import_type)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_printers() -> ApiResponse<Vec<String>> {
    reports::list_system_printers().map(ok).unwrap_or_else(fail)
}

fn restore_backup_record(
    state: &AppState,
    backup_id: i64,
) -> anyhow::Result<RestoreBackupResultDto> {
    if backup_id <= 0 {
        anyhow::bail!("备份记录不合法");
    }
    let conn = state.connection()?;
    let backup_path_text: String = conn.query_row(
        "SELECT backup_path FROM backup_logs WHERE id = ?1 AND status = 'success'",
        [backup_id],
        |row| row.get(0),
    )?;
    drop(conn);

    let backup_path = PathBuf::from(&backup_path_text);
    if !backup_path.exists() {
        anyhow::bail!("备份文件不存在");
    }
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let snapshot_path = state.backups_dir().join(format!("pre_restore_{stamp}.db"));
    db::restore_database_file(&state.db_path(), &backup_path, &snapshot_path)?;

    let conn = state.connection()?;
    db::init_schema(&conn)?;
    db::seed_settings(&conn)?;
    db::ensure_guest_customer(&conn)?;
    conn.execute(
        "INSERT INTO backup_logs (backup_path, backup_type, status, message, created_at)
         VALUES (?1, 'pre_restore', 'success', ?2, ?3)",
        params![
            snapshot_path.to_string_lossy().to_string(),
            "恢复前自动快照",
            now_text()
        ],
    )?;
    conn.execute(
        "INSERT INTO backup_logs (backup_path, backup_type, status, message, created_at)
         VALUES (?1, 'restore', 'success', ?2, ?3)",
        params![backup_path_text, "已从该备份恢复数据库", now_text()],
    )?;
    record_audit(
        &conn,
        AuditEvent {
            module: "backup",
            action: "restore",
            target_type: Some("backup_logs"),
            target_id: Some(backup_id),
            target_label: Some("数据库恢复"),
            result: "success",
            message: Some("数据库恢复完成"),
            details: Some(&format!("preRestore={}", snapshot_path.to_string_lossy())),
        },
    )?;

    Ok(RestoreBackupResultDto {
        restored_backup_path: backup_path.to_string_lossy().to_string(),
        pre_restore_backup_path: snapshot_path.to_string_lossy().to_string(),
        message: "数据库恢复完成，请重新打开应用确认数据".to_string(),
    })
}

fn create_inventory_adjustment_record(
    conn: &mut rusqlite::Connection,
    payload: CreateInventoryAdjustmentRequest,
) -> anyhow::Result<InventoryAdjustmentDto> {
    if payload.product_id <= 0 || payload.quantity_delta.abs() < f64::EPSILON {
        anyhow::bail!("调整商品和调整数量不合法");
    }
    let adjustment_type = validate_adjustment_type(&payload.adjustment_type)?;
    let reason = payload.reason.trim();
    if reason.is_empty() {
        anyhow::bail!("库存调整原因必填");
    }
    let tx = conn.transaction()?;
    let product = db::product_by_id(&tx, payload.product_id)?;
    if !product.is_active {
        anyhow::bail!("商品已停用，不能调整库存");
    }
    let unit_cost = money(product.avg_cost);
    let amount = money(payload.quantity_delta * unit_cost);
    let now = now_text();
    tx.execute(
        "INSERT INTO inventory_adjustments
         (adjustment_date, product_id, product_name, category, adjustment_type, quantity_delta,
          unit_cost, amount, reason, remark, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'normal', ?11, ?11)",
        params![
            normalize_date(&payload.adjustment_date),
            product.id,
            product.name,
            product.category,
            adjustment_type,
            money(payload.quantity_delta),
            unit_cost,
            amount,
            reason,
            payload.remark,
            now
        ],
    )?;
    let adjustment_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount,
          source_type, source_id, source_no, remark, created_at)
         VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                 'inventory_adjustment', ?6, ?7, ?8, ?9)",
        params![
            normalize_date(&payload.adjustment_date),
            product.id,
            money(payload.quantity_delta),
            unit_cost,
            amount,
            adjustment_id,
            format!("ADJ{adjustment_id:06}"),
            reason,
            now
        ],
    )?;
    db::recalc_stock_balance(&tx, product.id)?;
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "adjust",
            target_type: Some("products"),
            target_id: Some(product.id),
            target_label: Some(&product.name),
            result: "success",
            message: Some(reason),
            details: Some(&format!("quantityDelta={}", money(payload.quantity_delta))),
        },
    )?;
    tx.commit()?;
    inventory_adjustment_by_id(conn, adjustment_id)
}

fn void_inventory_adjustment_record(
    conn: &mut rusqlite::Connection,
    id: i64,
    reason: Option<String>,
) -> anyhow::Result<InventoryAdjustmentDto> {
    let tx = conn.transaction()?;
    let (product_id, product_name, adjustment_date, quantity_delta, unit_cost, amount, status): (
        i64,
        String,
        String,
        f64,
        f64,
        f64,
        String,
    ) = tx.query_row(
        "SELECT product_id, product_name, adjustment_date, quantity_delta, unit_cost, amount, status
         FROM inventory_adjustments WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    if status != "normal" {
        anyhow::bail!("库存调整记录已作废");
    }
    let void_reason = reason.unwrap_or_else(|| "作废库存调整".to_string());
    let now = now_text();
    tx.execute(
        "UPDATE inventory_adjustments
         SET status = 'voided', void_reason = ?1, voided_at = ?2, updated_at = ?2
         WHERE id = ?3",
        params![void_reason, now, id],
    )?;
    tx.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount,
          source_type, source_id, source_no, remark, created_at)
         VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                 'inventory_adjustment_void', ?6, ?7, ?8, ?9)",
        params![
            adjustment_date,
            product_id,
            money(-quantity_delta),
            unit_cost,
            money(-amount),
            id,
            format!("ADJ{id:06}"),
            void_reason,
            now
        ],
    )?;
    db::recalc_stock_balance(&tx, product_id)?;
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "void_adjustment",
            target_type: Some("inventory_adjustments"),
            target_id: Some(id),
            target_label: Some(&product_name),
            result: "success",
            message: Some("库存调整已作废"),
            details: None,
        },
    )?;
    tx.commit()?;
    inventory_adjustment_by_id(conn, id)
}

fn create_stocktake_record(
    conn: &mut rusqlite::Connection,
    payload: CreateStocktakeRequest,
) -> anyhow::Result<StocktakeRecordDto> {
    if payload.product_id <= 0 || payload.actual_stock < 0.0 {
        anyhow::bail!("盘点商品和实盘库存不合法");
    }
    let reason = payload.reason.trim();
    if reason.is_empty() {
        anyhow::bail!("盘点原因必填");
    }
    let tx = conn.transaction()?;
    let product = db::product_by_id(&tx, payload.product_id)?;
    if !product.is_active {
        anyhow::bail!("商品已停用，不能盘点");
    }
    let system_stock = money(product.current_stock);
    let actual_stock = money(payload.actual_stock);
    let difference_quantity = money(actual_stock - system_stock);
    let unit_cost = money(product.avg_cost);
    let difference_amount = money(difference_quantity * unit_cost);
    let now = now_text();
    tx.execute(
        "INSERT INTO stocktake_records
         (stocktake_date, product_id, product_name, category, system_stock, actual_stock,
          difference_quantity, unit_cost, difference_amount, reason, remark, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'normal', ?12, ?12)",
        params![
            normalize_date(&payload.stocktake_date),
            product.id,
            product.name,
            product.category,
            system_stock,
            actual_stock,
            difference_quantity,
            unit_cost,
            difference_amount,
            reason,
            payload.remark,
            now
        ],
    )?;
    let stocktake_id = tx.last_insert_rowid();
    if difference_quantity.abs() > f64::EPSILON {
        tx.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount,
              source_type, source_id, source_no, remark, created_at)
             VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                     'stocktake', ?6, ?7, ?8, ?9)",
            params![
                normalize_date(&payload.stocktake_date),
                product.id,
                difference_quantity,
                unit_cost,
                difference_amount,
                stocktake_id,
                format!("STK{stocktake_id:06}"),
                reason,
                now
            ],
        )?;
        db::recalc_stock_balance(&tx, product.id)?;
    }
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "stocktake",
            target_type: Some("products"),
            target_id: Some(product.id),
            target_label: Some(&product.name),
            result: "success",
            message: Some(reason),
            details: Some(&format!("difference={difference_quantity}")),
        },
    )?;
    tx.commit()?;
    stocktake_by_id(conn, stocktake_id)
}

fn void_stocktake_record(
    conn: &mut rusqlite::Connection,
    id: i64,
    reason: Option<String>,
) -> anyhow::Result<StocktakeRecordDto> {
    let tx = conn.transaction()?;
    let (
        product_id,
        product_name,
        stocktake_date,
        difference_quantity,
        unit_cost,
        difference_amount,
        status,
    ): (i64, String, String, f64, f64, f64, String) = tx.query_row(
        "SELECT product_id, product_name, stocktake_date, difference_quantity, unit_cost,
                difference_amount, status
         FROM stocktake_records WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    if status != "normal" {
        anyhow::bail!("盘点记录已作废");
    }
    let void_reason = reason.unwrap_or_else(|| "作废盘点记录".to_string());
    let now = now_text();
    tx.execute(
        "UPDATE stocktake_records
         SET status = 'voided', void_reason = ?1, voided_at = ?2, updated_at = ?2
         WHERE id = ?3",
        params![void_reason, now, id],
    )?;
    if difference_quantity.abs() > f64::EPSILON {
        tx.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount,
              source_type, source_id, source_no, remark, created_at)
             VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                     'stocktake_void', ?6, ?7, ?8, ?9)",
            params![
                stocktake_date,
                product_id,
                money(-difference_quantity),
                unit_cost,
                money(-difference_amount),
                id,
                format!("STK{id:06}"),
                void_reason,
                now
            ],
        )?;
        db::recalc_stock_balance(&tx, product_id)?;
    }
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "void_stocktake",
            target_type: Some("stocktake_records"),
            target_id: Some(id),
            target_label: Some(&product_name),
            result: "success",
            message: Some("盘点记录已作废"),
            details: None,
        },
    )?;
    tx.commit()?;
    stocktake_by_id(conn, id)
}

fn inventory_adjustments(
    conn: &rusqlite::Connection,
    filter: InventoryAdjustmentFilterRequest,
) -> anyhow::Result<Vec<InventoryAdjustmentDto>> {
    let mut sql = "SELECT id, adjustment_date, product_id, product_name, category, adjustment_type,
                          quantity_delta, unit_cost, amount, reason, remark, status, void_reason,
                          voided_at, created_at
                   FROM inventory_adjustments WHERE 1 = 1"
        .to_string();
    append_common_inventory_filters(
        &mut sql,
        "adjustment_date",
        filter.start_date,
        filter.end_date,
        filter.product_id,
        filter.category,
        filter.status,
    );
    sql.push_str(" ORDER BY adjustment_date DESC, id DESC LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_inventory_adjustment)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn stocktakes(
    conn: &rusqlite::Connection,
    filter: StocktakeFilterRequest,
) -> anyhow::Result<Vec<StocktakeRecordDto>> {
    let mut sql = "SELECT id, stocktake_date, product_id, product_name, category, system_stock,
                          actual_stock, difference_quantity, unit_cost, difference_amount, reason,
                          remark, status, void_reason, voided_at, created_at
                   FROM stocktake_records WHERE 1 = 1"
        .to_string();
    append_common_inventory_filters(
        &mut sql,
        "stocktake_date",
        filter.start_date,
        filter.end_date,
        filter.product_id,
        filter.category,
        filter.status,
    );
    sql.push_str(" ORDER BY stocktake_date DESC, id DESC LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_stocktake)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn audit_logs(
    conn: &rusqlite::Connection,
    filter: AuditLogFilterRequest,
) -> anyhow::Result<Vec<AuditLogDto>> {
    let mut sql = "SELECT id, log_time, module, action, target_type, target_id, target_label,
                          result, message, details
                   FROM audit_logs WHERE 1 = 1"
        .to_string();
    if let Some(module) = filter.module.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND module = '{}'", escape_sql(&module)));
    }
    if let Some(action) = filter.action.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND action = '{}'", escape_sql(&action)));
    }
    if let Some(start) = filter.start_date.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND log_time >= '{}'", escape_sql(&start)));
    }
    if let Some(end) = filter.end_date.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND log_time <= '{} 23:59:59'", escape_sql(&end)));
    }
    sql.push_str(" ORDER BY log_time DESC, id DESC LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(AuditLogDto {
            id: row.get(0)?,
            log_time: row.get(1)?,
            module: row.get(2)?,
            action: row.get(3)?,
            target_type: row.get(4)?,
            target_id: row.get(5)?,
            target_label: row.get(6)?,
            result: row.get(7)?,
            message: row.get(8)?,
            details: row.get(9)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn append_common_inventory_filters(
    sql: &mut String,
    date_column: &str,
    start_date: Option<String>,
    end_date: Option<String>,
    product_id: Option<i64>,
    category: Option<String>,
    status: Option<String>,
) {
    if let Some(start) = start_date.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(
            " AND {date_column} >= '{}'",
            escape_sql(&normalize_date(&start))
        ));
    }
    if let Some(end) = end_date.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(
            " AND {date_column} <= '{}'",
            escape_sql(&normalize_date(&end))
        ));
    }
    if let Some(product_id) = product_id {
        sql.push_str(&format!(" AND product_id = {product_id}"));
    }
    if let Some(category) = category.filter(|value| !value.is_empty() && value != "全部") {
        sql.push_str(&format!(" AND category = '{}'", escape_sql(&category)));
    }
    if let Some(status) = status.filter(|value| !value.is_empty() && value != "全部") {
        sql.push_str(&format!(" AND status = '{}'", escape_sql(&status)));
    }
}

fn inventory_adjustment_by_id(
    conn: &rusqlite::Connection,
    id: i64,
) -> anyhow::Result<InventoryAdjustmentDto> {
    conn.query_row(
        "SELECT id, adjustment_date, product_id, product_name, category, adjustment_type,
                quantity_delta, unit_cost, amount, reason, remark, status, void_reason,
                voided_at, created_at
         FROM inventory_adjustments WHERE id = ?1",
        [id],
        map_inventory_adjustment,
    )
    .map_err(Into::into)
}

fn stocktake_by_id(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<StocktakeRecordDto> {
    conn.query_row(
        "SELECT id, stocktake_date, product_id, product_name, category, system_stock,
                actual_stock, difference_quantity, unit_cost, difference_amount, reason,
                remark, status, void_reason, voided_at, created_at
         FROM stocktake_records WHERE id = ?1",
        [id],
        map_stocktake,
    )
    .map_err(Into::into)
}

fn map_inventory_adjustment(row: &rusqlite::Row<'_>) -> rusqlite::Result<InventoryAdjustmentDto> {
    Ok(InventoryAdjustmentDto {
        id: row.get(0)?,
        adjustment_date: row.get(1)?,
        product_id: row.get(2)?,
        product_name: row.get(3)?,
        category: row.get(4)?,
        adjustment_type: row.get(5)?,
        quantity_delta: row.get(6)?,
        unit_cost: row.get(7)?,
        amount: row.get(8)?,
        reason: row.get(9)?,
        remark: row.get(10)?,
        status: row.get(11)?,
        void_reason: row.get(12)?,
        voided_at: row.get(13)?,
        created_at: row.get(14)?,
    })
}

fn map_stocktake(row: &rusqlite::Row<'_>) -> rusqlite::Result<StocktakeRecordDto> {
    Ok(StocktakeRecordDto {
        id: row.get(0)?,
        stocktake_date: row.get(1)?,
        product_id: row.get(2)?,
        product_name: row.get(3)?,
        category: row.get(4)?,
        system_stock: row.get(5)?,
        actual_stock: row.get(6)?,
        difference_quantity: row.get(7)?,
        unit_cost: row.get(8)?,
        difference_amount: row.get(9)?,
        reason: row.get(10)?,
        remark: row.get(11)?,
        status: row.get(12)?,
        void_reason: row.get(13)?,
        voided_at: row.get(14)?,
        created_at: row.get(15)?,
    })
}

struct AuditEvent<'a> {
    module: &'a str,
    action: &'a str,
    target_type: Option<&'a str>,
    target_id: Option<i64>,
    target_label: Option<&'a str>,
    result: &'a str,
    message: Option<&'a str>,
    details: Option<&'a str>,
}

fn record_audit(conn: &rusqlite::Connection, event: AuditEvent<'_>) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO audit_logs
         (log_time, module, action, target_type, target_id, target_label, result, message, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            now_text(),
            event.module,
            event.action,
            event.target_type,
            event.target_id,
            event.target_label,
            event.result,
            event.message,
            event.details
        ],
    )?;
    Ok(())
}

fn validate_adjustment_type(value: &str) -> anyhow::Result<String> {
    let normalized = value.trim();
    match normalized {
        "loss" | "increase" | "scrap" | "self_use" | "other" => Ok(normalized.to_string()),
        _ => anyhow::bail!("不支持的库存调整类型"),
    }
}

fn escape_sql(value: &str) -> String {
    value.replace('\'', "''")
}

fn batch_ids_sql(ids: &[i64]) -> anyhow::Result<String> {
    if ids.is_empty() {
        anyhow::bail!("请选择要批量编辑的记录");
    }
    if ids.iter().any(|id| *id <= 0) {
        anyhow::bail!("批量编辑记录不合法");
    }
    Ok(ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(","))
}

fn text_assignment(column: &str, value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        format!("{column} = NULL")
    } else {
        format!("{column} = '{}'", escape_sql(trimmed))
    }
}

fn batch_update_products_record(
    conn: &rusqlite::Connection,
    payload: BatchUpdateProductsRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    let ids_sql = batch_ids_sql(&payload.ids)?;
    let mut sets = Vec::new();
    if let Some(category) = payload.category {
        let category = category.trim();
        if category.is_empty() {
            anyhow::bail!("商品类别不能为空");
        }
        sets.push(format!("category = '{}'", escape_sql(category)));
    }
    if let Some(value) = payload.safety_stock {
        if value < 0.0 {
            anyhow::bail!("安全库存不能小于 0");
        }
        sets.push(format!("safety_stock = {}", money(value)));
    }
    if let Some(value) = payload.default_price {
        if value < 0.0 {
            anyhow::bail!("默认售价不能小于 0");
        }
        sets.push(format!("default_price = {}", money(value)));
    }
    if let Some(unit) = payload.unit {
        sets.push(text_assignment("unit", &unit));
    }
    if let Some(active) = payload.is_active {
        sets.push(format!("is_active = {}", if active { 1 } else { 0 }));
    }
    if sets.is_empty() {
        anyhow::bail!("没有要更新的商品字段");
    }
    sets.push(format!("updated_at = '{}'", escape_sql(&now_text())));
    let sql = format!(
        "UPDATE products SET {} WHERE id IN ({ids_sql})",
        sets.join(", ")
    );
    let affected = conn.execute(&sql, [])?;
    Ok(BatchUpdateResultDto {
        affected_count: affected as i64,
    })
}

fn batch_update_customers_record(
    conn: &rusqlite::Connection,
    payload: BatchUpdateCustomersRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    let ids_sql = batch_ids_sql(&payload.ids)?;
    if payload.is_active == Some(false) {
        let guest_count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM customers WHERE id IN ({ids_sql}) AND id = ?1"),
            [db::ensure_guest_customer(conn)?],
            |row| row.get(0),
        )?;
        if guest_count > 0 {
            let guest_name = db::guest_customer_name(conn)?;
            anyhow::bail!("{guest_name}是系统默认客户，不能批量停用");
        }
    }
    let mut sets = Vec::new();
    if let Some(region) = payload.region {
        sets.push(text_assignment("region", &region));
    }
    if let Some(remark) = payload.remark {
        sets.push(text_assignment("remark", &remark));
    }
    if let Some(active) = payload.is_active {
        sets.push(format!("is_active = {}", if active { 1 } else { 0 }));
    }
    if sets.is_empty() {
        anyhow::bail!("没有要更新的客户字段");
    }
    sets.push(format!("updated_at = '{}'", escape_sql(&now_text())));
    let sql = format!(
        "UPDATE customers SET {} WHERE id IN ({ids_sql})",
        sets.join(", ")
    );
    let affected = conn.execute(&sql, [])?;
    Ok(BatchUpdateResultDto {
        affected_count: affected as i64,
    })
}

fn batch_update_suppliers_record(
    conn: &rusqlite::Connection,
    payload: BatchUpdateSuppliersRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    let ids_sql = batch_ids_sql(&payload.ids)?;
    let mut sets = Vec::new();
    if let Some(contact) = payload.contact {
        sets.push(text_assignment("contact", &contact));
    }
    if let Some(phone) = payload.phone {
        sets.push(text_assignment("phone", &phone));
    }
    if let Some(address) = payload.address {
        sets.push(text_assignment("address", &address));
    }
    if let Some(remark) = payload.remark {
        sets.push(text_assignment("remark", &remark));
    }
    if sets.is_empty() {
        anyhow::bail!("没有要更新的供应商字段");
    }
    sets.push(format!("updated_at = '{}'", escape_sql(&now_text())));
    let sql = format!(
        "UPDATE suppliers SET {} WHERE id IN ({ids_sql})",
        sets.join(", ")
    );
    let affected = conn.execute(&sql, [])?;
    Ok(BatchUpdateResultDto {
        affected_count: affected as i64,
    })
}

fn create_customer_record(
    conn: &rusqlite::Connection,
    payload: CustomerPayload,
) -> anyhow::Result<CustomerDto> {
    let name = payload.name.trim();
    if name.is_empty() {
        anyhow::bail!("客户名称必填");
    }
    let guest_name = db::guest_customer_name(conn)?;
    if name == guest_name || name == db::GUEST_CUSTOMER_NAME {
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

    let guest_name = db::guest_customer_name(conn)?;
    let is_guest = db::is_guest_customer(conn, id)?;
    if is_guest && name != guest_name {
        anyhow::bail!("{guest_name}是系统默认客户，名称不能修改");
    }
    if !is_guest && (name == guest_name || name == db::GUEST_CUSTOMER_NAME) {
        anyhow::bail!("{guest_name}是系统默认客户，不能重复创建");
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

#[derive(Clone)]
struct ParsedRuleImportRow {
    dto: CustomerProductRuleImportRowDto,
    payload: Option<SaveCustomerProductRuleRequest>,
}

fn preview_customer_product_rule_import_record(
    conn: &rusqlite::Connection,
    file_path: &str,
) -> anyhow::Result<CustomerProductRuleImportPreviewDto> {
    let rows = parse_customer_product_rule_import_rows(conn, file_path)?;
    Ok(rule_import_preview(
        rows.into_iter().map(|row| row.dto).collect(),
    ))
}

fn import_customer_product_rules_record(
    conn: &rusqlite::Connection,
    file_path: &str,
) -> anyhow::Result<CustomerProductRuleImportResultDto> {
    let rows = parse_customer_product_rule_import_rows(conn, file_path)?;
    let mut output_rows = Vec::new();
    let mut imported_count = 0;
    for row in rows {
        let mut dto = row.dto;
        if let Some(payload) = row.payload {
            match save_customer_product_rule_record(conn, payload) {
                Ok(_) => {
                    imported_count += 1;
                    dto.status = "imported".to_string();
                    dto.message = Some("已导入".to_string());
                }
                Err(error) => {
                    dto.status = "error".to_string();
                    dto.message = Some(error.to_string());
                }
            }
        }
        output_rows.push(dto);
    }
    let create_count = output_rows
        .iter()
        .filter(|row| row.status == "imported" && row.action == "create")
        .count() as i64;
    let overwrite_count = output_rows
        .iter()
        .filter(|row| row.status == "imported" && row.action == "overwrite")
        .count() as i64;
    let error_count = output_rows
        .iter()
        .filter(|row| row.status == "error")
        .count() as i64;
    let skipped_count = output_rows
        .iter()
        .filter(|row| row.status == "skipped")
        .count() as i64;
    Ok(CustomerProductRuleImportResultDto {
        imported_count,
        create_count,
        overwrite_count,
        error_count,
        skipped_count,
        rows: output_rows,
    })
}

fn rule_import_preview(
    rows: Vec<CustomerProductRuleImportRowDto>,
) -> CustomerProductRuleImportPreviewDto {
    let valid_count = rows.iter().filter(|row| row.status == "valid").count() as i64;
    let create_count = rows
        .iter()
        .filter(|row| row.status == "valid" && row.action == "create")
        .count() as i64;
    let overwrite_count = rows
        .iter()
        .filter(|row| row.status == "valid" && row.action == "overwrite")
        .count() as i64;
    let error_count = rows.iter().filter(|row| row.status == "error").count() as i64;
    let skipped_count = rows.iter().filter(|row| row.status == "skipped").count() as i64;
    CustomerProductRuleImportPreviewDto {
        total_count: rows.len() as i64,
        valid_count,
        create_count,
        overwrite_count,
        error_count,
        skipped_count,
        rows,
    }
}

fn parse_customer_product_rule_import_rows(
    conn: &rusqlite::Connection,
    file_path: &str,
) -> anyhow::Result<Vec<ParsedRuleImportRow>> {
    let mut workbook = open_workbook_auto(file_path)?;
    let sheet_name = workbook
        .sheet_names()
        .iter()
        .find(|name| name.as_str() == "客户商品规则")
        .cloned()
        .or_else(|| workbook.sheet_names().first().cloned())
        .ok_or_else(|| anyhow::anyhow!("Excel 中没有可读取的工作表"))?;
    let range = workbook.worksheet_range(&sheet_name)?;
    let mut row_iter = range.rows();
    let header_row = row_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("客户商品规则导入表缺少表头"))?;
    let headers = rule_import_headers(header_row);
    let customer_col =
        required_rule_import_column(&headers, &["客户", "客户名称", "customer", "customername"])?;
    let product_col =
        required_rule_import_column(&headers, &["商品", "商品名称", "product", "productname"])?;

    let mut parsed_rows = Vec::new();
    for (index, row) in row_iter.enumerate() {
        let row_number = index as i64 + 2;
        parsed_rows.push(parse_rule_import_row(
            conn,
            row,
            row_number,
            &headers,
            customer_col,
            product_col,
        ));
    }
    Ok(parsed_rows)
}

fn parse_rule_import_row(
    conn: &rusqlite::Connection,
    row: &[Data],
    row_number: i64,
    headers: &HashMap<String, usize>,
    customer_col: usize,
    product_col: usize,
) -> ParsedRuleImportRow {
    let customer_name = rule_cell_text(row.get(customer_col));
    let product_name = rule_cell_text(row.get(product_col));
    let category = optional_text(rule_import_text(row, headers, &["类别", "category"]));
    let fixed_price = rule_import_number(row, headers, &["固定售价", "售价", "fixedprice"]);
    let threshold_quantity =
        rule_import_number(row, headers, &["每满数量", "满数量", "thresholdquantity"]);
    let gift_product_name = optional_text(rule_import_text(
        row,
        headers,
        &["赠品商品", "赠品", "giftproduct", "giftproductname"],
    ));
    let gift_quantity = rule_import_number(row, headers, &["赠品数量", "giftquantity"]);
    let direct_discount_amount =
        rule_import_number(row, headers, &["直接折现", "折现", "directdiscountamount"]);
    let monthly_credit_amount =
        rule_import_number(row, headers, &["生成月费", "月费", "monthlycreditamount"]);
    let credit_category = optional_text(rule_import_text(
        row,
        headers,
        &["月费可用类别", "可用类别", "creditcategory"],
    ));
    let remark = optional_text(rule_import_text(row, headers, &["备注", "remark"]));

    let mut dto = CustomerProductRuleImportRowDto {
        row_number,
        customer_name,
        product_name,
        category,
        fixed_price,
        threshold_quantity,
        gift_product_name,
        gift_quantity,
        direct_discount_amount,
        monthly_credit_amount,
        credit_category,
        remark,
        action: "skip".to_string(),
        status: "skipped".to_string(),
        message: Some("空行".to_string()),
    };
    if rule_import_row_is_empty(&dto) {
        return ParsedRuleImportRow { dto, payload: None };
    }

    match build_rule_import_payload(conn, &dto) {
        Ok((payload, action)) => {
            dto.status = "valid".to_string();
            dto.action = action;
            dto.message = None;
            ParsedRuleImportRow {
                dto,
                payload: Some(payload),
            }
        }
        Err(error) => {
            dto.status = "error".to_string();
            dto.action = "skip".to_string();
            dto.message = Some(error.to_string());
            ParsedRuleImportRow { dto, payload: None }
        }
    }
}

fn build_rule_import_payload(
    conn: &rusqlite::Connection,
    row: &CustomerProductRuleImportRowDto,
) -> anyhow::Result<(SaveCustomerProductRuleRequest, String)> {
    if row.customer_name.trim().is_empty() {
        anyhow::bail!("客户不能为空");
    }
    if row.product_name.trim().is_empty() {
        anyhow::bail!("商品不能为空");
    }
    let customer_id = lookup_import_customer_id(conn, &row.customer_name)?;
    let product_id = lookup_import_product_id(conn, &row.product_name, row.category.as_deref())?;
    validate_non_negative(row.fixed_price, "固定售价")?;
    validate_non_negative(row.direct_discount_amount, "直接折现")?;
    validate_non_negative(row.monthly_credit_amount, "生成月费")?;
    if let Some(threshold) = row.threshold_quantity {
        if threshold <= 0.0 {
            anyhow::bail!("每满数量必须大于 0");
        }
    }
    let gift_product_id = match row.gift_product_name.as_ref() {
        Some(name) => Some(lookup_import_product_id(conn, name, None)?),
        None => None,
    };
    if gift_product_id.is_some() {
        if row.threshold_quantity.is_none() {
            anyhow::bail!("买赠规则必须填写每满数量");
        }
        if row.gift_quantity.unwrap_or(0.0) <= 0.0 {
            anyhow::bail!("买赠规则必须填写赠品数量");
        }
    }
    if gift_product_id.is_none() && row.gift_quantity.is_some() {
        anyhow::bail!("填写赠品数量时必须填写赠品商品");
    }
    let has_rule = row.fixed_price.is_some()
        || gift_product_id.is_some()
        || row.direct_discount_amount.is_some()
        || row.monthly_credit_amount.is_some();
    if !has_rule {
        anyhow::bail!("未填写有效计价规则");
    }
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM customer_product_rules
             WHERE customer_id = ?1 AND product_id = ?2 AND is_active = 1
             LIMIT 1",
            params![customer_id, product_id],
            |row| row.get(0),
        )
        .optional()?;
    let action = if existing.is_some() {
        "overwrite"
    } else {
        "create"
    }
    .to_string();
    Ok((
        SaveCustomerProductRuleRequest {
            id: None,
            customer_id,
            product_id,
            fixed_price: row.fixed_price,
            threshold_quantity: row.threshold_quantity,
            gift_product_id,
            gift_quantity: row.gift_quantity,
            direct_discount_amount: row.direct_discount_amount,
            monthly_credit_amount: row.monthly_credit_amount,
            credit_category: row.credit_category.clone(),
            is_active: true,
            remark: row.remark.clone(),
        },
        action,
    ))
}

fn lookup_import_customer_id(conn: &rusqlite::Connection, name: &str) -> anyhow::Result<i64> {
    conn.query_row(
        "SELECT id FROM customers WHERE name = ?1 AND is_active = 1",
        [name.trim()],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| anyhow::anyhow!("客户不存在或已停用：{}", name.trim()))
}

fn lookup_import_product_id(
    conn: &rusqlite::Connection,
    name: &str,
    category: Option<&str>,
) -> anyhow::Result<i64> {
    let name = name.trim();
    let mut sql = "SELECT id FROM products WHERE name = ?1 AND is_active = 1".to_string();
    if let Some(category) = category.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(
            " AND category = '{}'",
            escape_sql(category.trim())
        ));
    }
    sql.push_str(" ORDER BY id LIMIT 2");
    let mut stmt = conn.prepare(&sql)?;
    let ids = stmt
        .query_map([name], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    match ids.as_slice() {
        [id] => Ok(*id),
        [] => anyhow::bail!("商品不存在或已停用：{name}"),
        _ => anyhow::bail!("商品名称重复，请填写类别：{name}"),
    }
}

fn validate_non_negative(value: Option<f64>, label: &str) -> anyhow::Result<()> {
    if value.is_some_and(|item| item < 0.0) {
        anyhow::bail!("{label}不能小于 0");
    }
    Ok(())
}

fn rule_import_row_is_empty(row: &CustomerProductRuleImportRowDto) -> bool {
    row.customer_name.trim().is_empty()
        && row.product_name.trim().is_empty()
        && row.category.as_deref().unwrap_or("").trim().is_empty()
        && row.fixed_price.is_none()
        && row.threshold_quantity.is_none()
        && row
            .gift_product_name
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        && row.gift_quantity.is_none()
        && row.direct_discount_amount.is_none()
        && row.monthly_credit_amount.is_none()
        && row
            .credit_category
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        && row.remark.as_deref().unwrap_or("").trim().is_empty()
}

fn rule_import_headers(row: &[Data]) -> HashMap<String, usize> {
    row.iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            let key = normalize_rule_import_header(&rule_cell_text(Some(cell)));
            if key.is_empty() {
                None
            } else {
                Some((key, index))
            }
        })
        .collect()
}

fn required_rule_import_column(
    headers: &HashMap<String, usize>,
    names: &[&str],
) -> anyhow::Result<usize> {
    rule_import_column(headers, names).ok_or_else(|| anyhow::anyhow!("缺少必需表头：{}", names[0]))
}

fn rule_import_column(headers: &HashMap<String, usize>, names: &[&str]) -> Option<usize> {
    names
        .iter()
        .find_map(|name| headers.get(&normalize_rule_import_header(name)).copied())
}

fn rule_import_text(row: &[Data], headers: &HashMap<String, usize>, names: &[&str]) -> String {
    rule_import_column(headers, names)
        .map(|index| rule_cell_text(row.get(index)))
        .unwrap_or_default()
}

fn rule_import_number(
    row: &[Data],
    headers: &HashMap<String, usize>,
    names: &[&str],
) -> Option<f64> {
    rule_import_column(headers, names).and_then(|index| rule_cell_number(row.get(index)))
}

fn normalize_rule_import_header(value: &str) -> String {
    value.trim().replace([' ', '_'], "").to_ascii_lowercase()
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn rule_cell_text(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(value)) => value.trim().to_string(),
        Some(Data::Float(value)) => {
            if value.fract().abs() < f64::EPSILON {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Some(Data::Int(value)) => value.to_string(),
        Some(Data::Bool(value)) => value.to_string(),
        Some(Data::DateTime(value)) => value.to_string(),
        Some(Data::DateTimeIso(value)) => value.clone(),
        Some(Data::DurationIso(value)) => value.clone(),
        Some(Data::Error(value)) => format!("{value:?}"),
        Some(Data::Empty) | None => String::new(),
    }
}

fn rule_cell_number(cell: Option<&Data>) -> Option<f64> {
    match cell {
        Some(Data::Float(value)) => Some(money(*value)),
        Some(Data::Int(value)) => Some(*value as f64),
        Some(Data::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<f64>().ok().map(money)
            }
        }
        _ => None,
    }
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
    if let Some(value) = payload.template_store_name {
        db::set_setting(conn, "template_store_name", &value)?;
    }
    if let Some(value) = payload.template_footer_text {
        db::set_setting(conn, "template_footer_text", &value)?;
    }
    if let Some(value) = payload.template_show_barcode {
        db::set_setting(
            conn,
            "template_show_barcode",
            if value { "true" } else { "false" },
        )?;
    }
    if let Some(value) = payload.template_product_label {
        db::set_setting(conn, "template_product_label", &value)?;
    }
    if let Some(value) = payload.template_quantity_label {
        db::set_setting(conn, "template_quantity_label", &value)?;
    }
    if let Some(value) = payload.template_price_label {
        db::set_setting(conn, "template_price_label", &value)?;
    }
    if let Some(value) = payload.template_amount_label {
        db::set_setting(conn, "template_amount_label", &value)?;
    }
    if let Some(value) = payload.template_remark_label {
        db::set_setting(conn, "template_remark_label", &value)?;
    }
    if let Some(value) = payload.template_orientation {
        db::set_setting(conn, "template_orientation", &value)?;
    }
    if let Some(value) = payload.template_margin {
        db::set_setting(conn, "template_margin", &value.to_string())?;
    }
    Ok(true)
}

fn run_data_self_check_record<F>(
    conn: &rusqlite::Connection,
    file_exists: F,
) -> anyhow::Result<DataSelfCheckDto>
where
    F: Fn(&str) -> bool,
{
    let mut issues = Vec::new();
    let mut inventory_checked = 0;
    let mut orders_checked = 0;
    let mut credits_checked = 0;
    let mut documents_checked = 0;

    let mut inventory_stmt = conn.prepare(
        "SELECT p.id, p.name,
                COALESCE(s.current_stock, 0) AS current_stock,
                COALESCE(SUM(CASE
                  WHEN m.movement_type IN ('inbound', 'initial_stock') THEN m.quantity
                  WHEN m.movement_type IN ('outbound', 'gift_outbound') THEN -m.quantity
                  WHEN m.movement_type = 'stocktake_adjustment' THEN m.quantity
                  ELSE 0
                END), 0) AS recalculated_stock
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         LEFT JOIN inventory_movements m ON m.product_id = p.id
         GROUP BY p.id, p.name, s.current_stock",
    )?;
    let inventory_rows = inventory_stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, f64>(3)?,
        ))
    })?;
    for row in inventory_rows {
        let (id, name, current, recalculated) = row?;
        inventory_checked += 1;
        if (current - recalculated).abs() > 0.01 {
            issues.push(DataSelfCheckIssueDto {
                check_code: "inventory_balance".to_string(),
                severity: "error".to_string(),
                target_type: "product".to_string(),
                target_id: Some(id),
                target_label: name,
                message: "库存余额与库存流水重算不一致".to_string(),
                details: Some(format!("current={current}, recalculated={recalculated}")),
            });
        }
    }

    let mut order_stmt = conn.prepare(
        "SELECT o.id, o.order_no,
                o.product_sales_amount,
                COALESCE(SUM(CASE WHEN oi.line_type = 'normal' THEN oi.amount ELSE 0 END), 0) AS item_sales,
                o.cost_amount,
                COALESCE(SUM(CASE WHEN oi.line_type = 'normal' THEN oi.cost_amount ELSE 0 END), 0) AS item_cost,
                o.profit_amount,
                COALESCE(SUM(oi.profit_amount), 0) AS item_profit
         FROM orders o
         LEFT JOIN order_items oi ON oi.order_id = o.id
         WHERE o.status = 'normal'
         GROUP BY o.id, o.order_no, o.product_sales_amount, o.cost_amount, o.profit_amount",
    )?;
    let order_rows = order_stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
            row.get::<_, f64>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, f64>(7)?,
        ))
    })?;
    for row in order_rows {
        let (id, order_no, sales, item_sales, cost, item_cost, profit, item_profit) = row?;
        orders_checked += 1;
        if (sales - item_sales).abs() > 0.01
            || (cost - item_cost).abs() > 0.01
            || (profit - item_profit).abs() > 0.01
        {
            issues.push(DataSelfCheckIssueDto {
                check_code: "order_totals".to_string(),
                severity: "error".to_string(),
                target_type: "order".to_string(),
                target_id: Some(id),
                target_label: order_no,
                message: "订单汇总金额与订单明细不一致".to_string(),
                details: Some(format!(
                    "sales={sales}/{item_sales}, cost={cost}/{item_cost}, profit={profit}/{item_profit}"
                )),
            });
        }
    }

    let mut credit_stmt = conn.prepare(
        "SELECT id, source_order_no, amount, used_amount, remaining_amount
         FROM monthly_credits
         WHERE status <> 'voided'",
    )?;
    let credit_rows = credit_stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;
    for row in credit_rows {
        let (id, source_order_no, amount, used_amount, remaining_amount) = row?;
        credits_checked += 1;
        let expected = money(amount - used_amount);
        if (remaining_amount - expected).abs() > 0.01 {
            issues.push(DataSelfCheckIssueDto {
                check_code: "monthly_credit_remaining".to_string(),
                severity: "error".to_string(),
                target_type: "monthly_credit".to_string(),
                target_id: Some(id),
                target_label: source_order_no,
                message: "月费剩余金额与生成金额减已用金额不一致".to_string(),
                details: Some(format!("remaining={remaining_amount}, expected={expected}")),
            });
        }
    }

    let mut document_stmt = conn.prepare(
        "SELECT id, order_no, file_path
         FROM documents
         WHERE COALESCE(status, 'normal') <> 'voided'",
    )?;
    let document_rows = document_stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in document_rows {
        let (id, order_no, file_path) = row?;
        documents_checked += 1;
        if !file_exists(&file_path) {
            issues.push(DataSelfCheckIssueDto {
                check_code: "document_file_missing".to_string(),
                severity: "warning".to_string(),
                target_type: "document".to_string(),
                target_id: Some(id),
                target_label: order_no,
                message: "单据档案文件不存在".to_string(),
                details: Some(file_path),
            });
        }
    }

    Ok(DataSelfCheckDto {
        checked_at: now_text(),
        issue_count: issues.len() as i64,
        inventory_checked,
        orders_checked,
        credits_checked,
        documents_checked,
        issues,
    })
}

fn write_self_check_export(path: &Path, check: &DataSelfCheckDto) -> anyhow::Result<()> {
    let mut text = format!(
        "EasyInventory 数据自检\n时间：{}\n异常数：{}\n库存：{} 订单：{} 月费：{} 单据：{}\n\n",
        check.checked_at,
        check.issue_count,
        check.inventory_checked,
        check.orders_checked,
        check.credits_checked,
        check.documents_checked
    );
    for issue in &check.issues {
        text.push_str(&format!(
            "[{}] {} {} {} {}\n{}\n\n",
            issue.severity,
            issue.check_code,
            issue.target_type,
            issue.target_label,
            issue.message,
            issue.details.clone().unwrap_or_default()
        ));
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn diagnostic_summary(state: &AppState) -> anyhow::Result<DiagnosticSummaryDto> {
    let conn = state.connection()?;
    let database_path = state.db_path();
    Ok(DiagnosticSummaryDto {
        generated_at: now_text(),
        database_path: database_path.to_string_lossy().to_string(),
        logs_dir: state.logs_dir().to_string_lossy().to_string(),
        backups_dir: state.backups_dir().to_string_lossy().to_string(),
        exports_dir: state.exports_dir().to_string_lossy().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database_size: std::fs::metadata(database_path)
            .map(|metadata| metadata.len() as i64)
            .unwrap_or(0),
        backup_count: count_query(&conn, "SELECT COUNT(*) FROM backup_logs")?,
        latest_backup_at: conn
            .query_row(
                "SELECT MAX(created_at) FROM backup_logs WHERE status = 'success'",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten(),
        product_count: count_query(&conn, "SELECT COUNT(*) FROM products")?,
        customer_count: count_query(&conn, "SELECT COUNT(*) FROM customers")?,
        order_count: count_query(&conn, "SELECT COUNT(*) FROM orders")?,
        document_count: count_query(&conn, "SELECT COUNT(*) FROM documents")?,
        setting_count: count_query(&conn, "SELECT COUNT(*) FROM settings")?,
        latest_logs: latest_log_lines(&state.logs_dir(), 40)?,
    })
}

fn export_diagnostic_package_record(state: &AppState) -> anyhow::Result<DiagnosticPackageDto> {
    let conn = state.connection()?;
    let summary = diagnostic_summary(state)?;
    let mut text = String::from("EasyInventory 诊断包\n");
    text.push_str(&format!(
        "生成时间：{}\n版本：{}\n",
        summary.generated_at, summary.version
    ));
    text.push_str(&format!(
        "数据库：{}\n大小：{}\n",
        summary.database_path, summary.database_size
    ));
    text.push_str(&format!(
        "统计：商品 {}，客户 {}，订单 {}，单据 {}，备份 {}\n\n",
        summary.product_count,
        summary.customer_count,
        summary.order_count,
        summary.document_count,
        summary.backup_count
    ));
    text.push_str("设置：\n");
    let mut settings_stmt =
        conn.prepare("SELECT key, COALESCE(value, '') FROM settings ORDER BY key")?;
    let settings = settings_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in settings {
        let (key, value) = row?;
        text.push_str(&format!("{key}={value}\n"));
    }
    text.push_str("\n最近日志：\n");
    for line in &summary.latest_logs {
        text.push_str(line);
        text.push('\n');
    }

    let path = state.exports_dir().join(format!(
        "diagnostic_package_{}.txt",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    std::fs::write(&path, text)?;
    Ok(DiagnosticPackageDto {
        file_path: path.to_string_lossy().to_string(),
        message: "诊断包已导出，包含日志、设置和基础统计，不包含客户明细".to_string(),
    })
}

fn count_query(conn: &rusqlite::Connection, sql: &str) -> anyhow::Result<i64> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn latest_log_lines(logs_dir: &Path, limit: usize) -> anyhow::Result<Vec<String>> {
    let mut files = std::fs::read_dir(logs_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .collect::<Vec<_>>();
    files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    let Some(entry) = files.pop() else {
        return Ok(Vec::new());
    };
    let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
    let mut lines = content.lines().map(ToString::to_string).collect::<Vec<_>>();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    Ok(lines)
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

    fn seed_adjustment_product(conn: &Connection) {
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('盘点商品', '盘点', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES ('2026-06-01', 1, 'inbound', 10, 5, 50, 'test', '初始库存', ?1)",
            [&now],
        )
        .unwrap();
        db::recalc_stock_balance(conn, 1).unwrap();
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
    fn create_customer_record_with_legacy_guest_name_reuses_configured_guest_customer() {
        let conn = memory_conn();
        let guest_id = db::ensure_guest_customer(&conn).unwrap();
        db::set_setting(&conn, "guest_customer_name", "临时客户").unwrap();
        let renamed_id = db::ensure_guest_customer(&conn).unwrap();

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
        let customer_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM customers", [], |row| row.get(0))
            .unwrap();

        assert_eq!(renamed_id, guest_id);
        assert_eq!(customer.id, guest_id);
        assert_eq!(customer.name, "临时客户");
        assert_eq!(customer_count, 1);
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
    fn batch_update_products_updates_requested_fields_only() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, unit, is_active, created_at, updated_at)
             VALUES
             ('商品A', '旧类', 1, 0, '个', 1, ?1, ?1),
             ('商品B', '旧类', 2, 0, '个', 1, ?1, ?1),
             ('商品C', '旧类', 3, 0, '个', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let result = batch_update_products_record(
            &conn,
            BatchUpdateProductsRequest {
                ids: vec![1, 2],
                category: Some("新类".to_string()),
                safety_stock: Some(5.0),
                default_price: Some(9.5),
                unit: Some("箱".to_string()),
                is_active: Some(false),
            },
        )
        .unwrap();

        let updated_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products
                 WHERE id IN (1, 2) AND category = '新类' AND safety_stock = 5
                   AND default_price = 9.5 AND unit = '箱' AND is_active = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let untouched_category: String = conn
            .query_row("SELECT category FROM products WHERE id = 3", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(result.affected_count, 2);
        assert_eq!(updated_count, 2);
        assert_eq!(untouched_category, "旧类");
    }

    #[test]
    fn batch_update_customers_and_suppliers_update_requested_fields() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, remark, created_at, updated_at)
             VALUES
             ('旧区', '客户A', 1, '旧备注', ?1, ?1),
             ('旧区', '客户B', 1, '旧备注', ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (name, contact, phone, address, is_active, remark, created_at, updated_at)
             VALUES
             ('供应商A', '旧联系人', '旧电话', '旧地址', 1, '旧备注', ?1, ?1),
             ('供应商B', '旧联系人', '旧电话', '旧地址', 1, '旧备注', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let customers = batch_update_customers_record(
            &conn,
            BatchUpdateCustomersRequest {
                ids: vec![1, 2],
                region: Some("新区".to_string()),
                remark: Some("新备注".to_string()),
                is_active: Some(false),
            },
        )
        .unwrap();
        let suppliers = batch_update_suppliers_record(
            &conn,
            BatchUpdateSuppliersRequest {
                ids: vec![1, 2],
                contact: Some("新联系人".to_string()),
                phone: Some("新电话".to_string()),
                address: Some("新地址".to_string()),
                remark: Some("新备注".to_string()),
            },
        )
        .unwrap();

        let customer_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM customers
                 WHERE region = '新区' AND remark = '新备注' AND is_active = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let supplier_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM suppliers
                 WHERE contact = '新联系人' AND phone = '新电话'
                   AND address = '新地址' AND remark = '新备注'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(customers.affected_count, 2);
        assert_eq!(suppliers.affected_count, 2);
        assert_eq!(customer_count, 2);
        assert_eq!(supplier_count, 2);
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
    fn inventory_adjustment_updates_stock_and_void_reverses_it() {
        let mut conn = memory_conn();
        seed_adjustment_product(&conn);

        let adjustment = create_inventory_adjustment_record(
            &mut conn,
            CreateInventoryAdjustmentRequest {
                adjustment_date: "2026-06-02".to_string(),
                product_id: 1,
                adjustment_type: "loss".to_string(),
                quantity_delta: -2.0,
                reason: "破损".to_string(),
                remark: Some("货架破损".to_string()),
            },
        )
        .unwrap();
        let stock_after_adjustment: f64 = conn
            .query_row(
                "SELECT current_stock FROM stock_balances WHERE product_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let audit_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_logs", [], |row| row.get(0))
            .unwrap();

        assert_eq!(adjustment.product_id, 1);
        assert_eq!(adjustment.quantity_delta, -2.0);
        assert_eq!(adjustment.status, "normal");
        assert_eq!(stock_after_adjustment, 8.0);
        assert_eq!(audit_count, 1);

        void_inventory_adjustment_record(&mut conn, adjustment.id, Some("录入错误".to_string()))
            .unwrap();
        let stock_after_void: f64 = conn
            .query_row(
                "SELECT current_stock FROM stock_balances WHERE product_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let movement_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM inventory_movements WHERE source_type LIKE 'inventory_adjustment%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let voided_status: String = conn
            .query_row(
                "SELECT status FROM inventory_adjustments WHERE id = ?1",
                [adjustment.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stock_after_void, 10.0);
        assert_eq!(movement_count, 2);
        assert_eq!(voided_status, "voided");
    }

    #[test]
    fn stocktake_records_difference_and_void_reverses_it() {
        let mut conn = memory_conn();
        seed_adjustment_product(&conn);

        let stocktake = create_stocktake_record(
            &mut conn,
            CreateStocktakeRequest {
                stocktake_date: "2026-06-03".to_string(),
                product_id: 1,
                actual_stock: 6.0,
                reason: "月度盘点".to_string(),
                remark: None,
            },
        )
        .unwrap();
        let stock_after_stocktake: f64 = conn
            .query_row(
                "SELECT current_stock FROM stock_balances WHERE product_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stocktake.system_stock, 10.0);
        assert_eq!(stocktake.actual_stock, 6.0);
        assert_eq!(stocktake.difference_quantity, -4.0);
        assert_eq!(stock_after_stocktake, 6.0);

        void_stocktake_record(&mut conn, stocktake.id, Some("复盘错误".to_string())).unwrap();
        let stock_after_void: f64 = conn
            .query_row(
                "SELECT current_stock FROM stock_balances WHERE product_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let voided_status: String = conn
            .query_row(
                "SELECT status FROM stocktake_records WHERE id = ?1",
                [stocktake.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stock_after_void, 10.0);
        assert_eq!(voided_status, "voided");
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
    fn customer_product_rule_import_previews_then_imports_valid_rows() {
        let conn = memory_conn();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.xlsx");
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('默认', '客户A', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES
             ('商品A', '饮料', 10, 0, 1, ?1, ?1),
             ('赠品A', '饮料', 0, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        let old_rule_id = save_customer_product_rule_record(
            &conn,
            SaveCustomerProductRuleRequest {
                id: None,
                customer_id: 1,
                product_id: 1,
                fixed_price: Some(5.0),
                threshold_quantity: None,
                gift_product_id: None,
                gift_quantity: None,
                direct_discount_amount: None,
                monthly_credit_amount: None,
                credit_category: None,
                is_active: true,
                remark: Some("旧规则".to_string()),
            },
        )
        .unwrap();
        write_rule_import_workbook(&path);

        let preview =
            preview_customer_product_rule_import_record(&conn, path.to_str().unwrap()).unwrap();
        let unchanged_fixed_price: f64 = conn
            .query_row(
                "SELECT fixed_price FROM customer_product_rules WHERE id = ?1",
                [old_rule_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(preview.total_count, 3);
        assert_eq!(preview.valid_count, 1);
        assert_eq!(preview.overwrite_count, 1);
        assert_eq!(preview.error_count, 1);
        assert_eq!(preview.skipped_count, 1);
        assert_eq!(unchanged_fixed_price, 5.0);

        let result = import_customer_product_rules_record(&conn, path.to_str().unwrap()).unwrap();
        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM customer_product_rules WHERE customer_id = 1 AND product_id = 1 AND is_active = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let old_is_active: i64 = conn
            .query_row(
                "SELECT is_active FROM customer_product_rules WHERE id = ?1",
                [old_rule_id],
                |row| row.get(0),
            )
            .unwrap();
        let imported: (f64, f64, i64, f64, f64, f64, String) = conn
            .query_row(
                "SELECT fixed_price, threshold_quantity, gift_product_id, gift_quantity,
                        direct_discount_amount, monthly_credit_amount, credit_category
                 FROM customer_product_rules
                 WHERE customer_id = 1 AND product_id = 1 AND is_active = 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(result.imported_count, 1);
        assert_eq!(result.overwrite_count, 1);
        assert_eq!(result.error_count, 1);
        assert_eq!(result.skipped_count, 1);
        assert_eq!(active_count, 1);
        assert_eq!(old_is_active, 0);
        assert_eq!(imported, (8.0, 10.0, 2, 1.0, 2.0, 3.0, "饮料".to_string()));
    }

    fn write_rule_import_workbook(path: &std::path::Path) {
        let mut book = umya_spreadsheet::new_file();
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.set_name("客户商品规则");
        let headers = [
            "客户",
            "商品",
            "类别",
            "固定售价",
            "每满数量",
            "赠品商品",
            "赠品数量",
            "直接折现",
            "生成月费",
            "月费可用类别",
            "备注",
        ];
        for (index, header) in headers.iter().enumerate() {
            sheet
                .get_cell_mut(test_cell_address((index + 1) as u32, 1))
                .set_value(*header);
        }
        let valid = [
            "客户A",
            "商品A",
            "饮料",
            "8",
            "10",
            "赠品A",
            "1",
            "2",
            "3",
            "饮料",
            "覆盖导入",
        ];
        let invalid = [
            "不存在客户",
            "商品A",
            "饮料",
            "9",
            "",
            "",
            "",
            "",
            "",
            "",
            "异常行",
        ];
        for (index, value) in valid.iter().enumerate() {
            sheet
                .get_cell_mut(test_cell_address((index + 1) as u32, 2))
                .set_value(*value);
        }
        for (index, value) in invalid.iter().enumerate() {
            sheet
                .get_cell_mut(test_cell_address((index + 1) as u32, 3))
                .set_value(*value);
        }
        sheet.get_cell_mut("A4").set_value(" ");
        umya_spreadsheet::writer::xlsx::write(&book, path).unwrap();
    }

    fn test_cell_address(column: u32, row: u32) -> String {
        const NAMES: [&str; 11] = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K"];
        format!("{}{}", NAMES[(column - 1) as usize], row)
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
                template_store_name: Some("测试门店".to_string()),
                template_footer_text: None,
                template_show_barcode: None,
                template_product_label: None,
                template_quantity_label: None,
                template_price_label: None,
                template_amount_label: None,
                template_remark_label: None,
                template_orientation: None,
                template_margin: None,
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
        assert_eq!(
            db::setting(&conn, "template_store_name")
                .unwrap()
                .as_deref(),
            Some("测试门店")
        );
    }

    #[test]
    fn data_self_check_detects_core_data_inconsistencies() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (id, name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES (1, '测试商品', '测试', 10, 0, 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
             VALUES (1, 9, 1, 9, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES ('2026-06-01', 1, 'inbound', 10, 1, 10, 'test', 'test', ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES (1, '测试', '测试客户', 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (id, order_no, order_date, customer_id, customer_name, product_sales_amount,
              customer_payable_amount, cost_amount, profit_amount, status, created_at, updated_at)
             VALUES (1, '20260601001', '2026-06-01', 1, '测试客户', 99, 99, 1, 98, 'normal', ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO order_items
             (order_id, line_type, product_id, product_name, category, quantity, unit_price,
              amount, avg_cost, cost_amount, profit_amount, sort_order)
             VALUES (1, 'normal', 1, '测试商品', '测试', 1, 10, 10, 1, 1, 9, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO monthly_credits
             (source_order_id, source_order_no, customer_id, category, amount, used_amount,
              remaining_amount, generated_date, available_month, status, created_at, updated_at)
             VALUES (1, '20260601001', 1, '测试', 100, 30, 99, '2026-06-01', '2026-07', 'available', ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO documents
             (order_id, order_no, customer_id, customer_name, file_path, file_type, created_at)
             VALUES (1, '20260601001', 1, '测试客户', 'C:/not-exists/order.xlsx', 'xlsx', ?1)",
            params![now],
        )
        .unwrap();

        let result = run_data_self_check_record(&conn, |_| false).unwrap();
        let codes = result
            .issues
            .iter()
            .map(|issue| issue.check_code.as_str())
            .collect::<Vec<_>>();

        assert!(codes.contains(&"inventory_balance"));
        assert!(codes.contains(&"order_totals"));
        assert!(codes.contains(&"monthly_credit_remaining"));
        assert!(codes.contains(&"document_file_missing"));
        assert_eq!(result.issue_count, 4);
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
