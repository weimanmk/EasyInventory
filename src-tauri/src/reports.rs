use crate::app::AppState;
use crate::models::{
    DailyProfitSummary, DocumentDto, DocumentFilterRequest, ExportDataRequest,
    InventoryReportRequest, InventoryReportRowDto, MonthlyCreditFilterRequest, PrintStatusDto,
    ProfitAnalyticsMetricDto, ProfitAnalyticsRequest, ProfitAnalyticsResponse,
    ProfitAnalyticsTrendPointDto, ProfitBreakdownDto, ProfitFilterRequest,
};
use crate::utils::{money, now_text, safe_file_name};
use anyhow::{anyhow, Context};
use rusqlite::{params, Connection};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use umya_spreadsheet::{
    self, writer::xlsx, Border, HorizontalAlignmentValues, OrientationValues, Style,
    VerticalAlignmentValues,
};

type ExportRows = Vec<Vec<String>>;
type ExportTable = (&'static str, Vec<&'static str>, ExportRows);

pub fn daily_profit_summary(conn: &Connection, date: &str) -> anyhow::Result<DailyProfitSummary> {
    conn.query_row(
        "SELECT
           COUNT(*),
           COALESCE(SUM(product_sales_amount), 0),
           COALESCE(SUM(customer_payable_amount), 0),
           COALESCE(SUM(direct_discount_amount), 0),
           COALESCE(SUM(monthly_credit_used), 0),
           COALESCE(SUM(brand_subsidy_amount), 0),
           COALESCE(SUM(cost_amount), 0),
           COALESCE(SUM(gift_cost_amount), 0),
           COALESCE(SUM(profit_amount), 0)
         FROM orders
         WHERE order_date = ?1 AND status = 'normal'",
        [date],
        |row| {
            Ok(DailyProfitSummary {
                date: date.to_string(),
                order_count: row.get(0)?,
                product_sales_amount: row.get(1)?,
                customer_payable_amount: row.get(2)?,
                direct_discount_amount: row.get(3)?,
                monthly_credit_used: row.get(4)?,
                brand_subsidy_amount: row.get(5)?,
                cost_amount: row.get(6)?,
                gift_cost_amount: row.get(7)?,
                profit_amount: row.get(8)?,
            })
        },
    )
    .map_err(Into::into)
}

pub fn get_profit_analytics(
    conn: &Connection,
    request: ProfitAnalyticsRequest,
) -> anyhow::Result<ProfitAnalyticsResponse> {
    if request.start_date.trim().is_empty() || request.end_date.trim().is_empty() {
        return Err(anyhow!("利润统计日期范围不能为空"));
    }
    let period_expr = profit_period_expression(&request.period)?;
    let order_where = profit_order_where(&request);
    let summary = profit_analytics_summary(conn, &order_where)?;
    let trend = profit_analytics_trend(conn, period_expr, &order_where)?;
    let category_breakdown = profit_analytics_category_breakdown(conn, &request, &order_where)?;
    let customer_breakdown = profit_analytics_customer_breakdown(conn, &order_where)?;

    Ok(ProfitAnalyticsResponse {
        summary,
        trend,
        category_breakdown,
        customer_breakdown,
    })
}

fn profit_period_expression(period: &str) -> anyhow::Result<&'static str> {
    match period {
        "day" => Ok("o.order_date"),
        "month" => Ok("substr(o.order_date, 1, 7)"),
        "year" => Ok("substr(o.order_date, 1, 4)"),
        _ => Err(anyhow!("不支持的利润统计周期: {period}")),
    }
}

fn profit_order_where(request: &ProfitAnalyticsRequest) -> String {
    let mut conditions = vec![
        "o.status = 'normal'".to_string(),
        format!("o.order_date >= '{}'", escape_sql(&request.start_date)),
        format!("o.order_date <= '{}'", escape_sql(&request.end_date)),
    ];
    if let Some(customer_id) = request.customer_id {
        conditions.push(format!("o.customer_id = {customer_id}"));
    }
    if let Some(category) = active_category(request.category.as_deref()) {
        conditions.push(format!(
            "EXISTS (
               SELECT 1 FROM order_items oi_filter
               WHERE oi_filter.order_id = o.id AND oi_filter.category = '{}'
             )",
            escape_sql(category)
        ));
    }
    conditions.join(" AND ")
}

fn active_category(category: Option<&str>) -> Option<&str> {
    category
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "全部")
}

fn profit_analytics_summary(
    conn: &Connection,
    order_where: &str,
) -> anyhow::Result<ProfitAnalyticsMetricDto> {
    let sql = format!(
        "SELECT
           COUNT(*),
           COALESCE(SUM(product_sales_amount), 0),
           COALESCE(SUM(customer_payable_amount), 0),
           COALESCE(SUM(direct_discount_amount), 0),
           COALESCE(SUM(monthly_credit_used), 0),
           COALESCE(SUM(brand_subsidy_amount), 0),
           COALESCE(SUM(cost_amount), 0),
           COALESCE(SUM(gift_cost_amount), 0),
           COALESCE(SUM(profit_amount), 0)
         FROM orders o
         WHERE {order_where}"
    );
    conn.query_row(&sql, [], |row| {
        Ok(ProfitAnalyticsMetricDto {
            order_count: row.get(0)?,
            product_sales_amount: row.get(1)?,
            customer_payable_amount: row.get(2)?,
            direct_discount_amount: row.get(3)?,
            monthly_credit_used: row.get(4)?,
            brand_subsidy_amount: row.get(5)?,
            cost_amount: row.get(6)?,
            gift_cost_amount: row.get(7)?,
            profit_amount: row.get(8)?,
        })
    })
    .map_err(Into::into)
}

