use crate::utils::{ensure_dir, now_text};
use crate::{db, logger};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<StateInner>,
}

pub struct StateInner {
    pub app_dir: PathBuf,
    pub data_dir: PathBuf,
    pub backups_dir: PathBuf,
    pub orders_dir: PathBuf,
    pub exports_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub config_dir: PathBuf,
    pub db_path: PathBuf,
    pub import_result: Mutex<Option<crate::models::ImportResult>>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> anyhow::Result<Self> {
        let base = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let data_dir = base.join("data");
        let backups_dir = base.join("backups");
        let orders_dir = base.join("orders");
        let exports_dir = base.join("exports");
        let logs_dir = base.join("logs");
        let config_dir = base.join("config");
        let db_path = data_dir.join("inventory.db");

        Ok(Self {
            inner: Arc::new(StateInner {
                app_dir: base,
                data_dir,
                backups_dir,
                orders_dir,
                exports_dir,
                logs_dir,
                config_dir,
                db_path,
                import_result: Mutex::new(None),
            }),
        })
    }

    pub fn ensure_ready(&self) -> anyhow::Result<()> {
        ensure_dir(&self.inner.app_dir)?;
        ensure_dir(&self.inner.data_dir)?;
        ensure_dir(&self.inner.backups_dir)?;
        ensure_dir(&self.inner.orders_dir)?;
        ensure_dir(&self.inner.exports_dir)?;
        ensure_dir(&self.inner.logs_dir)?;
        ensure_dir(&self.inner.config_dir)?;
        logger::init(&self.inner.logs_dir);
        logger::info(
            "app",
            format!(
                "启动应用，数据库路径：{}",
                self.inner.db_path.to_string_lossy()
            ),
        );

        let conn = self.connection()?;
        db::init_schema(&conn)?;
        db::seed_settings(&conn)?;
        db::ensure_guest_customer(&conn)?;
        db::backup_on_startup_if_needed(self)?;
        logger::info("app", "应用初始化完成");
        Ok(())
    }

    pub fn connection(&self) -> anyhow::Result<Connection> {
        db::open_database_connection(&self.inner.db_path)
    }

    pub fn db_path(&self) -> PathBuf {
        self.inner.db_path.clone()
    }

    pub fn data_dir(&self) -> PathBuf {
        self.inner.data_dir.clone()
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.inner.backups_dir.clone()
    }

    pub fn orders_dir(&self) -> PathBuf {
        self.inner.orders_dir.clone()
    }

    pub fn exports_dir(&self) -> PathBuf {
        self.inner.exports_dir.clone()
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.inner.logs_dir.clone()
    }

    pub fn set_import_result(&self, result: crate::models::ImportResult) {
        if let Ok(mut guard) = self.inner.import_result.lock() {
            *guard = Some(result);
        }
    }

    pub fn import_result(&self) -> Option<crate::models::ImportResult> {
        self.inner
            .import_result
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub fn app_status(&self) -> crate::models::AppStatusDto {
        crate::models::AppStatusDto {
            database_path: self.db_path().to_string_lossy().to_string(),
            data_dir: self.data_dir().to_string_lossy().to_string(),
            backups_dir: self.backups_dir().to_string_lossy().to_string(),
            orders_dir: self.orders_dir().to_string_lossy().to_string(),
            exports_dir: self.exports_dir().to_string_lossy().to_string(),
            logs_dir: self.logs_dir().to_string_lossy().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn log_backup(
        &self,
        backup_path: &str,
        backup_type: &str,
        status: &str,
        message: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO backup_logs (backup_path, backup_type, status, message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (backup_path, backup_type, status, message, now_text()),
        )?;
        logger::info("backup", format!("{backup_type} {status}: {backup_path}"));
        Ok(())
    }
}

#[cfg(test)]
impl AppState {
    pub fn new_for_test(base: PathBuf) -> Self {
        let data_dir = base.join("data");
        let backups_dir = base.join("backups");
        let orders_dir = base.join("orders");
        let exports_dir = base.join("exports");
        let logs_dir = base.join("logs");
        let config_dir = base.join("config");
        let db_path = data_dir.join("inventory.db");

        Self {
            inner: Arc::new(StateInner {
                app_dir: base,
                data_dir,
                backups_dir,
                orders_dir,
                exports_dir,
                logs_dir,
                config_dir,
                db_path,
                import_result: Mutex::new(None),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateInboundRequest;
    use crate::services::inventory_service;
    use std::sync::{Arc, Barrier};

    fn ready_test_state() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new_for_test(dir.keep());
        state.ensure_ready().unwrap();
        state
    }

    #[test]
    fn runtime_connection_uses_busy_timeout() {
        let state = ready_test_state();
        let conn = state.connection().unwrap();
        let busy_timeout_ms: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();

        assert_eq!(busy_timeout_ms, 10_000);
    }

    #[test]
    fn runtime_connections_handle_concurrent_writes() {
        let state = ready_test_state();
        let conn = state.connection().unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('运行时并发商品', '压力', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        drop(conn);

        let worker_count = 8;
        let barrier = Arc::new(Barrier::new(worker_count));
        let handles = (0..worker_count)
            .map(|_| {
                let state = state.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let mut conn = state.connection().unwrap();
                    barrier.wait();
                    inventory_service::create_inbound(
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

        let conn = state.connection().unwrap();
        let inbound_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM inbound_records", [], |row| row.get(0))
            .unwrap();

        assert_eq!(inbound_count, worker_count as i64);
    }
}
