mod app;
mod commands;
mod db;
mod excel;
mod logger;
mod models;
mod orders;
mod reports;
mod utils;

use app::AppState;
use commands::*;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::new(app.handle())?;
            state.ensure_ready()?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            write_client_log,
            list_products,
            create_product,
            update_product,
            disable_product,
            find_product_by_barcode,
            list_customers,
            create_customer,
            update_customer,
            disable_customer,
            list_suppliers,
            create_supplier,
            update_supplier,
            disable_supplier,
            create_inbound,
            list_inbound_records,
            preview_quote,
            save_order,
            get_order,
            list_orders,
            export_order_document,
            print_order_document,
            print_order_document_with_options,
            void_order,
            list_customer_product_rules,
            save_customer_product_rule,
            disable_customer_product_rule,
            delete_customer_product_rule,
            list_monthly_credits,
            get_available_monthly_credits,
            close_monthly_credit,
            void_monthly_credit,
            list_customer_balances,
            list_payment_records,
            create_payment,
            void_payment,
            get_daily_profit_summary,
            get_profit_analytics,
            list_profit_records,
            list_inventory_report,
            list_documents,
            open_document,
            export_document,
            print_document,
            export_data,
            open_exports_folder,
            open_logs_folder,
            import_excel,
            get_import_status,
            create_backup,
            list_backups,
            open_backup_folder,
            list_settings,
            save_settings,
            list_printers,
            get_app_status
        ])
        .run(tauri::generate_context!())
        .expect("failed to run EasyInventory");
}