fn profit_analytics_trend(
    conn: &Connection,
    period_expr: &str,
    order_where: &str,
) -> anyhow::Result<Vec<ProfitAnalyticsTrendPointDto>> {
    let sql = format!(
        "SELECT
           {period_expr},
           COUNT(*),
           COALESCE(SUM(product_sales_amount), 0),
           COALESCE(SUM(customer_payable_amount), 0),
           COALESCE(SUM(direct_discount_amount), 0),
           COALESCE(SUM(monthly_credit_used), 0),
           COALESCE(SUM(brand_subsidy_amount), 0),
           COALESCE(SUM(cost_amount), 0),
           COALESCE(SUM(gift_cost_amount), 0),
           COALESCE(SUM(profit_amount), 0)
         FROM orders o
         WHERE {order_where}
         GROUP BY 1
         ORDER BY 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProfitAnalyticsTrendPointDto {
                period: row.get(0)?,
                order_count: row.get(1)?,
                product_sales_amount: row.get(2)?,
                customer_payable_amount: row.get(3)?,
                direct_discount_amount: row.get(4)?,
                monthly_credit_used: row.get(5)?,
                brand_subsidy_amount: row.get(6)?,
                cost_amount: row.get(7)?,
                gift_cost_amount: row.get(8)?,
                profit_amount: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn profit_analytics_category_breakdown(
    conn: &Connection,
    request: &ProfitAnalyticsRequest,
    order_where: &str,
) -> anyhow::Result<Vec<ProfitBreakdownDto>> {
    let mut where_sql =
        format!("{order_where} AND oi.category IS NOT NULL AND TRIM(oi.category) <> ''");
    if let Some(category) = active_category(request.category.as_deref()) {
        where_sql.push_str(&format!(" AND oi.category = '{}'", escape_sql(category)));
    }
    let sql = format!(
        "SELECT
           oi.category,
           COUNT(DISTINCT o.id),
           COALESCE(SUM(oi.amount), 0),
           COALESCE(SUM(oi.amount), 0),
           COALESCE(SUM(oi.cost_amount), 0),
           COALESCE(SUM(oi.profit_amount), 0)
         FROM order_items oi
         JOIN orders o ON o.id = oi.order_id
         WHERE {where_sql}
         GROUP BY oi.category
         ORDER BY COALESCE(SUM(oi.profit_amount), 0) DESC,
                  COALESCE(SUM(oi.amount), 0) DESC,
                  oi.category
         LIMIT 20"
    );
    profit_breakdown_rows(conn, &sql)
}

fn profit_analytics_customer_breakdown(
    conn: &Connection,
    order_where: &str,
) -> anyhow::Result<Vec<ProfitBreakdownDto>> {
    let sql = format!(
        "SELECT
           o.customer_name,
           COUNT(*),
           COALESCE(SUM(o.product_sales_amount), 0),
           COALESCE(SUM(o.customer_payable_amount), 0),
           COALESCE(SUM(o.cost_amount), 0),
           COALESCE(SUM(o.profit_amount), 0)
         FROM orders o
         WHERE {order_where}
         GROUP BY o.customer_id, o.customer_name
         ORDER BY COALESCE(SUM(o.profit_amount), 0) DESC,
                  COALESCE(SUM(o.product_sales_amount), 0) DESC,
                  o.customer_name
         LIMIT 20"
    );
    profit_breakdown_rows(conn, &sql)
}

fn profit_breakdown_rows(conn: &Connection, sql: &str) -> anyhow::Result<Vec<ProfitBreakdownDto>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProfitBreakdownDto {
                name: row.get(0)?,
                order_count: row.get(1)?,
                product_sales_amount: row.get(2)?,
                customer_payable_amount: row.get(3)?,
                cost_amount: row.get(4)?,
                profit_amount: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn export_order_document(state: &AppState, order_id: i64) -> anyhow::Result<String> {
    let conn = state.connection()?;
    let detail = crate::orders::get_order_detail(&conn, order_id)?;
    let customer_folder = state
        .orders_dir()
        .join(safe_file_name(&detail.order.customer_name));
    std::fs::create_dir_all(&customer_folder)?;
    let file_path = customer_folder.join(format!(
        "{}_{}.xlsx",
        detail.order.order_no,
        safe_file_name(&detail.order.customer_name)
    ));
    write_order_workbook(&file_path, &detail)?;
    let now = now_text();
    conn.execute(
        "INSERT INTO documents (order_id, order_no, customer_id, customer_name, file_path, file_type, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'xlsx', ?6)
         ON CONFLICT DO NOTHING",
        params![
            detail.order.id,
            detail.order.order_no,
            detail.order.customer_id,
            detail.order.customer_name,
            file_path.to_string_lossy().to_string(),
            now
        ],
    )?;
    conn.execute(
        "UPDATE orders SET document_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![file_path.to_string_lossy().to_string(), now, order_id],
    )?;
    Ok(file_path.to_string_lossy().to_string())
}

pub fn print_document(
    conn: &Connection,
    document_id: i64,
    printer_name: Option<String>,
) -> anyhow::Result<PrintStatusDto> {
    let document = document_by_id(conn, document_id)?;
    let message = open_or_print_file(&document.file_path, printer_name.as_deref())?;
    conn.execute(
        "UPDATE documents SET print_count = print_count + 1, printed_at = ?1 WHERE id = ?2",
        params![now_text(), document_id],
    )?;
    conn.execute(
        "UPDATE orders SET print_count = print_count + 1, updated_at = ?1 WHERE id = ?2",
        params![now_text(), document.order_id],
    )?;
    Ok(PrintStatusDto {
        file_path: document.file_path,
        printer_name,
        message,
    })
}

pub fn open_document(conn: &Connection, document_id: i64) -> anyhow::Result<String> {
    let document = document_by_id(conn, document_id)?;
    open::that(&document.file_path).with_context(|| "无法打开单据文件，请检查文件是否存在")?;
    Ok(document.file_path)
}

pub fn list_documents(
    conn: &Connection,
    filter: DocumentFilterRequest,
) -> anyhow::Result<Vec<DocumentDto>> {
    let mut sql = String::from(
        "SELECT d.id, d.order_id, d.order_no, d.customer_id, d.customer_name, d.file_path,
                d.file_type, d.printed_at, d.print_count, d.created_at, COALESCE(d.status, 'normal')
         FROM documents d
         JOIN orders o ON o.id = d.order_id
         WHERE 1 = 1",
    );
    let mut conditions = Vec::new();
    if let Some(customer_id) = filter.customer_id {
        conditions.push(format!("d.customer_id = {customer_id}"));
    }
    if let Some(start_date) = filter.start_date {
        conditions.push(format!("o.order_date >= '{}'", escape_sql(&start_date)));
    }
    if let Some(end_date) = filter.end_date {
        conditions.push(format!("o.order_date <= '{}'", escape_sql(&end_date)));
    }
    if let Some(order_no) = filter.order_no {
        conditions.push(format!("d.order_no LIKE '%{}%'", escape_sql(&order_no)));
    }
    if let Some(printed) = filter.printed {
        if printed {
            conditions.push("d.print_count > 0".to_string());
        } else {
            conditions.push("d.print_count = 0".to_string());
        }
    }
    if let Some(status) = filter
        .status
        .filter(|value| !value.is_empty() && value != "全部")
    {
        conditions.push(format!(
            "COALESCE(d.status, 'normal') = '{}'",
            escape_sql(&status)
        ));
    }
    for condition in conditions {
        sql.push_str(" AND ");
        sql.push_str(&condition);
    }
    sql.push_str(" ORDER BY d.created_at DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
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
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn list_profit_records(
    conn: &Connection,
    filter: ProfitFilterRequest,
) -> anyhow::Result<Vec<crate::models::OrderDto>> {
    let category = filter.category.clone();
    let request = crate::models::ListOrdersRequest {
        start_date: filter.start_date,
        end_date: filter.end_date,
        customer_id: filter.customer_id,
        order_no: None,
        status: Some("normal".to_string()),
    };
    let mut orders = crate::orders::list_orders(conn, request)?;
    if let Some(category) = category.filter(|value| !value.is_empty() && value != "全部") {
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) > 0 FROM order_items
             WHERE order_id = ?1 AND category = ?2",
        )?;
        let mut filtered = Vec::new();
        for order in orders {
            let has_category: bool =
                stmt.query_row(params![order.id, &category], |row| row.get(0))?;
            if has_category {
                filtered.push(order);
            }
        }
        orders = filtered;
    }
    Ok(orders)
}

pub fn list_inventory_report(
    conn: &Connection,
    request: InventoryReportRequest,
) -> anyhow::Result<Vec<InventoryReportRowDto>> {
    let movement_where = movement_date_condition(&request);
    let mut sql = format!(
        "SELECT p.id, p.name, p.category, p.barcode,
                COALESCE(inbound.inbound_quantity, 0),
                COALESCE(inbound.inbound_amount, 0),
                COALESCE(outbound.outbound_quantity, 0),
                COALESCE(outbound.outbound_amount, 0),
                COALESCE(gift.gift_quantity, 0),
                COALESCE(s.current_stock, 0),
                COALESCE(s.avg_cost, 0),
                COALESCE(s.stock_value, 0)
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         LEFT JOIN (
           SELECT product_id, SUM(quantity) AS inbound_quantity, SUM(amount) AS inbound_amount
           FROM inventory_movements
           WHERE movement_type = 'inbound' {movement_where}
           GROUP BY product_id
         ) inbound ON inbound.product_id = p.id
         LEFT JOIN (
           SELECT product_id, SUM(quantity) AS outbound_quantity, SUM(amount) AS outbound_amount
           FROM inventory_movements
           WHERE movement_type = 'outbound' {movement_where}
           GROUP BY product_id
         ) outbound ON outbound.product_id = p.id
         LEFT JOIN (
           SELECT product_id, SUM(quantity) AS gift_quantity
           FROM inventory_movements
           WHERE movement_type = 'gift_outbound' {movement_where}
           GROUP BY product_id
         ) gift ON gift.product_id = p.id
         WHERE p.is_active = 1"
    );
    if let Some(category) = request
        .category
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(&format!(" AND p.category = '{}'", escape_sql(&category)));
    }
    if let Some(keyword) = request.keyword.filter(|value| !value.is_empty()) {
        let keyword = escape_sql(&keyword);
        sql.push_str(&format!(
            " AND (p.name LIKE '%{keyword}%' OR p.barcode LIKE '%{keyword}%')"
        ));
    }
    sql.push_str(" ORDER BY p.category, p.name LIMIT 10000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(InventoryReportRowDto {
            product_id: row.get(0)?,
            product_name: row.get(1)?,
            category: row.get(2)?,
            barcode: row.get(3)?,
            inbound_quantity: row.get(4)?,
            inbound_amount: row.get(5)?,
            outbound_quantity: row.get(6)?,
            outbound_amount: row.get(7)?,
            gift_quantity: row.get(8)?,
            current_stock: row.get(9)?,
            avg_cost: row.get(10)?,
            stock_value: row.get(11)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn movement_date_condition(request: &InventoryReportRequest) -> String {
    let mut conditions = String::new();
    if let Some(start_date) = request
        .start_date
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        conditions.push_str(&format!(
            " AND movement_date >= '{}'",
            escape_sql(start_date)
        ));
    }
    if let Some(end_date) = request.end_date.as_ref().filter(|value| !value.is_empty()) {
        conditions.push_str(&format!(" AND movement_date <= '{}'", escape_sql(end_date)));
    }
    conditions
}

fn document_by_id(conn: &Connection, id: i64) -> anyhow::Result<DocumentDto> {
    conn.query_row(
        "SELECT id, order_id, order_no, customer_id, customer_name, file_path,
                file_type, printed_at, print_count, created_at, COALESCE(status, 'normal')
         FROM documents WHERE id = ?1",
        [id],
        |row| {
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
        },
    )
    .map_err(Into::into)
}

pub fn export_data(state: &AppState, request: ExportDataRequest) -> anyhow::Result<String> {
    let conn = state.connection()?;
    let (title, headers, rows) = export_data_table(&conn, &request)?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let file_path = state
        .exports_dir()
        .join(format!("{}_{}.xlsx", safe_file_name(title), stamp));
    write_table_workbook(&file_path, title, &headers, &rows)?;
    Ok(file_path.to_string_lossy().to_string())
}

fn export_data_table(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    match request.export_type.as_str() {
        "products" => export_products(conn, request),
        "customers" => export_customers(conn, request),
        "inbounds" => export_inbounds(conn, request),
        "inventory_report" => export_inventory_report(conn, request),
        "monthly_credits" => export_monthly_credits(conn, request),
        "profits" => export_profits(conn, request),
        _ => Err(anyhow!("不支持的导出类型：{}", request.export_type)),
    }
}

pub fn list_system_printers() -> anyhow::Result<Vec<String>> {
    if cfg!(target_os = "windows") {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-Command",
            "Get-Printer | Select-Object -ExpandProperty Name",
        ]);
        #[cfg(target_os = "windows")]
        command.creation_flags(0x08000000);
        let output = command.output();
        if let Ok(output) = output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let printers = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                if !printers.is_empty() {
                    return Ok(printers);
                }
            }
        }
    }
    Ok(Vec::new())
}

