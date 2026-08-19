#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::*;
    use crate::utils::now_text;
    use rusqlite::{params, Connection};
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
        let quote = crate::services::order_service::preview_quote(
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

        let response = crate::services::order_service::save_order(
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

        crate::services::order_service::void_order(
            &mut conn,
            response.order_id,
            Some("测试作废".to_string()),
        )
        .unwrap();

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

        let rows = crate::services::order_service::list_orders(
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
    fn list_orders_and_credits_treat_sql_fragments_as_text() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('安全', '安全客户', 1, ?1, ?1)",
            [&now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES
             ('SAFE-001', '2026-06-01', 1, '安全客户', 10, 'normal', ?1, ?1),
             ('QUOTE-001', '2026-06-02', 1, '安全客户', 20, 'voided', ?1, ?1)",
            [&now],
        )
        .unwrap();
        let available_month = crate::utils::next_month(&now);
        conn.execute(
            "INSERT INTO monthly_credits
             (source_order_id, source_order_no, customer_id, category, amount, used_amount, remaining_amount,
              generated_date, available_month, status, created_at, updated_at)
             VALUES
             (1, 'SAFE-001', 1, '特殊''类别', 30, 0, 30, '2026-06-01', ?2, 'pending', ?1, ?1)",
            params![&now, &available_month],
        )
        .unwrap();

        let injected_orders = crate::services::order_service::list_orders(
            &conn,
            ListOrdersRequest {
                start_date: None,
                end_date: None,
                customer_id: None,
                order_no: Some("' OR 1=1 --".to_string()),
                status: Some("normal' OR 1=1 --".to_string()),
            },
        )
        .unwrap();
        let quoted_orders = crate::services::order_service::list_orders(
            &conn,
            ListOrdersRequest {
                start_date: None,
                end_date: None,
                customer_id: None,
                order_no: Some("SAFE".to_string()),
                status: Some("normal".to_string()),
            },
        )
        .unwrap();
        let injected_credits = crate::services::order_service::list_monthly_credits(
            &conn,
            MonthlyCreditFilterRequest {
                customer_id: Some(1),
                category: Some("' OR 1=1 --".to_string()),
                status: Some("pending' OR 1=1 --".to_string()),
                start_date: None,
                end_date: None,
                available_month: None,
            },
        )
        .unwrap();
        let quoted_credits = crate::services::order_service::list_monthly_credits(
            &conn,
            MonthlyCreditFilterRequest {
                customer_id: Some(1),
                category: Some("特殊'类别".to_string()),
                status: Some("pending".to_string()),
                start_date: None,
                end_date: None,
                available_month: Some(available_month),
            },
        )
        .unwrap();

        assert!(injected_orders.is_empty());
        assert_eq!(quoted_orders.len(), 1);
        assert_eq!(quoted_orders[0].order_no, "SAFE-001");
        assert!(injected_credits.is_empty());
        assert_eq!(quoted_credits.len(), 1);
        assert_eq!(quoted_credits[0].category, "特殊'类别");
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

        let available = crate::services::order_service::available_monthly_credits(
            &conn,
            1,
            "饮料".to_string(),
            "2026-06-15".to_string(),
        )
        .unwrap();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].remaining_amount, 30.0);

        crate::services::order_service::close_or_void_credit(&conn, 1, "closed").unwrap();
        crate::services::order_service::close_or_void_credit(&conn, 2, "voided").unwrap();

        let closed = crate::services::order_service::list_monthly_credits(
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
        let voided = crate::services::order_service::list_monthly_credits(
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
                    crate::services::order_service::save_order(
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
        let response = crate::services::order_service::save_order(&mut conn, request).unwrap();
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
        let rows = crate::services::order_service::list_orders(
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
