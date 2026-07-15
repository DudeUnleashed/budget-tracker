mod commands;
mod db;
mod models;
mod occurrences;

use commands::accounts::{
    create_account, get_account_balance, list_accounts, set_default_account, update_account,
};
use commands::budgets::{create_budget, delete_budget, get_budget_progress, list_budgets, update_budget};
use commands::dashboard::get_month_summary;
use commands::data::{export_data, import_data};
use commands::subscriptions::{
    cancel_subscription, create_subscription, list_subscriptions, pause_subscription,
    reactivate_subscription, update_occurrence, update_subscription, update_subscription_amount,
};
use commands::tags::{create_tag, list_tags, update_tag};
use commands::transactions::{
    create_cross_ledger_movement, create_transaction, create_transfer, delete_transaction,
    list_transactions_for_month, search_transactions, update_transaction,
};
use db::Db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let conn = db::init(&app_data_dir);
            app.manage(Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_account,
            list_accounts,
            update_account,
            set_default_account,
            get_account_balance,
            create_tag,
            list_tags,
            update_tag,
            create_transaction,
            create_transfer,
            create_cross_ledger_movement,
            update_transaction,
            delete_transaction,
            list_transactions_for_month,
            search_transactions,
            create_subscription,
            list_subscriptions,
            update_subscription,
            update_subscription_amount,
            pause_subscription,
            cancel_subscription,
            reactivate_subscription,
            update_occurrence,
            create_budget,
            list_budgets,
            update_budget,
            delete_budget,
            get_budget_progress,
            get_month_summary,
            export_data,
            import_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
