mod catalog_commands;
mod import_backup_commands;
mod inventory_commands;
mod order_commands;
mod report_document_commands;
mod rule_account_commands;
mod settings_generalization_commands;
mod system_commands;

pub use catalog_commands::*;
pub use import_backup_commands::*;
pub use inventory_commands::*;
pub use order_commands::*;
pub use report_document_commands::*;
pub use rule_account_commands::*;
pub use settings_generalization_commands::*;
pub use system_commands::*;

use crate::logger;
use crate::models::ApiResponse;

pub(crate) fn ok<T: serde::Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse::ok(data)
}

pub(crate) fn fail<T: serde::Serialize>(err: anyhow::Error) -> ApiResponse<T> {
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
#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::*;
    use crate::services::{
        customer_account_service, customer_rule_service, customer_service, inventory_service,
        product_service, supplier_service,
    };
    use crate::utils::now_text;
    use rusqlite::params;
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

        let customer = customer_service::create_customer(
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

        let customer = customer_service::create_customer(
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

        let result = customer_service::update_customer(
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

        let suppliers = supplier_service::list_suppliers(
            &conn,
            Some(ListSuppliersRequest {
                keyword: Some("供应商A".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();
        let supplier = suppliers.first().unwrap();
        assert!(supplier.is_active);

        supplier_service::disable_supplier(&conn, supplier_id).unwrap();
        let disabled = supplier_service::list_suppliers(
            &conn,
            Some(ListSuppliersRequest {
                keyword: Some("供应商A".to_string()),
                is_active: Some(false),
            }),
        )
        .unwrap();
        let disabled = disabled.first().unwrap();
        assert!(!disabled.is_active);
    }

    #[test]
    fn product_service_defaults_to_active_products_and_uses_text_filters() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES
             ('启用商品', '普通分类', 'ACTIVE001', 1, 0, 1, ?1, ?1),
             ('停用商品', '普通分类', 'DISABLED001', 2, 0, 0, ?1, ?1),
             ('带引号商品', '特殊''分类', 'QUOTE001', 3, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
             VALUES (1, 0, 0, 0, ?1), (2, 0, 0, 0, ?1), (3, 0, 0, 0, ?1)",
            [&now],
        )
        .unwrap();

        let defaults = crate::services::product_service::list_products(&conn, None).unwrap();
        let injected = crate::services::product_service::list_products(
            &conn,
            Some(ListProductsRequest {
                category: Some("特殊'分类".to_string()),
                keyword: Some("' OR 1=1 --".to_string()),
                only_low_stock: None,
                only_in_stock: None,
                is_active: Some(true),
            }),
        )
        .unwrap();
        let quoted = crate::services::product_service::list_products(
            &conn,
            Some(ListProductsRequest {
                category: Some("特殊'分类".to_string()),
                keyword: Some("带引号".to_string()),
                only_low_stock: None,
                only_in_stock: None,
                is_active: Some(true),
            }),
        )
        .unwrap();

        assert_eq!(defaults.len(), 2);
        assert!(defaults.iter().all(|product| product.is_active));
        assert!(injected.is_empty());
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].name, "带引号商品");
    }

    #[test]
    fn customer_service_defaults_to_active_customers_and_keeps_guest_first() {
        let conn = memory_conn();
        db::ensure_guest_customer(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, address, is_active, created_at, updated_at)
             VALUES
             ('普通地区', '普通客户', '地址A', 1, ?1, ?1),
             ('普通地区', '停用客户', '地址B', 0, ?1, ?1),
             ('特殊''地区', '带引号客户', '地址C', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let defaults = crate::services::customer_service::list_customers(&conn, None).unwrap();
        let injected = crate::services::customer_service::list_customers(
            &conn,
            Some(ListCustomersRequest {
                region: Some("特殊'地区".to_string()),
                keyword: Some("' OR 1=1 --".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();
        let quoted = crate::services::customer_service::list_customers(
            &conn,
            Some(ListCustomersRequest {
                region: Some("特殊'地区".to_string()),
                keyword: Some("带引号".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();

        assert_eq!(defaults[0].name, db::GUEST_CUSTOMER_NAME);
        assert!(defaults.iter().all(|customer| customer.is_active));
        assert!(injected.is_empty());
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].name, "带引号客户");
    }

    #[test]
    fn supplier_service_defaults_to_active_suppliers_and_uses_text_filters() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO suppliers (name, contact, phone, address, is_active, remark, created_at, updated_at)
             VALUES
             ('启用供应商', '联系人A', '1001', '地址A', 1, '备注A', ?1, ?1),
             ('停用供应商', '联系人B', '1002', '地址B', 0, '备注B', ?1, ?1),
             ('带引号供应商', '特殊''联系人', '1003', '地址C', 1, '备注C', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let defaults = crate::services::supplier_service::list_suppliers(&conn, None).unwrap();
        let injected = crate::services::supplier_service::list_suppliers(
            &conn,
            Some(ListSuppliersRequest {
                keyword: Some("' OR 1=1 --".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();
        let quoted = crate::services::supplier_service::list_suppliers(
            &conn,
            Some(ListSuppliersRequest {
                keyword: Some("特殊'联系人".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();

        assert_eq!(defaults.len(), 2);
        assert!(defaults.iter().all(|supplier| supplier.is_active));
        assert!(injected.is_empty());
        assert_eq!(quoted.len(), 1);
        assert_eq!(quoted[0].name, "带引号供应商");
    }

    #[test]
    fn list_product_and_customer_filters_treat_sql_fragments_as_text() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES
             ('安全商品', '普通分类', 'SAFE001', 1, 0, 1, ?1, ?1),
             ('带引号商品', '特殊''分类', 'QUOTE001', 2, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
             VALUES (1, 0, 0, 0, ?1), (2, 0, 0, 0, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (region, name, address, is_active, created_at, updated_at)
             VALUES
             ('普通地区', '安全客户', '地址A', 1, ?1, ?1),
             ('特殊''地区', '带引号客户', '地址B', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let injected_products = product_service::list_products(
            &conn,
            Some(ListProductsRequest {
                category: Some("特殊'分类".to_string()),
                keyword: Some("' OR 1=1 --".to_string()),
                only_low_stock: None,
                only_in_stock: None,
                is_active: Some(true),
            }),
        )
        .unwrap();
        let quoted_products = product_service::list_products(
            &conn,
            Some(ListProductsRequest {
                category: Some("特殊'分类".to_string()),
                keyword: Some("带引号".to_string()),
                only_low_stock: None,
                only_in_stock: None,
                is_active: Some(true),
            }),
        )
        .unwrap();
        let injected_customers = customer_service::list_customers(
            &conn,
            Some(ListCustomersRequest {
                region: Some("特殊'地区".to_string()),
                keyword: Some("' OR 1=1 --".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();
        let quoted_customers = customer_service::list_customers(
            &conn,
            Some(ListCustomersRequest {
                region: Some("特殊'地区".to_string()),
                keyword: Some("带引号".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();

        assert!(injected_products.is_empty());
        assert_eq!(quoted_products.len(), 1);
        assert_eq!(quoted_products[0].name, "带引号商品");
        assert!(injected_customers.is_empty());
        assert_eq!(quoted_customers.len(), 1);
        assert_eq!(quoted_customers[0].name, "带引号客户");
    }

    #[test]
    fn operational_filters_treat_sql_fragments_as_text() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES
             ('安全商品', '特殊''类别', 10, 0, 1, ?1, ?1),
             ('普通商品', '普通类别', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_adjustments
             (adjustment_date, product_id, product_name, category, adjustment_type, quantity_delta,
              unit_cost, amount, reason, status, created_at, updated_at)
             VALUES
             ('2026-06-01', 1, '安全商品', '特殊''类别', 'increase', 1, 10, 10, '测试', 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stocktake_records
             (stocktake_date, product_id, product_name, category, system_stock, actual_stock,
              difference_quantity, unit_cost, difference_amount, reason, status, created_at, updated_at)
             VALUES
             ('2026-06-02', 1, '安全商品', '特殊''类别', 1, 2, 1, 10, 10, '测试', 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_logs
             (log_time, module, action, target_type, target_id, target_label, result)
             VALUES
             ('2026-06-01 10:00:00', '库存''模块', '调整''动作', 'inventory', 1, '安全商品', 'success')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (region, name, address, is_active, created_at, updated_at)
             VALUES
             ('特殊''地区', '安全客户', '地址A', 1, ?1, ?1),
             ('普通地区', '普通客户', '地址B', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES ('SAFE-001', '2026-06-01', 1, '安全客户', 100, 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let injected_adjustments =
            crate::repositories::inventory_control_repository::list_inventory_adjustments(
                &conn,
                InventoryAdjustmentFilterRequest {
                    start_date: None,
                    end_date: None,
                    product_id: None,
                    category: Some("' OR 1=1 --".to_string()),
                    status: Some("normal' OR 1=1 --".to_string()),
                },
            )
            .unwrap();
        let quoted_adjustments =
            crate::repositories::inventory_control_repository::list_inventory_adjustments(
                &conn,
                InventoryAdjustmentFilterRequest {
                    start_date: Some("2026-06-01".to_string()),
                    end_date: Some("2026-06-01".to_string()),
                    product_id: Some(1),
                    category: Some("特殊'类别".to_string()),
                    status: Some("normal".to_string()),
                },
            )
            .unwrap();
        let injected_stocktakes =
            crate::repositories::inventory_control_repository::list_stocktakes(
                &conn,
                StocktakeFilterRequest {
                    start_date: None,
                    end_date: None,
                    product_id: None,
                    category: Some("' OR 1=1 --".to_string()),
                    status: Some("normal' OR 1=1 --".to_string()),
                },
            )
            .unwrap();
        let quoted_stocktakes = crate::repositories::inventory_control_repository::list_stocktakes(
            &conn,
            StocktakeFilterRequest {
                start_date: Some("2026-06-02".to_string()),
                end_date: Some("2026-06-02".to_string()),
                product_id: Some(1),
                category: Some("特殊'类别".to_string()),
                status: Some("normal".to_string()),
            },
        )
        .unwrap();
        let injected_logs = crate::services::audit_service::list_audit_logs(
            &conn,
            Some(AuditLogFilterRequest {
                module: Some("' OR 1=1 --".to_string()),
                action: Some("调整' OR 1=1 --".to_string()),
                start_date: None,
                end_date: None,
            }),
        )
        .unwrap();
        let quoted_logs = crate::services::audit_service::list_audit_logs(
            &conn,
            Some(AuditLogFilterRequest {
                module: Some("库存'模块".to_string()),
                action: Some("调整'动作".to_string()),
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-01".to_string()),
            }),
        )
        .unwrap();
        let injected_balances = customer_account_service::list_customer_balances(
            &conn,
            Some(CustomerBalanceFilterRequest {
                region: Some("' OR 1=1 --".to_string()),
                keyword: Some("安全' OR 1=1 --".to_string()),
                only_unpaid: Some(false),
            }),
        )
        .unwrap();
        let quoted_balances = customer_account_service::list_customer_balances(
            &conn,
            Some(CustomerBalanceFilterRequest {
                region: Some("特殊'地区".to_string()),
                keyword: Some("安全".to_string()),
                only_unpaid: Some(true),
            }),
        )
        .unwrap();

        assert!(injected_adjustments.is_empty());
        assert_eq!(quoted_adjustments.len(), 1);
        assert!(injected_stocktakes.is_empty());
        assert_eq!(quoted_stocktakes.len(), 1);
        assert!(injected_logs.is_empty());
        assert_eq!(quoted_logs.len(), 1);
        assert!(injected_balances.is_empty());
        assert_eq!(quoted_balances.len(), 1);
        assert_eq!(
            crate::repositories::customer_rule_repository::lookup_import_product_id(
                &conn,
                "安全商品",
                Some("特殊'类别")
            )
            .unwrap(),
            1
        );
        assert!(
            crate::repositories::customer_rule_repository::lookup_import_product_id(
                &conn,
                "安全商品",
                Some("' OR 1=1 --")
            )
            .is_err()
        );
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

        let result = product_service::batch_update_products(
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

        let customers = customer_service::batch_update_customers(
            &conn,
            BatchUpdateCustomersRequest {
                ids: vec![1, 2],
                region: Some("新区".to_string()),
                remark: Some("新备注".to_string()),
                is_active: Some(false),
            },
        )
        .unwrap();
        let suppliers = supplier_service::batch_update_suppliers(
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

        let result = customer_account_service::create_payment(
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

        let result = inventory_service::create_inbound(
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
    fn inventory_service_creates_inbound_and_lists_records_with_filters() {
        let mut conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('入库商品', '入库''分类', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (name, is_active, created_at, updated_at)
             VALUES ('入库供应商', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();

        let response = crate::services::inventory_service::create_inbound(
            &mut conn,
            CreateInboundRequest {
                inbound_date: "2026-06-01".to_string(),
                product_id: 1,
                supplier_id: Some(1),
                quantity: 5.0,
                unit_cost: 3.0,
                remark: Some("首批".to_string()),
            },
        )
        .unwrap();
        let injected = crate::services::inventory_service::list_inbound_records(
            &conn,
            Some(ListInboundRecordsRequest {
                start_date: None,
                end_date: None,
                product_id: None,
                category: Some("' OR 1=1 --".to_string()),
            }),
        )
        .unwrap();
        let records = crate::services::inventory_service::list_inbound_records(
            &conn,
            Some(ListInboundRecordsRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-01".to_string()),
                product_id: Some(1),
                category: Some("入库'分类".to_string()),
            }),
        )
        .unwrap();

        assert_eq!(response.current_stock, 5.0);
        assert_eq!(response.avg_cost, 3.0);
        assert!(injected.is_empty());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].supplier_name.as_deref(), Some("入库供应商"));
        assert_eq!(records[0].amount, 15.0);
    }

    #[test]
    fn customer_account_service_tracks_payments_and_balances() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('账款''地区', '账款客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES ('PAY-001', '2026-06-01', 1, '账款客户', 120, 'normal', ?1, ?1)",
            [&now],
        )
        .unwrap();

        let payment = crate::services::customer_account_service::create_payment(
            &conn,
            CreatePaymentRequest {
                payment_date: "2026-06-02".to_string(),
                customer_id: 1,
                amount: 70.0,
                method: Some("现金".to_string()),
                related_order_id: Some(1),
                remark: Some("首款".to_string()),
            },
        )
        .unwrap();
        let payments = crate::services::customer_account_service::list_payment_records(
            &conn,
            Some(PaymentFilterRequest {
                customer_id: Some(1),
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                status: Some("normal".to_string()),
            }),
        )
        .unwrap();
        let balances = crate::services::customer_account_service::list_customer_balances(
            &conn,
            Some(CustomerBalanceFilterRequest {
                region: Some("账款'地区".to_string()),
                keyword: Some("账款客户".to_string()),
                only_unpaid: Some(true),
            }),
        )
        .unwrap();
        crate::services::customer_account_service::void_payment(&conn, payment.id).unwrap();
        let balances_after_void =
            crate::services::customer_account_service::list_customer_balances(
                &conn,
                Some(CustomerBalanceFilterRequest {
                    region: Some("账款'地区".to_string()),
                    keyword: Some("账款客户".to_string()),
                    only_unpaid: Some(true),
                }),
            )
            .unwrap();

        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].amount, 70.0);
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].balance, 50.0);
        assert_eq!(balances_after_void[0].balance, 120.0);
    }

    #[test]
    fn inventory_control_service_adjustment_lists_and_voids_with_audit() {
        let mut conn = memory_conn();
        seed_adjustment_product(&conn);

        let adjustment = crate::services::inventory_control_service::create_inventory_adjustment(
            &mut conn,
            CreateInventoryAdjustmentRequest {
                adjustment_date: "2026-06-02".to_string(),
                product_id: 1,
                adjustment_type: "loss".to_string(),
                quantity_delta: -2.0,
                reason: "破损".to_string(),
                remark: Some("测试".to_string()),
            },
        )
        .unwrap();
        let listed = crate::services::inventory_control_service::list_inventory_adjustments(
            &conn,
            InventoryAdjustmentFilterRequest {
                start_date: Some("2026-06-02".to_string()),
                end_date: Some("2026-06-02".to_string()),
                product_id: Some(1),
                category: Some("盘点".to_string()),
                status: Some("normal".to_string()),
            },
        )
        .unwrap();
        let voided = crate::services::inventory_control_service::void_inventory_adjustment(
            &mut conn,
            adjustment.id,
            Some("录错".to_string()),
        )
        .unwrap();
        let (stock, _avg_cost): (f64, f64) = conn
            .query_row(
                "SELECT current_stock, avg_cost FROM stock_balances WHERE product_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE module = 'inventory'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(adjustment.quantity_delta, -2.0);
        assert_eq!(listed.len(), 1);
        assert_eq!(voided.status, "voided");
        assert_eq!(stock, 10.0);
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn inventory_adjustment_updates_stock_and_void_reverses_it() {
        let mut conn = memory_conn();
        seed_adjustment_product(&conn);

        let adjustment = crate::services::inventory_control_service::create_inventory_adjustment(
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

        crate::services::inventory_control_service::void_inventory_adjustment(
            &mut conn,
            adjustment.id,
            Some("录入错误".to_string()),
        )
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

        let stocktake = crate::services::inventory_control_service::create_stocktake(
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

        crate::services::inventory_control_service::void_stocktake(
            &mut conn,
            stocktake.id,
            Some("复盘错误".to_string()),
        )
        .unwrap();
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

        let first_id = crate::services::customer_rule_service::save_customer_product_rule(
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
        let second_id = crate::services::customer_rule_service::save_customer_product_rule(
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
        let draft_id = crate::services::customer_rule_service::save_customer_product_rule(
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

        crate::services::customer_rule_service::disable_customer_product_rule(&conn, second_id)
            .unwrap();
        crate::services::customer_rule_service::delete_customer_product_rule(&conn, draft_id)
            .unwrap();

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
    fn customer_rule_service_lists_rules_with_text_filters() {
        let conn = memory_conn();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('规则', '重点客户', 1, ?1, ?1),
                    ('规则', '普通客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('规则商品A', '规则类', 10, 0, 1, ?1, ?1),
                    ('规则商品B', '其他类', 10, 0, 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        customer_rule_service::save_customer_product_rule(
            &conn,
            SaveCustomerProductRuleRequest {
                id: None,
                customer_id: 1,
                product_id: 1,
                fixed_price: Some(8.5),
                threshold_quantity: None,
                gift_product_id: None,
                gift_quantity: None,
                direct_discount_amount: None,
                monthly_credit_amount: None,
                credit_category: None,
                is_active: true,
                remark: None,
            },
        )
        .unwrap();
        customer_rule_service::save_customer_product_rule(
            &conn,
            SaveCustomerProductRuleRequest {
                id: None,
                customer_id: 2,
                product_id: 2,
                fixed_price: None,
                threshold_quantity: None,
                gift_product_id: None,
                gift_quantity: None,
                direct_discount_amount: Some(1.0),
                monthly_credit_amount: None,
                credit_category: None,
                is_active: true,
                remark: None,
            },
        )
        .unwrap();

        let filtered = customer_rule_service::list_customer_product_rules(
            &conn,
            Some(RuleFilterRequest {
                customer_id: Some(1),
                product_id: None,
                category: Some("规则类".to_string()),
                keyword: Some("重点".to_string()),
                is_active: Some(true),
                rule_type: Some("fixed".to_string()),
            }),
        )
        .unwrap();
        let injected = customer_rule_service::list_customer_product_rules(
            &conn,
            Some(RuleFilterRequest {
                customer_id: None,
                product_id: None,
                category: Some("' OR 1=1 --".to_string()),
                keyword: Some("重点' OR 1=1 --".to_string()),
                is_active: Some(true),
                rule_type: Some("fixed".to_string()),
            }),
        )
        .unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].customer_name, "重点客户");
        assert_eq!(filtered[0].product_name, "规则商品A");
        assert!(injected.is_empty());
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
        let old_rule_id = customer_rule_service::save_customer_product_rule(
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

        let preview = customer_rule_service::preview_customer_product_rule_import(
            &conn,
            path.to_str().unwrap(),
        )
        .unwrap();
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

        let result =
            customer_rule_service::import_customer_product_rules(&conn, path.to_str().unwrap())
                .unwrap();
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

        crate::services::settings_service::save_settings(
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

        let result =
            crate::services::diagnostics_service::run_data_self_check(&conn, |_| false).unwrap();
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
                    customer_account_service::create_payment(
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
        let balances = customer_account_service::list_customer_balances(
            &conn,
            Some(CustomerBalanceFilterRequest {
                region: None,
                keyword: None,
                only_unpaid: Some(true),
            }),
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

        let product = product_service::find_product_by_barcode(&conn, "SCAN001")
            .unwrap()
            .unwrap();
        assert_eq!(product.name, "扫码商品");
        assert!(product.is_active);
        assert!(product_service::find_product_by_barcode(&conn, "MISS")
            .unwrap()
            .is_none());
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

        let rows = customer_account_service::list_customer_balances(
            &conn,
            Some(CustomerBalanceFilterRequest {
                region: Some("欠款".to_string()),
                keyword: Some("客户A".to_string()),
                only_unpaid: Some(true),
            }),
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

        let suppliers = supplier_service::list_suppliers(
            &conn,
            Some(ListSuppliersRequest {
                keyword: Some("供应商A".to_string()),
                is_active: Some(true),
            }),
        )
        .unwrap();
        let supplier = suppliers.first().unwrap();
        assert_eq!(supplier.name, "供应商A");
        assert_eq!(supplier.contact.as_deref(), Some("张三"));
        assert_eq!(supplier.phone.as_deref(), Some("13800000000"));
        assert!(supplier.is_active);
    }
}