fn open_or_print_file(file_path: &str, printer_name: Option<&str>) -> anyhow::Result<String> {
    if let Some(printer) = printer_name.filter(|value| !value.trim().is_empty()) {
        if cfg!(target_os = "windows") {
            let mut command = Command::new("powershell");
            command.args([
                "-NoProfile",
                "-Command",
                "Start-Process -FilePath $args[0] -Verb PrintTo -ArgumentList $args[1]",
                file_path,
                printer,
            ]);
            #[cfg(target_os = "windows")]
            command.creation_flags(0x08000000);
            let result = command.output();
            if let Ok(output) = result {
                if output.status.success() {
                    return Ok(format!("已提交到打印机：{printer}"));
                }
            }
        }
        open::that(file_path).with_context(|| "无法打开单据文件，请检查文件是否存在")?;
        return Ok(format!(
            "无法直接提交到打印机：{printer}，已打开文件供手动打印"
        ));
    }

    open::that(file_path).with_context(|| "无法打开单据文件，请检查文件是否存在")?;
    Ok("已打开单据文件，请在关联程序中确认打印".to_string())
}

fn export_products(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let mut sql = String::from(
        "SELECT p.name, p.category, COALESCE(p.barcode, ''), COALESCE(p.unit, ''),
                p.default_price, p.safety_stock, COALESCE(s.current_stock, 0),
                COALESCE(s.avg_cost, 0), COALESCE(s.stock_value, 0), p.is_active
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         WHERE 1 = 1",
    );
    if let Some(category) = request.category.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND p.category = '{}'", escape_sql(category)));
    }
    if let Some(keyword) = request.keyword.as_ref().filter(|value| !value.is_empty()) {
        let keyword = escape_sql(keyword);
        sql.push_str(&format!(
            " AND (p.name LIKE '%{keyword}%' OR p.barcode LIKE '%{keyword}%')"
        ));
    }
    sql.push_str(" ORDER BY p.category, p.name");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                money(row.get::<_, f64>(4)?).to_string(),
                money(row.get::<_, f64>(5)?).to_string(),
                money(row.get::<_, f64>(6)?).to_string(),
                money(row.get::<_, f64>(7)?).to_string(),
                money(row.get::<_, f64>(8)?).to_string(),
                if row.get::<_, i64>(9)? == 1 {
                    "启用"
                } else {
                    "停用"
                }
                .to_string(),
            ])
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        "商品资料",
        vec![
            "商品名称",
            "类别",
            "条码",
            "单位",
            "默认售价",
            "安全库存",
            "当前库存",
            "平均进货价",
            "库存价值",
            "状态",
        ],
        rows,
    ))
}

