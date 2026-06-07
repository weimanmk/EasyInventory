mod app;
mod commands;
mod db;
mod domain;
mod excel;
mod generalization;
mod logger;
mod models;
mod orders;
mod reports;
mod repositories;
mod services;
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
            batch_update_products,
            find_product_by_barcode,
            list_customers,
            create_customer,
            update_customer,
            disable_customer,
            batch_update_customers,
            list_suppliers,
            create_supplier,
            update_supplier,
            disable_supplier,
            batch_update_suppliers,
            create_inbound,
            list_inbound_records,
            preview_quote,
            save_order,
            get_order,
            list_orders,
            export_order_document,
            export_order_pdf_document,
            print_order_document,
            print_order_document_with_options,
            void_order,
            list_customer_product_rules,
            save_customer_product_rule,
            disable_customer_product_rule,
            delete_customer_product_rule,
            preview_customer_product_rule_import,
            import_customer_product_rules,
            list_monthly_credits,
            get_available_monthly_credits,
            close_monthly_credit,
            void_monthly_credit,
            list_customer_balances,
            list_payment_records,
            create_payment,
            void_payment,
            get_customer_statement,
            export_customer_statement_pdf,
            get_daily_profit_summary,
            get_profit_analytics,
            list_profit_records,
            list_inventory_report,
            get_product_ranking,
            get_customer_analysis,
            get_supplier_purchase_ledger,
            list_documents,
            open_document,
            export_document,
            export_document_pdf,
            print_document,
            export_data,
            open_exports_folder,
            open_logs_folder,
            run_data_self_check,
            export_data_self_check,
            get_diagnostic_summary,
            export_diagnostic_package,
            import_excel,
            get_import_status,
            create_backup,
            list_backups,
            open_backup_folder,
            restore_backup,
            create_inventory_adjustment,
            list_inventory_adjustments,
            void_inventory_adjustment,
            create_stocktake,
            list_stocktakes,
            void_stocktake,
            list_audit_logs,
            list_settings,
            save_settings,
            get_setup_status,
            complete_setup,
            get_merchant_profile,
            save_merchant_profile,
            get_term_settings,
            save_term_settings,
            get_feature_flags,
            save_feature_flags,
            list_industry_templates,
            apply_industry_template,
            list_document_templates,
            apply_document_template,
            preview_generic_import,
            preview_generic_import_headers,
            confirm_generic_import,
            export_generic_import_report,
            download_import_template,
            save_import_mapping,
            list_import_mappings,
            list_printers,
            get_app_status
        ])
        .run(tauri::generate_context!())
        .expect("failed to run EasyInventory");
}
