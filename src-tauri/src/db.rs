use crate::app::AppState;
use crate::models::*;
use crate::utils::{money, now_text, today_text};
use anyhow::{anyhow, Context};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;

pub const GUEST_CUSTOMER_NAME: &str = "散客";
const GUEST_CUSTOMER_REGION: &str = "散客";

pub fn init_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS products (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          category TEXT NOT NULL,
          barcode TEXT,
          default_price REAL DEFAULT 0,
          safety_stock REAL DEFAULT 0,
          unit TEXT,
          is_active INTEGER DEFAULT 1,
          remark TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_products_name ON products(name);
        CREATE INDEX IF NOT EXISTS idx_products_category ON products(category);
        CREATE INDEX IF NOT EXISTS idx_products_barcode ON products(barcode);

        CREATE TABLE IF NOT EXISTS customers (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          region TEXT,
          name TEXT NOT NULL,
          address TEXT,
          phone TEXT,
          is_active INTEGER DEFAULT 1,
          remark TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_customers_region ON customers(region);
        CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(name);

        CREATE TABLE IF NOT EXISTS suppliers (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          contact TEXT,
          phone TEXT,
          address TEXT,
          is_active INTEGER DEFAULT 1,
          remark TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_suppliers_name ON suppliers(name);
        CREATE INDEX IF NOT EXISTS idx_suppliers_active ON suppliers(is_active);

        CREATE TABLE IF NOT EXISTS inventory_movements (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          movement_date TEXT NOT NULL,
          product_id INTEGER NOT NULL,
          movement_type TEXT NOT NULL,
          quantity REAL NOT NULL,
          unit_price REAL DEFAULT 0,
          amount REAL DEFAULT 0,
          source_type TEXT,
          source_id INTEGER,
          source_no TEXT,
          remark TEXT,
          created_at TEXT NOT NULL,
          FOREIGN KEY(product_id) REFERENCES products(id)
        );
        CREATE INDEX IF NOT EXISTS idx_movements_product_date ON inventory_movements(product_id, movement_date);
        CREATE INDEX IF NOT EXISTS idx_movements_type ON inventory_movements(movement_type);

        CREATE TABLE IF NOT EXISTS stock_balances (
          product_id INTEGER PRIMARY KEY,
          current_stock REAL DEFAULT 0,
          avg_cost REAL DEFAULT 0,
          stock_value REAL DEFAULT 0,
          updated_at TEXT NOT NULL,
          FOREIGN KEY(product_id) REFERENCES products(id)
        );

        CREATE TABLE IF NOT EXISTS inbound_records (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          inbound_date TEXT NOT NULL,
          product_id INTEGER NOT NULL,
          supplier_id INTEGER,
          supplier_name TEXT,
          quantity REAL NOT NULL,
          unit_cost REAL NOT NULL,
          amount REAL NOT NULL,
          remark TEXT,
          created_at TEXT NOT NULL,
          FOREIGN KEY(product_id) REFERENCES products(id),
          FOREIGN KEY(supplier_id) REFERENCES suppliers(id)
        );
        CREATE INDEX IF NOT EXISTS idx_inbound_supplier ON inbound_records(supplier_id);

        CREATE TABLE IF NOT EXISTS orders (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          order_no TEXT NOT NULL UNIQUE,
          order_date TEXT NOT NULL,
          customer_id INTEGER NOT NULL,
          customer_name TEXT NOT NULL,
          customer_address TEXT,
          product_sales_amount REAL DEFAULT 0,
          direct_discount_amount REAL DEFAULT 0,
          monthly_credit_used REAL DEFAULT 0,
          customer_payable_amount REAL DEFAULT 0,
          brand_subsidy_amount REAL DEFAULT 0,
          cost_amount REAL DEFAULT 0,
          gift_cost_amount REAL DEFAULT 0,
          profit_amount REAL DEFAULT 0,
          remark TEXT,
          document_path TEXT,
          print_count INTEGER DEFAULT 0,
          status TEXT DEFAULT 'normal',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY(customer_id) REFERENCES customers(id)
        );
        CREATE INDEX IF NOT EXISTS idx_orders_date ON orders(order_date);
        CREATE INDEX IF NOT EXISTS idx_orders_customer ON orders(customer_id);

        CREATE TABLE IF NOT EXISTS order_items (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          order_id INTEGER NOT NULL,
          line_type TEXT NOT NULL,
          product_id INTEGER,
          product_name TEXT,
          category TEXT,
          barcode TEXT,
          quantity REAL DEFAULT 0,
          unit_price REAL DEFAULT 0,
          amount REAL DEFAULT 0,
          avg_cost REAL DEFAULT 0,
          cost_amount REAL DEFAULT 0,
          profit_amount REAL DEFAULT 0,
          related_product_id INTEGER,
          rule_id INTEGER,
          monthly_credit_id INTEGER,
          remark TEXT,
          sort_order INTEGER DEFAULT 0,
          FOREIGN KEY(order_id) REFERENCES orders(id),
          FOREIGN KEY(product_id) REFERENCES products(id)
        );
        CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);

        CREATE TABLE IF NOT EXISTS customer_product_rules (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          customer_id INTEGER NOT NULL,
          product_id INTEGER NOT NULL,
          fixed_price REAL,
          threshold_quantity REAL,
          gift_product_id INTEGER,
          gift_quantity REAL,
          direct_discount_amount REAL,
          monthly_credit_amount REAL,
          credit_category TEXT,
          is_active INTEGER DEFAULT 1,
          remark TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY(customer_id) REFERENCES customers(id),
          FOREIGN KEY(product_id) REFERENCES products(id),
          FOREIGN KEY(gift_product_id) REFERENCES products(id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_active_rule_customer_product
        ON customer_product_rules(customer_id, product_id)
        WHERE is_active = 1;

        CREATE TABLE IF NOT EXISTS monthly_credits (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          source_order_id INTEGER NOT NULL,
          source_order_no TEXT NOT NULL,
          customer_id INTEGER NOT NULL,
          category TEXT NOT NULL,
          amount REAL NOT NULL,
          used_amount REAL DEFAULT 0,
          remaining_amount REAL NOT NULL,
          generated_date TEXT NOT NULL,
          available_month TEXT NOT NULL,
          status TEXT DEFAULT 'pending',
          remark TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY(source_order_id) REFERENCES orders(id),
          FOREIGN KEY(customer_id) REFERENCES customers(id)
        );
        CREATE INDEX IF NOT EXISTS idx_monthly_credits_customer ON monthly_credits(customer_id);
        CREATE INDEX IF NOT EXISTS idx_monthly_credits_status ON monthly_credits(status);

        CREATE TABLE IF NOT EXISTS payment_records (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          payment_date TEXT NOT NULL,
          customer_id INTEGER NOT NULL,
          amount REAL NOT NULL,
          method TEXT,
          related_order_id INTEGER,
          status TEXT DEFAULT 'normal',
          remark TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY(customer_id) REFERENCES customers(id),
          FOREIGN KEY(related_order_id) REFERENCES orders(id)
        );
        CREATE INDEX IF NOT EXISTS idx_payments_customer ON payment_records(customer_id);
        CREATE INDEX IF NOT EXISTS idx_payments_date ON payment_records(payment_date);
        CREATE INDEX IF NOT EXISTS idx_payments_status ON payment_records(status);

        CREATE TABLE IF NOT EXISTS documents (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          order_id INTEGER NOT NULL,
          order_no TEXT NOT NULL,
          customer_id INTEGER NOT NULL,
          customer_name TEXT NOT NULL,
          file_path TEXT NOT NULL,
          file_type TEXT DEFAULT 'xlsx',
          printed_at TEXT,
          print_count INTEGER DEFAULT 0,
          created_at TEXT NOT NULL,
          FOREIGN KEY(order_id) REFERENCES orders(id),
          FOREIGN KEY(customer_id) REFERENCES customers(id)
        );
        CREATE INDEX IF NOT EXISTS idx_documents_order ON documents(order_id);
        CREATE INDEX IF NOT EXISTS idx_documents_customer ON documents(customer_id);

        CREATE TABLE IF NOT EXISTS settings (
          key TEXT PRIMARY KEY,
          value TEXT,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS backup_logs (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          backup_path TEXT NOT NULL,
          backup_type TEXT NOT NULL,
          status TEXT NOT NULL,
          message TEXT,
          created_at TEXT NOT NULL
        );
        "#,
    )?;
    ensure_compatible_schema(conn)?;
    Ok(())
}

fn ensure_compatible_schema(conn: &Connection) -> anyhow::Result<()> {
    let has_document_status: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('documents') WHERE name = 'status'",
        [],
        |row| row.get(0),
    )?;
    if !has_document_status {
        conn.execute(
            "ALTER TABLE documents ADD COLUMN status TEXT DEFAULT 'normal'",
            [],
        )?;
    }
    ensure_column(conn, "inbound_records", "supplier_id", "INTEGER")?;
    ensure_column(conn, "inbound_records", "supplier_name", "TEXT")?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let sql = format!(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('{}') WHERE name = ?1",
        table.replace('\'', "''")
    );
    let exists: bool = conn.query_row(&sql, [column], |row| row.get(0))?;
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

pub fn seed_settings(conn: &Connection) -> anyhow::Result<()> {
    let now = now_text();
    let defaults = [
        ("allow_negative_stock", "false"),
        ("daily_auto_backup", "true"),
        ("default_export_format", "xlsx"),
        ("default_print_template", "excel"),
        ("default_printer", ""),
        ("last_auto_backup_date", ""),
    ];
    for (key, value) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
    }
    Ok(())
}

pub fn ensure_guest_customer(conn: &Connection) -> anyhow::Result<i64> {
    let now = now_text();
    let existing_id = conn
        .query_row(
            "SELECT id FROM customers WHERE name = ?1 ORDER BY is_active DESC, id LIMIT 1",
            [GUEST_CUSTOMER_NAME],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;

    if let Some(id) = existing_id {
        conn.execute(
            "UPDATE customers
             SET region = COALESCE(NULLIF(region, ''), ?1), is_active = 1, updated_at = ?2
             WHERE id = ?3",
            params![GUEST_CUSTOMER_REGION, now, id],
        )?;
        return Ok(id);
    }

    conn.execute(
        "INSERT INTO customers (region, name, address, phone, is_active, remark, created_at, updated_at)
         VALUES (?1, ?2, NULL, NULL, 1, ?3, ?4, ?4)",
        params![GUEST_CUSTOMER_REGION, GUEST_CUSTOMER_NAME, "系统默认客户", now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn is_guest_customer(conn: &Connection, id: i64) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT name = ?1 FROM customers WHERE id = ?2",
        params![GUEST_CUSTOMER_NAME, id],
        |row| row.get::<_, bool>(0),
    )
    .optional()
    .map(|value| value.unwrap_or(false))
    .map_err(Into::into)
}

pub fn setting(conn: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
    .map_err(Into::into)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now_text()],
    )?;
    Ok(())
}

pub fn backup_on_startup_if_needed(state: &AppState) -> anyhow::Result<()> {
    let conn = state.connection()?;
    let enabled = setting(&conn, "daily_auto_backup")?.unwrap_or_else(|| "true".to_string());
    if enabled != "true" {
        return Ok(());
    }
    let today = today_text();
    let last = setting(&conn, "last_auto_backup_date")?.unwrap_or_default();
    if last == today {
        return Ok(());
    }
    drop(conn);
    create_backup_file(state, "auto")?;
    let conn = state.connection()?;
    set_setting(&conn, "last_auto_backup_date", &today)?;
    Ok(())
}

pub fn create_backup_file(state: &AppState, backup_type: &str) -> anyhow::Result<String> {
    let db_path = state.db_path();
    if !db_path.exists() {
        return Err(anyhow!("数据库文件不存在"));
    }
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = state.backups_dir().join(format!("inventory_{stamp}.db"));
    fs::copy(&db_path, &backup_path).with_context(|| "复制数据库备份失败")?;
    let path_text = backup_path.to_string_lossy().to_string();
    state.log_backup(&path_text, backup_type, "success", None)?;
    Ok(path_text)
}

pub fn recalc_stock_balance(conn: &Connection, product_id: i64) -> anyhow::Result<(f64, f64)> {
    let inbound: (f64, f64) = conn.query_row(
        "SELECT COALESCE(SUM(quantity), 0), COALESCE(SUM(amount), 0)
         FROM inventory_movements
         WHERE product_id = ?1 AND movement_type = 'inbound'",
        [product_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let outbound_qty: f64 = conn.query_row(
        "SELECT COALESCE(SUM(quantity), 0)
         FROM inventory_movements
         WHERE product_id = ?1 AND movement_type IN ('outbound', 'gift_outbound')",
        [product_id],
        |row| row.get(0),
    )?;
    let adjustment_qty: f64 = conn.query_row(
        "SELECT COALESCE(SUM(quantity), 0)
         FROM inventory_movements
         WHERE product_id = ?1 AND movement_type = 'stocktake_adjustment'",
        [product_id],
        |row| row.get(0),
    )?;
    let current_stock = money(inbound.0 - outbound_qty + adjustment_qty);
    let avg_cost = if inbound.0.abs() > f64::EPSILON {
        money(inbound.1 / inbound.0)
    } else {
        0.0
    };
    let stock_value = money(current_stock * avg_cost);
    conn.execute(
        "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(product_id) DO UPDATE SET
           current_stock = excluded.current_stock,
           avg_cost = excluded.avg_cost,
           stock_value = excluded.stock_value,
           updated_at = excluded.updated_at",
        params![product_id, current_stock, avg_cost, stock_value, now_text()],
    )?;
    Ok((current_stock, avg_cost))
}

pub fn product_by_id(conn: &Connection, product_id: i64) -> anyhow::Result<ProductDto> {
    conn.query_row(
        "SELECT p.id, p.name, p.category, p.barcode, p.default_price, p.safety_stock, p.unit,
                COALESCE(s.current_stock, 0), COALESCE(s.avg_cost, 0), COALESCE(s.stock_value, 0),
                p.is_active, p.remark
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         WHERE p.id = ?1",
        [product_id],
        map_product,
    )
    .map_err(Into::into)
}

pub fn customer_by_id(conn: &Connection, customer_id: i64) -> anyhow::Result<CustomerDto> {
    conn.query_row(
        "SELECT id, region, name, address, phone, is_active, remark
         FROM customers WHERE id = ?1",
        [customer_id],
        |row| {
            Ok(CustomerDto {
                id: row.get(0)?,
                region: row.get(1)?,
                name: row.get(2)?,
                address: row.get(3)?,
                phone: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
                remark: row.get(6)?,
            })
        },
    )
    .map_err(Into::into)
}

pub fn map_product(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductDto> {
    Ok(ProductDto {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        barcode: row.get(3)?,
        default_price: row.get(4)?,
        safety_stock: row.get(5)?,
        unit: row.get(6)?,
        current_stock: row.get(7)?,
        avg_cost: row.get(8)?,
        stock_value: row.get(9)?,
        is_active: row.get::<_, i64>(10)? == 1,
        remark: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn memory_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn ensure_guest_customer_creates_active_fixed_customer() {
        let conn = memory_conn();

        let id = ensure_guest_customer(&conn).unwrap();
        let customer = customer_by_id(&conn, id).unwrap();

        assert_eq!(customer.name, GUEST_CUSTOMER_NAME);
        assert_eq!(customer.region.as_deref(), Some(GUEST_CUSTOMER_REGION));
        assert!(customer.is_active);
        assert!(is_guest_customer(&conn, id).unwrap());
    }

    #[test]
    fn ensure_guest_customer_reactivates_existing_guest_customer() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, remark, created_at, updated_at)
             VALUES ('', ?1, 0, '手动停用', ?2, ?2)",
            params![GUEST_CUSTOMER_NAME, now],
        )
        .unwrap();
        let existing_id = conn.last_insert_rowid();

        let id = ensure_guest_customer(&conn).unwrap();
        let customer = customer_by_id(&conn, id).unwrap();

        assert_eq!(id, existing_id);
        assert_eq!(customer.name, GUEST_CUSTOMER_NAME);
        assert_eq!(customer.region.as_deref(), Some(GUEST_CUSTOMER_REGION));
        assert!(customer.is_active);
    }

    #[test]
    fn recalc_stock_balance_uses_weighted_average_and_outbound_quantity() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('均价商品', '库存', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES
             ('2026-06-01', 1, 'inbound', 10, 5, 50, 'test', '第一批入库', ?1),
             ('2026-06-02', 1, 'inbound', 20, 8, 160, 'test', '第二批入库', ?1),
             ('2026-06-03', 1, 'outbound', 6, 10, 60, 'test', '销售出库', ?1),
             ('2026-06-03', 1, 'gift_outbound', 2, 0, 0, 'test', '赠品出库', ?1),
             ('2026-06-04', 1, 'stocktake_adjustment', -1, 0, 0, 'test', '盘点调整', ?1)",
            [&now],
        )
        .unwrap();

        let (current_stock, avg_cost) = recalc_stock_balance(&conn, 1).unwrap();
        let product = product_by_id(&conn, 1).unwrap();

        assert_eq!(current_stock, 21.0);
        assert_eq!(avg_cost, 7.0);
        assert_eq!(product.current_stock, 21.0);
        assert_eq!(product.avg_cost, 7.0);
        assert_eq!(product.stock_value, 147.0);
    }

    #[test]
    fn high_volume_product_and_customer_queries_stay_under_two_seconds() {
        let mut conn = memory_conn();
        let tx = conn.transaction().unwrap();
        let now = now_text();
        for index in 0..10_000 {
            tx.execute(
                "INSERT INTO products
                 (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 10, 0, 1, ?4, ?4)",
                params![
                    format!("商品{index:05}"),
                    format!("类别{}", index % 20),
                    format!("BC{index:05}"),
                    now
                ],
            )
            .unwrap();
            let product_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
                 VALUES (?1, 10, 5, 50, ?2)",
                params![product_id, now],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO customers (region, name, address, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 1, ?4, ?4)",
                params![
                    format!("地区{}", index % 30),
                    format!("客户{index:05}"),
                    format!("地址{index:05}"),
                    now
                ],
            )
            .unwrap();
        }
        tx.commit().unwrap();

        let started = Instant::now();
        let product_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products p
                 LEFT JOIN stock_balances s ON s.product_id = p.id
                 WHERE p.is_active = 1 AND p.category = '类别1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let product_elapsed = started.elapsed();

        let started = Instant::now();
        let customer_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM customers
                 WHERE is_active = 1 AND region = '地区1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let customer_elapsed = started.elapsed();

        assert_eq!(product_count, 500);
        assert_eq!(customer_count, 334);
        assert!(
            product_elapsed < Duration::from_secs(2),
            "商品万级查询耗时 {:?}",
            product_elapsed
        );
        assert!(
            customer_elapsed < Duration::from_secs(2),
            "客户万级查询耗时 {:?}",
            customer_elapsed
        );
    }
}