fn export_customers(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let mut sql = String::from(
        "SELECT COALESCE(region, ''), name, COALESCE(address, ''), COALESCE(phone, ''), is_active
         FROM customers WHERE 1 = 1",
    );
    if let Some(keyword) = request.keyword.as_ref().filter(|value| !value.is_empty()) {
        let keyword = escape_sql(keyword);
        sql.push_str(&format!(
            " AND (name LIKE '%{keyword}%' OR address LIKE '%{keyword}%')"
        ));
    }
    sql.push_str(" ORDER BY region, name");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                if row.get::<_, i64>(4)? == 1 {
                    "启用"
                } else {
                    "停用"
                }
                .to_string(),
            ])
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        "客户资料",
        vec!["地区", "客户名称", "地址", "电话", "状态"],
        rows,
    ))
}

fn export_inbounds(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let mut sql = String::from(
        "SELECT i.inbound_date, p.category, p.name, i.quantity, i.unit_cost, i.amount, COALESCE(i.remark, '')
         FROM inbound_records i
         JOIN products p ON p.id = i.product_id
         WHERE 1 = 1",
    );
    append_date_filters(&mut sql, "i.inbound_date", request);
    if let Some(category) = request.category.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND p.category = '{}'", escape_sql(category)));
    }
    sql.push_str(" ORDER BY i.inbound_date DESC, i.id DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                money(row.get::<_, f64>(3)?).to_string(),
                money(row.get::<_, f64>(4)?).to_string(),
                money(row.get::<_, f64>(5)?).to_string(),
                row.get::<_, String>(6)?,
            ])
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        "入库记录",
        vec!["日期", "类别", "商品", "数量", "进货价", "金额", "备注"],
        rows,
    ))
}

