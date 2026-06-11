use crate::logger;
use crate::models::{
    DataSelfCheckDto, DataSelfCheckIssueDto, DiagnosticPackageDto, DiagnosticSummaryDto,
};
use crate::utils::{money, now_text};
use rusqlite::OptionalExtension;
use std::path::Path;

pub fn run_data_self_check<F>(
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

pub fn write_self_check_export(path: &Path, check: &DataSelfCheckDto) -> anyhow::Result<()> {
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
            logger::redact_sensitive_text(&issue.target_label),
            issue.message,
            issue
                .details
                .as_deref()
                .map(logger::redact_sensitive_text)
                .unwrap_or_default()
        ));
    }
    std::fs::write(path, text)?;
    Ok(())
}

pub fn diagnostic_summary(
    conn: &rusqlite::Connection,
    database_path: &Path,
    logs_dir: &Path,
    backups_dir: &Path,
    exports_dir: &Path,
    version: &str,
) -> anyhow::Result<DiagnosticSummaryDto> {
    Ok(DiagnosticSummaryDto {
        generated_at: now_text(),
        database_path: path_hint(database_path),
        logs_dir: path_hint(logs_dir),
        backups_dir: path_hint(backups_dir),
        exports_dir: path_hint(exports_dir),
        version: version.to_string(),
        database_size: std::fs::metadata(database_path)
            .map(|metadata| metadata.len() as i64)
            .unwrap_or(0),
        backup_count: count_query(conn, "SELECT COUNT(*) FROM backup_logs")?,
        latest_backup_at: conn
            .query_row(
                "SELECT MAX(created_at) FROM backup_logs WHERE status = 'success'",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten(),
        product_count: count_query(conn, "SELECT COUNT(*) FROM products")?,
        customer_count: count_query(conn, "SELECT COUNT(*) FROM customers")?,
        order_count: count_query(conn, "SELECT COUNT(*) FROM orders")?,
        document_count: count_query(conn, "SELECT COUNT(*) FROM documents")?,
        setting_count: count_query(conn, "SELECT COUNT(*) FROM settings")?,
        latest_logs: latest_log_lines(logs_dir, 40)?,
    })
}

pub fn export_diagnostic_package(
    conn: &rusqlite::Connection,
    database_path: &Path,
    logs_dir: &Path,
    backups_dir: &Path,
    exports_dir: &Path,
    version: &str,
) -> anyhow::Result<DiagnosticPackageDto> {
    let summary = diagnostic_summary(
        conn,
        database_path,
        logs_dir,
        backups_dir,
        exports_dir,
        version,
    )?;
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
        text.push_str(&format!("{key}={}\n", redact_setting_value(&key, &value)));
    }
    text.push_str("\n最近日志：\n");
    for line in &summary.latest_logs {
        text.push_str(line);
        text.push('\n');
    }

    let path = exports_dir.join(format!(
        "diagnostic_package_{}.txt",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    std::fs::write(&path, text)?;
    Ok(DiagnosticPackageDto {
        file_path: path.to_string_lossy().to_string(),
        message: "诊断包已导出，日志、设置和路径信息已尽量脱敏".to_string(),
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
    let mut lines = content
        .lines()
        .map(logger::redact_sensitive_text)
        .collect::<Vec<_>>();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    Ok(lines)
}

fn path_hint(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| logger::final_path_component(&path.to_string_lossy()))
}

fn redact_setting_value(key: &str, value: &str) -> String {
    let line = logger::redact_sensitive_text(&format!("{key}={value}"));
    line.split_once('=')
        .map(|(_, redacted)| redacted.to_string())
        .unwrap_or_else(|| logger::redact_sensitive_text(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::Connection;

    #[test]
    fn diagnostic_package_redacts_sensitive_settings_paths_and_logs() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("inventory.db");
        let logs_dir = dir.path().join("logs");
        let backups_dir = dir.path().join("backups");
        let exports_dir = dir.path().join("exports");
        std::fs::create_dir_all(&logs_dir).unwrap();
        std::fs::create_dir_all(&backups_dir).unwrap();
        std::fs::create_dir_all(&exports_dir).unwrap();

        let conn = Connection::open(&db_path).unwrap();
        db::init_schema(&conn).unwrap();
        db::seed_settings(&conn).unwrap();
        db::set_setting(&conn, "merchant_phone", "13800000000").unwrap();
        db::set_setting(&conn, "merchant_address", "广东省深圳市南山区科技园").unwrap();
        db::set_setting(&conn, "merchant_logo_path", "C:/Users/ww/Desktop/logo.png").unwrap();
        std::fs::write(
            logs_dir.join("easyinventory_20260611.log"),
            "phone=13800000000 address=广东省深圳市南山区 filePath=C:/Users/ww/Desktop/orders/a.xlsx",
        )
        .unwrap();

        let package = export_diagnostic_package(
            &conn,
            &db_path,
            &logs_dir,
            &backups_dir,
            &exports_dir,
            "1.3.2",
        )
        .unwrap();
        let content = std::fs::read_to_string(package.file_path).unwrap();

        assert!(content.contains("merchant_phone=138***00"));
        assert!(content.contains("merchant_address=广东省深***"));
        assert!(content.contains("merchant_logo_path=logo.png"));
        assert!(content.contains("phone=138***00"));
        assert!(!content.contains("13800000000"));
        assert!(!content.contains("C:/Users/ww/Desktop"));
    }
}