fn export_inventory_report(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    let rows = list_inventory_report(
        conn,
        InventoryReportRequest {
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            category: request.category.clone(),
            keyword: request.keyword.clone(),
        },
    )?
    .into_iter()
    .map(|row| {
        vec![
            row.category,
            row.product_name,
            row.barcode.unwrap_or_default(),
            money(row.inbound_quantity).to_string(),
            money(row.inbound_amount).to_string(),
            money(row.outbound_quantity).to_string(),
            money(row.outbound_amount).to_string(),
            money(row.gift_quantity).to_string(),
            money(row.current_stock).to_string(),
            money(row.avg_cost).to_string(),
            money(row.stock_value).to_string(),
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "进销存报表",
        vec![
            "类别",
            "商品",
            "条码",
            "入库数量",
            "入库金额",
            "销售数量",
            "销售金额",
            "赠品数量",
            "当前库存",
            "平均成本",
            "库存价值",
        ],
        rows,
    ))
}

fn export_monthly_credits(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    let rows = crate::orders::list_monthly_credits(
        conn,
        MonthlyCreditFilterRequest {
            customer_id: request.customer_id,
            category: request.category.clone(),
            status: request.status.clone(),
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            available_month: None,
        },
    )?
    .into_iter()
    .map(|row| {
        vec![
            row.source_order_no,
            row.customer_name,
            row.category,
            money(row.amount).to_string(),
            money(row.used_amount).to_string(),
            money(row.remaining_amount).to_string(),
            row.generated_date,
            row.available_month,
            row.status,
            row.remark.unwrap_or_default(),
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "月费账本",
        vec![
            "来源订单",
            "客户",
            "类别",
            "生成金额",
            "已使用",
            "剩余",
            "生成日期",
            "可用月份",
            "状态",
            "备注",
        ],
        rows,
    ))
}

fn export_profits(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let rows = list_profit_records(
        conn,
        ProfitFilterRequest {
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            customer_id: request.customer_id,
            category: request.category.clone(),
        },
    )?
    .into_iter()
    .map(|row| {
        vec![
            row.order_date,
            row.order_no,
            row.customer_name,
            money(row.totals.product_sales_amount).to_string(),
            money(row.totals.customer_payable_amount).to_string(),
            money(row.totals.direct_discount_amount).to_string(),
            money(row.totals.monthly_credit_used).to_string(),
            money(row.totals.cost_amount).to_string(),
            money(row.totals.profit_amount).to_string(),
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "利润报表",
        vec![
            "日期",
            "单号",
            "客户",
            "销售额",
            "实收",
            "折现",
            "月费抵扣",
            "成本",
            "利润",
        ],
        rows,
    ))
}

fn append_date_filters(sql: &mut String, column: &str, request: &ExportDataRequest) {
    if let Some(start) = request
        .start_date
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        sql.push_str(&format!(" AND {column} >= '{}'", escape_sql(start)));
    }
    if let Some(end) = request.end_date.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND {column} <= '{}'", escape_sql(end)));
    }
}

fn write_table_workbook(
    path: &PathBuf,
    title: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) -> anyhow::Result<()> {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
    sheet.set_name(title);
    for (index, header) in headers.iter().enumerate() {
        let address = cell_address((index + 1) as u32, 1);
        sheet.get_cell_mut(address).set_value(*header);
        let column = column_name((index + 1) as u32);
        sheet.get_column_dimension_mut(&column).set_width(18.0);
    }
    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            let address = cell_address((column_index + 1) as u32, (row_index + 2) as u32);
            sheet.get_cell_mut(address).set_value(value);
        }
    }
    xlsx::write(&book, path)?;
    Ok(())
}

fn write_order_workbook(
    path: &PathBuf,
    detail: &crate::models::OrderDetailDto,
) -> anyhow::Result<()> {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
    sheet.set_name("单据");

    apply_order_template_layout(sheet);
    write_order_template_values(sheet, detail);

    xlsx::write(&book, path)?;
    Ok(())
}

fn apply_order_template_layout(sheet: &mut umya_spreadsheet::Worksheet) {
    for (column, width) in [
        ("A", 9.0),
        ("B", 13.0),
        ("C", 12.082_031_25),
        ("D", 14.25),
        ("E", 8.832_031_25),
        ("F", 8.75),
        ("G", 10.332_031_25),
        ("H", 10.0),
        ("I", 9.0),
        ("J", 13.0),
    ] {
        sheet.get_column_dimension_mut(column).set_width(width);
    }

    for row in 42..=64 {
        sheet.get_row_dimension_mut(&row).set_height(15.0);
    }

    for range in [
        "A42:J44", "B46:C46", "E45:F45", "G45:H45", "I45:J45", "I46:J46", "B47:C47", "I47:J47",
        "B48:C48", "I48:J48", "B49:C49", "I49:J49", "B50:C50", "I50:J50", "B51:C51", "I51:J51",
        "B52:C52", "I52:J52", "B53:C53", "I53:J53", "B54:C54", "I54:J54", "B55:C55", "I55:J55",
        "B56:C56", "I56:J56", "B57:C57", "I57:J57", "B58:C58", "I58:J58", "B59:C59", "I59:J59",
        "B60:C60", "I60:J60", "B61:C61", "I61:J61", "A62:B62", "C62:E62", "G62:H62", "I62:J62",
    ] {
        sheet.add_merge_cells(range);
    }

    sheet
        .get_page_setup_mut()
        .set_orientation(OrientationValues::Portrait)
        .set_paper_size(128);
    sheet
        .get_page_margins_mut()
        .set_left(0.0)
        .set_right(0.0)
        .set_top(0.0)
        .set_bottom(0.0)
        .set_header(0.0)
        .set_footer(0.0);
    let _ = sheet.add_defined_name("_xlnm.Print_Area", "'单据'!$A$42:$J$64");

    for row in 42..=64 {
        for column in 1..=10 {
            let address = cell_address(column, row);
            sheet.set_style(address, template_cell_style(column, row));
        }
    }
}

fn write_order_template_values(
    sheet: &mut umya_spreadsheet::Worksheet,
    detail: &crate::models::OrderDetailDto,
) {
    sheet.get_cell_mut("A42").set_value("科展商行");
    sheet.get_cell_mut("A45").set_value("客户:");
    sheet
        .get_cell_mut("B45")
        .set_value(&detail.order.customer_name);
    sheet.get_cell_mut("D45").set_value("地址：");
    sheet
        .get_cell_mut("E45")
        .set_value(detail.order.customer_address.clone().unwrap_or_default());
    sheet.get_cell_mut("G45").set_value(&detail.order.order_no);
    sheet
        .get_cell_mut("I45")
        .set_value(&detail.order.order_date);

    for (address, value) in [
        ("A46", "序号"),
        ("B46", "条码"),
        ("D46", "商品名称"),
        ("E46", "单位"),
        ("F46", "数量"),
        ("G46", "价格"),
        ("H46", "总价格"),
        ("I46", "备注"),
    ] {
        sheet.get_cell_mut(address).set_value(value);
    }

    for (index, item) in detail.items.iter().take(15).enumerate() {
        let row = 47 + index as u32;
        sheet
            .get_cell_mut(format!("A{row}"))
            .set_value_number((index + 1) as f64);
        sheet
            .get_cell_mut(format!("B{row}"))
            .set_value(item.barcode.clone().unwrap_or_default());
        sheet.get_cell_mut(format!("D{row}")).set_value(
            item.product_name
                .clone()
                .unwrap_or_else(|| item.line_type.clone()),
        );
        sheet.get_cell_mut(format!("E{row}")).set_value("件");
        sheet
            .get_cell_mut(format!("F{row}"))
            .set_value_number(item.quantity);
        sheet
            .get_cell_mut(format!("G{row}"))
            .set_value_number(item.unit_price);
        sheet
            .get_cell_mut(format!("H{row}"))
            .set_value_number(item.amount);
        sheet
            .get_cell_mut(format!("I{row}"))
            .set_value(item.remark.clone().unwrap_or_default());
    }

    let quantity_total = detail
        .items
        .iter()
        .take(15)
        .map(|item| item.quantity)
        .sum::<f64>();
    let amount_total = detail
        .items
        .iter()
        .take(15)
        .map(|item| item.amount)
        .sum::<f64>();

    sheet.get_cell_mut("A62").set_value("总金额");
    sheet
        .get_cell_mut("C62")
        .set_value_number(money(detail.order.totals.customer_payable_amount));
    sheet.get_cell_mut("F62").set_value_number(quantity_total);
    sheet.get_cell_mut("G62").set_value_number(amount_total);

    if let Some(remark) = &detail.order.remark {
        sheet.get_cell_mut("A64").set_value(remark);
    }
}

fn template_cell_style(column: u32, row: u32) -> Style {
    let mut style = base_template_style();

    if row == 42 {
        style.get_font_mut().get_font_name_mut().set_val("黑体");
        style.get_font_mut().get_font_size_mut().set_val(20.0);
        style.get_font_mut().set_bold(true);
    }

    if row == 45 && (column == 1 || column == 4) {
        style
            .get_alignment_mut()
            .set_horizontal(HorizontalAlignmentValues::Right);
    }

    if (47..=61).contains(&row) && (column == 7 || column == 8) {
        style
            .get_numbering_format_mut()
            .set_format_code("\\¥#,##0.00;[Red]\\¥\\-#,##0.00");
    }
    if row == 62 && column == 3 {
        style
            .get_numbering_format_mut()
            .set_format_code("[DBNum2][$RMB]General;[Red][DBNum2][$RMB]General");
    }
    if row == 62 && column == 7 {
        style
            .get_numbering_format_mut()
            .set_format_code("\\¥#,##0.00_);[Red]\\(\\¥#,##0.00\\)");
    }
    if row == 45 && column == 9 {
        style.get_numbering_format_mut().set_format_code("mm-dd-yy");
    }

    apply_template_borders(&mut style, column, row);
    style
}

fn base_template_style() -> Style {
    let mut style = Style::default();
    style.get_font_mut().get_font_name_mut().set_val("等线");
    style.get_font_mut().get_font_size_mut().set_val(11.0);
    style
        .get_alignment_mut()
        .set_horizontal(HorizontalAlignmentValues::Center);
    style
        .get_alignment_mut()
        .set_vertical(VerticalAlignmentValues::Center);
    style
}

fn apply_template_borders(style: &mut Style, column: u32, row: u32) {
    if row == 42 {
        set_bottom_border(style);
        return;
    }

    match row {
        45 => {
            if matches!(column, 1 | 4 | 7 | 9) {
                set_left_border(style);
            }
            if matches!(column, 1 | 5 | 6 | 7 | 8 | 9 | 10) {
                set_top_border(style);
            }
            if matches!(column, 5..=10) {
                set_right_border(style);
            }
            set_bottom_border(style);
        }
        46..=61 => set_all_borders(style),
        62 => {
            if matches!(column, 1 | 3 | 6 | 7 | 9) {
                set_left_border(style);
            }
            if matches!(column, 3 | 5 | 6 | 8 | 9 | 10) {
                set_right_border(style);
            }
            if column <= 10 {
                set_top_border(style);
            }
            if matches!(column, 1 | 2 | 3 | 4 | 5 | 9 | 10) {
                set_bottom_border(style);
            }
        }
        _ => {}
    }
}

fn set_all_borders(style: &mut Style) {
    set_left_border(style);
    set_right_border(style);
    set_top_border(style);
    set_bottom_border(style);
}

fn set_left_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_left_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn set_right_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_right_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn set_top_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_top_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn set_bottom_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_bottom_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn cell_address(column: u32, row: u32) -> String {
    format!("{}{}", column_name(column), row)
}

fn column_name(mut column: u32) -> String {
    let mut name = String::new();
    while column > 0 {
        let rem = ((column - 1) % 26) as u8;
        name.insert(0, (b'A' + rem) as char);
        column = (column - 1) / 26;
    }
    name
}

fn escape_sql(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::models::{
        OrderDetailDto, OrderDto, OrderItemDto, OrderTotalsDto, ProfitAnalyticsRequest,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn order_workbook_matches_source_print_template() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("order.xlsx");
        let detail = sample_order_detail();

        write_order_workbook(&path, &detail).unwrap();

        let book = umya_spreadsheet::reader::xlsx::read(&path).unwrap();
        let sheet = book.get_sheet_by_name("单据").unwrap();
        let merge_ranges = sheet
            .get_merge_cells()
            .iter()
            .map(|range| range.get_range())
            .collect::<Vec<_>>();

        assert!(merge_ranges.contains(&"A42:J44".to_string()));
        assert!(merge_ranges.contains(&"B47:C47".to_string()));
        assert!(merge_ranges.contains(&"C62:E62".to_string()));
        assert_eq!(*sheet.get_page_setup().get_paper_size(), 128);
        assert!(matches!(
            sheet.get_page_setup().get_orientation(),
            OrientationValues::Portrait
        ));
        assert_eq!(sheet.get_value("A42"), "科展商行");
        assert_eq!(sheet.get_value("D46"), "商品名称");
    }

    #[test]
    fn list_profit_records_filters_by_order_item_category() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_order(&conn, "20260601001", "饮料");
        insert_profit_order(&conn, "20260601002", "零食");

        let rows = list_profit_records(
            &conn,
            ProfitFilterRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-01".to_string()),
                customer_id: None,
                category: Some("饮料".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].order_no, "20260601001");
    }

    #[test]
    fn profit_analytics_groups_daily_monthly_and_yearly() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_analytics_order(
            &conn,
            "20250531001",
            "2025-05-31",
            1,
            "客户A",
            &[("饮料", 12.0, 5.0, 7.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260601001",
            "2026-06-01",
            1,
            "客户A",
            &[("饮料", 10.0, 6.0, 4.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260602001",
            "2026-06-02",
            2,
            "客户B",
            &[("零食", 30.0, 10.0, 20.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260602002",
            "2026-06-02",
            1,
            "客户A",
            &[("饮料", 99.0, 20.0, 79.0)],
            "voided",
        );

        let daily = get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "day".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();

        assert_eq!(daily.summary.order_count, 2);
        assert_eq!(daily.summary.product_sales_amount, 40.0);
        assert_eq!(daily.summary.cost_amount, 16.0);
        assert_eq!(daily.summary.profit_amount, 24.0);
        assert_eq!(daily.trend.len(), 2);
        assert_eq!(daily.trend[0].period, "2026-06-01");
        assert_eq!(daily.trend[0].profit_amount, 4.0);
        assert_eq!(daily.trend[1].period, "2026-06-02");
        assert_eq!(daily.trend[1].profit_amount, 20.0);

        let monthly = get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "month".to_string(),
                start_date: "2026-01-01".to_string(),
                end_date: "2026-12-31".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();
        assert_eq!(monthly.trend.len(), 1);
        assert_eq!(monthly.trend[0].period, "2026-06");
        assert_eq!(monthly.trend[0].profit_amount, 24.0);

        let yearly = get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "year".to_string(),
                start_date: "2025-01-01".to_string(),
                end_date: "2026-12-31".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();
        assert_eq!(yearly.trend.len(), 2);
        assert_eq!(yearly.trend[0].period, "2025");
        assert_eq!(yearly.trend[0].profit_amount, 7.0);
        assert_eq!(yearly.trend[1].period, "2026");
        assert_eq!(yearly.trend[1].profit_amount, 24.0);
    }

    #[test]
    fn profit_analytics_breaks_down_categories_from_order_items() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_analytics_order(
            &conn,
            "20260601001",
            "2026-06-01",
            1,
            "客户A",
            &[("饮料", 10.0, 6.0, 4.0), ("零食", 30.0, 10.0, 20.0)],
            "normal",
        );

        let analytics = get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "day".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();

        assert_eq!(analytics.category_breakdown.len(), 2);
        assert_eq!(analytics.category_breakdown[0].name, "零食");
        assert_eq!(analytics.category_breakdown[0].product_sales_amount, 30.0);
        assert_eq!(analytics.category_breakdown[0].profit_amount, 20.0);
        assert_eq!(analytics.category_breakdown[1].name, "饮料");
        assert_eq!(analytics.category_breakdown[1].product_sales_amount, 10.0);
        assert_eq!(analytics.category_breakdown[1].profit_amount, 4.0);
    }

    #[test]
    fn profit_analytics_filters_by_customer_and_category() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_analytics_order(
            &conn,
            "20260601001",
            "2026-06-01",
            1,
            "客户A",
            &[("饮料", 10.0, 6.0, 4.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260601002",
            "2026-06-01",
            2,
            "客户B",
            &[("饮料", 20.0, 7.0, 13.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260601003",
            "2026-06-01",
            1,
            "客户A",
            &[("零食", 30.0, 10.0, 20.0)],
            "normal",
        );

        let analytics = get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "day".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                customer_id: Some(1),
                category: Some("饮料".to_string()),
            },
        )
        .unwrap();

        assert_eq!(analytics.summary.order_count, 1);
        assert_eq!(analytics.summary.product_sales_amount, 10.0);
        assert_eq!(analytics.customer_breakdown.len(), 1);
        assert_eq!(analytics.customer_breakdown[0].name, "客户A");
        assert_eq!(analytics.customer_breakdown[0].profit_amount, 4.0);
    }

    #[test]
    fn list_documents_filters_by_customer_and_status() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('单据', '客户A', 1, ?1, ?1), ('单据', '客户B', 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES
             ('20260601001', '2026-06-01', 1, '客户A', 10, 'normal', ?1, ?1),
             ('20260602001', '2026-06-02', 1, '客户A', 20, 'voided', ?1, ?1),
             ('20260603001', '2026-06-03', 2, '客户B', 30, 'normal', ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO documents
             (order_id, order_no, customer_id, customer_name, file_path, file_type, print_count, status, created_at)
             VALUES
             (1, '20260601001', 1, '客户A', 'a.xlsx', 'xlsx', 0, 'normal', ?1),
             (2, '20260602001', 1, '客户A', 'b.xlsx', 'xlsx', 1, 'voided', ?1),
             (3, '20260603001', 2, '客户B', 'c.xlsx', 'xlsx', 0, 'normal', ?1)",
            rusqlite::params![now],
        )
        .unwrap();

        let rows = list_documents(
            &conn,
            DocumentFilterRequest {
                customer_id: Some(1),
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                order_no: Some("202606".to_string()),
                printed: Some(false),
                status: Some("normal".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].order_no, "20260601001");
        assert_eq!(rows[0].status, "normal");
    }

    #[test]
    fn export_data_rejects_unknown_export_type() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let result = export_data_table(
            &conn,
            &ExportDataRequest {
                export_type: "unknown".to_string(),
                start_date: None,
                end_date: None,
                customer_id: None,
                category: None,
                status: None,
                keyword: None,
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不支持的导出类型"));
    }

    #[test]
    fn inventory_report_summarizes_purchase_sales_and_gifts() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('报表商品', '报表类', 'RPT001', 10, 0, 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES
             ('2026-06-01', 1, 'inbound', 20, 5, 100, 'test', '入库', ?1),
             ('2026-06-02', 1, 'outbound', 6, 10, 60, 'test', '销售', ?1),
             ('2026-06-02', 1, 'gift_outbound', 2, 0, 0, 'test', '赠品', ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        db::recalc_stock_balance(&conn, 1).unwrap();

        let rows = list_inventory_report(
            &conn,
            InventoryReportRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: Some("报表类".to_string()),
                keyword: Some("RPT001".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].inbound_quantity, 20.0);
        assert_eq!(rows[0].inbound_amount, 100.0);
        assert_eq!(rows[0].outbound_quantity, 6.0);
        assert_eq!(rows[0].outbound_amount, 60.0);
        assert_eq!(rows[0].gift_quantity, 2.0);
        assert_eq!(rows[0].current_stock, 12.0);
    }

    #[test]
    fn high_volume_inventory_report_stays_under_two_seconds() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let tx = conn.transaction().unwrap();
        let now = now_text();
        for index in 0..10_000 {
            tx.execute(
                "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 10, 0, 1, ?4, ?4)",
                rusqlite::params![
                    format!("报表商品{index:05}"),
                    format!("类别{}", index % 20),
                    format!("RPT{index:05}"),
                    now
                ],
            )
            .unwrap();
            let product_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
                 VALUES (?1, 8, 5, 40, ?2)",
                rusqlite::params![product_id, now],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO inventory_movements
                 (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
                 VALUES
                 ('2026-06-01', ?1, 'inbound', 10, 5, 50, 'test', '入库', ?2),
                 ('2026-06-02', ?1, 'outbound', 2, 10, 20, 'test', '出库', ?2),
                 ('2026-06-02', ?1, 'gift_outbound', 1, 0, 0, 'test', '赠品', ?2)",
                rusqlite::params![product_id, now],
            )
            .unwrap();
        }
        tx.commit().unwrap();

        let started = Instant::now();
        let rows = list_inventory_report(
            &conn,
            InventoryReportRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: None,
                keyword: None,
            },
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert_eq!(rows.len(), 10_000);
        assert_eq!(rows[0].inbound_quantity, 10.0);
        assert_eq!(rows[0].outbound_quantity, 2.0);
        assert_eq!(rows[0].gift_quantity, 1.0);
        assert!(
            elapsed < Duration::from_secs(2),
            "万级进销存报表查询耗时 {:?}",
            elapsed
        );
    }

    fn insert_profit_order(conn: &Connection, order_no: &str, category: &str) {
        let now = now_text();
        conn.execute(
            "INSERT OR IGNORE INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES (1, '测试', '测试客户', 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES (?1, ?2, 10, 0, 1, ?3, ?3)",
            rusqlite::params![format!("{category}商品"), category, now],
        )
        .unwrap();
        let product_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount,
              customer_payable_amount, profit_amount, status, created_at, updated_at)
             VALUES (?1, '2026-06-01', 1, '测试客户', 10, 10, 4, 'normal', ?2, ?2)",
            rusqlite::params![order_no, now],
        )
        .unwrap();
        let order_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO order_items
             (order_id, line_type, product_id, product_name, category, quantity, unit_price,
              amount, avg_cost, cost_amount, profit_amount, sort_order)
             VALUES (?1, 'normal', ?2, ?3, ?4, 1, 10, 10, 6, 6, 4, 1)",
            rusqlite::params![order_id, product_id, format!("{category}商品"), category],
        )
        .unwrap();
    }

    fn insert_profit_analytics_order(
        conn: &Connection,
        order_no: &str,
        order_date: &str,
        customer_id: i64,
        customer_name: &str,
        items: &[(&str, f64, f64, f64)],
        status: &str,
    ) {
        let now = now_text();
        conn.execute(
            "INSERT OR IGNORE INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES (?1, '统计', ?2, 1, ?3, ?3)",
            rusqlite::params![customer_id, customer_name, now],
        )
        .unwrap();
        let sales_amount = items.iter().map(|item| item.1).sum::<f64>();
        let cost_amount = items.iter().map(|item| item.2).sum::<f64>();
        let profit_amount = items.iter().map(|item| item.3).sum::<f64>();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount,
              customer_payable_amount, cost_amount, profit_amount, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7, ?8, ?9, ?9)",
            rusqlite::params![
                order_no,
                order_date,
                customer_id,
                customer_name,
                sales_amount,
                cost_amount,
                profit_amount,
                status,
                now
            ],
        )
        .unwrap();
        let order_id = conn.last_insert_rowid();
        for (index, (category, amount, cost, profit)) in items.iter().enumerate() {
            conn.execute(
                "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 0, 1, ?4, ?4)",
                rusqlite::params![format!("{category}商品{order_no}{index}"), category, amount, now],
            )
            .unwrap();
            let product_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO order_items
                 (order_id, line_type, product_id, product_name, category, quantity, unit_price,
                  amount, avg_cost, cost_amount, profit_amount, sort_order)
                 VALUES (?1, 'normal', ?2, ?3, ?4, 1, ?5, ?5, ?6, ?6, ?7, ?8)",
                rusqlite::params![
                    order_id,
                    product_id,
                    format!("{category}商品{order_no}{index}"),
                    category,
                    amount,
                    cost,
                    profit,
                    index as i64 + 1
                ],
            )
            .unwrap();
        }
    }

    fn sample_order_detail() -> OrderDetailDto {
        OrderDetailDto {
            order: OrderDto {
                id: 1,
                order_no: "20260530001".to_string(),
                order_date: "2026-05-30".to_string(),
                customer_id: 1,
                customer_name: "测试客户".to_string(),
                customer_address: Some("测试地址".to_string()),
                totals: OrderTotalsDto {
                    product_sales_amount: 10.0,
                    customer_payable_amount: 10.0,
                    ..OrderTotalsDto::default()
                },
                remark: Some("测试备注".to_string()),
                document_path: None,
                print_count: 0,
                status: "normal".to_string(),
            },
            items: vec![OrderItemDto {
                id: 1,
                line_type: "normal".to_string(),
                product_id: Some(1),
                product_name: Some("测试商品".to_string()),
                category: Some("测试类别".to_string()),
                barcode: Some("123456".to_string()),
                quantity: 2.0,
                unit_price: 5.0,
                amount: 10.0,
                avg_cost: 3.0,
                cost_amount: 6.0,
                profit_amount: 4.0,
                rule_id: None,
                monthly_credit_id: None,
                remark: Some("行备注".to_string()),
                sort_order: 1,
            }],
        }
    }
}
